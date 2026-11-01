//! Generate the Task 049 Rust service wrappers with the real
//! `service-generate` command, into `OUT_DIR`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `(fixture stem, OUT_DIR subdirectory, --with-codec)`.
///
/// The Task 049 services are generated exactly as before (no codec), so their
/// tests keep proving the runtime accepts a HANDWRITTEN codec. Task 050's
/// `codec-oam` is generated with `--with-codec`. Task 051 generates the
/// NON-OAM qualified services with `--with-codec`: `runtime-test` a second
/// time (as `runtime_test_codec`, beside the unchanged handwritten-codec
/// `runtime_test`), plus the Choice, inheritance, and `$type` fixtures.
/// Task 052 adds the two xs:hexBinary codec services: the flipped Task 050
/// `codec-binary` control and the `codec-hexbinary` shape matrix. Task 053
/// adds `constrained-binary`: named length-constrained xs:hexBinary carriers,
/// and `constrained-binary-sleet`, its real-Sleet-routable companion. Task
/// 054 adds `member-keywords` (OAM) and `member-keywords-qualified`
/// (`urn:test`): Record fields / Choice alternatives whose Rust identifiers
/// are escaped (`field_type`, `AlternativeSelf`) while their OMS JSON keys stay
/// the source spellings. Task 057 adds `codec-duration`: named and direct
/// unconstrained xs:duration in every occurrence shape plus a Choice. Task
/// 058 adds `codec-bounded-ascii`: named bounded-ASCII String carriers,
/// including the zero-length `EmptyType` shape, in every occurrence shape.
const SERVICES: [(&str, &str, bool); 21] = [
    ("codec-patterned-integral", "codec_patterned_integral", true),
    ("codec-unicode31", "codec_unicode31", true),
    ("codec-time-zulu", "codec_time_zulu", true),
    ("codec-ipv6", "codec_ipv6", true),
    ("runtime-test", "runtime_test", false),
    ("runtime-oam", "runtime_oam", false),
    ("codec-oam", "codec_oam", true),
    ("runtime-test", "runtime_test_codec", true),
    ("codec-choice", "codec_choice", true),
    ("codec-inherit", "codec_inherit", true),
    ("codec-shape", "codec_shape", true),
    ("codec-binary", "codec_binary", true),
    ("codec-hexbinary", "codec_hexbinary", true),
    ("constrained-binary", "constrained_binary", true),
    ("constrained-binary-sleet", "constrained_binary_sleet", true),
    ("member-keywords", "member_keywords", true),
    (
        "member-keywords-qualified",
        "member_keywords_qualified",
        true,
    ),
    ("codec-duration", "codec_duration", true),
    ("codec-bounded-ascii", "codec_bounded_ascii", true),
    ("codec-structured-ascii", "codec_structured_ascii", true),
    ("codec-alternating-ascii", "codec_alternating_ascii", true),
];

/// Task 050: the pinned UCI 2.5 root (open-arsenal/uci/standard tag v2.5,
/// commit 093610b7753944059360d3236770ab446d039556) must have exactly this
/// SHA-256 before it is used. The bytes are never committed here.
const UCI_2_5_ROOT_SHA256: &str =
    "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27";

fn main() {
    println!("cargo::rustc-check-cfg=cfg(ams_gra_real_uci)");
    println!("cargo:rerun-if-env-changed=AMS_GRA_UCI_2_5_ROOT");
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate")
        .canonicalize()
        .expect("fixture directory");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    real_uci_services(&fixtures, &out);
    for (stem, directory, with_codec) in SERVICES {
        let schema = fixtures.join(format!("{stem}.xsd"));
        let contract = fixtures.join(format!("{stem}.yaml"));
        println!("cargo:rerun-if-changed={}", schema.display());
        println!("cargo:rerun-if-changed={}", contract.display());
        let target = out.join(directory);
        let _ = std::fs::remove_dir_all(&target);
        let mut report = Vec::new();
        let mut arguments: Vec<OsString> = vec![
            "service-generate".into(),
            "--schema".into(),
            schema.into(),
            "--contract".into(),
            contract.into(),
            "--language".into(),
            "rust".into(),
            "--world".into(),
            "closed-schema".into(),
            "--output".into(),
            target.into(),
        ];
        if with_codec {
            arguments.push("--with-codec".into());
        }
        generate(stem, arguments, &mut report);
    }
    real_uci_position_report(&fixtures, &out);
}

/// RUST-OMS-001/002: independently generated release identities, using the
/// unchanged Task102/103 contracts. Ordinary builds need neither UCI release;
/// explicit qualification requires BOTH and cannot silently compile skips.
fn real_uci_services(fixtures: &Path, out: &Path) {
    println!("cargo:rerun-if-env-changed=AMS_GRA_REQUIRE_REAL_UCI_LOG");
    println!("cargo:rerun-if-env-changed=AMS_GRA_REQUIRE_REAL_UCI_RDMA");
    println!("cargo:rerun-if-env-changed=AMS_GRA_REQUIRE_REAL_UCI_MULTIMESSAGE");
    println!("cargo:rerun-if-env-changed=AMS_GRA_REQUIRE_REAL_UCI_SURVIVABILITY");
    let required = std::env::var_os("AMS_GRA_REQUIRE_REAL_UCI_LOG").is_some()
        || std::env::var_os("AMS_GRA_REQUIRE_REAL_UCI_RDMA").is_some()
        || std::env::var_os("AMS_GRA_REQUIRE_REAL_UCI_MULTIMESSAGE").is_some()
        || std::env::var_os("AMS_GRA_REQUIRE_REAL_UCI_SURVIVABILITY").is_some();
    for (version, root_hash, include_hash) in [
        (
            "5",
            UCI_2_5_ROOT_SHA256,
            "4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b",
        ),
        (
            "6",
            "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b",
            "ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88",
        ),
    ] {
        let variable = format!("AMS_GRA_UCI_2_{version}_ROOT");
        for message in ["log", "rdma", "multimessage", "survivability"] {
            println!("cargo::rustc-check-cfg=cfg(ams_gra_real_uci_{message}2{version})");
        }
        println!("cargo:rerun-if-env-changed={variable}");
        let Some(root) = std::env::var_os(&variable) else {
            assert!(
                !required,
                "{variable} is required for pinned real-UCI qualification"
            );
            continue;
        };
        let root = PathBuf::from(root).canonicalize().expect("real UCI root");
        verify_hash(&root, root_hash);
        verify_hash(
            &root
                .parent()
                .unwrap()
                .join(format!("UCI_SecurityMarkings_v2_{version}_0.xsd")),
            include_hash,
        );
        for (message, task) in [
            ("log", "task102"),
            ("rdma", "task103"),
            ("multimessage", "rust-oms-003"),
            ("survivability", "rust-oms-004"),
        ] {
            let cfg = format!("ams_gra_real_uci_{message}2{version}");
            let contract = fixtures.join(format!("../{task}/uci2{version}-service.yaml"));
            println!("cargo:rerun-if-changed={}", contract.display());
            let target = out.join(format!("real_uci_{message}2{version}"));
            let plain = out.join(format!("real_uci_{message}2{version}_no_codec"));
            let common: Vec<OsString> = vec![
                "--schema".into(),
                root.clone().into(),
                "--contract".into(),
                contract.into(),
                "--language".into(),
                "rust".into(),
                "--world".into(),
                "closed-schema".into(),
            ];
            let mut check = vec!["service-check".into()];
            check.extend(common.clone());
            check.push("--with-codec".into());
            let mut readiness = Vec::new();
            generate(message, check, &mut readiness);
            if message == "survivability" {
                let report = String::from_utf8_lossy(&readiness);
                for required in [
                    "selected oms messages: 3",
                    "renderable selected oms messages: 3",
                    "status: READY",
                    "codec status: READY",
                ] {
                    assert!(report.lines().any(|line| line == required), "{report}");
                }
            }
            std::fs::create_dir_all(&target).expect("real-UCI output");
            std::fs::write(target.join("readiness.txt"), readiness).expect("readiness report");
            for (directory, codec) in [(&plain, false), (&target, true)] {
                let mut arguments = vec!["service-generate".into()];
                arguments.extend(common.clone());
                arguments.extend(["--output".into(), directory.clone().into()]);
                if codec {
                    arguments.push("--with-codec".into());
                }
                generate(message, arguments, &mut Vec::new());
            }
            // The codec must reuse the exact model and endpoint surface, with
            // only its sibling-module declaration added to service_api.rs.
            for entry in std::fs::read_dir(&plain).expect("no-codec output") {
                let path = entry.expect("generated file").path();
                if path.extension().is_some_and(|extension| extension == "rs") {
                    let name = path.file_name().unwrap();
                    let expected = std::fs::read_to_string(&path).expect("plain source");
                    let actual = std::fs::read_to_string(target.join(name)).expect("codec source");
                    let actual = if name == "service_api.rs" {
                        actual.replace(
                            "#[path = \"service_codec.rs\"]\npub mod service_codec;\n\n",
                            "",
                        )
                    } else {
                        actual
                    };
                    assert_eq!(
                        actual,
                        expected,
                        "{message} model/API parity: {}",
                        path.display()
                    );
                }
            }
            println!("cargo:rustc-cfg={cfg}");
        }
        println!("cargo:rustc-env={variable}_BUILD={}", root.display());
    }
}

fn verify_hash(path: &Path, expected: &str) {
    println!("cargo:rerun-if-changed={}", path.display());
    let digest = Command::new("sha256sum")
        .arg(path)
        .output()
        .expect("sha256sum");
    assert!(digest.status.success(), "cannot hash {}", path.display());
    assert_eq!(
        String::from_utf8_lossy(&digest.stdout)
            .split_whitespace()
            .next(),
        Some(expected),
        "{} is not a pinned UCI input",
        path.display()
    );
}

fn generate(label: &str, arguments: Vec<OsString>, report: &mut Vec<u8>) {
    if let Err(error) = ams_gra_codegen_oms::run(arguments, report) {
        panic!(
            "service-generate {label} failed: {error}\n{}",
            String::from_utf8_lossy(report)
        );
    }
}

/// Task 050: when `AMS_GRA_UCI_2_5_ROOT` names the pinned UCI 2.5 root,
/// verify its SHA-256, generate the REAL PositionReport model + service API +
/// codec with `--with-codec`, and enable `cfg(ams_gra_real_uci)`. Unset means
/// the real-UCI tests are compiled out (and say so); a wrong hash is a hard
/// build failure, never a silent skip.
fn real_uci_position_report(fixtures: &Path, out: &Path) {
    let Some(root) = std::env::var_os("AMS_GRA_UCI_2_5_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let root_display = root.display().to_string();
    println!("cargo:rerun-if-changed={}", root.display());
    let digest = Command::new("sha256sum")
        .arg(&root)
        .output()
        .expect("sha256sum is required to verify AMS_GRA_UCI_2_5_ROOT");
    let digest = String::from_utf8_lossy(&digest.stdout);
    assert_eq!(
        digest.split_whitespace().next(),
        Some(UCI_2_5_ROOT_SHA256),
        "AMS_GRA_UCI_2_5_ROOT {} is not the pinned UCI 2.5 root",
        root.display()
    );
    // Task 050: PositionReport. Task 052: SubsystemStream, the smallest real
    // UCI 2.5 message whose closure has a hexBinary value and whose only
    // earlier codec blocker was Binary.
    for (contract, directory, label) in [
        (
            "position-report-loop.yaml",
            "real_uci_position_report",
            "real UCI PositionReport",
        ),
        (
            "subsystem-stream-loop.yaml",
            "real_uci_subsystem_stream",
            "real UCI SubsystemStream",
        ),
        // Task 058: AMTI_SettingsCommand, the smallest real message made
        // READY by the bounded-ASCII String profiles (its `EmptyType`).
        (
            "real-bounded-ascii-amti-settings.yaml",
            "real_uci_amti_settings",
            "real UCI AMTI_SettingsCommand",
        ),
        (
            "real-structured-ascii-file-metadata.yaml",
            "real_uci_file_metadata",
            "real UCI FileMetadata",
        ),
    ] {
        let contract = fixtures.join(contract);
        println!("cargo:rerun-if-changed={}", contract.display());
        let target = out.join(directory);
        let _ = std::fs::remove_dir_all(&target);
        let arguments: Vec<OsString> = vec![
            "service-generate".into(),
            "--schema".into(),
            root.clone().into(),
            "--contract".into(),
            contract.into(),
            "--language".into(),
            "rust".into(),
            "--world".into(),
            "closed-schema".into(),
            "--output".into(),
            target.into(),
            "--with-codec".into(),
        ];
        generate(label, arguments, &mut Vec::new());
    }
    println!("cargo:rustc-cfg=ams_gra_real_uci");
    // The test hands Sleet the SAME verified root the model was generated from.
    println!(
        "cargo:rustc-env=AMS_GRA_UCI_2_5_ROOT_BUILD={}",
        root_display
    );
}
