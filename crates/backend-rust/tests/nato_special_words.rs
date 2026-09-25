//! Task 042: the generated Rust NATO special-words carrier, compiled and run.
//!
//! The shared corpus, generated boundary lengths, and a deterministic alphabet
//! sweep are executed against a real compiled program. Expected acceptance in
//! the sweep comes from an alphabet stated independently in this file, never
//! from the production classifier or validator.

mod common;

use ams_gra_oms_backend_rust::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn nato_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-nato-special-words.xsd"),
    )
    .expect("NATO special-words fixture should parse")
}

/// The suffix alphabet `[a-zA-Z\-_]`, stated independently of the generator.
fn expected_suffix_member(character: char) -> bool {
    matches!(character, 'A'..='Z' | 'a'..='z' | '-' | '_')
}

/// The generated API shape, asserted before anything is run.
#[test]
fn generated_nato_special_words_api_is_an_opaque_validated_carrier() {
    let source =
        generate(&nato_schema(), GenerationWorld::ClosedSchemaSet).expect("fixture must generate");

    for name in ["SpecialWord", "DerivedSpecialWord"] {
        assert!(
            source.contains(&format!("pub struct {name} {{\n    value: String,\n}}")),
            "{name} must be an opaque carrier"
        );
        assert!(
            !source.contains(&format!("impl Default for {name}")),
            "{name} must not gain an unchecked Default"
        );
    }
    assert!(source.contains("pub fn new(value: &str) -> Option<Self>"));
    assert!(source.contains("pub fn as_str(&self) -> &str"));
    assert!(!source.contains("pub value"));

    // Total-length facets and the literal prefix, per carrier.
    assert_eq!(source.matches("const MIN_LENGTH: usize = 6;").count(), 2);
    assert_eq!(source.matches("const MAX_LENGTH: usize = 261;").count(), 2);
    assert_eq!(
        source
            .matches("const PREFIX: &'static [u8] = b\"NATO:\";")
            .count(),
        2
    );
    // Explicit ASCII ranges.
    assert!(source.contains("matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'-' | b'_')"));

    // No regex engine, nothing Unicode- or locale-sensitive, no normalization,
    // and no str slicing before the prefix is known. Checked against the NATO
    // carrier's own block: the unchanged Task 037 carrier in the same unit
    // legitimately uses `is_ascii_alphanumeric` for its own alphabet.
    assert!(!source.contains("regex"), "no regex dependency");
    let start = source
        .find("pub struct SpecialWord {")
        .expect("SpecialWord must be generated");
    let end = source[start..]
        .find("pub struct DerivedSpecialWord {")
        .map(|offset| start + offset)
        .expect("DerivedSpecialWord follows");
    let nato = &source[start..end];
    for forbidden in [
        "regex",
        "Regex",
        "is_alphabetic",
        "is_alphanumeric",
        "is_ascii_alphanumeric",
        ".is_whitespace()",
        ".trim(",
        "to_lowercase",
        "to_uppercase",
        "strip_prefix",
        "&text[",
        "&value[",
    ] {
        assert!(
            !nato.contains(forbidden),
            "the generated NATO carrier must not reference {forbidden}"
        );
    }

    // Equality derived (stored-text equality IS xs:string value equality);
    // no ordering is claimed.
    let carrier = source
        .split("pub struct SpecialWord {")
        .next()
        .expect("SpecialWord must be generated");
    let derive = carrier.rsplit("#[derive(").next().expect("derive present");
    assert!(derive.contains("PartialEq") && derive.contains("Eq"));
    assert!(!derive.contains("Ord"), "derive was: {derive}");

    // Older profiles and the ordinary String control are unchanged.
    assert!(source.contains("pub struct Callsign {"));
    assert!(source.contains("pub struct SchemaVersion {"));
    assert!(source.contains("pub struct PreservedRemarks {"));
    assert!(source.contains("pub notes: String,"));
    // Composition uses only the existing containers.
    assert!(source.contains("pub alternate: Option<SpecialWord>,"));
    assert!(source.contains("pub history: BoundedVec<SpecialWord, 0, 4>,"));
    assert!(source.contains("pub required: BoundedVec<SpecialWord, 2, 3>,"));
    assert!(source.contains("pub log: UnboundedVec<SpecialWord, 0>,"));
}

/// The shared corpus, boundaries, and alphabet sweep, compiled and executed.
#[test]
fn generated_nato_special_words_validator_matches_the_shared_corpus() {
    let source =
        generate(&nato_schema(), GenerationWorld::ClosedSchemaSet).expect("fixture must generate");

    let mut probe = String::from("include!(\"generated.rs\");\n\nfn main() {\n");
    let cases = common::load_corpus(&common::nato_special_words_corpus_path());
    assert!(cases.len() > 100, "the corpus must be substantial");
    for (index, case) in cases.iter().enumerate() {
        // Cases are identified by index only, never by interpolating their own
        // (possibly braced) text into a format string.
        let input = common::escape_for_rust_source(&case.input);
        match &case.expected {
            Some(stored) => {
                let stored = common::escape_for_rust_source(stored);
                writeln!(
                    probe,
                    "    match SpecialWord::new(\"{input}\") {{\n\
                     \x20       Some(value) => {{\n\
                     \x20           assert_eq!(value.as_str(), \"{stored}\", \"case {index} storage\");\n\
                     \x20           let again = SpecialWord::new(value.as_str()).expect(\"case {index} reconstruct\");\n\
                     \x20           assert_eq!(again, value, \"case {index} idempotent\");\n\
                     \x20       }}\n\
                     \x20       None => panic!(\"must accept case {index}\"),\n\
                     \x20   }}"
                )
                .expect("writing to String cannot fail");
            }
            None => writeln!(
                probe,
                "    assert!(SpecialWord::new(\"{input}\").is_none(), \"must reject case {index}\");"
            )
            .expect("writing to String cannot fail"),
        }
    }
    probe.push_str(&boundary_probe());
    probe.push_str(&alphabet_probe());
    probe.push_str(COMPOSITION_PROBE);
    probe.push_str("    println!(\"ok\");\n}\n");

    let directory = std::env::temp_dir().join("ams-gra-oms-task042-rust-nato");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Rust probe directory");
    std::fs::write(directory.join("generated.rs"), &source).expect("write generated module");
    std::fs::write(directory.join("probe.rs"), &probe).expect("write probe");
    let output = Command::new("rustc")
        .current_dir(&directory)
        .args(["--edition", "2021", "-o", "probe", "probe.rs"])
        .output()
        .expect("rustc should be available in a Rust workspace");
    assert!(
        output.status.success(),
        "generated Rust module must compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let run = Command::new(directory.join("probe"))
        .output()
        .expect("compiled probe must run");
    assert!(
        run.status.success() && String::from_utf8_lossy(&run.stdout).trim() == "ok",
        "generated validator must agree with the shared corpus:\n{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::remove_dir_all(&directory).expect("remove Rust probe directory");
}

/// Suffix 0/1/255/256/257 and total 5/6/260/261/262, plus long invalid input.
fn boundary_probe() -> String {
    let mut probe = String::new();
    for (suffix, valid) in [
        (0, false),
        (1, true),
        (255, true),
        (256, true),
        (257, false),
    ] {
        let total = 5 + suffix;
        let check = if valid { "is_some" } else { "is_none" };
        writeln!(
            probe,
            "    {{\n\
             \x20       let text = String::from(\"NATO:\") + &\"a\".repeat({suffix});\n\
             \x20       assert_eq!(text.len(), {total});\n\
             \x20       let made = SpecialWord::new(&text);\n\
             \x20       assert!(made.{check}(), \"suffix {suffix} / total {total}\");\n\
             \x20       if let Some(value) = made {{ assert_eq!(value.as_str(), text); }}\n\
             \x20   }}"
        )
        .expect("writing to String cannot fail");
    }
    probe.push_str(
        "    // Total 262 of in-class characters, and long invalid inputs.\n\
         \x20   assert!(SpecialWord::new(&(String::from(\"NATO:\") + &\"-_\".repeat(128) + \"Z\")).is_none());\n\
         \x20   assert!(SpecialWord::new(&\"NATO:\".repeat(52)).is_none(), \"total 260, colons in suffix\");\n\
         \x20   assert!(SpecialWord::new(&\"x\".repeat(10_000)).is_none());\n\
         \x20   assert!(SpecialWord::new(&(String::from(\"NATO:\") + &\"a\".repeat(10_000))).is_none());\n\
         \x20   assert!(SpecialWord::new(&(String::from(\"NATO:\") + &\"a\".repeat(255) + \"1\")).is_none());\n\
         \x20   // Multibyte text at short lengths must be rejected without panicking.\n\
         \x20   for text in [\"\\u{e9}\", \"N\\u{e9}\", \"NAT\\u{1f600}\", \"\\u{1f600}\\u{1f600}\", \"NATO\\u{e9}\"] {\n\
         \x20       assert!(SpecialWord::new(text).is_none());\n\
         \x20   }\n\
         \x20   // 128 two-byte letters: 5 + 256 bytes = 261, inside maxLength in\n\
         \x20   // bytes but non-ASCII; rejected by the class, not the length.\n\
         \x20   assert!(SpecialWord::new(&(String::from(\"NATO:\") + &\"\\u{e9}\".repeat(128))).is_none());\n",
    );
    probe
}

/// Every ASCII candidate, plus representative non-ASCII characters, as the
/// single suffix character. Expected acceptance is computed HERE, from the
/// independently stated alphabet.
fn alphabet_probe() -> String {
    let mut probe = String::new();
    let non_ascii = [
        '\u{80}',
        '\u{a0}',
        '\u{c0}',
        '\u{e9}',
        '\u{ff}',
        '\u{410}',
        '\u{2013}',
        '\u{ff21}',
        '\u{1f600}',
    ];
    for character in (0_u8..=0x7F).map(char::from).chain(non_ascii) {
        let escaped = common::escape_for_rust_source(&format!("NATO:{character}"));
        let check = if expected_suffix_member(character) {
            "is_some"
        } else {
            "is_none"
        };
        writeln!(
            probe,
            "    assert!(SpecialWord::new(\"{escaped}\").{check}(), \"alphabet U+{:04X}\");",
            character as u32
        )
        .expect("writing to String cannot fail");
    }
    // The independently stated alphabet has exactly 54 ASCII members.
    assert_eq!(
        (0_u8..=0x7F)
            .map(char::from)
            .filter(|&character| expected_suffix_member(character))
            .count(),
        54
    );
    probe
}

/// Required/optional/bounded/unbounded composition and the derived carrier.
const COMPOSITION_PROBE: &str = r#"    let word = SpecialWord::new("NATO:Word").expect("valid");
    let marking = Marking {
        word: word.clone(),
        alternate: Some(SpecialWord::new("NATO:alt-one").expect("valid")),
        derived: DerivedSpecialWord::new("NATO:Derived_X").expect("valid"),
        reporter: Callsign::new("ALPHA 1").expect("valid"),
        version: SchemaVersion::new("002.5.0").expect("valid"),
        remarks: PreservedRemarks::new("free\ntext").expect("valid"),
        notes: String::from("ordinary"),
        history: BoundedVec::new(vec![SpecialWord::new("NATO:h").expect("valid")]).expect("0..4"),
        required: BoundedVec::new(vec![
            SpecialWord::new("NATO:r-a").expect("valid"),
            SpecialWord::new("NATO:r_b").expect("valid"),
        ])
        .expect("2..3"),
        log: UnboundedVec::new(vec![SpecialWord::new("NATO:L").expect("valid"); 300]).expect("0.."),
    };
    assert_eq!(marking.word.as_str(), "NATO:Word");
    assert_eq!(marking.alternate.as_ref().expect("present").as_str(), "NATO:alt-one");
    assert_eq!(marking.derived.as_str(), "NATO:Derived_X");
    assert!(DerivedSpecialWord::new("NATO:1").is_none(), "derived keeps the profile");
    assert_eq!(marking.history.as_slice()[0].as_str(), "NATO:h");
    assert_eq!(marking.required.as_slice()[1].as_str(), "NATO:r_b");
    assert_eq!(marking.log.as_slice().len(), 300);
    assert!(BoundedVec::<SpecialWord, 2, 3>::new(vec![word.clone()]).is_none());
    assert_eq!(word, SpecialWord::new("NATO:Word").expect("valid"));
    assert_ne!(word, SpecialWord::new("NATO:word").expect("valid"), "case preserved");
    let absent = Marking { alternate: None, ..marking };
    assert!(absent.alternate.is_none());
"#;

/// A client cannot bypass validation by touching the storage.
#[test]
fn the_private_nato_special_words_storage_cannot_be_reached_from_outside_the_module() {
    let source =
        generate(&nato_schema(), GenerationWorld::ClosedSchemaSet).expect("fixture must generate");
    let directory = std::env::temp_dir().join("ams-gra-oms-task042-rust-bypass");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Rust probe directory");
    std::fs::write(directory.join("generated.rs"), &source).expect("write generated module");

    let build = |body: &str| {
        let probe = format!(
            "mod generated {{\n    include!(\"generated.rs\");\n}}\n\nfn main() {{\n{body}\n}}\n"
        );
        std::fs::write(directory.join("probe.rs"), &probe).expect("write probe");
        Command::new("rustc")
            .current_dir(&directory)
            .args(["--edition", "2021", "-o", "probe", "probe.rs"])
            .output()
            .expect("rustc should be available")
    };

    // Positive control: the SAME harness compiles legitimate use, so each
    // failure below is attributable to privacy, not to the harness.
    let control = build(
        "    let made = generated::SpecialWord::new(\"NATO:ok\").unwrap();\n\
         \x20   assert_eq!(made.as_str(), \"NATO:ok\");",
    );
    assert!(
        control.status.success(),
        "the positive control must compile:\n{}",
        String::from_utf8_lossy(&control.stderr)
    );

    for (label, body) in [
        (
            "struct literal",
            "    let _ = generated::SpecialWord { value: String::from(\"nope\") };",
        ),
        (
            "field read",
            "    let made = generated::SpecialWord::new(\"NATO:ok\").unwrap();\n\
             \x20   let _ = made.value;",
        ),
        (
            "private validator call",
            "    let _ = generated::SpecialWord::is_nato_special_words(\"NATO:ok\");",
        ),
    ] {
        let output = build(body);
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
