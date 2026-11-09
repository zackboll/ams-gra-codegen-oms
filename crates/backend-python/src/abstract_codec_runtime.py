

# Keep duplicate evidence until the schema path is available. This replacement
# is emitted only for abstract-demanding schemas; established codecs keep bytes.
class _ObjectPairs(list):
    pass


def _member(value, member, path):
    try:
        return getattr(value, member)
    except AttributeError as error:
        raise CodecError(f'{path}: missing concrete member storage') from error


def _strict_tree(value, path):
    if type(value) is _ObjectPairs:
        result = {}
        for key, item in value:
            _unicode(key, path)
            if key in result:
                raise CodecError(f'{path}: duplicate member {key!r}')
            result[key] = _strict_tree(item, path + '.' + key)
        return result
    if type(value) is list:
        return [_strict_tree(item, path + '[' + str(index) + ']')
                for index, item in enumerate(value)]
    return value


def _loads(text):
    _expect(text, str, '$')
    _unicode(text, '$')
    try:
        tree = _json.loads(text, object_pairs_hook=_ObjectPairs, parse_int=_parse_integer,
                           parse_float=_reject_number, parse_constant=_reject_number)
    except _json.JSONDecodeError as error:
        raise CodecError(f'$: malformed JSON at line {error.lineno} column {error.colno}') from error
    return _strict_tree(tree, '$')