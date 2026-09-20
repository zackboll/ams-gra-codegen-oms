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
| Service Contract authority | OMS Service Contract material | Unchanged; consumed, never authored here |
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

## Portable Service Contract compatibility

A second external format is now consumed: the portable AMS GRA Service
Contract published by `zackboll/ams-gra-service-contract`.

```text
Portable Service Contract compatibility baseline:
zackboll/ams-gra-service-contract
4ea5be8dd36e9695bd58f2c36c6b3dd8075de249

contract_version: 0.1
```

The two projects are **not commit-lockstep**. Compatibility is defined by
`contract_version`, not repository SHA equality. A contract declaring any
other version is rejected rather than parsed on a best-effort basis.

What this does **not** change:

- OMS profile conformance and contract completion remain owned by the Service
  Contract project. No profile engine, worksheet, decisions, mapping, or
  scaffold logic was ported into this repository;
- an already-authored contract is consumed as-is. Nothing here infers
  Capabilities, groups functions, generates topics, or regenerates Section 3.3
  topology;
- backend generation behavior is unchanged. Task 030 adds a read-only planning
  path; it generates no service wrappers.

See `docs/service-contract-integration.md`.
