from typing import Callable as _Callable, Protocol as _Protocol, TypeVar as _TypeVar

_PublishPayload = _TypeVar('_PublishPayload', contravariant=True)
_SubscribePayload = _TypeVar('_SubscribePayload', covariant=True)
_Result = _TypeVar('_Result', covariant=True)


class PublishAdapter(_Protocol[_PublishPayload, _Result]):
    def publish(self, *, message_namespace: str, message_name: str, topic: str,
                payload: _PublishPayload) -> _Result: ...


class SubscribeAdapter(_Protocol[_SubscribePayload, _Result]):
    def subscribe(self, *, message_namespace: str, message_name: str, topic: str,
                  subscription_group: str | None,
                  handler: _Callable[[_SubscribePayload], None]) -> _Result: ...
