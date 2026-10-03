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

/// Task 050: generated with `--with-codec`, so this crate root also mounts
/// the GENERATED `service_codec` module beside `model` and `service_api`.
#[allow(clippy::all, clippy::pedantic)]
pub mod codec_oam {
    include!(concat!(env!("OUT_DIR"), "/codec_oam/service_api.rs"));
}

/// Task 051: the SAME `runtime-test.xsd` (namespace `urn:test`,
/// `elementFormDefault="qualified"`) generated with `--with-codec`. Its member
/// keys are Clark notation, `"{urn:test}Count"`. `runtime_test` above is
/// unchanged and still uses the handwritten codec.
#[allow(clippy::all, clippy::pedantic)]
pub mod runtime_test_codec {
    include!(concat!(
        env!("OUT_DIR"),
        "/runtime_test_codec/service_api.rs"
    ));
}

/// Task 051: a qualified non-OAM Choice (`urn:choice`).
#[allow(clippy::all, clippy::pedantic)]
pub mod codec_choice {
    include!(concat!(env!("OUT_DIR"), "/codec_choice/service_api.rs"));
}

/// Task 051: a qualified non-OAM inherited Record (`urn:inherit`).
#[allow(clippy::all, clippy::pedantic)]
pub mod codec_inherit {
    include!(concat!(env!("OUT_DIR"), "/codec_inherit/service_api.rs"));
}

/// Task 051: a qualified non-OAM closed abstract value (`urn:shape`).
#[allow(clippy::all, clippy::pedantic)]
pub mod codec_shape {
    include!(concat!(env!("OUT_DIR"), "/codec_shape/service_api.rs"));
}

/// Task 052: the Task 050 `codec-binary` control (one direct `xs:hexBinary`
/// field), whose codec flipped from NOT READY to READY.
#[allow(clippy::all, clippy::pedantic)]
pub mod codec_binary {
    include!(concat!(env!("OUT_DIR"), "/codec_binary/service_api.rs"));
}

/// Task 052: every supported hexBinary shape (required/optional/bounded/
/// unbounded direct, named, named-on-named, Choice) in the OAM namespace.
#[allow(clippy::all, clippy::pedantic)]
pub mod codec_hexbinary {
    include!(concat!(env!("OUT_DIR"), "/codec_hexbinary/service_api.rs"));
}

/// Task 053: named length-constrained xs:hexBinary carriers (exact, min,
/// max, min+max, derived, zero-length, three-level ancestry) in every
/// occurrence shape, in the OAM namespace.
#[allow(clippy::all, clippy::pedantic)]
pub mod constrained_binary {
    include!(concat!(
        env!("OUT_DIR"),
        "/constrained_binary/service_api.rs"
    ));
}

/// Task 053: the real-Sleet companion of `constrained_binary`: only shapes
/// where some legal OCTET counts also satisfy pinned Sleet's CHARACTER-count
/// facet check, so a legal document can route.
#[allow(clippy::all, clippy::pedantic)]
pub mod constrained_binary_sleet {
    include!(concat!(
        env!("OUT_DIR"),
        "/constrained_binary_sleet/service_api.rs"
    ));
}

/// Task 054: OAM Record fields / Choice alternatives whose Rust identifiers
/// are escaped (`field_type`, `field_self`, `AlternativeSelf`) while the OMS
/// JSON member keys stay `"Type"`, `"Self"`.
#[allow(clippy::all, clippy::pedantic)]
pub mod member_keywords {
    include!(concat!(env!("OUT_DIR"), "/member_keywords/service_api.rs"));
}

/// Task 054: the qualified NON-OAM companion (`urn:test`): escaped Rust
/// members, Clark-notation keys `"{urn:test}Type"`, `"{urn:test}Self"`.
#[allow(clippy::all, clippy::pedantic)]
pub mod member_keywords_qualified {
    include!(concat!(
        env!("OUT_DIR"),
        "/member_keywords_qualified/service_api.rs"
    ));
}

/// Task 057: named and direct unconstrained xs:duration (required, optional,
/// bounded, unbounded, Choice) in the OAM namespace, with the generated codec.
#[allow(clippy::all, clippy::pedantic)]
pub mod codec_duration {
    include!(concat!(env!("OUT_DIR"), "/codec_duration/service_api.rs"));
}

/// Task 058: named bounded-ASCII String carriers (including the zero-length
/// `EmptyType` shape) in every occurrence shape and a Choice, OAM namespace,
/// with the generated codec.
#[allow(clippy::all, clippy::pedantic)]
pub mod codec_bounded_ascii {
    include!(concat!(
        env!("OUT_DIR"),
        "/codec_bounded_ascii/service_api.rs"
    ));
}

/// Task 059: pinned deterministic structured-ASCII String carriers and codec.
#[allow(clippy::all, clippy::pedantic)]
pub mod codec_structured_ascii {
    include!(concat!(
        env!("OUT_DIR"),
        "/codec_structured_ascii/service_api.rs"
    ));
}

/// Task 050: the REAL UCI 2.5 PositionReport model, service API, and codec,
/// generated only when `AMS_GRA_UCI_2_5_ROOT` names the SHA-256-verified
/// pinned root (see build.rs).
#[cfg(ams_gra_real_uci)]
#[allow(clippy::all, clippy::pedantic)]
pub mod real_uci_position_report {
    include!(concat!(
        env!("OUT_DIR"),
        "/real_uci_position_report/service_api.rs"
    ));
}

/// Task 052: the REAL UCI 2.5 SubsystemStream model, service API, and codec
/// (optional direct `xs:hexBinary` `SubsystemStreamBinary`), generated only
/// with the SHA-256-verified pinned root (see build.rs).
/// Task 058: the REAL UCI 2.5 AMTI_SettingsCommand model, service API, and
/// codec (optional `UnassignAll : EmptyType`), generated only when
/// `AMS_GRA_UCI_2_5_ROOT` names the SHA-256-verified pinned root.
#[cfg(ams_gra_real_uci)]
#[allow(clippy::all, clippy::pedantic)]
pub mod real_uci_amti_settings {
    include!(concat!(
        env!("OUT_DIR"),
        "/real_uci_amti_settings/service_api.rs"
    ));
}

/// Task 059: REAL pinned FileMetadata, whose FileNameType is structured ASCII.
#[cfg(ams_gra_real_uci)]
#[allow(clippy::all, clippy::pedantic)]
pub mod real_uci_file_metadata {
    include!(concat!(
        env!("OUT_DIR"),
        "/real_uci_file_metadata/service_api.rs"
    ));
}

#[cfg(ams_gra_real_uci)]
#[allow(clippy::all, clippy::pedantic)]
pub mod real_uci_subsystem_stream {
    include!(concat!(
        env!("OUT_DIR"),
        "/real_uci_subsystem_stream/service_api.rs"
    ));
}

pub mod codec;
