"""Explicit pinned broker campaign. Not part of Fast/mock discovery."""
import asyncio
import importlib
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

from oms_lacal import CodecBinding, CodecRegistry, LacalRuntime, RuntimeConfig, ServerRejection
from support import ROOT, OAM


class PinnedSleetTests(unittest.IsolatedAsyncioTestCase):
    async def test_generated_codecs_route_policy_and_typed_delivery(self):
        binary = os.environ.get('AMS_GRA_SLEET_BIN')
        self.assertTrue(binary and Path(binary).is_file(), 'explicit pinned Sleet executable is required; no skip')
        with TemporaryDirectory(prefix='py005-broker-') as temp:
            path = Path(temp)
            generated = path / 'generated'
            schema = ROOT / 'tests/fixtures/python-runtime/oam.xsd'
            generator = os.environ.get('AMS_GRA_GENERATOR', str(ROOT/'target/debug/ams-gra-codegen-oms'))
            subprocess.run([generator,'service-generate','--schema',str(schema),'--contract',
                str(ROOT/'tests/fixtures/python-runtime/service.yaml'),'--language','python',
                '--world','closed-schema','--with-codec','--output',str(generated)],check=True,capture_output=True)
            sys.path.insert(0,str(path))
            model=importlib.import_module('generated.model')
            codec=importlib.import_module('generated.service_codec')
            api=importlib.import_module('generated.service_api')
            registry=CodecRegistry(CodecBinding(OAM,name,model.Payload,codec.encode_Payload,codec.decode_Payload,codec.CodecError)
                for name in ['RuntimeNotice','RuntimeOther'])
            services=path/'services.d';services.mkdir()
            (services/'python.toml').write_text('service_id="python-runtime"\n'
                'service_uuid="550e8400-e29b-41d4-a716-446655440005"\n'
                'allowed_topics=["runtime.loop"]\n[[topic_bindings]]\ntopic="runtime.loop"\nallowed_messages=["RuntimeNotice"]\n')
            with socket.socket() as sock:
                sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
            config=path/'sleet.toml'
            config.write_text(f'bind_addr="127.0.0.1:{port}"\nserver_id="python-runtime-test"\n'
                f'system_label="Python Runtime"\nschema_path={json.dumps(str(schema))}\n'
                'schema_version="000.1.0"\nsystem_uuid="550e8400-e29b-41d4-a716-446655440000"\n'
                f'services_dir={json.dumps(str(services))}\n')
            with (path/'broker.log').open('w+') as log:
                broker=subprocess.Popen([binary,str(config)],stdout=log,stderr=log,env={**os.environ,'RUST_LOG':'sleet=debug'})
                try:
                    # Test harness startup readiness only; runtime never retries.
                    async with asyncio.timeout(15):
                        while True:
                            self.assertIsNone(broker.poll(),'pinned broker exited during startup')
                            try:
                                reader,writer=await asyncio.open_connection('127.0.0.1',port)
                                writer.close();await writer.wait_closed();break
                            except ConnectionRefusedError:
                                await asyncio.sleep(.05)
                    runtime_config=RuntimeConfig(f'ws://127.0.0.1:{port}','python-runtime','000.1.0')
                    # Separate actual connections: subscriber and publisher.
                    async with LacalRuntime(runtime_config,registry) as subscriber, LacalRuntime(runtime_config,registry) as publisher:
                        received=asyncio.Queue()
                        handle=await api.function_loop.exchange_input.subscribe(subscriber,received.put_nowait)
                        value=model.Payload(count=model.Count(42),flag=True,text='Python λ')
                        await api.function_loop.exchange_output.publish(publisher,value)
                        self.assertEqual(await asyncio.wait_for(received.get(),5),value)
                        for name,topic in [('RuntimeNotice','unauthorized'),('RuntimeOther','runtime.loop')]:
                            with self.assertRaises(ServerRejection) as raised:
                                await publisher.publish(message_namespace=OAM,message_name=name,topic=topic,payload=value)
                            self.assertEqual(raised.exception.code,'Illegal-State')
                            await api.function_loop.exchange_output.publish(publisher,value)
                            self.assertEqual(await asyncio.wait_for(received.get(),5),value)
                        for name,topic in [('RuntimeNotice','unauthorized'),('RuntimeOther','runtime.loop')]:
                            with self.assertRaises(ServerRejection):
                                await subscriber.subscribe(message_namespace=OAM,message_name=name,topic=topic,
                                                           subscription_group=None,handler=received.put_nowait)
                        await api.function_loop.exchange_output.publish(publisher,value)
                        self.assertEqual(await asyncio.wait_for(received.get(),5),value)
                        await handle.unsubscribe()
                        self.assertFalse(handle.active)
                    # +OK publication acceptance plus MSG delivery already proves
                    # broker receipt; independently retain its own route marker.
                    log.flush();log.seek(0);evidence=log.read()
                    self.assertIn('route PUB message',evidence)
                    self.assertIn('RuntimeNotice',evidence)
                finally:
                    broker.terminate()
                    try: broker.wait(timeout=5)
                    except subprocess.TimeoutExpired: broker.kill();broker.wait(timeout=5)
            leaked=[t for t in asyncio.all_tasks() if t is not asyncio.current_task()
                    and t.get_name().startswith('oms-lacal-') and not t.done()]
            self.assertEqual(leaked,[])
            print('PY005 PINNED SLEET RECEIPT AND TYPED DELIVERY: PASSED',flush=True)


if __name__=='__main__': unittest.main()