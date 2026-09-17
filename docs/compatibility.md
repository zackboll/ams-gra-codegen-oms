# Compatibility: What This Project Changes and What It Does Not

## Compatibility promise

The initial architecture is intentionally **client-side only** from Sleet's perspective.

A generated client and a handwritten client should be indistinguishable at the LA-CAL boundary when given the same service identity, schema version, subscriptions, topics, and UCI payloads.

## Unchanged external interface

| Concern | Existing architecture | With `ams-gra-codegen-oms` |
|---|---|---|
| CAL pattern | LA-CAL | LA-CAL |
| Server | Sleet | Sleet |
| Connection | WebSocket | WebSocket |
| Protocol | OWP | OWP |
| UCI payload | JSON | JSON |
| Authoritative schema | UCI/OMS XSD | UCI/OMS XSD |
| ASB abstraction | Preserved | Preserved |
| Service registration | Existing Sleet/deployment configuration | Same |
| Handwritten clients | Supported | Still supported |
| MEL/high-rate data | Separate path | Unchanged |

## New internal build-time path

```text
OLD
---
application source
    |
manual strings / JSON models
    |
OWP + JSON

NEW
---
UCI XSD
    |
schema IR
    |
generated typed source
    |
small runtime
    |
OWP + JSON
```

## Interoperability acceptance criterion

For an initial generated client to be considered successful, it must connect to an **unmodified** Sleet instance and complete the same externally observable sequence as the reference/manual client:

1. WebSocket connection;
2. valid OWP initialization;
3. subscription creation;
4. receipt and decoding of a UCI message;
5. creation and validation of a typed UCI message;
6. JSON encoding;
7. publication through OWP;
8. unsubscribe/disconnect behavior.

## No automatic compliance claim

Generation can reduce implementation mistakes, but it does not by itself make an application OMS- or AMS-GRA-compliant. Service contracts, configuration, policy, deployment artifacts, security requirements, and conformance testing still apply.
