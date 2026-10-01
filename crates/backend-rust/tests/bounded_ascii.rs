//! Task 058: generated Rust bounded-ASCII carriers, compiled with warnings
//! denied and run against the shared corpus for EVERY admitted alphabet.

mod common;

use ams_gra_oms_backend_rust::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn source() -> String {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-bounded-ascii.xsd"),
    )
    .expect("fixture parses");
    generate(&schema, GenerationWorld::ClosedSchemaSet).expect("fixture generates")
}

fn rustc(dir: &Path, file: &str, deny: bool) -> std::process::Output {
    let mut command = Command::new("rustc");
    command
        .current_dir(dir)
        .args(["--edition", "2021", "-o", "probe", file]);
    if deny {
        command.args(["-D", "warnings"]);
    }
    command.output().expect("rustc runs")
}

/// Opaque API: private storage, `new` / `as_str`, no `Default`, and the
/// class rendered from numeric ranges rather than from the XSD spelling.
#[test]
fn bounded_ascii_carriers_are_opaque_validated_values() {
    let source = source();
    for carrier in ["Blank", "Tail8", "Serial7", "Fixed10", "DerivedTail"] {
        assert!(
            source.contains(&format!("pub struct {carrier} {{\n    value: String,\n}}")),
            "{carrier}"
        );
    }
    assert!(source.contains("pub fn new(value: &str) -> Option<Self>"));
    assert!(source.contains("pub fn as_str(&self) -> &str"));
    assert!(!source.contains("impl Default for"));
    assert!(!source.contains("pub value"));
    // `length` is one equality; min/max is an interval; ranges are numeric.
    assert!(source.contains("const LENGTH: usize = 0;"));
    assert!(source.contains("const LENGTH: usize = 8;"));
    assert!(source.contains("const MIN_LENGTH: usize = 4;"));
    assert!(source.contains("matches!(byte, 0x20 | 0x30..=0x39 | 0x41..=0x5A)"));
    assert!(
        source.contains("matches!(byte, 0x20..=0x2E | 0x30..=0x39 | 0x3B..=0x60 | 0x7B..=0x7E)")
    );
    for forbidden in ["regex", "Regex", ".trim(", "to_lowercase", "to_uppercase"] {
        assert!(!source.contains(forbidden), "{forbidden}");
    }
    // Storage composition, unchanged occurrence semantics.
    for needle in [
        "pub marker: Blank,",
        "pub aircraft: Tail8,",
        "pub optionalmarker: Option<Blank>,",
        "pub optionalword: Option<Word20>,",
        "pub markers: BoundedVec<Blank, 0, 3>,",
        "pub codes: UnboundedVec<Code4, 0>,",
        "pub derived: DerivedTail,",
        "pub notes: String,",
        "pub launch: Letters3,",
        "pub serial: Serial7,",
        "ByEmpty(Blank),",
    ] {
        assert!(source.contains(needle), "missing {needle:?}");
    }
}

/// No struct literal, field read or `Default` compiles outside the module.
#[test]
fn bounded_ascii_storage_is_private() {
    let dir = std::env::temp_dir().join("ams-gra-task058-rust-private");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("generated.rs"), source()).unwrap();
    for body in [
        "let _ = generated::Blank { value: String::new() };",
        "let v = generated::Blank::new(\"\").unwrap(); let _ = v.value;",
        "let _: generated::Blank = Default::default();",
    ] {
        std::fs::write(
            dir.join("illegal.rs"),
            format!("mod generated {{ include!(\"generated.rs\"); }}\nfn main() {{ {body} }}\n"),
        )
        .unwrap();
        let output = rustc(&dir, "illegal.rs", false);
        assert!(!output.status.success(), "{body} must not compile");
    }
    std::fs::remove_dir_all(dir).unwrap();
}

/// Every corpus case of every carrier, plus composition and zero-length
/// storage, executed by a probe compiled with `-D warnings`.
#[test]
fn generated_bounded_ascii_validators_match_the_shared_corpus() {
    let groups = common::bounded_ascii_cases();
    assert_eq!(groups.len(), 23, "one group per representative");
    let mut probe = String::from("include!(\"generated.rs\");\n\nfn main() {\n");
    let mut total = 0;
    for group in &groups {
        let carrier = &group.carrier;
        for (index, case) in group.cases.iter().enumerate() {
            let input = common::escape_for_rust_source(&case.input);
            match &case.expected {
                Some(stored) => {
                    let stored = common::escape_for_rust_source(stored);
                    writeln!(
                        probe,
                        "    assert_eq!({carrier}::new(\"{input}\").map(|v| v.as_str().to_owned()).as_deref(), Some(\"{stored}\"), \"{carrier} case {index}\");"
                    )
                    .unwrap();
                }
                None => writeln!(
                    probe,
                    "    assert!({carrier}::new(\"{input}\").is_none(), \"{carrier} case {index}\");"
                )
                .unwrap(),
            }
            total += 1;
        }
    }
    assert!(total >= 700, "{total} cases");
    probe.push_str(COMPOSITION);
    probe.push_str("    println!(\"ok\");\n}\n");
    let dir = std::env::temp_dir().join("ams-gra-task058-rust-corpus");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("generated.rs"), source()).unwrap();
    std::fs::write(dir.join("probe.rs"), &probe).unwrap();
    let built = rustc(&dir, "probe.rs", true);
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let run = Command::new(dir.join("probe")).output().unwrap();
    assert!(
        run.status.success() && String::from_utf8_lossy(&run.stdout).trim() == "ok",
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// Zero-length values in every storage position, spare capacity, the
/// derived chain, inheritance, Choice, case and SPACE significance.
const COMPOSITION: &str = r#"    let empty = Blank::new("").unwrap();
    assert_eq!(empty.as_str(), "");
    assert_eq!(empty, Blank::new("").unwrap());
    let payload = Payload {
        marker: empty.clone(),
        aircraft: Tail8::new("AB12 XY ").unwrap(),
        optionalmarker: Some(empty.clone()),
        optionalword: None,
        markers: BoundedVec::new(vec![empty.clone(), empty.clone()]).unwrap(),
        codes: UnboundedVec::new(vec![Code4::new("aB3z").unwrap(); 40]).unwrap(),
        derived: DerivedTail::new("        ").unwrap(),
        notes: String::from(" free text "),
    };
    let moved = payload.clone();
    assert_eq!(moved.marker.as_str(), "");
    assert_eq!(moved.aircraft.as_str(), "AB12 XY ");
    assert_eq!(moved.optionalmarker.as_ref().unwrap().as_str(), "");
    assert!(moved.optionalword.is_none());
    assert_eq!(moved.markers.as_slice().len(), 2);
    assert!(moved.markers.as_slice().iter().all(|m| m.as_str().is_empty()));
    assert_eq!(moved.codes.as_slice().len(), 40);
    assert_eq!(moved.derived.as_str().len(), 8);
    assert_eq!(moved.notes, " free text ");
    assert!(BoundedVec::<Blank, 0, 3>::new(vec![empty.clone(); 4]).is_none());
    assert!(DerivedTail::new("AB12XY").is_none(), "the chain keeps length 8");
    let concrete = Concrete {
        launch: Letters3::new("ABC").unwrap(),
        serial: Serial7::new("\"'\\[]{}").unwrap(),
    };
    assert_eq!(concrete.launch.as_str(), "ABC");
    assert_eq!(concrete.serial.as_str(), "\"'\\[]{}");
    match Pick::ByEmpty(empty.clone()) {
        Pick::ByEmpty(value) => assert_eq!(value.as_str(), ""),
        Pick::ByDigits(_) => unreachable!(),
    }
    assert!(Pick::ByDigits(Digits4::new("0042").unwrap()) != Pick::ByEmpty(empty));
    assert_ne!(Word20::new("abc").unwrap(), Word20::new("abc ").unwrap());
    assert_ne!(Word20::new("abc").unwrap(), Word20::new("ABC").unwrap());
"#;
