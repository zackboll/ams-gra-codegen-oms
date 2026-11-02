

def _checked_duration(value, path):
    # One parser, shared with construction. Keep exception translation narrow.
    try:
        return _model._check_duration(value)
    except (TypeError, ValueError) as error:
        raise CodecError(f'{path}: {error}') from error