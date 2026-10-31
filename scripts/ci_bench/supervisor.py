"""Trusted offline non-root wrapper: keep cgroup alive after worker process exit.

No candidate imports here. Child stdout is still untrusted evidence. Host verifies
the cgroup has only this init PID before its final sample/acknowledgement.
"""
import subprocess
import sys


def main():
    result = subprocess.run(['python3', '/trusted/scripts/ci_bench/worker.py', *sys.argv[1:]],
                            stdin=subprocess.DEVNULL, check=False)
    if result.returncode:
        raise SystemExit(result.returncode)
    print('CI_BENCH_WORKER_EXITED', flush=True)
    if sys.stdin.readline() != 'accounted\n':
        raise RuntimeError('host resource accounting acknowledgement missing')


if __name__ == '__main__':
    main()