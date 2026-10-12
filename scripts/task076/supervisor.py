"""Linux-only, sequential owned sessions. Not containment for detached sessions."""
import ctypes
import os
import signal
import subprocess
import time
import threading
import selectors
from pathlib import Path


def members(pgid):
    result = {}
    for path in Path('/proc').glob('[0-9]*/stat'):
        try:
            fields = path.read_text().rsplit(')', 1)[1].split()
            if int(fields[2]) == pgid:
                result[int(path.parent.name)] = (fields[0], fields[19])
        except (FileNotFoundError, ProcessLookupError):
            continue
    return result


def terminate(process, grace=0.2, final=3):
    """Leader remains unreaped until group signaling ends, preventing PGID reuse."""
    pgid = process.pid
    if pgid == os.getpgrp():
        raise RuntimeError('refusing caller group')
    def send(sig):
        try:
            os.killpg(pgid, sig)
        except ProcessLookupError:
            pass
    send(signal.SIGTERM)
    deadline = time.monotonic() + grace
    while time.monotonic() < deadline:
        time.sleep(min(0.01, max(0, deadline-time.monotonic())))
    # Always escalate after grace, independent of leader exit or pipe closure.
    owned = members(pgid)
    send(signal.SIGKILL)
    process.wait(timeout=final)
    deadline = time.monotonic() + final
    while True:
        remaining = members(pgid)
        for pid, identity in remaining.items():
            if pid in owned and identity[1] == owned[pid][1]:
                try:
                    os.waitpid(pid, os.WNOHANG)
                except ChildProcessError:
                    pass  # May still belong to its responsible parent.
        remaining = members(pgid)
        if not remaining:
            return
        if time.monotonic() >= deadline:
            raise RuntimeError('cleanup incomplete: live/zombie owned members '+repr(remaining))
        time.sleep(0.01)


def run_checked(command, env, *, phase='broker', timeout=300,
                marker='TASK076 BROKER: EXECUTED', log=None, grace=0.2):
    if not Path('/proc/self/stat').exists():
        raise RuntimeError('Linux /proc process control required')
    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(36, 1, 0, 0, 0) != 0:  # PR_SET_CHILD_SUBREAPER
        raise OSError(ctypes.get_errno(), 'subreaper unavailable')
    if threading.current_thread() is not threading.main_thread():
        raise RuntimeError('owned supervisor requires main thread')
    # All ownership state exists before any acquisition. Caught handlers (unlike
    # signal masks/ignored dispositions) reset at exec in the child.
    process = None
    selector = None
    streams = {}
    reason = None
    failures = []
    pending = []
    deferred = True
    saved = {sig: signal.getsignal(sig) for sig in (signal.SIGINT, signal.SIGTERM)}
    changed = []
    def interrupted(signum, frame):
        if deferred:
            if not pending:
                pending.append(signum)
        else:
            raise KeyboardInterrupt('supervisor interrupted by '+signal.Signals(signum).name)
    def attempt(label, action):
        try:
            action()
        except BaseException as exc:
            failures.append(label+': '+type(exc).__name__+': '+str(exc)[:2000])
    try:
        for sig in saved:
            signal.signal(sig, interrupted)
            changed.append(sig)
        process = subprocess.Popen(command, env=env, stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE, text=True, start_new_session=True)
        # Startup deferral covers Popen return -> ownership assignment. No early
        # poll/wait/context exit may reap the identity before group signaling.
        deferred = False
        if pending:
            raise KeyboardInterrupt('supervisor interrupted during launch: '+signal.Signals(pending[0]).name)
        selector = selectors.DefaultSelector()
        for stream in (process.stdout, process.stderr):
            streams[stream] = bytearray()
            os.set_blocking(stream.fileno(), False)
            selector.register(stream, selectors.EVENT_READ)
        deadline = time.monotonic()+timeout
        while True:
            for key, _ in selector.select(min(0.05, max(0, deadline-time.monotonic()))):
                data = os.read(key.fileobj.fileno(), 65536)
                if data:
                    streams[key.fileobj].extend(data)
                    if sum(map(len, streams.values())) > 8*1024*1024:
                        raise RuntimeError(phase+' output budget exceeded')
                else:
                    selector.unregister(key.fileobj)
            exited = os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            if exited and not selector.get_map():
                if exited.si_code != os.CLD_EXITED or exited.si_status:
                    raise RuntimeError(phase+' probe failed: exit/signal '+str(exited.si_status))
                text = b''.join(streams.values()).decode('utf-8', 'replace')
                if marker not in text:
                    raise RuntimeError(phase+' probe missing execution marker')
                break
            if time.monotonic() >= deadline:
                raise RuntimeError(phase+' process timeout; observation inconclusive')
    except BaseException as exc:
        reason = exc
    finally:
        # Deferral records ordinary cleanup cancellation rather than dropping it.
        deferred = True
        try:
            if process is not None:
                attempt('group cleanup', lambda: terminate(process, grace=grace))
            if selector is not None:
                attempt('selector close', selector.close)
            # Pipes are owned by Popen, not by successful buffer/selector setup.
            if process is not None:
                for stream in (process.stdout, process.stderr):
                    if stream is not None:
                        attempt('stream close', stream.close)
            if log:
                attempt('diagnostic log', lambda: Path(log).write_bytes(b''.join(streams.values())))
        finally:
            for sig in reversed(changed):
                attempt('signal restore '+signal.Signals(sig).name,
                        lambda sig=sig: signal.signal(sig, saved[sig]))
    if pending and reason is None:
        reason = KeyboardInterrupt('supervisor interrupted during lifecycle: '+signal.Signals(pending[0]).name)
    if failures:
        raise RuntimeError(str(reason or phase+' completed')+'; cleanup failure: '+ '; '.join(failures)) from reason
    if reason:
        raise reason
