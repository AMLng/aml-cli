extern crate yaml_rust;
use yaml_rust::{yaml, YamlEmitter};
use linked_hash_map::LinkedHashMap;

use std::env;
use std::fs::File;
use std::io::prelude::*;

use serde::{Serialize, Deserialize};

use serde_json::json;

#[macro_use]
extern crate tera;
use tera::{Context, Tera, GlobalFn, Value};

fn print_indent(indent: usize) {
    for _ in 0..indent {
        print!("    ");
    }
}

fn dump_node(doc: &yaml::Yaml, indent: usize) {
    match *doc {
        yaml::Yaml::Array(ref v) => {
            for x in v {
                dump_node(x, indent + 1);
            }
        }
        yaml::Yaml::Hash(ref h) => {
            for (k, v) in h {
                print_indent(indent);
                dump_node(v, indent + 1);
            }
        }
        _ => {
            print_indent(indent);
            println!("{:?}", doc);
        }
    }
}

fn collect_child_resources(resource: &yaml::Yaml, base_url: &str, to: &mut Vec<yaml::Yaml>) {
    if let yaml::Yaml::Hash(ref h) = resource {
        let mut child_desc_map: LinkedHashMap<yaml::Yaml, yaml::Yaml> = LinkedHashMap::new();

        for (k, v) in h {
            if let yaml::Yaml::String(ref key) = k {
                if key.starts_with("/") {
                    println!("found subchild resource: {} base: {}", key, base_url);

                    let child_base_url = String::from(base_url) + key;
                    collect_child_resources(v, &child_base_url, to);
                } else {
                    child_desc_map.insert(k.clone(), v.clone());
                }
            }
        }

        child_desc_map.insert(yaml::Yaml::String(String::from("url")),
                              yaml::Yaml::String(String::from(base_url)));
        to.push(yaml::Yaml::Hash(child_desc_map));
    }
}

fn append_resource_tree(resource: &yaml::Yaml, resource_url: &yaml::Yaml, to: &mut Vec<yaml::Yaml>) {
    let mut resource_desc_map = match resource.clone() {
        yaml::Yaml::Hash(h) => h,
        _ => LinkedHashMap::new()
    };
    let mut keys_to_remove: Vec<yaml::Yaml> = Vec::new();
    let mut child_resources: Vec<yaml::Yaml> = Vec::new();

    for (k, v) in &resource_desc_map {
        if let yaml::Yaml::String(ref key) = k {
            if key.starts_with("/") {
                println!("found child resource: {}", key);
                keys_to_remove.push(k.clone());
                collect_child_resources(v, key, &mut child_resources);
            }
        }
    }

    for key in &mut keys_to_remove {
        resource_desc_map.remove(key);
    }

    resource_desc_map.insert(yaml::Yaml::String(String::from("child_resources")),
                             yaml::Yaml::Array(child_resources));
    resource_desc_map.insert(yaml::Yaml::String(String::from("url")), resource_url.clone());
    to.push(yaml::Yaml::Hash(resource_desc_map));
}

fn split_resources(full_aml: &yaml::Yaml, header_only: &mut LinkedHashMap<yaml::Yaml, yaml::Yaml>, resources: &mut Vec<yaml::Yaml>) {
    if let yaml::Yaml::Hash(ref h) = full_aml {
        for (k, v) in h {
            if let yaml::Yaml::String(key) = k {
                if key.starts_with("/") {
                    // found resource, append it and all of it childs to resources list
                    println!("found resource: {}", key);
                    append_resource_tree(v, k, resources);
                } else { // copy everything else to header_only map
                    header_only.insert(k.clone(), v.clone());
                }
            }
        }
    } else {
        println!("root must be a hash")
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum AmlAddr {
    StrAddr(String),
    NumAddr(u32)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct AmlInterface {
    name: String,
    endianness: Option<String>,
    addr: Option<Vec<AmlAddr>>,
    align: Option<String>,
    blocks: Option<String>
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct AmlHeader {
    name: String,
    version: String,
    description: String,
    datasheet: Option<String>,
    interface: Option<AmlInterface>
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct AmlResource {
    url: String,
    field: String
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Resource {
    name: String,
    readonly: bool,
    pod: bool,
    r#type: String
}

fn find_properties_in() -> GlobalFn {
    Box::new(move |args| -> tera::Result<Value> {
        let v = serde_json::to_value(&vec![
            Resource { name: String::from("led"), readonly: false, pod: true, r#type: String::from("quint8") } ]).unwrap();
        Ok(v)
    })
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() < 2 {
        println!("No args given");
        return;
    }
    let mut f = File::open(&args[1]).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    //println!("String len: {}", s.len());

    let docs = yaml::YamlLoader::load_from_str(&s).unwrap();

    //let mut header = yaml::Yaml::Hash(LinkedHashMap::new());
    let mut header_map: LinkedHashMap<yaml::Yaml, yaml::Yaml> = LinkedHashMap::new();
    //let mut resources = yaml::Yaml::Array(Vec::new());
    let mut resources_vec: Vec<yaml::Yaml> = Vec::new();
    let mut resources_map: LinkedHashMap<yaml::Yaml, yaml::Yaml> = LinkedHashMap::new();
    for doc in &docs {
        //dump_node(doc, 0);
        split_resources(doc, &mut header_map, &mut resources_vec);
    }
    let header_node = yaml::Yaml::Hash(header_map);
    resources_map.insert(yaml::Yaml::String(String::from("root_resources")),
                         yaml::Yaml::Array(resources_vec));
    let resources_node = yaml::Yaml::Hash(resources_map);


    let mut header_str = String::new();
    let mut resources_str = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut header_str);
        emitter.dump(&header_node).unwrap();
    }
    {
        let mut emitter = YamlEmitter::new(&mut resources_str);
        emitter.dump(&resources_node).unwrap();
    }
    //println!("header: {}", header_str);
    println!("resources: {}", resources_str);

    let mut tera = Tera::default();
    tera.register_function("find_properties_in", find_properties_in());
    let templates = vec![("/Volumes/red/Design/AML/qt/cpp-macros.jinja", Some("cpp-macros.jinja")),
                         ("/Volumes/red/Design/AML/qt/qt-header.hpp", Some("qt-header.hpp"))];

    match tera.add_template_files(templates) {
        Ok(t) => println!("Templates loaded"),
        Err(e) => {
            for ee in e.iter() {
                println!("Template load error: {}", ee)
            }

        }
    };
    let mut context = tera::Context::new();
    context.insert("className", &"FancyLED");
    context.insert("resources", &vec![
        Resource { name: String::from("led"), readonly: false, pod: true, r#type: String::from("quint8") } ]);

    let result = tera.render("qt-header.hpp", &context);

    if result.is_err() {
        for e in result.unwrap_err().iter() {
            println!("err: {}", e);
        }
    } else {
        println!("Rendered: {}", result.unwrap());
    }

//    match result {
//        Ok(s) => println!("rendere: {}", s),
//        Err(e) => {
//            for ee in e.unwrap_err().iter() {
//                println!("err: {}", ee);
//            }
//
//        }
//    }

    //let header_de: AmlHeader = serde_yaml::from_str(&header_str).unwrap();
    //println!("{:?}", header_de);

    /*let mut res = Vec::new();
    res.push(AmlResource { url: String::from("/u1"), field: String::from("f1") });
    res.push(AmlResource { url: String::from("/u2"), field: String::from("f2") });
    let res_ser = serde_yaml::to_string(&res).unwrap();
    println!("{}", res_ser);*/
}
