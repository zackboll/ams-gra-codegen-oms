# Public References Used for the Initial Architecture

Checked: **2026-09-17**.

## AMS GRA Hello World Starter Kit

### Overview and Terminology

https://open-arsenal.gitlab.io/ams-gra/hello-world-sk/getting-started/tutorials/0-overview.html

Relevant architectural points:

- CAL is the required software API boundary protecting an OMS Service from the underlying network transport.
- LA-CAL uses WebSockets and JSON for language-agnostic access.
- ASB is the logical UCI messaging network.
- Sleet is used as LA-CAL infrastructure in the Starter Kit ecosystem.
- MEL/high-rate sensor access and CAL/UCI messaging are different paths.

### How to Build an OMS Service

https://open-arsenal.gitlab.io/ams-gra/hello-world-sk/getting-started/tutorials/1-build-oms-service.html

Relevant implementation points:

- the tutorial connects to Sleet with a WebSocket;
- the endpoint shown is `/owp`;
- examples explicitly construct `INIT`, `SUB`, and `PUB` OWP frames;
- UCI data is constructed/parsing as JSON in the language-agnostic example;
- the tutorial describes CAL as protecting services from the ASB's underlying transport.

This is the primary evidence for the statement that the public Starter Kit currently demonstrates manual language-agnostic integration rather than a shared multi-language UCI code-generation workflow.

### Starter Kit Architecture

https://open-arsenal.gitlab.io/ams-gra/hello-world-sk/getting-started/architecture.html

Relevant points:

- Sleet is described as an OMS LA-CAL implementation using WebSocket + OWP for UCI messaging.
- high-rate sensor data is kept on separate MEL/data paths.

## UCI Standard

https://gitlab.com/open-arsenal/uci/standard

The UCI standard repository is the authoritative upstream source for the UCI schema and related specification material consumed conceptually by this project.

This project should not silently fork or redefine that model.

The repository's `v2.6` tag is the current public UCI release. Its `v2.5` tag
remains relevant to this project's interoperability baseline because the public
AMS GRA Hello World tutorial initializes Sleet with schema identifier
`002.5.0`. “Latest public UCI” and “Sleet interoperability baseline” are
therefore tracked as distinct compatibility targets; adopting 2.6 does not
silently replace the 2.5 baseline. See [UCI Schema
Compatibility](uci-schema-compatibility.md) for dated frontend probe results.

## OMS Standard

https://gitlab.com/open-arsenal/oms/standard

The OMS standard repository is the authoritative upstream source for CAL and related OMS specification material.

## Rust CAL (`rcal`)

https://docs.rs/rcal/latest/rcal/

https://docs.rs/rcal/latest/rcal/uci/index.html

Relevant points:

- the crate separates CAL-facing interfaces from an Abstract Service Bus layer;
- its UCI module documents generated message types;
- its UCI `types` module is described as generated from XSD by the build script;
- it references the OMS CAL and C++ CAL Interface Generation specifications.

This demonstrates that schema-driven generation is compatible with the OMS ecosystem and provides useful prior art, while this repository's objective is broader: one normalized IR feeding several language backends.

## Scope of the “missing codegen” statement

The architecture documents intentionally say:

> The current public AMS GRA Hello World Starter Kit does not present a shared general multi-language generator from UCI XSD to typed LA-CAL bindings.

They do **not** say:

> OMS has no code-generation capability.

The latter would be too broad and is contradicted by current generated Rust UCI types and language-specific OMS generation specifications.
