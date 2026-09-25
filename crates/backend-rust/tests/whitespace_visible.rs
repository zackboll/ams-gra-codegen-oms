//! Task 041: the generated Rust whitespace-visible carriers, compiled and run.
//!
//! Asserting on generated *text* alone would not prove the emitted validators
//! behave the way the authoritative XSD requires, so both shared corpora are
//! executed against a real compiled program. The collapse half additionally
//! changes the stored value, which only execution can demonstrate.

mod common;

use ams_gra_oms_backend_rust::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn whitespace_visible_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-whitespace-visible.xsd"),
    )
    .expect("whitespace-visible fixture should parse")
}

/// The generated API shape, asserted before anything is run.
#[test]
fn generated_whitespace_visible_api_is_an_opaque_validated_carrier() {
    let source = generate(
        &whitespace_visible_schema(),
        GenerationWorld::ClosedSchemaSet,
    )
    .expect("fixture must generate");

    // Validated lexical carriers with private storage and checked construction.
    for name in [
        "CollapsedRemarks",
        "CollapsedNarrative",
        "PreservedRemarks",
        "PreservedNarrative",
        "PreservedQuery",
    ] {
        assert!(
            source.contains(&format!("pub struct {name} {{\n    value: String,\n}}")),
            "{name} must be an opaque carrier"
        );
    }
    assert!(source.contains("pub fn new(value: &str) -> Option<Self>"));
    assert!(source.contains("pub fn as_str(&self) -> &str"));
    assert!(!source.contains("pub value"));
    // No unchecked Default: construction must go through `new`.
    assert!(!source.contains("impl Default for CollapsedRemarks"));
    assert!(!source.contains("impl Default for PreservedRemarks"));

    // The profile is PARAMETERIZED over whole evidenced triples, so each carrier
    // must carry its OWN bounds. A single shared constant would widen 1024 to
    // 4096, and a single shared minimum would let the preserve half accept "".
    assert!(source.contains("const MAX_LENGTH: usize = 1024;"));
    assert!(source.contains("const MAX_LENGTH: usize = 4096;"));
    assert!(source.contains("const MIN_LENGTH: usize = 0;"));
    assert!(source.contains("const MIN_LENGTH: usize = 1;"));

    // The class is the exact ordinal interval PLUS the two escaped characters.
    assert!(source.contains("const MIN_CODE_POINT: u8 = 0x20;"));
    assert!(source.contains("const MAX_CODE_POINT: u8 = 0x7E;"));
    assert!(source.contains("const LINE_FEED: u8 = 0x0A;"));
    assert!(source.contains("const CARRIAGE_RETURN: u8 = 0x0D;"));

    // The collapse half gets the normalization helpers. The fixture has exactly
    // three collapse carriers (1024, 4096, and the derived chain member), so a
    // fourth would mean a preserve carrier wrongly acquired them -- which would
    // also be dead code under the workspace's `-D warnings`.
    assert_eq!(
        source.matches("fn collapse(text: &str) -> String").count(),
        3
    );
    assert_eq!(
        source
            .matches("fn is_xml_whitespace(character: char) -> bool")
            .count(),
        3
    );

    // Documentation says "normalized" for collapse and "preserved" for preserve,
    // never "exactly as supplied" indiscriminately.
    assert!(source.contains("the collapse-normalized form of the input, not the input itself"));
    assert!(source.contains("the caller's text preserved unchanged"));

    // No regex engine, and nothing locale-sensitive. `is_whitespace` is
    // specifically forbidden: it is true of U+00A0, which this class rejects.
    for forbidden in [
        "regex",
        "Regex",
        "lazy_static",
        "once_cell",
        "is_ascii_graphic",
        "is_ascii_whitespace",
        // The Unicode predicate, as a CALL: `.is_whitespace()`. Its bare name
        // appears in the carrier's own `is_xml_whitespace`, which is the
        // deliberate four-character replacement for it.
        ".is_whitespace()",
        "split_ascii_whitespace",
        "split_whitespace",
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

    // Equality IS derived: for xs:string the value space is the set of lexical
    // forms, so stored-text equality is genuine XML Schema value equality.
    let carrier = source
        .split("pub struct CollapsedRemarks {")
        .next()
        .expect("CollapsedRemarks must be generated");
    let derive = carrier
        .rsplit("#[derive(")
        .next()
        .expect("CollapsedRemarks must carry a derive");
    assert!(derive.contains("PartialEq"), "derive was: {derive}");
    assert!(derive.contains("Eq"), "derive was: {derive}");
    // XML Schema defines no order relation on `string`, so none is claimed.
    assert!(!derive.contains("Ord"), "derive was: {derive}");

    // Older profiles still render, in the same unit.
    assert!(source.contains("pub struct Callsign {"));
    assert!(source.contains("pub struct SchemaVersion {"));
    // Ordinary unconstrained String keeps its plain representation.
    assert!(source.contains("pub notes: String,"));
}

/// Both shared corpora, executed against the compiled generated module.
///
/// Each corpus is run against its own policy's carrier, so a validator that
/// silently implemented the wrong policy fails here rather than passing a merged
/// and weakened expectation set.
#[test]
fn generated_whitespace_visible_validators_match_the_shared_corpora() {
    let source = generate(
        &whitespace_visible_schema(),
        GenerationWorld::ClosedSchemaSet,
    )
    .expect("fixture must generate");

    let mut probe = String::from("include!(\"generated.rs\");\n\nfn main() {\n");
    for (carrier, path) in [
        (
            "CollapsedRemarks",
            common::whitespace_visible_collapse_corpus_path(),
        ),
        (
            "PreservedRemarks",
            common::whitespace_visible_preserve_corpus_path(),
        ),
    ] {
        let cases = common::load_corpus(&path);
        assert!(!cases.is_empty(), "{carrier} corpus must not be empty");
        for (index, case) in cases.iter().enumerate() {
            // The corpora contain braced cases, so a case's own text must never
            // be interpolated into a format string. Cases are identified by
            // carrier plus corpus index, which is brace-free by construction.
            let input = common::escape_for_rust_source(&case.input);
            match &case.expected {
                Some(stored) => {
                    let stored = common::escape_for_rust_source(stored);
                    writeln!(
                        probe,
                        "    match {carrier}::new(\"{input}\") {{\n\
                         \x20       Some(value) => assert_eq!(value.as_str(), \"{stored}\", \"{carrier} case {index}\"),\n\
                         \x20       None => panic!(\"{carrier} must accept case {index}\"),\n\
                         \x20   }}"
                    )
                    .expect("writing to String cannot fail");
                    // Reconstruction / idempotence: the stored form is itself a
                    // valid input mapping to the same stored form.
                    writeln!(
                        probe,
                        "    assert_eq!(\n\
                         \x20       {carrier}::new(\"{stored}\").map(|value| value.as_str().to_owned()),\n\
                         \x20       Some(String::from(\"{stored}\")),\n\
                         \x20       \"{carrier} case {index} must be idempotent\",\n\
                         \x20   );"
                    )
                    .expect("writing to String cannot fail");
                }
                None => {
                    writeln!(
                        probe,
                        "    assert!({carrier}::new(\"{input}\").is_none(), \"{carrier} must reject case {index}\");"
                    )
                    .expect("writing to String cannot fail");
                }
            }
        }
    }
    probe.push_str(&semantics_probe());
    probe.push_str("    println!(\"ok\");\n}\n");

    let directory = std::env::temp_dir().join("ams-gra-oms-task041-rust-whitespace-visible");
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
        "generated validators must agree with the shared corpora"
    );
    std::fs::remove_dir_all(&directory).expect("remove Rust probe directory");
}

/// The assertions the line-oriented corpora cannot express.
///
/// Long inputs, the two differing maxima, and the raw-versus-normalized length
/// rule all need generated lengths rather than literal corpus text.
fn semantics_probe() -> String {
    let mut probe = String::new();
    // The raw/normalized length distinction: a RAW input far longer than
    // maxLength is ACCEPTED when its NORMALIZED form fits. This is the property
    // that fails if normalization is applied after the length facet.
    probe.push_str(
        "    let raw = String::from(\"a\") + &\" \".repeat(4000) + \"b\";\n\
         \x20   assert!(raw.len() > 1024, \"the raw input must exceed maxLength\");\n\
         \x20   assert_eq!(CollapsedRemarks::new(&raw).unwrap().as_str(), \"a b\");\n",
    );
    // Normalized length exactly N and N+1, for both maxima.
    probe.push_str(
        "    assert!(CollapsedRemarks::new(&\"x\".repeat(1024)).is_some(), \"1024 fits 0..1024\");\n\
         \x20   assert!(CollapsedRemarks::new(&\"x\".repeat(1025)).is_none(), \"1025 exceeds 1024\");\n\
         \x20   assert!(PreservedRemarks::new(&\"x\".repeat(1024)).is_some());\n\
         \x20   assert!(PreservedRemarks::new(&\"x\".repeat(1025)).is_none());\n\
         \x20   assert!(CollapsedNarrative::new(&\"x\".repeat(4096)).is_some(), \"4096 fits 0..4096\");\n\
         \x20   assert!(CollapsedNarrative::new(&\"x\".repeat(4097)).is_none(), \"4097 exceeds 4096\");\n\
         \x20   assert!(PreservedNarrative::new(&\"x\".repeat(4096)).is_some());\n\
         \x20   assert!(PreservedNarrative::new(&\"x\".repeat(4097)).is_none());\n",
    );
    // A length BETWEEN 1024 and 4096, proving the two bounds really differ.
    probe.push_str(
        "    let between = \"x\".repeat(2000);\n\
         \x20   assert!(CollapsedRemarks::new(&between).is_none(), \"2000 exceeds the 1024 member\");\n\
         \x20   assert!(CollapsedNarrative::new(&between).is_some(), \"2000 fits the 4096 member\");\n\
         \x20   assert!(PreservedRemarks::new(&between).is_none());\n\
         \x20   assert!(PreservedNarrative::new(&between).is_some());\n",
    );
    // The two minima differ, for the SAME input.
    probe.push_str(
        "    assert_eq!(CollapsedRemarks::new(\"\").unwrap().as_str(), \"\", \"minLength 0 accepts empty\");\n\
         \x20   assert!(PreservedRemarks::new(\"\").is_none(), \"minLength 1 rejects empty\");\n\
         \x20   assert_eq!(PreservedQuery::new(\"\").unwrap().as_str(), \"\", \"the 2.5 query shape has minLength 0\");\n\
         \x20   assert_eq!(PreservedQuery::new(\"  \").unwrap().as_str(), \"  \", \"preserve does not trim\");\n",
    );
    // TAB: rejected under preserve, normalized to SPACE under collapse. Same
    // input, opposite outcomes, which is the whole point of the policy axis.
    probe.push_str(
        "    assert!(PreservedRemarks::new(\"a\\tb\").is_none(), \"the preserved class excludes TAB\");\n\
         \x20   assert_eq!(CollapsedRemarks::new(\"a\\tb\").unwrap().as_str(), \"a b\", \"collapse maps TAB to SPACE\");\n",
    );
    // Equality is a claimed property, so it is exercised. Under collapse,
    // different spellings become the SAME stored value and compare EQUAL; under
    // preserve, whitespace stays significant.
    probe.push_str(
        "    assert_eq!(\n\
         \x20       CollapsedRemarks::new(\"a b\").unwrap(),\n\
         \x20       CollapsedRemarks::new(\"  a   b  \").unwrap(),\n\
         \x20       \"normalized spellings must compare equal\",\n\
         \x20   );\n\
         \x20   assert_ne!(\n\
         \x20       PreservedRemarks::new(\"a b\").unwrap(),\n\
         \x20       PreservedRemarks::new(\"a  b\").unwrap(),\n\
         \x20       \"preserved whitespace must stay significant\",\n\
         \x20   );\n\
         \x20   assert_ne!(PreservedRemarks::new(\"a\\nb\").unwrap(), PreservedRemarks::new(\"a b\").unwrap());\n",
    );
    // The transitive chain member keeps the collapse profile it inherits.
    probe.push_str(
        "    assert_eq!(DerivedCollapsedRemarks::new(\" a  b \").unwrap().as_str(), \"a b\");\n",
    );
    probe
}

/// Task 041: a client cannot bypass validation by touching the storage.
#[test]
fn the_private_whitespace_visible_storage_cannot_be_reached_from_outside_the_module() {
    let source = generate(
        &whitespace_visible_schema(),
        GenerationWorld::ClosedSchemaSet,
    )
    .expect("fixture must generate");

    let directory = std::env::temp_dir().join("ams-gra-oms-task041-rust-bypass");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Rust probe directory");
    // Placing the generated code in a *module* is what makes privacy
    // meaningful: `include!` at crate root would put the field in scope.
    std::fs::write(directory.join("generated.rs"), &source).expect("write generated module");

    for (label, body) in [
        (
            "struct literal",
            "    let _ = generated::CollapsedRemarks { value: String::from(\"nope\") };",
        ),
        (
            "field read",
            "    let value = generated::PreservedRemarks::new(\"ok\").unwrap();\n\
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
