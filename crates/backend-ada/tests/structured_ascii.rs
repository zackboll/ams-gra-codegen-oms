//! Task 059: generated Ada carriers against the same compiler-backed corpus.
mod common;
use ams_gra_oms_backend_ada::AdaBackend;
use ams_gra_oms_codegen_core::{Backend, GenerationWorld};
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::{fmt::Write as _, path::Path, process::Command};

#[test]
fn generated_structured_ascii_ada_corpus_under_both_policies() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-structured-ascii.xsd"),
    )
    .unwrap();
    let files = AdaBackend
        .generate(&schema, GenerationWorld::ClosedSchemaSet)
        .unwrap();
    let dir = std::env::temp_dir().join("task059-ada-corpus");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for file in files {
        std::fs::write(dir.join(file.relative_path), file.contents).unwrap();
    }
    let groups = common::structured_ascii_cases();
    assert_eq!(groups.len(), 12);
    let mut probe = String::from(
        "with Ada.Text_IO; use Ada.Text_IO;\nwith Test.Structured; use Test.Structured;\nprocedure Probe is\n   procedure Fail (Label : String) is\n   begin\n      Put_Line (\"FAIL: \" & Label);\n      raise Program_Error;\n   end Fail;\nbegin\n",
    );
    for group in groups {
        for (index, case) in group.cases.iter().enumerate() {
            let input = common::ada_literal(&case.input);
            let label = format!("\"{} {index}\"", group.carrier);
            match &case.expected {
                Some(stored) => writeln!(probe,"   declare V : constant {} := Create ({input}); begin if Value (V) /= {} then Fail ({label}); end if; end;",group.carrier,common::ada_literal(stored)),
                None => writeln!(probe,"   begin declare V : constant {} := Create ({input}); begin Fail ({label} & Value (V)); end; exception when Constraint_Error => null; end;",group.carrier),
            }.unwrap();
        }
    }
    probe.push_str("   Put_Line (\"ok\");\nend Probe;\n");
    for (flags, policy) in [
        (["-q", "-f", "probe.adb"], ""),
        (
            ["-q", "-f", "probe.adb"],
            "pragma Assertion_Policy (Ignore);\n",
        ),
    ] {
        std::fs::write(dir.join("probe.adb"), format!("{policy}{probe}")).unwrap();
        let built = Command::new("gnatmake")
            .current_dir(&dir)
            .args(flags)
            .output()
            .expect("GNAT required");
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        let run = Command::new(dir.join("probe")).output().unwrap();
        assert!(
            run.status.success(),
            "{}{}",
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}
