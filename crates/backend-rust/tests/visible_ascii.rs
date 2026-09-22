//! Task 039: the generated Rust visible-ASCII carrier, compiled and run.
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

fn visible_ascii_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-visible-ascii.xsd"),
    )
    .expect("visible-ASCII fixture should parse")
}

/// The generated API shape, asserted before anything is run.
#[test]
fn generated_visible_ascii_api_is_an_opaque_validated_carrier() {
    let source = generate(&visible_ascii_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("fixture must generate");

    // A validated lexical carrier with private storage.
    assert!(source.contains("pub struct Callsign {\n    value: String,\n}"));
    assert!(source.contains("pub fn new(value: &str) -> Option<Self>"));
    assert!(source.contains("pub fn as_str(&self) -> &str"));
    assert!(!source.contains("pub value"));

    // The profile is PARAMETERIZED: each carrier must carry its OWN bounds.
    // A single shared constant here would silently widen `ShortLabel` to 256.
    assert!(source.contains("const MIN_LENGTH: usize = 1;"));
    assert!(source.contains("const MAX_LENGTH: usize = 256;"));
    assert!(source.contains("const MAX_LENGTH: usize = 32;"));
    assert!(source.contains("const MIN_LENGTH: usize = 2;"));
    assert!(source.contains("const MAX_LENGTH: usize = 4;"));

    // The class is the exact ordinal interval, not a locale-sensitive guess.
    assert!(source.contains("const MIN_CODE_POINT: u8 = 0x20;"));
    assert!(source.contains("const MAX_CODE_POINT: u8 = 0x7E;"));

    // Equality IS derived: for xs:string the value space is the set of lexical
    // forms, so stored-text equality is genuine XML Schema value equality.
    let carrier = source
        .split("pub struct Callsign {")
        .next()
        .expect("Callsign must be generated");
    let derive = carrier
        .rsplit("#[derive(")
        .next()
        .expect("Callsign must carry a derive");
    assert!(derive.contains("PartialEq"), "derive was: {derive}");
    assert!(derive.contains("Eq"), "derive was: {derive}");
    // XML Schema defines no order relation on `string`, so none is claimed.
    assert!(!derive.contains("Ord"), "derive was: {derive}");

    // No regex engine, and nothing locale-sensitive. `is_ascii_graphic` is
    // specifically forbidden: it EXCLUDES space, which this class admits.
    for forbidden in [
        "regex",
        "Regex",
        "lazy_static",
        "once_cell",
        "is_ascii_graphic",
        "is_alphanumeric",
        // Calls, not prose: the carrier's own documentation says the words
        // "trimming" and "collapsing" precisely to record that it does neither.
        ".trim(",
        ".trim_start(",
        ".trim_end(",
        "to_lowercase",
    ] {
        assert!(
            !source.contains(forbidden),
            "generated Rust must not reference {forbidden}"
        );
    }

    // Task 034 composition: the optional named occurrence is the ordinary
    // `Option<Callsign>`, with no visible-string-specific optional path.
    assert!(source.contains("pub primary: Callsign,"));
    assert!(source.contains("pub alternate: Option<Callsign>,"));
    assert!(source.contains("pub label: Option<ShortLabel>,"));

    // A transitive restriction that adds no facets is equally renderable.
    assert!(source.contains("pub struct DerivedLabel {"));
    assert!(source.contains("pub derived: DerivedLabel,"));

    // The Task 037 and Task 038 profiles still render, in the same unit.
    assert!(source.contains("pub struct SchemaVersion {"));
    assert!(source.contains("pub struct Uuid {"));

    // Ordinary unconstrained String keeps its existing plain representation.
    assert!(source.contains("pub notes: String,"));

    // Equality propagation: a record holding only equality-capable members must
    // RETAIN its derives.
    assert!(source.contains("#[derive(Debug, Clone, PartialEq, Eq)]\npub struct Payload {"));
}

/// Every shared-corpus case, executed against the compiled generated module.
#[test]
fn generated_visible_ascii_validator_matches_the_shared_corpus() {
    let source = generate(&visible_ascii_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("fixture must generate");
    let cases = common::load_corpus(&common::visible_ascii_corpus_path());

    let mut probe = String::from("include!(\"generated.rs\");\n\nfn main() {\n");
    for (index, case) in cases.iter().enumerate() {
        // The corpus contains braced cases, so a case's own text must never be
        // interpolated into a `panic!`/`assert!` FORMAT string. Cases are
        // identified by corpus index instead, which is brace-free by
        // construction.
        let input = common::escape_for_rust_source(&case.input);
        match &case.expected {
            Some(stored) => {
                let stored = common::escape_for_rust_source(stored);
                writeln!(
                    probe,
                    "    match Callsign::new(\"{input}\") {{\n\
                     \x20       Some(value) => assert_eq!(value.as_str(), \"{stored}\", \"case {index}\"),\n\
                     \x20       None => panic!(\"must accept case {index}\"),\n\
                     \x20   }}"
                )
                .expect("writing to String cannot fail");
            }
            None => {
                writeln!(
                    probe,
                    "    assert!(Callsign::new(\"{input}\").is_none(), \"must reject case {index}\");"
                )
                .expect("writing to String cannot fail");
            }
        }
    }
    probe.push_str(&boundary_probe());
    probe.push_str("    println!(\"ok\");\n}\n");

    let directory = std::env::temp_dir().join("ams-gra-oms-task039-rust-visible-ascii");
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

/// The assertions the shared corpus cannot express, appended to the probe.
///
/// Three things are pinned here: the four ordinal boundaries, the fact that
/// the bounds really are per-declaration, and equality under
/// `whiteSpace = preserve`.
fn boundary_probe() -> String {
    let mut probe = String::new();
    // The four ordinal boundaries, pinned directly rather than only via the
    // corpus, so this interval can never silently become "printable ASCII".
    probe.push_str(
        "    assert!(Callsign::new(\"\\u{1f}\").is_none(), \"U+001F must be rejected\");\n\
         \x20   assert!(Callsign::new(\"\\u{20}\").is_some(), \"U+0020 must be accepted\");\n\
         \x20   assert!(Callsign::new(\"\\u{7e}\").is_some(), \"U+007E must be accepted\");\n\
         \x20   assert!(Callsign::new(\"\\u{7f}\").is_none(), \"U+007F must be rejected\");\n",
    );
    // The bounds really are per-declaration. `ShortLabel` is 1..32 and
    // `CountryCode` is 2..4, so a value legal for one must be illegal for the
    // others exactly where the facets say.
    probe.push_str(
        "    let long = \"x\".repeat(33);\n\
         \x20   assert!(Callsign::new(&long).is_some(), \"33 fits 1..256\");\n\
         \x20   assert!(ShortLabel::new(&long).is_none(), \"33 exceeds 1..32\");\n\
         \x20   assert!(ShortLabel::new(&\"x\".repeat(32)).is_some(), \"32 fits 1..32\");\n\
         \x20   assert!(CountryCode::new(\"x\").is_none(), \"1 is below minLength 2\");\n\
         \x20   assert!(CountryCode::new(\"xx\").is_some(), \"2 meets minLength 2\");\n\
         \x20   assert!(CountryCode::new(\"xxxx\").is_some(), \"4 meets maxLength 4\");\n\
         \x20   assert!(CountryCode::new(\"xxxxx\").is_none(), \"5 exceeds maxLength 4\");\n\
         \x20   assert!(DerivedLabel::new(&long).is_none(), \"the chain keeps 1..32\");\n",
    );
    // Equality is a claimed property, so it is exercised rather than merely
    // asserted in the generated text. Spaces and case are significant, and
    // whiteSpace = preserve means "abc" and "abc " are DISTINCT values.
    probe.push_str(
        "    let plain = Callsign::new(\"abc\").unwrap();\n\
         \x20   let trailing = Callsign::new(\"abc \").unwrap();\n\
         \x20   let leading = Callsign::new(\" abc\").unwrap();\n\
         \x20   assert_eq!(plain, Callsign::new(\"abc\").unwrap());\n\
         \x20   assert_ne!(plain, trailing, \"a trailing space must not be trimmed\");\n\
         \x20   assert_ne!(plain, leading, \"a leading space must not be trimmed\");\n\
         \x20   assert_ne!(leading, trailing);\n\
         \x20   assert_eq!(trailing.as_str(), \"abc \", \"spelling must be preserved\");\n\
         \x20   assert_eq!(leading.as_str(), \" abc\");\n\
         \x20   assert_ne!(plain, Callsign::new(\"ABC\").unwrap(), \"case is significant\");\n",
    );
    probe
}

/// Task 039: a client cannot bypass validation by touching the storage.
#[test]
fn the_private_visible_ascii_storage_cannot_be_reached_from_outside_the_module() {
    let source = generate(&visible_ascii_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("fixture must generate");

    let directory = std::env::temp_dir().join("ams-gra-oms-task039-rust-bypass");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Rust probe directory");
    // Placing the generated code in a *module* is what makes privacy
    // meaningful: `include!` at crate root would put the field in scope.
    std::fs::write(directory.join("generated.rs"), &source).expect("write generated module");

    for (label, body) in [
        (
            "struct literal",
            "    let _ = generated::Callsign { value: String::from(\"nope\") };",
        ),
        (
            "field read",
            "    let value = generated::Callsign::new(\"ok\").unwrap();\n\
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
