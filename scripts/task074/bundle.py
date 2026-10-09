"""Research-only, fail-closed XML/XSD probe. Not an OMS JSON validator.

No production caller, approval authority or publication permission is supplied.
The caller must run this inside its bounded worker process.
"""
import hashlib
import io
import json
import re
from pathlib import Path, PurePosixPath

from lxml import etree as E

X = "{http://www.w3.org/2001/XMLSchema}"
I = "{http://www.w3.org/2001/XMLSchema-instance}"
BASE = "file:///task074-approved/"
PIN = {
    "lxml": "5.4.0",
    "libxml": "2.9.14",
    "etree_sha256": "27d8b7e79e217e6946ad6871c13b6bdfbcf55a1a3f6b3ab590cb892fc3a5deca",
    "libxml_sha256": "8f2643af68a9eba917929046806dabd1848867068d0eee0f6799288a8e886963",
}


class Rejected(Exception):
    pass


def digest(data):
    return hashlib.sha256(data).hexdigest()


def verify_toolchain():
    actual = {
        "lxml": ".".join(map(str, E.LXML_VERSION[:3])),
        "libxml": ".".join(map(str, E.LIBXML_VERSION)),
        "etree_sha256": digest(Path(E.__file__).read_bytes()),
        "libxml_sha256": digest(Path('/usr/lib/x86_64-linux-gnu/libxml2.so.2.9.14').read_bytes()),
    }
    if actual != PIN:
        raise Rejected("toolchain-pin")
    return actual


def safe_name(name):
    p = PurePosixPath(name)
    if not name or p.is_absolute() or any(x in ("..", ".") for x in name.split('/')) or ':' in name or '\\' in name:
        raise Rejected("path")
    return name


def parser(resolver=None):
    p = E.XMLParser(no_network=True, resolve_entities=False, load_dtd=False,
                    huge_tree=False, recover=False)
    if resolver is not None:
        p.resolvers.add(resolver)
    return p


def parse(data, schema=False, resolver=None, url=None):
    if len(data) > (16 * 1024 * 1024 if schema else 64 * 1024):
        raise Rejected("size")
    # UTF-8-only research profile; alternate encodings are not accepted.
    try:
        text = data.decode('utf-8')
    except UnicodeDecodeError as exc:
        raise Rejected("utf8") from exc
    if '<!DOCTYPE' in text or '<!ENTITY' in text:
        raise Rejected("dtd")
    encoding = re.search(r'<\?xml\s[^?]*encoding\s*=\s*[\'"]([^\'"]+)', text)
    if encoding and encoding.group(1).lower() != 'utf-8':
        raise Rejected("encoding")
    try:
        root = E.parse(io.BytesIO(data), parser(resolver), base_url=url)
    except E.XMLSyntaxError as exc:
        raise Rejected("xml") from exc
    if root.docinfo.doctype:
        raise Rejected("dtd")
    if not schema:
        stack = [(root.getroot(), 1)]
        while stack:
            node, depth = stack.pop()
            if depth > 64:
                raise Rejected("depth")
            if node.get(I + 'schemaLocation') is not None or node.get(I + 'noNamespaceSchemaLocation') is not None:
                raise Rejected("schema-location")
            stack.extend((child, depth + 1) for child in node)
    return root


class SnapshotResolver(E.Resolver):
    def __init__(self, files):
        super().__init__()
        self.files = files
        self.requests = []

    def resolve(self, url, public_id, context):
        self.requests.append(url)
        if not url.startswith(BASE) or url[len(BASE):] not in self.files:
            raise Rejected("resolution")
        return self.resolve_string(self.files[url[len(BASE):]], context, base_url=url)


class Bundle:
    def __init__(self, root, manifest, *, research=False):
        verify_toolchain()
        if manifest['toolchain'] != PIN:
            raise Rejected("toolchain-manifest")
        # A test flag is NOT a deployment approval mechanism.
        if not research:
            raise Rejected("authority-not-established")
        root = Path(root).resolve()
        self.manifest = manifest
        self.files = {}
        namespaces = {}
        for row in manifest['files']:
            name = safe_name(row['path'])
            if name in self.files:
                raise Rejected("duplicate-path")
            path = root / name
            if any(part.is_symlink() for part in [path, *path.parents] if part != root):
                raise Rejected("symlink")
            if not path.resolve().is_relative_to(root):
                raise Rejected("path")
            try:
                data = path.read_bytes()
            except FileNotFoundError as exc:
                raise Rejected("missing-file") from exc
            if digest(data) != row['sha256']:
                raise Rejected("digest")
            tree = parse(data, schema=True)
            if tree.getroot().tag != X + 'schema' or tree.getroot().get('targetNamespace', '') != row['namespace']:
                raise Rejected("namespace")
            namespaces[name] = row['namespace']
            self.files[name] = data
        # Require an explicit, complete, deterministic DFS closure. No ambient
        # catalogs, schema-location hints, namespace search or network fallback.
        seen = []

        def visit(name):
            if name in seen:
                return
            if name not in self.files:
                raise Rejected("closure")
            seen.append(name)
            tree = parse(self.files[name], schema=True)
            for node in tree.getroot():
                if node.tag == X + 'redefine':
                    raise Rejected("redefine-unsupported")
                if node.tag not in (X + 'include', X + 'import'):
                    continue
                location = safe_name(node.get('schemaLocation', ''))
                dep = str(PurePosixPath(name).parent / location)
                if dep not in self.files:
                    raise Rejected("closure")
                expected = namespaces[name] if node.tag == X + 'include' else node.get('namespace', '')
                if namespaces[dep] != expected:
                    raise Rejected("dependency-namespace")
                visit(dep)

        visit(safe_name(manifest['root']))
        if seen != [row['path'] for row in manifest['files']]:
            raise Rejected("closure-order")
        self.resolver = SnapshotResolver(self.files)
        doc = parse(self.files[manifest['root']], schema=True, resolver=self.resolver,
                    url=BASE + manifest['root'])
        try:
            self.validator = E.XMLSchema(doc)
        except E.XMLSchemaParseError as exc:
            raise Rejected("schema:" + exc.error_log.last_error.type_name) from exc

    def validate(self, data):
        doc = parse(data, resolver=self.resolver)
        try:
            self.validator.assertValid(doc)
        except E.DocumentInvalid as exc:
            raise Rejected("invalid:" + exc.error_log[0].type_name) from exc
        # QName allow-list is policy additional to XSD validity. This narrow
        # probe checks explicit xsi:type, not implicit types/substitution groups.
        for node in doc.iter():
            lexical = node.get(I + 'type')
            if lexical is None:
                continue
            if ':' in lexical:
                prefix, local = lexical.split(':')
                namespace = node.nsmap.get(prefix, '')
            else:
                local, namespace = lexical, node.nsmap.get(None, '')
            if '{' + namespace + '}' + local not in self.manifest['approved_concrete_qnames']:
                raise Rejected("unapproved-type")
        return "valid-xml-only"


def make_manifest(root, name, release, approved):
    """Fixture helper only: computing hashes never confers deployment approval."""
    root = Path(root)
    files = []
    seen = set()

    def visit(path):
        if path in seen:
            return
        seen.add(path)
        data = (root / path).read_bytes()
        tree = parse(data, schema=True)
        files.append({'path': path, 'sha256': digest(data),
                      'namespace': tree.getroot().get('targetNamespace', '')})
        for node in tree.getroot():
            if node.tag in (X + 'include', X + 'import'):
                visit(str(PurePosixPath(path).parent / safe_name(node.get('schemaLocation', ''))))

    visit(name)
    return {'bundle_id': 'task074-synthetic-' + release, 'version': 1,
            'root': name, 'files': files, 'public_uci_release': release,
            'approved_concrete_qnames': approved, 'toolchain': PIN,
            'approval_provenance': {'status': 'UNESTABLISHED', 'owner': None,
                                    'evidence': None, 'scope': 'research-only'}}


if __name__ == '__main__':
    import sys
    bundle_root, manifest_path, xml_path = map(Path, sys.argv[1:])
    # No command-line switch bypasses the missing authority gate.
    Bundle(bundle_root, json.loads(manifest_path.read_text())).validate(xml_path.read_bytes())