

def _header_value(value, path, carrier):
    # Use the actual generated checked constructor on ingress AND egress.
    # Translate only its documented validation failures, retaining schema paths.
    _scalar(value, path, str)
    try:
        return carrier(value)
    except (TypeError, ValueError) as error:
        raise CodecError(f'{path}: {carrier.__schema_name__[1]}: {error}') from error


def _header_text(value, path, carrier):
    _expect(value, carrier, path)
    try:
        text = value.value
    except AttributeError as error:
        raise CodecError(f'{path}: {carrier.__schema_name__[1]}: missing value') from error
    return _header_value(text, path, carrier).value