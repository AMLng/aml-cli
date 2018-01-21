# -*- coding: utf-8 -*-
import yaml
import jinja2
import pprint

import logging
import chromalog
from chromalog.mark.helpers.simple import success, error, important

chromalog.basicConfig(format="%(message)s", level=logging.DEBUG)
logger = logging.getLogger()

#logger.debug("This is a debug message")
#filename = r'/var/lib/status'

#logger.info("Booting up system: %s", success("OK"))
#logger.info("Booting up network: %s", error("FAIL"))
#logger.info("Reading file at %s: %s", important(filename), success("OK"))

pp = pprint.PrettyPrinter(indent = 4)

amlfile = 'usbled.aml'
try:
	logger.info("Loading: %s", important(amlfile))
	with open(amlfile) as f:
		aml = yaml.safe_load(f)
	logger.info("Parsing: %s", success("OK"))
except IOError as e:
	logger.error("IOError: %s", e.strerror)
except yaml.YAMLError, e:
	logger.error("YAMLError: %s", e)
	if hasattr(e, 'problem_mark'):
		mark = e.problem_mark
		logger.error("Error position: (%s:%s)", important(mark.line+1), important(mark.column+1))
#pp.pprint(aml)

def set_property(name):
	return 'd->msgPackProtocol->setProperty("' + str(name) + '", ' + name + ');'

def find_properties_in(resources):
	return list(r for r in resources if isinstance(r, Property))

templateLoader = jinja2.FileSystemLoader(searchpath = './')
templateEnvironment = jinja2.Environment(loader = templateLoader, trim_blocks = True, lstrip_blocks = True)

templateEnvironment.globals.update(set_property = set_property)
templateEnvironment.globals.update(find_properties_in = find_properties_in)

hppTemplate = templateEnvironment.get_template('qt-header.hpp')
hppPrivateTemplate = templateEnvironment.get_template('qt-private-header.hpp')
cppTemplate = templateEnvironment.get_template('qt-source.cpp')

hpp_bm_template = templateEnvironment.get_template('baremetal-header.hpp')

PROPERTY_TRAITS = ['writeable', 'persistent', 'observable', 'ratelimited']

def is_pod_type(type):
	if 'int' in type: # intX, uintX
		return True
	if type in ['bool', 'float', 'double']:
		return True
	return False

class Property:
	def __init__(self, name, definition):
		self.name = name
		self.description = definition.get('description', '')
		self.traits = []
		for trait in definition.get('is', []):
			if trait in PROPERTY_TRAITS:
				self.traits.append(trait)
			else:
				self.isValid = False
				self.errorString = 'Unkown trait ' + trait
				return
		self.readonly = 'writeable' not in self.traits
		if 'type' in definition:
			self.type = definition['type']
			self.pod = is_pod_type(self.type)
		else:
			self.isValid = False
			self.errorString = 'No type specified.'
			return

		self.isValid = True

	def __repr__(self):
		return '<Property ' + self.name + '>'

# Given url and position in the aml tree, construct more efficient representation or return as is
# For example convert string(url) to hash, or remove ambigious symbols from string(url) so that it still be unique
# Or create one or several numbers as in CoAP
def msgpack_protocol_minify_url(url, tree_position, resources):
    """ Create unique number from tree_position and total number of root resources
    @param url string or list of strings (ex.: '/led' or ['/acc', '/sample_rate']
    @param tree_position number of list of numbers (ex.: 0 or [0, 1])
    @param resources all the resources in aml
    @return number
    """


resources_definitions = {k:v for k,v in aml.items() if k[0] == '/'}
pp.pprint(resources_definitions)
tree_position = [0, 0]
resources = []
for url, definition in resources_definitions.items():
	if 'of' in definition:
		resource_type = definition['of']
		if resource_type == 'properties':
			p = Property(url[1:], definition)
			if p.isValid:
				# TODO: find previous minified_url for this resource from git tags and us it
				#p.minified_url = msgpack_protocol_minify_url(resource, tree_position, resources)
				resources.append(p)
			else:
				print('Invalid property definition for ' + url + '. ' + p.errorString)

		# TODO: childResources = + recursive call

	else:
		print('Unkown resource type for ' + url)

# pp.pprint(resources)
# pp.pprint(find_properties_in(resources))

hpp = hppTemplate.render(className = aml['title'], resources = resources)
cpp = cppTemplate.render(className = aml['title'], resources = resources)
hpp_p = hppPrivateTemplate.render(className = aml['title'],
	resources = resources,
	final_protocol_includes = '#include "3rdparty/msgpack-protocol-qt/msgpackprotocol.hpp"',
	final_protocol_members = 'aml::protocols::MsgPackProtocol *finalProtocol;')
hpp_bm = hpp_bm_template.render(className = aml['title'], resources = resources)

with open("/home/roman/temp/aml-static/" + aml['title'].lower() + ".hpp", "w") as f:
	f.write(hpp)

with open("/home/roman/temp/aml-static/" + aml['title'].lower() + ".cpp", "w") as f:
	f.write(cpp)

with open("/home/roman/temp/aml-static/" + aml['title'].lower() + "_p.hpp", "w") as f:
	f.write(hpp_p)

with open("/home/roman/temp/aml-bm/" + aml['title'].lower() + ".hpp", "w") as f:
	f.write(hpp_bm)