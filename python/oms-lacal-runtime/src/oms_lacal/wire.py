"""OWP identifiers and lossless global-element boundaries, not payload codecs."""
import json
import re
from uuid import UUID

OAM_NAMESPACE = 'https://www.vdl.afrl.af.mil/programs/oam'
_IDENTIFIER = re.compile(r'[A-Za-z0-9_.-]+', re.ASCII)
_LOCAL = re.compile(r'[A-Za-z_][A-Za-z0-9_.-]*', re.ASCII)


class LacalError(Exception):
    """Runtime failure. Messages deliberately exclude payloads/server details."""


class InvalidInput(LacalError): pass
class NotConnected(LacalError): pass
class ProtocolError(LacalError): pass
class TransportFailure(LacalError): pass
class OperationTimeout(LacalError): pass
class ResourceLimit(LacalError): pass
class CodecFailure(LacalError): pass
class CodecInternalFailure(LacalError): pass
class UnknownSubscription(LacalError): pass
class HandlerFailure(LacalError): pass


class ServerRejection(LacalError):
    def __init__(self, code):
        self.code = code
        super().__init__('server rejected operation: ' + code)


ERROR_CODES = frozenset({
    'Unsupported-Version', 'Unsupported-Schema', 'Unsupported-Service',
    'Illegal-Operation', 'Illegal-Argument', 'Illegal-State', 'Internal-Error',
    'Invalid-Message',
})


def identifier(value, role):
    if type(value) is not str or not _IDENTIFIER.fullmatch(value):
        raise InvalidInput('invalid ' + role + ' identifier')
    return value


def global_name(namespace, local):
    if (type(namespace) is not str or not namespace or
            any(ch.isspace() or ch in '{}"\\' or ord(ch) < 32 for ch in namespace) or
            type(local) is not str or not _LOCAL.fullmatch(local)):
        raise InvalidInput('invalid qualified message identity')
    return local if namespace == OAM_NAMESPACE else '{' + namespace + '}' + local


def _pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ProtocolError('duplicate JSON member')
        result[key] = value
    return result


def _number(token):
    # Syntax is checked by stdlib JSON; keep all number lexemes intact here.
    # The generated decoder, on the original slice, owns numeric type policy.
    return token


def _constant(token):
    raise ProtocolError('non-standard JSON constant')


_DECODER = json.JSONDecoder(object_pairs_hook=_pairs, parse_int=_number,
                            parse_float=_number, parse_constant=_constant)


def _unicode(text):
    try:
        text.encode('utf-8')
    except UnicodeEncodeError:
        raise ProtocolError('invalid Unicode text') from None


def _space(text, at):
    while at < len(text) and text[at] in ' \t\r\n':
        at += 1
    return at


def unwrap(expected, text):
    """Validate envelope, return the exact raw JSON body substring.

    raw_decode validates nested syntax and duplicate decoded member names. Its
    temporary values are discarded; numeric lexemes and all body bytes reach
    the generated strict decoder unchanged. No collapsed dict is reserialized.
    """
    _unicode(text)
    try:
        at = _space(text, 0)
        if at >= len(text) or text[at] != '{':
            raise ProtocolError('global element must be an object')
        at = _space(text, at + 1)
        key, at = _DECODER.raw_decode(text, at)
        if type(key) is not str or key != expected:
            raise ProtocolError('unexpected global element identity')
        at = _space(text, at)
        if at >= len(text) or text[at] != ':':
            raise ProtocolError('invalid global element member')
        start = _space(text, at + 1)
        _, end = _DECODER.raw_decode(text, start)
        at = _space(text, end)
        if at >= len(text) or text[at] != '}' or _space(text, at + 1) != len(text):
            raise ProtocolError('global element must have exactly one member')
        return text[start:end]
    except json.JSONDecodeError:
        raise ProtocolError('malformed global element JSON') from None
    except (RecursionError, MemoryError):
        raise ResourceLimit('JSON parsing resource limit') from None


def info(text):
    _unicode(text)
    try:
        value = json.loads(text, object_pairs_hook=_pairs, parse_constant=_constant,
                           parse_int=_constant, parse_float=_constant)
    except json.JSONDecodeError:
        raise ProtocolError('malformed INFO JSON') from None
    if type(value) is not dict:
        raise ProtocolError('INFO must be an object')
    for key in ('version', 'server_id', 'system_label'):
        if type(value.get(key)) is not str or not value[key]:
            raise ProtocolError('invalid INFO metadata')
    uuids = value.get('uuids')
    if type(uuids) is not dict:
        raise ProtocolError('invalid INFO identifiers')
    try:
        for key in ('system', 'service'):
            if type(uuids.get(key)) is not str:
                raise ValueError()
            UUID(uuids[key])
        if 'subsystem' in uuids:
            UUID(uuids['subsystem'])
        for key in ('capabilities', 'components'):
            if key in uuids:
                if type(uuids[key]) is not dict:
                    raise ValueError()
                for name, uuid in uuids[key].items():
                    if type(name) is not str or type(uuid) is not str:
                        raise ValueError()
                    UUID(uuid)
    except (ValueError, TypeError, AttributeError):
        raise ProtocolError('invalid INFO UUID metadata') from None
    if 'connect_urls' in value and (type(value['connect_urls']) is not list or
            any(type(url) is not str for url in value['connect_urls'])):
        raise ProtocolError('invalid INFO connect URLs')
    return value


def server_error(frame):
    fields = re.split('[ \t]+', frame, maxsplit=2)
    if len(fields) < 2 or fields[1] not in ERROR_CODES:
        raise ProtocolError('invalid server error frame')
    # Never expose arbitrary server details, which may echo an entire payload.
    return ServerRejection(fields[1])