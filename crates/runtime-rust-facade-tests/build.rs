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
const SERVICES: [(&str, &str, bool); 7] = [
    ("runtime-test", "runtime_test", false),
    ("runtime-oam", "runtime_oam", false),
    ("codec-oam", "codec_oam", true),
    ("runtime-test", "runtime_test_codec", true),
    ("codec-choice", "codec_choice", true),
    ("codec-inherit", "codec_inherit", true),
    ("codec-shape", "codec_shape", true),
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
    let contract = fixtures.join("position-report-loop.yaml");
    println!("cargo:rerun-if-changed={}", contract.display());
    let target = out.join("real_uci_position_report");
    let _ = std::fs::remove_dir_all(&target);
    let arguments: Vec<OsString> = vec![
        "service-generate".into(),
        "--schema".into(),
        root.into(),
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
    generate("real UCI PositionReport", arguments, &mut Vec::new());
    println!("cargo:rustc-cfg=ams_gra_real_uci");
    // The test hands Sleet the SAME verified root the model was generated from.
    println!(
        "cargo:rustc-env=AMS_GRA_UCI_2_5_ROOT_BUILD={}",
        root_display
    );
}
