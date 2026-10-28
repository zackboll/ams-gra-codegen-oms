#!/usr/bin/env python3
"""Lightweight documentation policy; never execute Markdown snippets."""
from pathlib import Path
from ci_selection import ROOT

paths = [ROOT / 'README.md', *sorted((ROOT / 'docs').rglob('*.md'))]
for path in paths:
    text = path.read_bytes().decode('utf-8', errors='strict')
    if '\0' in text or any(line.startswith(('<<<<<<< ', '>>>>>>> ')) for line in text.splitlines()):
        raise ValueError('invalid UTF-8/conflict/NUL documentation: ' + str(path))
    if path.stat().st_mode & 0o111:
        raise ValueError('executable documentation requires explicit policy review: ' + str(path))
print('CI_DOCUMENTATION_POLICY_COMPLETE files=' + str(len(paths)))