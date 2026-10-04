//! Task 060: production-generated factored carriers accept arbitrary String bounds.
//! Every policy is compiled without optimization and with strict overflow checks.

use ams_gra_oms_backend_ada::AdaBackend;
use ams_gra_oms_codegen_core::{Backend, GenerationWorld};
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::{fmt::Write as _, path::Path, process::Command};

#[test]
fn generated_factored_ascii_ada_bounds_under_strict_overflow() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate/codec-alternating-ascii.xsd");
    let schema = load_schema_document(&fixture).expect("alternating ASCII fixture");
    let files = AdaBackend
        .generate(&schema, GenerationWorld::ClosedSchemaSet)
        .expect("production Ada generation");
    let dir = std::env::temp_dir().join(format!("task060-ada-bounds-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("probe directory");
    for file in files {
        std::fs::write(dir.join(file.relative_path), file.contents).expect("generated source");
    }

    let mut probe = String::from(PROBE_HEAD);
    let valid = [
        "1.2.3.4",
        "1.2.3.99",
        "1.2.3.199",
        "1.2.3.249",
        "1.2.3.255",
        "0.0.0.0",
        "255.255.255.255",
    ];
    // Lexical controls all reach the matcher; null/short controls fail facets.
    let invalid = [
        "01.2.3.4",
        "1.2.3.256",
        "1..2.3.4",
        "123.4.5",
        "1.2.3.4.5",
        ".1.2.3.4",
        "1.2.3.4.",
        "1.2.3.x",
        "",
        "1",
        "1.2",
        "1.2.3",
    ];
    for (cases, accept) in [(&valid[..], "True"), (&invalid[..], "False")] {
        for spelling in cases {
            writeln!(probe, "   Check_Placements (\"{spelling}\", {accept});")
                .expect("client case");
        }
    }
    probe
        .push_str("   Ada.Text_IO.Put_Line (\"ADA FACTORED ASCII BOUNDS: PASSED\");\nend Probe;\n");

    for (policy, flags, prefix) in [
        ("default", &[][..], ""),
        ("assertions-enabled", &["-gnata"][..], ""),
        (
            "assertions-ignored",
            &["-gnata"][..],
            "pragma Assertion_Policy (Ignore);\n",
        ),
    ] {
        std::fs::write(dir.join("probe.adb"), format!("{prefix}{probe}")).expect("client");
        let built = Command::new("gnatmake")
            .current_dir(&dir)
            .args(["-q", "-f", "-gnat2022", "-O0", "-gnato"])
            .args(flags)
            .arg("probe.adb")
            .output()
            .expect("GNAT required");
        println!(
            "\nTASK060 ADA BOUNDS {policy} compile exit: {:?}",
            built.status.code()
        );
        assert!(
            built.status.success(),
            "{policy}; generated files retained at {}: {}{}",
            dir.display(),
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
        let run = Command::new(dir.join("probe"))
            .output()
            .expect("client runs");
        let stdout = String::from_utf8_lossy(&run.stdout);
        println!(
            "TASK060 ADA BOUNDS {policy} execution exit: {:?}",
            run.status.code()
        );
        assert!(
            run.status.success()
                && stdout
                    .trim_end()
                    .ends_with("ADA FACTORED ASCII BOUNDS: PASSED"),
            "{policy}; generated files retained at {}: {stdout}{}",
            dir.display(),
            String::from_utf8_lossy(&run.stderr)
        );
        println!(
            "TASK060 ADA BOUNDS {policy}: PASSED ({} valid + {} invalid, each 1-based/shifted/high)",
            valid.len(),
            invalid.len()
        );
    }
    std::fs::remove_dir_all(dir).expect("remove passing probe");
}

// Explicit branches/exceptions, never pragma Assert: these checks also run
// under Assertion_Policy (Ignore). Rejecting arithmetic overflow is not lexical
// rejection: invalid inputs must have the constructor's exact diagnostic.
const PROBE_HEAD: &str = r#"with Ada.Exceptions;
with Ada.Text_IO;
with Programs.Oam;
procedure Probe is
   procedure Check (Input, Expected, Placement : String; Expected_Valid : Boolean) is
   begin
      declare
         Item : constant Programs.Oam.AddressType := Programs.Oam.Create (Input);
      begin
         if not Expected_Valid then
            raise Program_Error with Placement & " accepted invalid " & Expected;
         end if;
         if Programs.Oam.Value (Item) /= Expected then
            raise Program_Error with Placement & " changed spelling " & Expected;
         end if;
      end;
   exception
      when Error : Constraint_Error =>
         Ada.Text_IO.Put_Line (Placement & " " & Expected & ": " &
           Ada.Exceptions.Exception_Information (Error));
         if Expected_Valid or else Ada.Exceptions.Exception_Message (Error) /=
           "invalid structured-ASCII string"
         then
            raise Program_Error with "unexpected constructor rejection";
         end if;
   end Check;

   procedure Check_Placements (Spelling : String; Expected_Valid : Boolean) is
      One : String (1 .. Spelling'Length) := Spelling;
      Shifted : String (37 .. 37 + (Spelling'Length - 1)) := Spelling;
   begin
      Check (One, Spelling, "1-based", Expected_Valid);
      Check (Shifted, Spelling, "shifted", Expected_Valid);
      if Spelling'Length = 0 then
         declare
            High_Null : String (Positive'Last .. Positive'Last - 1) := "";
         begin
            Check (High_Null, Spelling, "high-null", Expected_Valid);
         end;
      else
         declare
            High : String (Positive'Last - (Spelling'Length - 1) .. Positive'Last)
              := Spelling;
         begin
            Check (High, Spelling, "high", Expected_Valid);
         end;
      end if;
   end Check_Placements;
begin
"#;
