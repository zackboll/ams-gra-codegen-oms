//! Task 059: generated checked carriers against the shared corpus.
mod common;
use ams_gra_oms_backend_rust::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::{fmt::Write as _, path::Path, process::Command};

#[test]
fn generated_structured_ascii_rust_corpus() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-structured-ascii.xsd"),
    )
    .unwrap();
    let model = generate(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
    let dir = std::env::temp_dir().join("task059-rust-corpus");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("generated.rs"), model).unwrap();
    let groups = common::structured_ascii_cases();
    assert_eq!(groups.len(), 12);
    let mut probe = String::from("include!(\"generated.rs\");\nfn main() {\n");
    for group in groups {
        for (index, case) in group.cases.iter().enumerate() {
            let input = common::escape_for_rust_source(&case.input);
            let expected = case
                .expected
                .as_ref()
                .map(|s| format!("Some(\"{}\")", common::escape_for_rust_source(s)))
                .unwrap_or_else(|| "None".into());
            writeln!(probe,"assert_eq!({}::new(\"{input}\").map(|v| v.as_str().to_owned()).as_deref(), {expected}, \"{} {index}\");",group.carrier,group.carrier).unwrap();
        }
    }
    probe.push_str("let p = Payload { required: Imo::new(\"IMO1234567\").unwrap(), optional: Some(Prf::new(\"178\").unwrap()), bounded: BoundedVec::new(vec![FileName::new(\"a.b\").unwrap()]).unwrap(), unbounded: UnboundedVec::new(vec![Octal::new(\"7\").unwrap(); 40]).unwrap() }; assert_eq!(p.unbounded.as_slice().len(), 40);\n}\n");
    std::fs::write(dir.join("probe.rs"), probe).unwrap();
    let built = Command::new("rustc")
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
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let run = Command::new(dir.join("probe")).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::remove_dir_all(dir).unwrap();
}
