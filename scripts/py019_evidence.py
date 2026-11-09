"""Independent PY-019 transport/source evidence checks, not a codec oracle."""
import hashlib
import json
from pathlib import Path


def require(condition, message):
    if not condition:
        raise ValueError(message)


def verify_transport(observations):
    deliveries = [row for row in observations if row.get('outcome') == 'typed delivery']
    require(len(deliveries) == 6, 'missing or duplicate typed deliveries')
    expected = {(name, optional) for name in ['Zeta', 'Deep-Leaf', 'Alpha'] for optional in [False, True]}
    require({(row['subtype'], row['optional']) for row in deliveries} == expected, 'wrong subtype inventory')
    for row in deliveries:
        frame = row['raw_frame']
        require(frame.startswith('MSG independent '), 'wrong raw subscriber identity')
        body = json.loads(frame.split(' ', 2)[2])['AbstractNotice']
        require(body['Required']['$type'] == row['subtype'], 'raw subtype identity differs')
        require(('Optional' in body) == row['optional'], 'optional presence differs')
    broker = [row for row in observations if row.get('outcome') == 'broker rejection']
    runtime = [row for row in observations if row.get('outcome') == 'runtime envelope rejection']
    require(len(broker) == 6 and len(runtime) == 1, 'missing original malformed controls')
    require(all(row.get('decoder_delta') == 0 for row in broker + runtime), 'predecoder rejection relabelled')
    require(all(row.get('code') == 'Invalid-Message' for row in broker), 'wrong broker rejection')
    require(runtime[0].get('runtime_event') == 'ProtocolError', 'wrong envelope rejection')
    decoded = [row for row in observations if row.get('outcome') == 'generated decoder rejection']
    require(len(decoded) == 1, 'missing or duplicate generated rejection')
    row = decoded[0]
    require(row.get('classification') == 'schema-valid unsupported concrete-position discriminator',
            'schema-valid boundary relabelled malformed')
    require(row.get('broker_ack') == '+OK' and row.get('decoder_delta') == 1, 'decoder entry not established')
    require(row.get('runtime_event') == 'CodecFailure' and row.get('category') == 'CodecError',
            'wrong decoder/runtime error category')
    require(row.get('path') == '$.Concrete' and bool(row.get('subscription_id')), 'wrong path or SID')
    require(row.get('decoder_input') == row.get('input'), 'decoder did not receive original JSON')
    require(row.get('raw_frame') == 'MSG independent {"AbstractNotice":' + row['input'] + '}',
            'raw observer did not receive original envelope')
    candidate = json.loads(row['input'])
    require(candidate['Concrete']['$type'] == 'Zeta', 'wrong concrete boundary candidate')
    cleanup = [row for row in observations if row.get('outcome') == 'unsubscribe and callback silence']
    require(len(cleanup) == 1 and cleanup[0].get('decoder_delta') == 0
            and cleanup[0].get('subscriptions') == 0, 'unsubscribe/silence evidence absent')
    return dict(deliveries=6, broker_rejections=6, envelope_rejections=1, decoder_rejections=1)


def verify_files(root, manifest):
    """Exact safe file inventory; rejects missing, added, redirected or changed data."""
    root = Path(root)
    require(type(manifest) is dict and bool(manifest), 'empty or malformed evidence manifest')
    for name, digest in manifest.items():
        path = Path(name)
        require(not path.is_absolute() and '..' not in path.parts and name == path.as_posix(), 'unsafe evidence path')
        source = root / path
        require(not source.is_symlink() and source.is_file(), 'missing or redirected evidence')
        require(all(not parent.is_symlink() for parent in source.parents if parent != root.parent),
                'redirected evidence parent')
        require(type(digest) is str and hashlib.sha256(source.read_bytes()).hexdigest() == digest,
                'evidence digest mismatch')
    actual = {p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file() or p.is_symlink()}
    require(actual == set(manifest), 'unexpected evidence inventory')