mod common;

use ams_gra_oms_backend_ada::AdaBackend;
use ams_gra_oms_codegen_core::{Backend, GenerationWorld};
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

#[test]
fn direct_date_time_and_named_zulu_share_the_gnat_parser() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-direct-datetime.xsd"),
    )
    .unwrap();
    let files = AdaBackend
        .generate(&schema, GenerationWorld::ClosedSchemaSet)
        .unwrap();
    let spec = &files
        .iter()
        .find(|file| file.relative_path.to_string_lossy() == "test-temporal.ads")
        .unwrap()
        .contents;
    let body = &files
        .iter()
        .find(|file| file.relative_path.to_string_lossy() == "test-temporal.adb")
        .unwrap()
        .contents;
    assert!(spec.contains("Value : XML_Schema_Date_Time;"));
    assert!(spec.contains("type Payload_Timestamp_Optional"));
    assert_eq!(
        body.matches("function Is_Date_Time (Text : String) return Boolean is")
            .count(),
        1
    );
    let mut probe = String::from(
        "with Test.Temporal;\nprocedure Probe is\n   procedure Valid (Input, Expected : String) is\n      Made : constant Test.Temporal.XML_Schema_Date_Time := Test.Temporal.Create (Input);\n   begin\n      if Test.Temporal.Value (Made) /= Expected then raise Program_Error; end if;\n   end Valid;\n   procedure Invalid (Input : String) is\n   begin\n      declare\n         Made : constant Test.Temporal.XML_Schema_Date_Time := Test.Temporal.Create (Input);\n      begin\n         if Test.Temporal.Value (Made)'Length >= 0 then raise Program_Error; end if;\n      end;\n   exception\n      when Constraint_Error => null;\n   end Invalid;\nbegin\n",
    );
    for case in common::load_cases()
        .into_iter()
        .chain(common::direct_date_time_cases())
    {
        let input = common::ada_literal(&case.input);
        if let Some(expected) = case.expected {
            writeln!(
                probe,
                "   Valid ({input}, {});",
                common::ada_literal(&expected)
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
            writeln!(probe, "   Invalid ({input});").unwrap();
        }
    }
    probe.push_str("   declare\n      Absent : Test.Temporal.Payload_Timestamp_Optional;\n   begin\n      if Absent.Is_Present then raise Program_Error; end if;\n   end;\n   declare\n      Present : Test.Temporal.Payload_Timestamp_Optional (True) := (Is_Present => True, Value => Test.Temporal.Create (\"2026-09-25T12:00:00+05:30\"));\n   begin\n      if Test.Temporal.Value (Present.Value) /= \"2026-09-25T12:00:00+05:30\" then raise Program_Error; end if;\n   end;\n   declare\n      S : constant String (9 .. 28) := \"2026-09-25T12:34:56Z\";\n   begin\n      Valid (S, \"2026-09-25T12:34:56Z\");\n   end;\n   declare\n      Rejected : Boolean := False;\n   begin\n      begin\n         declare\n            Bad : Test.Temporal.XML_Schema_Date_Time;\n         begin\n            Valid (Test.Temporal.Value (Bad), \"\");\n         end;\n      exception\n         when Program_Error => Rejected := True;\n      end;\n      if not Rejected then raise Program_Error; end if;\n   end;\nend Probe;\n");
    let dir = std::env::temp_dir().join("ams-gra-task046-ada-direct");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for file in &files {
        std::fs::write(dir.join(&file.relative_path), &file.contents).unwrap();
    }
    std::fs::write(dir.join("probe.adb"), probe).unwrap();
    for (flags, policy) in [
        (vec!["-q", "-f", "probe.adb"], ""),
        (
            vec!["-q", "-f", "-gnata", "probe.adb"],
            "pragma Assertion_Policy (Ignore);\n",
        ),
    ] {
        let probe = std::fs::read_to_string(dir.join("probe.adb")).unwrap();
        let probe = probe.trim_start_matches("pragma Assertion_Policy (Ignore);\n");
        std::fs::write(dir.join("probe.adb"), format!("{policy}{probe}")).unwrap();
        let output = Command::new("gnatmake")
            .current_dir(&dir)
            .args(&flags)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = Command::new(dir.join("probe")).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}
