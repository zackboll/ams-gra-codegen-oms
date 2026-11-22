#!/usr/bin/env python3
"""Opt-in staged-client smoke against a test endpoint with a separate publisher."""

import argparse
from pathlib import Path
import re
import subprocess
import sys

sys.dont_write_bytecode = True
import check as deploy

FAMILIES = (('Log', 'uci.log'), ('RDMA_InitializeSetup', 'uci.rdma'),
            ('SurvivabilityRiskLevel', 'uci.survivability'))
FLAGS = {'A': (), 'B': ('--with-rdma',), 'C': ('--with-rdma', '--with-survivability')}


def smoke(bundle, args):
    deploy.check(bundle, args)
    flags = FLAGS[args.profile]
    argv = [bundle / deploy.binary_path(args), '--url', args.test_url, '--service-id', args.service_id,
            '--duration-seconds', str(args.duration_seconds), '--max-messages', str(args.max_messages), *flags]
    if args.ca_file is not None:
        argv.extend(('--ca-file', args.ca_file))
    # Preserve the operator's trust and loader configuration for the real run.
    # Do not accumulate potentially large logs in memory.
    families = FAMILIES[:len(flags) + 1]
    roster = ' '.join(f'sid=sub-{i} message={name} topic={topic}' for i, (name, topic) in enumerate(families, 1))
    connected = ready = clean = False
    received = set()
    unsubscribed = []
    with subprocess.Popen(argv, stdout=subprocess.PIPE, text=True, errors='replace') as child:
        for line in child.stdout:
            print(line, end='', flush=True)
            if line.startswith(f'CONNECTED owp=1.0 schema={deploy.schema(args)} '):
                connected = True
            elif line.startswith('READY ' + roster + ' ('):
                ready = connected
            elif ready and (match := re.match(r'RECEIVED (\w+) #\d+ ', line)):
                received.add(match[1])
            elif match := re.match(r'UNSUBSCRIBED sid=(sub-\d+) acknowledged', line):
                unsubscribed.append(match[1])
            elif line.startswith('CLOSED close=ok '):
                clean = line.rstrip().endswith('resources-released=yes')
        status = child.wait()
    if status != 0:
        print(f'SMOKE FAILED: client status {status} (not rewritten to success)', file=sys.stderr)
        return status if status > 0 else 128 - status
    deploy.require(connected and ready, 'missing CONNECTED or full-profile READY')
    deploy.require(received == {name for name, _ in families}, 'expected typed message from each enabled family')
    deploy.require(clean and unsubscribed == [f'sub-{i}' for i in range(len(families), 0, -1)],
                   'missing clean shutdown or acknowledged reverse UNSUBs')
    print('SMOKE OK: full profile acknowledged, typed reception and clean shutdown (client exit 0)')
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    deploy.selection(parser)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('--test-url', required=True, help='explicitly authorized non-production test endpoint')
    parser.add_argument('--service-id', required=True)
    parser.add_argument('--duration-seconds', type=int, required=True)
    parser.add_argument('--max-messages', type=int, required=True)
    parser.add_argument('--ca-file', type=Path)
    args = parser.parse_args()
    if not 1 <= args.duration_seconds <= 3600 or not len(FLAGS[args.profile]) + 1 <= args.max_messages <= 100000:
        parser.error('duration must be 1..3600 and message limit must cover the selected profile (at most 100000)')
    try:
        return smoke(args.bundle.resolve(strict=True), args)
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        print(f'SMOKE FAILED: {exc}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())