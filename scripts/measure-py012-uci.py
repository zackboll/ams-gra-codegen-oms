#!/usr/bin/env python3
"""Reuse the established 24-selection measurement with PY-012 source metadata."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py012_measurement', ROOT / 'scripts/measure-py011-uci.py')
measurement = importlib.util.module_from_spec(spec)
spec.loader.exec_module(measurement)
spec = importlib.util.spec_from_file_location('py012_pins', ROOT / 'scripts/task101_evidence.py')
pins = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pins)


def annotate(value):
    value['source']['base'] = 'ab438eab77d0b202a033bec2b2fa99508bbebc8f'
    value['source']['base_tree'] = '02c0df930e7af0c8e487994ac25e5a69d1104a33'
    value['schema_includes'] = {}
    for version in measurement.driver.PINS:
        root = Path(os.environ['AMS_GRA_UCI_' + version.replace('.', '_') + '_ROOT'])
        include = root.parent / f'UCI_SecurityMarkings_v2_{version[-1]}_0.xsd'
        digest = hashlib.sha256(include.read_bytes()).hexdigest()
        measurement.driver.require(digest == pins.INCLUDES[version.replace('.', '')], 'include hash mismatch')
        value['schema_includes'][version] = dict(name=include.name, sha256=digest)
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generator', type=Path, required=True)
    parser.add_argument('--closure-tool', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=ROOT)
    args = parser.parse_args()
    value = measurement.measure(args.generator.resolve(), args.closure_tool.resolve(),
                                args.output.resolve(), args.source_root.resolve())
    (args.output / 'measurements.json').write_text(json.dumps(annotate(value), indent=2) + '\n')
    print('PY012 MEASUREMENTS COMPLETE: selections=24 (not interoperability)')


if __name__ == '__main__':
    main()