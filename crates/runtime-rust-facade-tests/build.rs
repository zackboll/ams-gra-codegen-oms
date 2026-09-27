//! Generate the Task 049 Rust service wrappers with the real
//! `service-generate` command, into `OUT_DIR`.

use std::ffi::OsString;
use std::path::PathBuf;

/// `(fixture stem, OUT_DIR subdirectory)`.
const SERVICES: [(&str, &str); 2] = [
    ("runtime-test", "runtime_test"),
    ("runtime-oam", "runtime_oam"),
];

fn main() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate")
        .canonicalize()
        .expect("fixture directory");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    for (stem, directory) in SERVICES {
        let schema = fixtures.join(format!("{stem}.xsd"));
        let contract = fixtures.join(format!("{stem}.yaml"));
        println!("cargo:rerun-if-changed={}", schema.display());
        println!("cargo:rerun-if-changed={}", contract.display());
        let target = out.join(directory);
        let _ = std::fs::remove_dir_all(&target);
        let mut report = Vec::new();
        let arguments: Vec<OsString> = vec![
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
        if let Err(error) = ams_gra_codegen_oms::run(arguments, &mut report) {
            panic!(
                "service-generate {stem} failed: {error}\n{}",
                String::from_utf8_lossy(&report)
            );
        }
    }
}
