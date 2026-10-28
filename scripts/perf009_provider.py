"""Portable-only hardening around the unchanged PERF-005 provider mechanism."""
import json
from pathlib import Path
import re
import tomllib

from perf005_ci import validate_toolchain
from perf005_provider import Pilot, digest, source_identity, sources, verify_variant
from perf009_evidence import verify_preparation

ORIGIN = "sha512:cc5dd0a199115fee269ec1ef963b80abbb56e3029e817d5dbfe71e90fff09d0535ae5be86dedb6d3fbcc74cfa2f3e2e7885de800e5805f9f596507814dce6d4b"


def validate_lock(path, compiler):
    data = tomllib.loads(path.read_text())
    releases = [s.get("release", {}) for s in data["solution"]["state"]]
    required = {"gnatcoll": "26.0.0", "gnatcoll_minimal": "26.0.0", "gprbuild": "26.0.1",
                "gnat_external" if compiler == "14" else "gnat_native":
                "14.2.0" if compiler == "14" else "16.1.0"}
    for name, version in required.items():
        matches = [r for r in releases if r.get("name") == name]
        if len(matches) != 1 or matches[0].get("version") != version:
            raise RuntimeError("portable dependency lock differs")
        if name.startswith("gnatcoll") and ORIGIN not in matches[0].get("origin", {}).get("hashes", []):
            raise RuntimeError("portable provider origin differs")
    return data


def verify_provider(variant, expected, expected_manifest):
    if digest(variant / "complete.json") != expected_manifest:
        raise RuntimeError("portable prepared manifest changed")
    record = verify_variant(variant, expected)
    if any(p.stat().st_mode & 0o222 for p in (variant / "artifacts").rglob("*")):
        raise RuntimeError("portable provider is writable")
    return record


def verify_live_identity(text, compiler):
    validate_toolchain(text, compiler)
    hashes = re.findall(r"(?m)^([0-9a-f]{64})  (.+)$", text.decode())
    required = {"gcc", "gnat", "gprbuild", "system.ads", "libgnat.a"}
    checked = set()
    for expected, name in hashes:
        path = Path(name)
        if path.name in required:
            if not path.is_file() or digest(path) != expected:
                raise RuntimeError("portable live compiler/runtime identity changed")
            checked.add(path.name)
    if checked != required:
        raise RuntimeError("portable live identity inventory incomplete")


class PortablePilot(Pilot):
    def __init__(self, scratch, compiler, checked, jobs, traced=True):
        super().__init__(scratch, compiler, checked, jobs)
        self.traced = traced
        self.prepared_hashes = {}

    def configuration(self, directory, roots):
        result = super().configuration(directory, roots)
        verify_live_identity((self.scratch / f"{directory.name}-provider-identity.log").read_bytes(), self.compiler)
        return result

    def prepare(self, directory, roots, identity):
        super().prepare(directory, roots, identity)
        for policy in ("normal", "Check", "Ignore"):
            variant = self.root / policy
            if self.traced:
                verify_preparation(self.scratch / f"provider-{policy}-build.profile.json")
            self.prepared_hashes[policy] = digest(variant / "complete.json")
        (self.scratch / "portable-prepared.json").write_text(json.dumps(
            self.prepared_hashes, indent=2) + "\n")

    def build(self, command, directory, stem, expected):
        # The inherited mechanism rechecks every resolved lock and source set.
        # Hash the published manifest against an independently retained digest,
        # not against a manifest that an object mutation could also rewrite.
        policy = stem.name.removesuffix("-build").rsplit("-", 1)[1]
        if policy not in ("normal", "Check", "Ignore"):
            raise RuntimeError("unknown portable policy")
        identity_log = self.scratch / f"{directory.name}-provider-identity.log"
        if identity_log.exists():
            verify_live_identity(identity_log.read_bytes(), self.compiler)
        for p, expected_hash in self.prepared_hashes.items():
            variant = self.root / p
            identity = dict(self.identity, policy=p,
                            assertion_policy="" if p == "normal" else f"pragma Assertion_Policy ({p});\n")
            verify_provider(variant, identity, expected_hash)
        # Keep publication after all mandatory preparation checks. A failed
        # adapter never returns success or changes compilation strategy.
        result = super().build(command, directory, stem, expected)
        validate_lock(directory / "alire/alire.lock", self.compiler)
        if source_identity(sources(directory)) != self.identity["sources"]:
            raise RuntimeError("portable source identity changed")
        variant = self.root / policy
        key = dict(self.identity, policy=policy,
                   assertion_policy="" if policy == "normal" else f"pragma Assertion_Policy ({policy});\n")
        verify_provider(variant, key, self.prepared_hashes[policy])
        verify_live_identity(identity_log.read_bytes(), self.compiler)
        return result