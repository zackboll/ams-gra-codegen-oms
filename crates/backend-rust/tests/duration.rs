//! Task 057: XML Schema `duration` checked lexical carriers in Rust.
//!
//! The shared `tests/fixtures/temporal/duration.txt` corpus is run against
//! BOTH the named zero-facet carrier (`Span`) and the direct support carrier
//! (`XmlSchemaDuration`), compiled with warnings denied.

mod common;

use ams_gra_oms_backend_rust::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_ir::SchemaIr;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn fixture(name: &str) -> SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../xsd-frontend/tests/fixtures/{name}")),
    )
    .unwrap()
}

fn compile_and_run(tag: &str, source: &str, probe: &str) {
    let dir = std::env::temp_dir().join(format!("ams-gra-task057-rust-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("generated.rs"), source).unwrap();
    std::fs::write(dir.join("probe.rs"), probe).unwrap();
    let output = Command::new("rustc")
        .current_dir(&dir)
        .args([
            "--edition",
            "2021",
            "-D",
            "warnings",
            "-o",
            "probe",
            "probe.rs",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let run = Command::new(dir.join("probe")).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// Private lexical storage: neither carrier can be built around the checked
/// constructor, so an invalid (for example empty) spelling is unobservable.
#[test]
fn duration_carriers_cannot_be_built_outside_the_checked_constructor() {
    let source = generate(
        &fixture("backend-duration.xsd"),
        GenerationWorld::ClosedSchemaSet,
    )
    .unwrap();
    for (tag, carrier) in [("direct", "XmlSchemaDuration"), ("named", "Span")] {
        let dir = std::env::temp_dir().join(format!("ams-gra-task057-rust-private-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("generated.rs"), &source).unwrap();
        std::fs::write(
            dir.join("illegal.rs"),
            format!(
                "mod generated {{ include!(\"generated.rs\"); }}\n\
                 fn main() {{ let _ = generated::{carrier} {{ lexical: String::new() }}; }}\n"
            ),
        )
        .unwrap();
        let output = Command::new("rustc")
            .current_dir(&dir)
            .args(["--edition", "2021", "-o", "illegal", "illegal.rs"])
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "{carrier} must reject construction"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("private"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn named_and_direct_duration_share_one_parser_and_the_corpus() {
    let source = generate(
        &fixture("backend-duration.xsd"),
        GenerationWorld::ClosedSchemaSet,
    )
    .unwrap();
    // One shared parser, one direct carrier, one named carrier.
    assert_eq!(source.matches("struct XmlSchemaDurationParser;").count(), 1);
    assert_eq!(source.matches("pub struct XmlSchemaDuration {").count(), 1);
    assert_eq!(source.matches("pub struct Span {").count(), 1);
    // Duration never drags in the dateTime parser.
    assert!(!source.contains("XmlSchemaDateTimeParser"));
    // Required, optional, bounded, unbounded, inherited and Choice storage.
    for needle in [
        "pub namedduration: Span,",
        "pub directduration: XmlSchemaDuration,",
        "pub optionalduration: Option<XmlSchemaDuration>,",
        "pub repeatedduration: BoundedVec<XmlSchemaDuration, 0, 3>,",
        "pub unboundedduration: UnboundedVec<XmlSchemaDuration, 0>,",
        "pub struct Concrete {\n    pub step: XmlSchemaDuration,",
        "    ByDuration(XmlSchemaDuration),",
    ] {
        assert!(source.contains(needle), "missing {needle:?}");
    }
    // No value-space claims: no equality/ordering/Default on the carriers,
    // nor on the structures that hold them.
    for carrier in ["XmlSchemaDuration", "Span"] {
        let derive = source
            .split(&format!("pub struct {carrier} {{"))
            .next()
            .unwrap()
            .rsplit("#[derive(")
            .next()
            .unwrap();
        assert!(derive.starts_with("Clone, Debug)]"), "{carrier}: {derive}");
    }
    assert!(!source.contains("impl Default for"));
    assert!(source.contains("#[derive(Debug, Clone)]\npub struct Payload"));
    assert!(source.contains("#[derive(Debug, Clone)]\npub enum Pick"));
    // The duration-free declaration keeps its full equality.
    assert!(source.contains("#[derive(Debug, Clone, PartialEq, Eq)]\npub struct Unrelated"));

    let mut probe =
        String::from("#![allow(dead_code)]\ninclude!(\"generated.rs\");\nfn main() {\n");
    let (mut valid, mut invalid) = (0, 0);
    for case in common::duration_cases() {
        let input = common::escape_for_source(&case.input);
        if let Some(expected) = case.expected {
            let expected = common::escape_for_source(&expected);
            writeln!(
                probe,
                "assert_eq!(XmlSchemaDuration::new(\"{input}\").unwrap().as_str(), \"{expected}\");\n\
                 assert_eq!(Span::new(\"{input}\").unwrap().as_str(), \"{expected}\");"
            )
            .unwrap();
            valid += 1;
        } else {
            writeln!(
                probe,
                "assert!(XmlSchemaDuration::new(\"{input}\").is_none());\n\
                 assert!(Span::new(\"{input}\").is_none());"
            )
            .unwrap();
            invalid += 1;
        }
    }
    assert!(
        valid >= 30 && invalid >= 60,
        "{valid} valid / {invalid} invalid"
    );
    // No canonicalization; composition in every storage position.
    probe.push_str(concat!(
        "assert_eq!(XmlSchemaDuration::new(\"P12M\").unwrap().as_str(), \"P12M\");\n",
        "let span = Span::new(\"PT1H\").unwrap();\n",
        "let direct = XmlSchemaDuration::new(\"-P1D\").unwrap();\n",
        "let payload = Payload {\n",
        "    namedduration: span.clone(),\n",
        "    directduration: direct.clone(),\n",
        "    optionalduration: Some(direct.clone()),\n",
        "    repeatedduration: BoundedVec::new(vec![direct.clone(), direct.clone()]).unwrap(),\n",
        "    unboundedduration: UnboundedVec::new(vec![direct.clone(); 5]).unwrap(),\n",
        "};\n",
        "let moved = payload;\n",
        "assert_eq!(moved.namedduration.as_str(), \"PT1H\");\n",
        "assert_eq!(moved.directduration.as_str(), \"-P1D\");\n",
        "assert_eq!(moved.optionalduration.as_ref().unwrap().as_str(), \"-P1D\");\n",
        "assert_eq!(moved.repeatedduration.as_slice().len(), 2);\n",
        "assert_eq!(moved.unboundedduration.as_slice().len(), 5);\n",
        "assert!(BoundedVec::<XmlSchemaDuration, 0, 3>::new(vec![direct.clone(); 4]).is_none());\n",
        "let inherited = Concrete { step: XmlSchemaDuration::new(\"PT0.5S\").unwrap(), label: String::new() };\n",
        "assert_eq!(inherited.step.as_str(), \"PT0.5S\");\n",
        "match Pick::ByDuration(direct) { Pick::ByDuration(d) => assert_eq!(d.as_str(), \"-P1D\"), Pick::ByCount(_) => unreachable!() }\n",
        "}\n",
    ));
    compile_and_run("corpus", &source, &probe);
}
