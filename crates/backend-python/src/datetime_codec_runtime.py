

def _checked_datetime(value, path, zulu=False):
    # Same validator on ingress and egress, including forged/mutated carriers.
    try:
        return _model._check_datetime(value, zulu)
    except (TypeError, ValueError) as error:
        raise CodecError(f'{path}: {error}') from error
