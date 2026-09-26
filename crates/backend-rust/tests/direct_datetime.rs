mod common;

use ams_gra_oms_backend_rust::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

#[test]
fn direct_date_time_and_named_zulu_share_the_compiled_parser() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-direct-datetime.xsd"),
    )
    .unwrap();
    let source = generate(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
    assert_eq!(source.matches("struct XmlSchemaDateTimeParser;").count(), 1);
    assert!(source.contains("pub observed: XmlSchemaDateTime"));
    assert!(source.contains("pub timestamp: Option<XmlSchemaDateTime>"));
    assert!(source.contains("BoundedVec<XmlSchemaDateTime"));
    assert!(source.contains("UnboundedVec<XmlSchemaDateTime"));
    assert!(source.contains("#[derive(Debug, Clone)]\npub struct Payload"));
    let ordinary = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-record-inheritance.xsd"),
    )
    .unwrap();
    let ordinary_source = generate(&ordinary, GenerationWorld::ClosedSchemaSet).unwrap();
    assert!(ordinary_source.contains("#[derive(Debug, Clone, PartialEq, Eq)]\npub struct"));
    let carrier_derive = source
        .split("pub struct XmlSchemaDateTime")
        .next()
        .unwrap()
        .rsplit("#[derive(")
        .next()
        .unwrap();
    assert!(!carrier_derive.contains("PartialEq"));
    assert!(!carrier_derive.contains("Default"));
    assert!(!source.contains("impl Default for XmlSchemaDateTime"));
    let mut probe = String::from("include!(\"generated.rs\");\nfn main() {\n");
    for case in common::load_cases()
        .into_iter()
        .chain(common::direct_date_time_cases())
    {
        let input = common::escape_for_source(&case.input);
        if let Some(expected) = case.expected {
            let expected = common::escape_for_source(&expected);
            writeln!(
                probe,
                "assert_eq!(XmlSchemaDateTime::new(\"{input}\").unwrap().as_str(), \"{expected}\");"
            )
            .unwrap();
        } else if ![
            "2026-09-20T12:34:56",
            "2026-09-20T12:34:56+00:00",
            "2026-09-20T12:34:56-00:00",
            "2026-09-20T12:34:56+05:00",
            "2026-09-20T12:34:56-13:30",
            "2026-09-20T12:34:56+14:00",
        ]
        .contains(&case.input.as_str())
        {
            writeln!(
                probe,
                "assert!(XmlSchemaDateTime::new(\"{input}\").is_none());"
            )
            .unwrap();
        }
    }
    for value in [
        "2026-09-25T12:00:00",
        "2026-09-25T12:00:00+00:00",
        "2026-09-25T12:00:00-05:00",
    ] {
        writeln!(probe, "assert!(XmlSchemaDateTime::new(\"{value}\").is_some()); assert!(Instant::new(\"{value}\").is_none());").unwrap();
    }
    probe.push_str("assert!(Instant::new(\"2026-09-25T12:00:00Z\").is_some());\n}\n");
    let dir = std::env::temp_dir().join("ams-gra-task046-rust-direct");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("generated.rs"), source).unwrap();
    std::fs::write(dir.join("probe.rs"), probe).unwrap();
    let output = Command::new("rustc")
        .current_dir(&dir)
        .args(["--edition", "2021", "-o", "probe", "probe.rs"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(Command::new(dir.join("probe")).status().unwrap().success());
    std::fs::write(dir.join("illegal.rs"), "mod generated { include!(\"generated.rs\"); }\nfn main() { let _ = generated::XmlSchemaDateTime { lexical: String::new() }; }\n").unwrap();
    let privacy = Command::new("rustc")
        .current_dir(&dir)
        .args(["--edition", "2021", "-o", "illegal", "illegal.rs"])
        .output()
        .unwrap();
    assert!(
        !privacy.status.success(),
        "private lexical storage must reject external construction"
    );
    assert!(String::from_utf8_lossy(&privacy.stderr).contains("private"));
    std::fs::remove_dir_all(dir).unwrap();
}
