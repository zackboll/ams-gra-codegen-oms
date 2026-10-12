#!/usr/bin/env python3
"""Opt-in differential campaign; no broker/validator/schema missing-input success."""
import argparse
import json
import os
import subprocess
from pathlib import Path

import hashlib
import sys
import tempfile
from supervisor import run_checked

REPO = Path(__file__).resolve().parents[2]

def digest(data):
    return hashlib.sha256(data).hexdigest()

def read_artifact(path, token, ids):
    try:
        if not 0 < path.stat().st_size <= 2 * 1024 * 1024:
            raise ValueError("size")
        value = json.loads(path.read_bytes())
        if value["token"] != token or [r["id"] for r in value["results"]] != ids:
            raise ValueError("stale or incomplete")
        if [r["id"] for r in value["campaign"]["cases"]] != ids:
            raise ValueError("corpus identities")
        if len(value["campaign"]["bundles"]) != 6 or not isinstance(value["validator"], dict):
            raise ValueError("incomplete phase")
        return value
    except (OSError, ValueError, KeyError, TypeError) as exc:
        raise RuntimeError("validator artifact invalid: " + str(exc)) from exc


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--scratch', type=Path, required=True)
    p.add_argument('--sleet-checkout', type=Path, required=True)
    p.add_argument('--binary', type=Path, required=True)
    p.add_argument('--uci', action='append', required=True)
    a = p.parse_args()
    assert a.scratch.is_absolute() and a.scratch.is_dir()
    assert a.binary.is_file(), 'missing broker binary'
    revision = 'e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b'
    assert subprocess.check_output(['git','-C',str(a.sleet_checkout),'rev-parse','HEAD'],text=True).strip()==revision
    assert not subprocess.check_output(['git','-C',str(a.sleet_checkout),'status','--porcelain'],text=True).strip(), 'modified broker checkout'
    assert len(a.uci)==2 and len(set(a.uci))==2, 'both verified releases required'
    invocation = Path(tempfile.mkdtemp(prefix='task076a-', dir=a.scratch))
    token = invocation.name
    expected = json.loads((REPO/'tests/fixtures/task076/expected-results.json').read_bytes())
    ids = [r['id'] for r in expected['results']]
    env = dict(os.environ)
    run_checked([sys.executable, str(Path(__file__).with_name('validator_worker.py')),
                 '--scratch', str(invocation), '--token', token,
                 *[v for path in a.uci for v in ('--uci', path)]], env,
                phase='validator', timeout=60, marker='TASK076 VALIDATOR: COMPLETE',
                log=invocation/'validator.log')
    artifact = read_artifact(invocation/'validator.json', token, ids)
    campaign, results = artifact['campaign'], artifact['results']
    assert results == [{k:v for k,v in r.items() if k!='sleet'} for r in expected['results']], 'validator outcomes changed'
    normalized = json.loads(json.dumps(campaign))
    for row in normalized['cases']:
        row['raw_path'] = Path(row['raw_path']).name
        if row['reference_path']:
            row['reference_path'] = Path(row['reference_path']).name
    for bundle in normalized['bundles'].values():
        del bundle['root']
    assert normalized == expected['corpus'], 'worker corpus mismatch'
    assert artifact['validator'] == expected['validator'], 'worker pin mismatch'
    a.scratch = invocation
    campaign_path = a.scratch/'campaign.json'
    campaign_path.write_text(json.dumps(campaign,indent=2,sort_keys=True))
    env = dict(os.environ, AMS_GRA_SLEET_BIN=str(a.binary), TASK076_CAMPAIGN=str(campaign_path), TASK076_SCRATCH=str(a.scratch))
    run_checked(['cargo','+1.95.0','test','--locked','-p','ams-gra-oms-runtime-rust-facade-tests','--test','task076_sleet','--','--ignored','--exact','task076_broker','--nocapture'],env, log=a.scratch/'broker.log')
    broker_path = a.scratch/'broker.json'
    assert 0 < broker_path.stat().st_size <= 2 * 1024 * 1024, 'broker artifact size'
    observations = json.loads(broker_path.read_bytes())
    assert set(observations)=={r['id'] for r in results}
    for r in results:
        r['sleet']=observations[r['id']]
    # Freeze no host paths, ports, timestamps or process IDs.
    for row in campaign['cases']:
        row['raw_path']=Path(row['raw_path']).name
        if row['reference_path']:
            row['reference_path']=Path(row['reference_path']).name
    for v in campaign['bundles'].values():
        del v['root']
    output = dict(disposition='POLYMORPHIC_COMPATIBILITY_PARTIALLY_CHARACTERIZED',
                  corpus=campaign, results=results, validator=artifact['validator'],
                  broker={'source_sha':revision,'binary_sha256':digest(a.binary.read_bytes()),
                          'lock_sha256':digest((a.sleet_checkout/'Cargo.lock').read_bytes()),
                          'toolchain':(a.sleet_checkout/'rust-toolchain.toml').read_text()})
    output['source_sha256'] = {str(p.relative_to(REPO)):digest(p.read_bytes()) for p in
        [Path(__file__), Path(__file__).with_name('corpus.py'),
         Path(__file__).with_name('supervisor.py'), Path(__file__).with_name('validator_worker.py'),
         REPO/'scripts/task075/mapper.py', REPO/'scripts/task074/bundle.py',
         REPO/'crates/runtime-rust-facade-tests/tests/task076_sleet.rs']}
    frozen = REPO/'tests/fixtures/task076/expected-results.json'
    historical = json.loads(frozen.read_bytes())
    assert output['results']==historical['results'], 'historical outcomes changed'
    assert output['corpus']==historical['corpus'], 'historical corpus changed'
    assert output['broker']==historical['broker'] and output['validator']==historical['validator'], 'pins changed'
    output['supervision'] = dict(validator_cpu_seconds=30, validator_as_bytes=768*1024*1024,
        validator_core_bytes=0, validator_wall_seconds=60, broker_wall_seconds=300,
        cleanup='linux-owned-session-subreaper', artifact_binding='fresh-invocation-token')
    if 'supervision' in historical:
        assert output == historical, 'corrective frozen evidence changed'
    (a.scratch/'results.json').write_text(json.dumps(output,indent=2,sort_keys=True)+'\n')
    print('TASK076: '+str(len(results))+' cases executed')


if __name__=='__main__':
    main()