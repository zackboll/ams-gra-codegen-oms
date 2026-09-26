//! Task 049 test-only consumer of GENERATED Rust service APIs.
//!
//! Each module below is a `service-generate` output (see `build.rs`),
//! included verbatim. Its public shape is exactly what an application sees:
//! `<service>::service_api::{PublishAdapter, SubscribeAdapter}` plus one
//! `publish` / `subscribe` per OMS endpoint. [`codec::TestCodec`] is a
//! handwritten payload codec for the fixture types; it is evidence for the
//! `OmsJsonCodec` boundary, not generated codec output.

/// `runtime-test.xsd` / `runtime-test.yaml`: namespace `urn:test`.
#[allow(clippy::all, clippy::pedantic)]
pub mod runtime_test {
    include!(concat!(env!("OUT_DIR"), "/runtime_test/service_api.rs"));
}

/// `runtime-oam.xsd` / `runtime-oam.yaml`: the OAM namespace.
#[allow(clippy::all, clippy::pedantic)]
pub mod runtime_oam {
    include!(concat!(env!("OUT_DIR"), "/runtime_oam/service_api.rs"));
}

pub mod codec;
