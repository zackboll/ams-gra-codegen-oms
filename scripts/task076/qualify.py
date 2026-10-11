#!/usr/bin/env python3
"""Opt-in differential campaign; no broker/validator/schema missing-input success."""
import argparse
import json
import os
import signal
import subprocess
from pathlib import Path

from corpus import prepare, REPO
from mapper import Mapper
from bundle import Bundle, Rejected, digest, verify_toolchain
from lxml import etree as E


def outcome(action):
    try:
        value = action()
        return value['result'] if isinstance(value, dict) else value
    except Rejected as exc:
        return str(exc)


def run_checked(command, env):
    with subprocess.Popen(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                          text=True, start_new_session=True) as process:
        try:
            stdout, stderr = process.communicate(timeout=300)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid,signal.SIGTERM)
            try:
                process.communicate(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid,signal.SIGKILL)
                process.communicate()
            raise RuntimeError('broker process timeout; observation inconclusive')
        if process.returncode:
            raise RuntimeError('broker probe failed: '+stdout[-4000:]+stderr[-4000:])
    if 'TASK076 BROKER: EXECUTED' not in stdout+stderr:
        raise RuntimeError('broker probe missing execution marker')


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
    verify_toolchain()
    campaign = prepare(a.scratch, a.uci)
    bundles = {k:Bundle(Path(v['root']),v['manifest'],research=True) for k,v in campaign['bundles'].items()}
    results = []
    for row in campaign['cases']:
        b = bundles[row['bundle']]
        raw = Path(row['raw_path']).read_bytes()
        assert digest(raw)==row['raw_sha256']
        mapper = Mapper(b,row['message_qname'])
        mapped = outcome(lambda:mapper.assess(raw))
        assert mapped==row['expected'], (row['id'],mapped,row['expected'])
        reference = None
        if row['reference_path']:
            xml = Path(row['reference_path']).read_bytes()
            reference = outcome(lambda:b.validate(xml))
            assert reference==row['reference_expected'], (row['id'],reference)
            if mapped=='research-schema-valid-not-authorized':
                assert reference=='valid-xml-only'
                # Exact expanded identities, content and xsi:type, not prefix spelling.
                def items(data):
                    def node(n):
                        lexical = n.get('{http://www.w3.org/2001/XMLSchema-instance}type')
                        ty = None
                        if lexical:
                            prefix, local = lexical.split(':')
                            ty = '{'+n.nsmap[prefix]+'}'+local
                        return [n.tag, ty, n.text if not len(n) else None,
                                [node(c) for c in n if isinstance(c.tag,str)]]
                    return node(E.fromstring(data))
                assert items(mapper.map(raw)['xml'])==items(xml), row['id']
        results.append({'id':row['id'],'mapper':mapped,'reference':reference})
    campaign_path = a.scratch/'campaign.json'
    campaign_path.write_text(json.dumps(campaign,indent=2,sort_keys=True))
    env = dict(os.environ, AMS_GRA_SLEET_BIN=str(a.binary), TASK076_CAMPAIGN=str(campaign_path), TASK076_SCRATCH=str(a.scratch))
    run_checked(['cargo','+1.95.0','test','--locked','-p','ams-gra-oms-runtime-rust-facade-tests','--test','task076_sleet','--','--ignored','--exact','task076_broker','--nocapture'],env)
    observations = json.loads((a.scratch/'broker.json').read_bytes())
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
                  corpus=campaign, results=results, validator=verify_toolchain(),
                  broker={'source_sha':revision,'binary_sha256':digest(a.binary.read_bytes()),
                          'lock_sha256':digest((a.sleet_checkout/'Cargo.lock').read_bytes()),
                          'toolchain':(a.sleet_checkout/'rust-toolchain.toml').read_text()})
    output['source_sha256'] = {str(p.relative_to(REPO)):digest(p.read_bytes()) for p in
        [Path(__file__), Path(__file__).with_name('corpus.py'),
         REPO/'scripts/task075/mapper.py', REPO/'scripts/task074/bundle.py',
         REPO/'crates/runtime-rust-facade-tests/tests/task076_sleet.rs']}
    frozen = REPO/'tests/fixtures/task076/expected-results.json'
    if frozen.exists():
        assert output==json.loads(frozen.read_bytes()), 'frozen differential evidence changed'
    (a.scratch/'results.json').write_text(json.dumps(output,indent=2,sort_keys=True)+'\n')
    print('TASK076: '+str(len(results))+' cases executed')


if __name__=='__main__':
    main()