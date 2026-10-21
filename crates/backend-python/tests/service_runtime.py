"""Generated code qualification: unittest assertions remain active under -O."""
import importlib
from pathlib import Path
import py_compile
import sys
from typing import get_type_hints, get_args
import unittest

directory = Path(sys.argv.pop(1)).resolve()
package = sys.argv.pop(1)
if sys.version_info < (3, 11):
    raise RuntimeError('Python 3.11+ required')
for source in directory.glob('*.py'):
    py_compile.compile(str(source), doraise=True)
if package == 'package':
    sys.path.insert(0, str(directory.parent))
    api = importlib.import_module(directory.name + '.service_api')
    model = importlib.import_module(directory.name + '.model')
else:
    sys.path.insert(0, str(directory))
    api = importlib.import_module('service_api')
    model = importlib.import_module('model')


class ServiceTests(unittest.TestCase):
    def setUp(self):
        self.function = api.function_schema_class
        self.pub = self.function.exchange_input
        self.sub = self.function.exchange_output
        self.value = model.Payload(id=model.Identifier(42), state=model.State.active,
                                   flags=(True, False))

    def test_metadata_and_order(self):
        self.assertEqual(api.SERVICE_NAME, 'Python "Demo" λ')
        self.assertEqual(api.SERVICE_VERSION, '0.3.0')
        self.assertEqual(api.SERVICE_KIND, 'service')
        self.assertEqual([f.ID for f in api.FUNCTIONS], ['class', 'final'])
        self.assertEqual(self.function.NAME, 'Mission "Data" λ')
        self.assertEqual([e.ID for e in self.function.EXCHANGES],
                         ['input', 'output', 'repeated', 'same-payload'])
        self.assertEqual(self.pub.KIND, 'oms_message')
        self.assertEqual(self.pub.DIRECTION, 'output')
        self.assertEqual(self.pub.MANDATE, 'mandatory')
        self.assertEqual(self.sub.DIRECTION, 'input')
        self.assertEqual(self.sub.MANDATE, 'optional')
        self.assertEqual(self.sub.TOPIC, 'incoming.topic')
        self.assertEqual(self.sub.SUBSCRIPTION_GROUP, 'workers-λ')
        self.assertEqual(self.pub.MESSAGE_NAMESPACE, 'urn:test:python-service')
        self.assertEqual(self.pub.MESSAGE_NAME, 'SelectedReport')
        self.assertIsNone(self.pub.SUBSCRIPTION_GROUP)

    def test_payload_typing(self):
        self.assertIs(self.pub.Payload, model.Payload)
        self.assertIs(get_type_hints(self.pub.publish)['payload'], model.Payload)
        self.assertEqual(get_args(get_type_hints(self.pub.publish)['adapter'])[0],
                         model.Payload)
        handler = get_type_hints(self.sub.subscribe)['handler']
        self.assertEqual(get_args(handler), ([model.Payload], type(None)))
        self.assertIs(self.sub.Payload, self.pub.Payload)
        self.assertIs(api.function_final.exchange_input.Payload, model.Payload)
        self.assertFalse(hasattr(self.pub, 'subscribe'))
        self.assertFalse(hasattr(self.sub, 'publish'))

    def test_publish_forwards_identity_not_payload_identity(self):
        calls = []
        token = object()

        class Mock:
            def publish(self, **kwargs):
                calls.append(kwargs)
                return token

        for endpoint, name in [(self.pub, 'SelectedReport'),
                               (self.function.exchange_same_payload, 'OtherReport')]:
            self.assertIs(endpoint.publish(Mock(), self.value), token)
            self.assertEqual(calls[-1], dict(message_namespace='urn:test:python-service',
                                            message_name=name, topic='common.topic',
                                            payload=self.value))
            self.assertIs(calls[-1]['payload'], self.value)

    def test_subscribe_forwards_handler_and_does_not_create_handle(self):
        calls = []
        received = []
        token = object()
        handler = received.append

        class Mock:
            def subscribe(self, **kwargs):
                calls.append(kwargs)
                return token

        for endpoint, topic, group in [
            (self.sub, 'incoming.topic', 'workers-λ'),
            (self.function.exchange_repeated, 'second.topic', None),
            (api.function_final.exchange_input, 'final.topic', None),
        ]:
            self.assertIs(endpoint.subscribe(Mock(), handler), token)
            self.assertEqual(calls[-1], dict(message_namespace='urn:test:python-service',
                                            message_name='SelectedReport', topic=topic,
                                            subscription_group=group, handler=handler))
            self.assertIs(calls[-1]['handler'], handler)
        self.assertEqual(received, [])  # The facade does not dispatch.
        calls[0]['handler'](self.value)
        self.assertEqual(received, [self.value])

    def test_adapter_exceptions_propagate(self):
        class Mock:
            def publish(self, **kwargs):
                raise RuntimeError('adapter failure')

            def subscribe(self, **kwargs):
                raise RuntimeError('adapter failure')

        with self.assertRaisesRegex(RuntimeError, 'adapter failure'):
            self.pub.publish(Mock(), self.value)
        with self.assertRaisesRegex(RuntimeError, 'adapter failure'):
            self.sub.subscribe(Mock(), lambda value: None)


if __name__ == '__main__':
    unittest.main()