

import builtins as _nato_special_words_builtins


def _check_nato_special_words(text: str) -> None:
    _check(text, str)
    if not 6 <= len(text) <= 261:
        raise ValueError('NATO SpecialWords requires 6..261 characters')
    if not text.startswith('NATO:'):
        raise ValueError('NATO SpecialWords requires the exact NATO: prefix')
    if _nato_special_words_builtins.any(
        not ('A' <= char <= 'Z' or 'a' <= char <= 'z' or char in '-_')
        for char in text[5:]
    ):
        raise ValueError('NATO SpecialWords suffix requires ASCII letters, hyphen or underscore')