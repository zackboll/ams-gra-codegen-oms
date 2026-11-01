

import builtins as _visible_ascii_builtins


def _check_visible_ascii(text: str, minimum: int, maximum: int) -> None:
    # Exact Task 039 domain. SPACE is significant, including at either end.
    _check(text, str)
    if not minimum <= len(text) <= maximum:
        raise ValueError(f'visible ASCII: expected {minimum}..{maximum} characters')
    if _visible_ascii_builtins.any(not ' ' <= char <= '~' for char in text):
        raise ValueError('visible ASCII: expected only U+0020..U+007E')