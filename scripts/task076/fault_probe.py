"""Synthetic supervision faults only; never used by real qualification."""
import json
import os
import resource
import signal
import sys
import time
from pathlib import Path


def ready(path, phase):
    Path(path).write_text(json.dumps(dict(pid=os.getpid(), pgid=os.getpgrp(), phase=phase,
        start=Path('/proc/self/stat').read_text().rsplit(')',1)[1].split()[19],
        limits=[resource.getrlimit(r) for r in
                (resource.RLIMIT_CPU, resource.RLIMIT_AS, resource.RLIMIT_CORE)])))


def worker_fault(mode, path):
    # Inject dependencies at import time AFTER the real entry point sets limits.
    import importlib.abc
    import importlib.util
    import runpy
    class Loader(importlib.abc.MetaPathFinder, importlib.abc.Loader):
        def find_spec(self, fullname, path=None, target=None):
            if fullname in ('corpus', 'mapper', 'bundle'):
                return importlib.util.spec_from_loader(fullname, self)
        def create_module(self, spec):
            return None
        def exec_module(self, module):
            def fault(phase):
                ready(path, phase)
                if mode.startswith('stall'):
                    signal.pause()
                if mode=='exit':
                    raise SystemExit(7)
                if mode=='signal':
                    os.kill(os.getpid(),signal.SIGKILL)
                if mode in ('pin','compilation'):
                    raise RuntimeError('toolchain-pin' if mode=='pin' else 'schema:compile-failure')
                if mode=='cpu':
                    resource.setrlimit(resource.RLIMIT_CPU,(1,1))
                    while True:
                        pass
                if mode=='memory':
                    resource.setrlimit(resource.RLIMIT_AS,(64*1024*1024,)*2)
                    bytearray(128*1024*1024)
            if module.__name__=='corpus':
                def prepare(*args):
                    Path(path).write_text('{}')
                    if mode=='stall-prepare': fault('prepare')
                    return {'bundles':{'test':{'root':'/tmp','manifest':{}}},
                            'cases':[dict(id='test',bundle='test',raw_path=path,
                              raw_sha256='digest',message_qname='Message',expected='rejected',
                              reference_path=path,reference_expected='rejected')]}
                module.prepare=prepare
            if module.__name__=='bundle':
                class Rejected(Exception):
                    pass
                class Bundle:
                    def __init__(self,*args,**kwargs):
                        if mode not in ('stall-map','stall-reference'):
                            fault('constructor')
                    def validate(self,*args):
                        fault('reference')
                        raise Rejected('rejected')
                module.Bundle=Bundle
                module.Rejected=Rejected
                module.digest=lambda data:'digest'
                def verify():
                    if mode=='pin': fault('toolchain')
                    return {}
                module.verify_toolchain=verify
            if module.__name__=='mapper':
                class Mapper:
                    def __init__(self,*args): pass
                    def assess(self,*args):
                        if mode=='stall-map': fault('mapping')
                        from bundle import Rejected
                        raise Rejected('rejected')
                module.Mapper=Mapper
    sys.meta_path.insert(0,Loader())
    sys.argv=['validator_worker','--scratch',str(Path(path).parent),'--token','fault',
              '--uci','a','--uci','b']
    runpy.run_path(str(Path(__file__).with_name('validator_worker.py')),run_name='__main__')


def group_fault(mode, path):
    read_fd, write_fd = os.pipe()
    pid = os.fork()
    if pid==0:
        os.close(read_fd)
        if mode!='pipe':
            null=os.open('/dev/null',os.O_WRONLY)
            os.dup2(null,1); os.dup2(null,2); os.close(null)
        if mode=='race':
            def end_on_term(*args):
                time.sleep(.05)
                os._exit(0)
            signal.signal(signal.SIGTERM,end_on_term)
        else:
            signal.signal(signal.SIGTERM,signal.SIG_IGN)
        ready(path,'descendant')
        os.write(write_fd,b'R');os.close(write_fd)
        while True: signal.pause()
    os.close(write_fd)
    assert os.read(read_fd,1)==b'R'
    os.close(read_fd)
    if mode=='failed': raise SystemExit(4)
    if mode=='marker': raise SystemExit(0)
    if mode=='interrupt': os.kill(os.getpid(),signal.SIGINT)
    while True: signal.pause()


def scenario(kind, path):
    from supervisor import run_checked
    ready_path = str(Path(path)/'ready.json')
    # Independent in-process outer alarm, plus caller's external watchdog.
    def alarm(*args): raise RuntimeError('outer watchdog')
    signal.signal(signal.SIGALRM,alarm);signal.alarm(10)
    sentinel = os.fork()
    if sentinel==0:
        os.setsid()
        ready(str(Path(path)/'sentinel.json'),'sentinel')
        while True: signal.pause()
    try:
        worker = kind.startswith(('stall','cpu','memory','exit','signal','pin','compilation'))
        command=[sys.executable,__file__,'worker' if worker else 'group',kind,ready_path]
        if kind in ('parent-interrupt','parent-term'):
            import threading
            def interrupt():
                deadline=time.monotonic()+5
                while not Path(ready_path).exists() and time.monotonic()<deadline:
                    time.sleep(.005)
                os.kill(os.getpid(),signal.SIGINT if kind=='parent-interrupt' else signal.SIGTERM)
            threading.Thread(target=interrupt,daemon=True).start()
        try:
            run_checked(command,os.environ,phase='validator' if worker else 'broker',
                        timeout=3 if kind in ('cpu','memory') else .4,
                        marker='never',grace=.2,log=Path(path)/'fault.log')
            raise AssertionError('fault unexpectedly succeeded')
        except (RuntimeError, KeyboardInterrupt) as exc:
            assert 'cleanup failure' not in str(exc),str(exc)
            if kind=='cpu':
                assert 'exit/signal 9' in str(exc),str(exc)
            if kind=='memory':
                assert 'exit/signal 1' in str(exc),str(exc)
                assert 'MemoryError' in (Path(path)/'fault.log').read_text()
            if kind in ('pin','compilation'):
                assert ('toolchain-pin' if kind=='pin' else 'schema:compile-failure') in (Path(path)/'fault.log').read_text()
            record=json.loads(Path(ready_path).read_bytes())
            assert record['pid']!=os.getpid()
            if worker:
                assert record['limits']==[[30,30],[768*1024*1024]*2,[0,0]],record
                assert 'validator' in str(exc)
            else:
                assert not Path('/proc/'+str(record['pid'])).exists(),record
            os.kill(sentinel,0)
            print(json.dumps(dict(kind=kind,result=str(exc) or 'KeyboardInterrupt',ready=True,
                                 limits_verified=worker,sentinel_alive=True)))
    finally:
        signal.alarm(0)
        os.kill(sentinel,signal.SIGKILL);os.waitpid(sentinel,0)


if __name__=='__main__':
    action, mode, path = sys.argv[1:]
    {'worker':worker_fault,'group':group_fault,'scenario':scenario}[action](mode,path)