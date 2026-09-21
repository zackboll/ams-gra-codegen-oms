//! Task 037: the generated Rust schema-version carrier, compiled and run.
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

fn string_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-schema-version.xsd"),
    )
    .expect("string fixture should parse")
}

/// The generated API shape, asserted before anything is run.
#[test]
fn generated_schema_version_api_is_an_opaque_validated_carrier() {
    let source = generate(&string_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("string fixture must generate");

    // A validated lexical carrier with private storage.
    assert!(source.contains("pub struct SchemaVersion {\n    value: String,\n}"));
    assert!(source.contains("pub fn new(value: &str) -> Option<Self>"));
    assert!(source.contains("pub fn as_str(&self) -> &str"));
    assert!(!source.contains("pub value"));

    // Unlike the Task 036 DateTime carrier, equality IS derived: for xs:string
    // the value space is the set of lexical forms, so stored-text equality is
    // genuine XML Schema value equality.
    let carrier = source
        .split("pub struct SchemaVersion {")
        .next()
        .expect("SchemaVersion must be generated");
    let derive = carrier
        .rsplit("#[derive(")
        .next()
        .expect("SchemaVersion must carry a derive");
    assert!(derive.contains("PartialEq"), "derive was: {derive}");
    assert!(derive.contains("Eq"), "derive was: {derive}");
    // XML Schema defines no order relation on `string`, so none is claimed.
    assert!(!derive.contains("Ord"), "derive was: {derive}");

    // No regex engine or other third-party runtime dependency.
    for forbidden in ["regex", "Regex", "lazy_static", "once_cell"] {
        assert!(
            !source.contains(forbidden),
            "generated Rust must not reference {forbidden}"
        );
    }

    // Task 034 composition: the optional named occurrence is the ordinary
    // `Option<SchemaVersion>`, with no String-profile-specific optional path.
    assert!(source.contains("pub declared: SchemaVersion,"));
    assert!(source.contains("pub negotiated: Option<SchemaVersion>,"));
    assert!(source.contains("pub peer: Option<PeerVersion>,"));

    // Ordinary unconstrained String keeps its existing plain representation.
    assert!(source.contains("pub label: String,"));

    // Task 037 equality propagation: a record holding only equality-capable
    // members must RETAIN its derives. This is the regression that keeps
    // String-profile consumers from inheriting the DateTime carrier's
    // deliberate absence of equality.
    assert!(source.contains("#[derive(Debug, Clone, PartialEq, Eq)]\npub struct Payload {"));
}

/// Every shared-corpus case, executed against the compiled generated module.
#[test]
fn generated_schema_version_validator_matches_the_shared_corpus() {
    let source = generate(&string_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("string fixture must generate");
    let cases = common::load_corpus(&common::string_corpus_path());

    let mut probe = String::from("include!(\"generated.rs\");\n\nfn main() {\n");
    for case in &cases {
        let input = common::escape_for_source(&case.input);
        match &case.expected {
            Some(stored) => {
                let stored = common::escape_for_source(stored);
                writeln!(
                    probe,
                    "    match SchemaVersion::new(\"{input}\") {{\n\
                     \x20       Some(value) => assert_eq!(value.as_str(), \"{stored}\", \"input {input}\"),\n\
                     \x20       None => panic!(\"must accept: {input}\"),\n\
                     \x20   }}"
                )
                .expect("writing to String cannot fail");
            }
            None => {
                writeln!(
                    probe,
                    "    assert!(SchemaVersion::new(\"{input}\").is_none(), \"must reject: {input}\");"
                )
                .expect("writing to String cannot fail");
            }
        }
    }
    // Equality is a claimed property of the carrier, so it is exercised rather
    // than merely asserted in the generated text.
    probe.push_str(
        "    assert_eq!(SchemaVersion::new(\"002.5.0\"), SchemaVersion::new(\"002.5.0\"));\n\
         \x20   assert_ne!(SchemaVersion::new(\"002.5.0\"), SchemaVersion::new(\"002.5.1\"));\n",
    );
    probe.push_str("    println!(\"ok\");\n}\n");

    let directory = std::env::temp_dir().join("ams-gra-oms-task037-rust-string");
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

/// Task 037: a client cannot bypass validation by touching the storage.
///
/// The carrier's field is private, so constructing one directly or reading
/// `value` from outside the generated module must fail to compile. A runtime
/// assertion would not prove this; only the compiler can.
#[test]
fn the_private_storage_cannot_be_reached_from_outside_the_module() {
    let source = generate(&string_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("string fixture must generate");

    let directory = std::env::temp_dir().join("ams-gra-oms-task037-rust-bypass");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Rust probe directory");
    // Placing the generated code in a *module* is what makes privacy
    // meaningful: `include!` at crate root would put the field in scope.
    std::fs::write(directory.join("generated.rs"), &source).expect("write generated module");

    for (label, body) in [
        (
            "struct literal",
            "    let _ = generated::SchemaVersion { value: String::from(\"nope\") };",
        ),
        (
            "field read",
            "    let value = generated::SchemaVersion::new(\"002.5.0\").unwrap();\n\
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
