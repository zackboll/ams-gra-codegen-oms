"""Bounded research CAL mapping metadata, NOT an XSD validator or ingress API."""
import json
import re
import sys
from pathlib import Path

from lxml import etree as E

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'task074'))
from bundle import I, X, Rejected, digest, parse

OAM = 'https://www.vdl.afrl.af.mil/programs/oam'
XS = X[1:-1]
LIMITS = dict(bytes=65536, depth=32, members=2048, array=256,
              number=128, output=65536)


class Failure(Rejected):
    def __init__(self, stage, reason):
        self.stage, self.reason = stage, reason
        super().__init__(stage + ':' + reason)


class Number(str):
    """Original RFC 8259 number token; never a binary floating point value."""


def fail(stage, reason):
    raise Failure(stage, reason)


def structural_children(node):
    """Interpret element children only; inert XML nodes/annotations are metadata."""
    return [n for n in node if isinstance(n.tag, str) and n.tag != X+'annotation']


def qname(key):
    if type(key) is not str:
        fail('shape', 'qname-token')
    if key.startswith('{'):
        match = re.fullmatch(r'\{([^{}]+)\}([^{}]+)', key)
        if not match:
            fail('shape', 'qname')
        ns, local = match.groups()
    else:
        ns, local = OAM, key
    # This bounded profile admits ASCII NCNames only, not every XML NCName.
    if not re.fullmatch(r'[A-Za-z_][A-Za-z_0-9.-]*', local):
        fail('unsupported', 'non-profile-qname')
    if any(ord(c) < 32 or 0xD800 <= ord(c) <= 0xDFFF for c in ns):
        fail('shape', 'namespace-character')
    return '{' + ns + '}' + local


def raw_parse(raw, limits):
    if len(raw) > limits['bytes']:
        fail('policy', 'byte-limit')
    try:
        text = raw.decode('utf-8')
    except UnicodeDecodeError:
        fail('syntax', 'utf8')
    # Pre-allocation depth guard, not a JSON grammar parser. json.loads owns
    # syntax, escape decoding and number grammar. Ignore brackets in strings.
    quoted = escaped = False
    depth = 0
    for char in text:
        if quoted:
            if escaped:
                escaped = False
            elif char == '\\':
                escaped = True
            elif char == '"':
                quoted = False
        elif char == '"':
            quoted = True
        elif char in '[{':
            depth += 1
            if depth > limits['depth']:
                fail('policy', 'depth-limit')
        elif char in ']}':
            depth -= 1
    members = 0

    def pairs(rows):
        nonlocal members
        members += len(rows)
        if members > limits['members']:
            fail('policy', 'member-limit')
        result = {}
        for key, value in rows:
            if key in result:
                fail('policy', 'duplicate-name')
            result[key] = value
        return result

    def number(token):
        if len(token) > limits['number']:
            fail('policy', 'number-limit')
        return Number(token)

    def constant(token):
        fail('syntax', 'non-json-number')

    try:
        value = json.loads(text, object_pairs_hook=pairs, parse_int=number,
                           parse_float=number, parse_constant=constant)
    except (ValueError, RecursionError):
        fail('syntax', 'json')
    stack = [value]
    while stack:
        node = stack.pop()
        if isinstance(node, list):
            if len(node) > limits['array']:
                fail('policy', 'array-limit')
            stack.extend(node)
        elif isinstance(node, dict):
            stack.extend(node.values())
    return value


class Mapper:
    def __init__(self, bundle, expected, *, limits=None):
        self.bundle = bundle
        self.expected = expected
        self.limits = {**LIMITS, **(limits or {})}
        self.types, self.elements = {}, {}
        # Only verified immutable snapshot bytes; no ambient file reads here.
        for data in bundle.files.values():
            root = parse(data, schema=True).getroot()
            ns = root.get('targetNamespace', '')
            for node in structural_children(root):
                name = node.get('name')
                if name and node.tag in (X+'complexType', X+'simpleType', X+'element'):
                    target = self.elements if node.tag == X+'element' else self.types
                    target['{' + ns + '}' + name] = node
        self.namespaces = sorted({ns for k in [*self.types, *self.elements]
                                  if (ns := E.QName(k).namespace) is not None})
        self.nsmap = {'xsi': I[1:-1], **{f'n{i}': ns for i, ns in enumerate(self.namespaces)}}

    def schema_qname(self, node, key):
        value = node.get(key)
        if not value:
            fail('unsupported', 'anonymous-type')
        if ':' in value:
            prefix, local = value.split(':', 1)
            ns = node.nsmap.get(prefix)
        else:
            ns, local = node.nsmap.get(None, ''), value
        if ns is None:
            fail('bundle', 'schema-qname')
        return '{' + ns + '}' + local

    def definition(self, name):
        if name not in self.types:
            fail('mapping', 'unresolved-type')
        return self.types[name]

    def content(self, name, seen=()):
        if name in seen or len(seen) >= self.limits['depth']:
            fail('unsupported', 'recursive-derivation')
        node = self.definition(name)
        if node.tag != X+'complexType':
            fail('mapping', 'not-complex-type')
        if node.get('mixed') == 'true':
            fail('unsupported', 'mixed')
        children = structural_children(node)
        if len(children) == 1 and children[0].tag == X+'complexContent':
            derivations = structural_children(children[0])
            if len(derivations) != 1 or derivations[0].tag != X+'extension':
                fail('unsupported', 'complex-restriction')
            ext = derivations[0]
            base = self.schema_qname(ext, 'base')
            parts = self.content(base, (*seen, name))
            children = structural_children(ext)
        else:
            parts = []
        if any(n.tag not in (X+'sequence', X+'choice') for n in children):
            fail('unsupported', 'complex-content')
        return parts + children

    def derived(self, actual, expected):
        seen = set()
        while actual != expected:
            if actual in seen:
                fail('unsupported', 'recursive-derivation')
            seen.add(actual)
            node = self.definition(actual)
            ext = node.find(X+'complexContent/'+X+'extension')
            if ext is None:
                return False
            actual = self.schema_qname(ext, 'base')
        return True

    def primitive(self, name, seen=()):
        if name.startswith(X):
            local = name[len(X):]
            integers = {'integer', 'long', 'int', 'short', 'byte', 'nonNegativeInteger',
                        'positiveInteger', 'nonPositiveInteger', 'negativeInteger',
                        'unsignedLong', 'unsignedInt', 'unsignedShort', 'unsignedByte'}
            if local in integers or local == 'decimal':
                return 'number'
            if local == 'boolean':
                return 'boolean'
            if local in {'string', 'normalizedString', 'token', 'dateTime', 'date', 'time',
                         'duration', 'anyURI'}:
                return 'string'
            fail('unsupported', 'simple-primitive')
        if name in seen or len(seen) >= self.limits['depth']:
            fail('unsupported', 'recursive-simple-type')
        node = self.definition(name)
        restriction = node.find(X+'restriction')
        if restriction is None:
            fail('unsupported', 'simple-list-union')
        return self.primitive(self.schema_qname(restriction, 'base'), (*seen, name))

    def simple(self, value, name):
        primitive = self.primitive(name)
        if primitive == 'number':
            if type(value) is not Number:
                fail('shape', 'number-token')
            # CAL Table 6.1-3 preserves token characters. Exponent spelling is
            # legal JSON but not XSD decimal lexical space; do not normalize it.
            if 'e' in value.lower():
                fail('unsupported', 'decimal-exponent')
            return str(value)
        if primitive == 'boolean':
            if type(value) is not bool:
                fail('shape', 'boolean-token')
            return 'true' if value else 'false'
        if type(value) is not str:
            fail('shape', 'string-token')
        if any(not (ord(c) in (9, 10, 13) or 32 <= ord(c) <= 0xD7FF or
                    0xE000 <= ord(c) <= 0xFFFD or 0x10000 <= ord(c) <= 0x10FFFF) for c in value):
            fail('mapping', 'xml-character')
        return value

    def member_name(self, node):
        if node.get('ref') or not node.get('name'):
            fail('unsupported', 'element-ref')
        schema = node.getroottree().getroot()
        if not schema.get('targetNamespace'):
            fail('unsupported', 'unqualified-member')
        if node.get('form', schema.get('elementFormDefault', 'unqualified')) != 'qualified':
            fail('unsupported', 'unqualified-member')
        return '{' + schema.get('targetNamespace', '') + '}' + node.get('name')

    def particle_keys(self, particle):
        if particle.tag == X+'element':
            return {self.member_name(particle)}
        if particle.tag not in (X+'sequence', X+'choice'):
            fail('unsupported', 'particle')
        return set().union(*(self.particle_keys(n) for n in structural_children(particle)))

    def emptiable(self, particle, depth=0):
        """Bounded XSD particle emptiability, only the already-supported forms.

        Structures §3.9.6: minOccurs zero or empty effective total range.
        Sequence needs all children empty; choice needs an empty alternative.
        Inspect all children so unsupported terms cannot hide in an empty branch.
        """
        if depth >= self.limits['depth']:
            fail('unsupported', 'particle-depth')
        if particle.tag == X+'element':
            self.member_name(particle)
            return particle.get('minOccurs', '1') == '0'
        if particle.tag not in (X+'sequence', X+'choice'):
            fail('unsupported', 'particle')
        if particle.get('maxOccurs', '1') != '1':
            fail('unsupported', 'repeated-group')
        empty = [self.emptiable(n, depth+1) for n in structural_children(particle)]
        return (particle.get('minOccurs', '1') == '0' or
                (all(empty) if particle.tag == X+'sequence' else not empty or any(empty)))

    def particle(self, parent, particle, values, used, path):
        if not isinstance(particle.tag, str) or particle.tag == X+'annotation':
            return
        if particle.tag in (X+'sequence', X+'choice'):
            if particle.get('maxOccurs', '1') != '1':
                fail('unsupported', 'repeated-group')
            if particle.get('minOccurs', '1') == '0' and not (self.particle_keys(particle) & values.keys()):
                return
            children = structural_children(particle)
            if particle.tag == X+'choice':
                children = [n for n in children if self.particle_keys(n) & values.keys()]
                if len(children) > 1:
                    fail('shape', 'choice-conflict')
                if not children and not self.emptiable(particle):
                    fail('shape', 'choice-missing')
            for child in children:
                self.particle(parent, child, values, used, path)
            return
        if particle.tag != X+'element':
            fail('unsupported', 'particle')
        name = self.member_name(particle)
        if name not in values:
            if int(particle.get('minOccurs', '1')):
                fail('shape', 'missing-member')
            return
        if name in used:
            fail('unsupported', 'ambiguous-particle')
        used.add(name)
        maximum = particle.get('maxOccurs', '1')
        repeated = maximum == 'unbounded' or int(maximum) > 1
        value = values[name]
        if repeated:
            if type(value) is not list:
                fail('shape', 'array-required')
            rows = value
        else:
            if isinstance(value, list):
                fail('shape', 'scalar-required')
            rows = [value]
        if len(rows) < int(particle.get('minOccurs', '1')) or (maximum != 'unbounded' and len(rows) > int(maximum)):
            fail('shape', 'occurrence')
        if particle.get('nillable') == 'true' or particle.get('fixed') is not None or particle.get('default') is not None:
            fail('unsupported', 'element-value-policy')
        for row in rows:
            child = E.SubElement(parent, name)
            self.value(child, row, self.schema_qname(particle, 'type'), path)

    def value(self, element, value, expected, path):
        if expected in path:
            fail('unsupported', 'recursive-value')
        if value is None:
            fail('shape', 'null')
        if expected.startswith(X) or self.definition(expected).tag == X+'simpleType':
            text = self.simple(value, expected)
            self.output_budget += len(text.encode('utf-8')) * 6
            if self.output_budget > self.limits['output']:
                fail('policy', 'output-limit')
            element.text = text
            return
        if type(value) is not dict:
            fail('shape', 'object-required')
        actual = qname(value['$type']) if '$type' in value else expected
        definition = self.definition(actual)
        if definition.tag != X+'complexType':
            fail('mapping', 'not-complex-type')
        if definition.get('abstract') == 'true':
            fail('mapping', 'abstract-type' if '$type' in value else 'missing-type')
        if not self.derived(actual, expected):
            fail('mapping', 'unrelated-type')
        if '$type' in value:
            ns, local = E.QName(actual).namespace, E.QName(actual).localname
            prefix = next(k for k, v in self.nsmap.items() if v == ns)
            element.set(I+'type', prefix + ':' + local)
        values = {}
        for key, row in value.items():
            if key == '$type':
                continue
            name = qname(key)
            if name in values:
                fail('policy', 'duplicate-expanded-qname')
            values[name] = row
        used = set()
        for part in self.content(actual):
            self.particle(element, part, values, used, (*path, expected))
        if values.keys() - used:
            fail('shape', 'unknown-member')

    def map(self, raw):
        value = raw_parse(raw, self.limits)
        if type(value) is not dict or len(value) != 1:
            fail('envelope', 'one-member-object')
        key = next(iter(value))
        if qname(key) != self.expected:
            fail('envelope', 'wrong-global')
        if self.expected not in self.elements:
            fail('mapping', 'undefined-global')
        if E.QName(self.expected).namespace is None:
            fail('unsupported', 'unqualified-member')
        root = E.Element(self.expected, nsmap=self.nsmap)
        self.output_budget = 0
        self.value(root, value[key], self.schema_qname(self.elements[self.expected], 'type'), ())
        xml = E.tostring(root, encoding='UTF-8')
        if len(xml) > self.limits['output']:
            fail('policy', 'output-limit')
        return {'raw': raw, 'raw_sha256': digest(raw), 'xml': xml,
                'xml_sha256': digest(xml), 'result': 'mapped-research-only'}

    def assess(self, raw):
        evidence = self.map(raw)
        try:
            self.bundle.validate(evidence['xml'])
        except Rejected as exc:
            fail('xsd' if str(exc).startswith('invalid:') else 'assessment-policy', str(exc))
        return {**evidence, 'result': 'research-schema-valid-not-authorized'}