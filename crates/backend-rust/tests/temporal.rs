//! Task 036: the generated Rust DateTime Zulu carrier, compiled and run.
//!
//! Asserting on generated *text* alone would not prove the emitted validator
//! behaves the way XML Schema requires, so every case in the shared corpus is
//! executed against a real compiled program.

mod common;

use ams_gra_oms_backend_rust::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn temporal_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-temporal-datetime.xsd"),
    )
    .expect("temporal fixture should parse")
}

/// The generated API shape, asserted before anything is run.
#[test]
fn generated_date_time_api_is_an_opaque_validated_carrier() {
    let source = generate(&temporal_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("temporal fixture must generate");

    // A validated lexical carrier, not a timestamp.
    assert!(source.contains("pub struct Instant {\n    lexical: String,\n}"));
    assert!(source.contains("pub fn new(value: &str) -> Option<Self>"));
    assert!(source.contains("pub fn as_str(&self) -> &str"));

    // The storage field is private: no `pub lexical`.
    assert!(!source.contains("pub lexical"));

    // No equality or ordering is claimed for the carrier. Two distinct legal
    // spellings can denote one XML Schema value, so deriving these would
    // publish a semantics Task 036 has not implemented.
    let carrier = source
        .split("pub struct Instant {")
        .next()
        .expect("Instant must be generated");
    let derive = carrier
        .rsplit("#[derive(")
        .next()
        .expect("Instant must carry a derive");
    assert!(!derive.contains("PartialEq"), "derive was: {derive}");
    assert!(!derive.contains("Eq"), "derive was: {derive}");
    assert!(!derive.contains("Ord"), "derive was: {derive}");
    assert!(!source.contains("impl PartialEq for Instant"));

    // No third-party temporal or regex dependency is introduced, and no epoch
    // or calendar-library representation is used.
    for forbidden in [
        "chrono",
        "time::",
        "regex",
        "SystemTime",
        "UNIX_EPOCH",
        "Duration",
    ] {
        assert!(
            !source.contains(forbidden),
            "generated Rust must not reference {forbidden}"
        );
    }

    // Task 034 composition: the optional named occurrence is the ordinary
    // `Option<Instant>`, with no DateTime-specific optional path.
    assert!(source.contains("pub timestamp: Option<Instant>,"));
    assert!(source.contains("pub observed: Instant,"));
    assert!(source.contains("pub expires: Option<Deadline>,"));

    // A record holding a carrier cannot derive equality either.
    assert!(source.contains("#[derive(Debug, Clone)]\npub struct Payload {"));
}

/// Every shared-corpus case, executed against the compiled generated module.
#[test]
fn generated_date_time_validator_matches_the_shared_corpus() {
    let source = generate(&temporal_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("temporal fixture must generate");
    let cases = common::load_cases();

    let mut probe = String::from("include!(\"generated.rs\");\n\nfn main() {\n");
    for case in &cases {
        let input = common::escape_for_source(&case.input);
        match &case.expected {
            Some(normalized) => {
                let normalized = common::escape_for_source(normalized);
                writeln!(
                    probe,
                    "    match Instant::new(\"{input}\") {{\n\
                     \x20       Some(value) => assert_eq!(value.as_str(), \"{normalized}\", \"input {input}\"),\n\
                     \x20       None => panic!(\"must accept: {input}\"),\n\
                     \x20   }}"
                )
                .expect("writing to String cannot fail");
            }
            None => {
                writeln!(
                    probe,
                    "    assert!(Instant::new(\"{input}\").is_none(), \"must reject: {input}\");"
                )
                .expect("writing to String cannot fail");
            }
        }
    }
    probe.push_str("    println!(\"ok\");\n}\n");

    let directory = std::env::temp_dir().join("ams-gra-oms-task036-rust-temporal");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Rust probe directory");
    std::fs::write(directory.join("generated.rs"), &source).expect("write generated module");
    std::fs::write(directory.join("probe.rs"), &probe).expect("write probe");

    let status = Command::new("rustc")
        .current_dir(&directory)
        .args(["--edition", "2021", "-o", "probe", "probe.rs"])
        .status()
        .expect("rustc should be available in a Rust workspace");
    assert!(status.success(), "generated Rust module must compile");

    let run = Command::new(directory.join("probe"))
        .status()
        .expect("compiled probe must run");
    assert!(
        run.success(),
        "generated validator must agree with the shared corpus"
    );
    std::fs::remove_dir_all(&directory).expect("remove Rust probe directory");
}

/// Task 036: a client cannot bypass validation by touching the storage.
///
/// The carrier's field is private, so constructing one directly or reading
/// `lexical` from outside the generated module must fail to compile. A runtime
/// assertion would not prove this; only the compiler can.
#[test]
fn the_private_storage_cannot_be_reached_from_outside_the_module() {
    let source = generate(&temporal_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("temporal fixture must generate");

    let directory = std::env::temp_dir().join("ams-gra-oms-task036-rust-bypass");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Rust probe directory");
    // Placing the generated code in a *module* is what makes privacy
    // meaningful: `include!` at crate root would put the field in scope.
    std::fs::write(directory.join("generated.rs"), &source).expect("write generated module");

    for (label, body) in [
        (
            "struct literal",
            "    let _ = generated::Instant { lexical: String::from(\"garbageZ\") };",
        ),
        (
            "field read",
            "    let value = generated::Instant::new(\"2026-09-20T12:34:56Z\").unwrap();\n\
             \x20   let _ = value.lexical;",
        ),
    ] {
        let probe = format!(
            "mod generated {{\n    include!(\"generated.rs\");\n}}\n\nfn main() {{\n{body}\n}}\n"
        );
        std::fs::write(directory.join("probe.rs"), &probe).expect("write probe");
        let output = Command::new("rustc")
            .current_dir(&directory)
            .args(["--edition", "2021", "-o", "probe", "probe.rs"])
            .output()
            .expect("rustc should be available in a Rust workspace");
        assert!(
            !output.status.success(),
            "the {label} bypass must not compile"
        );
        let diagnostics = String::from_utf8_lossy(&output.stderr);
        assert!(
            diagnostics.contains("private"),
            "the {label} bypass must be rejected for privacy, got:\n{diagnostics}"
        );
    }
    std::fs::remove_dir_all(&directory).expect("remove Rust probe directory");
}
