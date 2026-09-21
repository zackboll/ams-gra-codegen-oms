//! Task 038: the generated Rust UUID carrier, compiled and run.
//!
//! Asserting on generated *text* alone would not prove the emitted validator
//! behaves the way the authoritative XSD requires, so every case in the shared
//! corpus is executed against a real compiled program.

mod common;

use ams_gra_oms_backend_rust::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn uuid_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-uuid.xsd"),
    )
    .expect("uuid fixture should parse")
}

/// The generated API shape, asserted before anything is run.
#[test]
fn generated_uuid_api_is_an_opaque_validated_carrier() {
    let source =
        generate(&uuid_schema(), GenerationWorld::ClosedSchemaSet).expect("fixture must generate");

    // A validated lexical carrier with private storage.
    assert!(source.contains("pub struct Uuid {\n    value: String,\n}"));
    assert!(source.contains("pub fn new(value: &str) -> Option<Self>"));
    assert!(source.contains("pub fn as_str(&self) -> &str"));
    assert!(!source.contains("pub value"));

    // Equality IS derived: for xs:string the value space is the set of lexical
    // forms, so stored-text equality is genuine XML Schema value equality.
    let carrier = source
        .split("pub struct Uuid {")
        .next()
        .expect("Uuid must be generated");
    let derive = carrier
        .rsplit("#[derive(")
        .next()
        .expect("Uuid must carry a derive");
    assert!(derive.contains("PartialEq"), "derive was: {derive}");
    assert!(derive.contains("Eq"), "derive was: {derive}");
    // XML Schema defines no order relation on `string`, so none is claimed.
    assert!(!derive.contains("Ord"), "derive was: {derive}");

    // No regex engine, and no UUID library: the carrier is the validated
    // spelling, never a parsed 128-bit value.
    for forbidden in [
        "regex",
        "Regex",
        "lazy_static",
        "once_cell",
        "u128",
        "from_u128",
    ] {
        assert!(
            !source.contains(forbidden),
            "generated Rust must not reference {forbidden}"
        );
    }

    // Task 034 composition: the optional named occurrence is the ordinary
    // `Option<Uuid>`, with no UUID-specific optional path.
    assert!(source.contains("pub identifier: Uuid,"));
    assert!(source.contains("pub correlation: Option<Uuid>,"));
    assert!(source.contains("pub peer: Option<PeerUuid>,"));

    // The Task 037 profile still renders, in the same generated unit.
    assert!(source.contains("pub struct SchemaVersion {"));
    assert!(source.contains("pub version: SchemaVersion,"));

    // Ordinary unconstrained String keeps its existing plain representation.
    assert!(source.contains("pub label: String,"));

    // Equality propagation: a record holding only equality-capable members must
    // RETAIN its derives.
    assert!(source.contains("#[derive(Debug, Clone, PartialEq, Eq)]\npub struct Payload {"));
}

/// Every shared-corpus case, executed against the compiled generated module.
#[test]
fn generated_uuid_validator_matches_the_shared_corpus() {
    let source =
        generate(&uuid_schema(), GenerationWorld::ClosedSchemaSet).expect("fixture must generate");
    let cases = common::load_corpus(&common::uuid_corpus_path());

    let mut probe = String::from("include!(\"generated.rs\");\n\nfn main() {\n");
    for (index, case) in cases.iter().enumerate() {
        let input = common::escape_for_source(&case.input);
        // The corpus contains a braced case, so a case's own text must never be
        // interpolated into a `panic!`/`assert!` FORMAT string. Cases are
        // identified by corpus index instead, which is brace-free by
        // construction.
        match &case.expected {
            Some(stored) => {
                let stored = common::escape_for_source(stored);
                writeln!(
                    probe,
                    "    match Uuid::new(\"{input}\") {{\n\
                     \x20       Some(value) => assert_eq!(value.as_str(), \"{stored}\", \"case {index}\"),\n\
                     \x20       None => panic!(\"must accept case {index}\"),\n\
                     \x20   }}"
                )
                .expect("writing to String cannot fail");
            }
            None => {
                writeln!(
                    probe,
                    "    assert!(Uuid::new(\"{input}\").is_none(), \"must reject case {index}\");"
                )
                .expect("writing to String cannot fail");
            }
        }
    }
    // Equality is a claimed property of the carrier, so it is exercised rather
    // than merely asserted in the generated text. Case is significant: two
    // otherwise-valid UUIDs differing only in letter case are DISTINCT
    // xs:string values, and no case-insensitive comparison is introduced.
    probe.push_str(
        "    let lower = Uuid::new(\"123e4567-e89b-12d3-a456-42661417400f\").unwrap();\n\
         \x20   let upper = Uuid::new(\"123E4567-E89B-12D3-A456-42661417400F\").unwrap();\n\
         \x20   assert_eq!(lower, Uuid::new(\"123e4567-e89b-12d3-a456-42661417400f\").unwrap());\n\
         \x20   assert_ne!(lower, upper, \"case must remain significant\");\n\
         \x20   assert_eq!(upper.as_str(), \"123E4567-E89B-12D3-A456-42661417400F\");\n",
    );
    probe.push_str("    println!(\"ok\");\n}\n");

    let directory = std::env::temp_dir().join("ams-gra-oms-task038-rust-uuid");
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

/// Task 038: a client cannot bypass validation by touching the storage.
#[test]
fn the_private_uuid_storage_cannot_be_reached_from_outside_the_module() {
    let source =
        generate(&uuid_schema(), GenerationWorld::ClosedSchemaSet).expect("fixture must generate");

    let directory = std::env::temp_dir().join("ams-gra-oms-task038-rust-bypass");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Rust probe directory");
    // Placing the generated code in a *module* is what makes privacy
    // meaningful: `include!` at crate root would put the field in scope.
    std::fs::write(directory.join("generated.rs"), &source).expect("write generated module");

    for (label, body) in [
        (
            "struct literal",
            "    let _ = generated::Uuid { value: String::from(\"nope\") };",
        ),
        (
            "field read",
            "    let value = generated::Uuid::new(\"00000000-0000-0000-0000-000000000000\").unwrap();\n\
             \x20   let _ = value.value;",
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
