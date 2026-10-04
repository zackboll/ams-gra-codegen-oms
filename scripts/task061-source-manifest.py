#!/usr/bin/env python3
"""Identify tested source, including relevant unstaged AND untracked files."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

repo = Path(__file__).resolve().parent.parent
names = subprocess.check_output(["git", "-C", str(repo), "ls-files", "-co", "--exclude-standard", "-z"]).decode().split("\0")
files = {name: hashlib.sha256((repo / name).read_bytes()).hexdigest()
         for name in sorted(set(names)) if name and (repo / name).is_file()}
identity = hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest()
manifest = {"head": subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"]).decode().strip(), "identity": identity, "files": files}
Path(sys.argv[1]).write_text(json.dumps(manifest, indent=2) + "\n")
print(identity)