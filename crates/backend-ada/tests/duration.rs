//! Task 057: XML Schema `duration` checked lexical carriers under GNAT.
//!
//! The shared `tests/fixtures/temporal/duration.txt` corpus is run against
//! BOTH the named zero-facet carrier (`Span`) and the direct support carrier
//! (`XML_Schema_Duration`), compiled and executed twice: with the default
//! policy and with `-gnata` plus `pragma Assertion_Policy (Ignore)`, proving
//! the Task 040 invalid-default protection is not assertion-dependent.

mod common;

use ams_gra_oms_backend_ada::AdaBackend;
use ams_gra_oms_codegen_core::{Backend, GenerationWorld};
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

const PROBE_HEAD: &str = "with Ada.Strings.Unbounded;
with Test.Duration; use Test.Duration;
procedure Probe is
   procedure Valid (Input, Expected : String) is
      D : constant XML_Schema_Duration := Create (Input);
      N : constant Span := Create (Input);
   begin
      if Value (D) /= Expected or else Value (N) /= Expected then
         raise Program_Error with \"stored \" & Value (D);
      end if;
   end Valid;
   procedure Invalid (Input : String) is
      Rejected : Natural := 0;
   begin
      begin
         declare
            D : constant XML_Schema_Duration := Create (Input);
         begin
            if Value (D)'Length >= 0 then raise Program_Error; end if;
         end;
      exception
         when Constraint_Error => Rejected := Rejected + 1;
      end;
      begin
         declare
            N : constant Span := Create (Input);
         begin
            if Value (N)'Length >= 0 then raise Program_Error; end if;
         end;
      exception
         when Constraint_Error => Rejected := Rejected + 1;
      end;
      if Rejected /= 2 then raise Program_Error; end if;
   end Invalid;
begin
";

/// Composition, lifecycle and default-rejection checks after the corpus.
const PROBE_TAIL: &str = "   --  No canonicalization.
   Valid (\"P12M\", \"P12M\");
   --  A non-1-based input string.
   declare
      S : constant String (7 .. 12) := \"P1DT2H\";
   begin
      Valid (S, \"P1DT2H\");
   end;
   declare
      D : constant XML_Schema_Duration := Create (\"-P1D\");
      Absent : Payload_OptionalDuration_Optional;
      Present : constant Payload_OptionalDuration_Optional :=
        (Is_Present => True, Value => D);
      Bounded : Payload_RepeatedDuration_Sequence :=
        To_Sequence ((1 => D, 2 => Create (\"PT1S\")));
      Unbounded : Payload_UnboundedDuration_Sequence;
      Copy : XML_Schema_Duration := D;
      Inherited : constant Concrete :=
        (Step => Create (\"PT0.5S\"), Label => Standard.Ada.Strings.Unbounded.Null_Unbounded_String);
      Choice : constant Pick := (Kind => ByDuration_Kind, ByDuration => D);
   begin
      if Absent.Is_Present then raise Program_Error; end if;
      if Value (Present.Value) /= \"-P1D\" then raise Program_Error; end if;
      Append (Bounded, D);
      if Length (Bounded) /= 3 or else Value (Element (Bounded, 2)) /= \"PT1S\" then
         raise Program_Error;
      end if;
      for I in 1 .. 50 loop
         Append (Unbounded, D);
      end loop;
      if Length (Unbounded) /= 50 then raise Program_Error; end if;
      --  A record holding a carrier is built by aggregate, never defaulted.
      declare
         Whole : constant Payload :=
           (NamedDuration => Create (\"PT1H\"), DirectDuration => D,
            OptionalDuration => Present, RepeatedDuration => Bounded,
            UnboundedDuration => Unbounded);
      begin
         Copy := Whole.DirectDuration;
         if Value (Copy) /= \"-P1D\" or else Value (Whole.NamedDuration) /= \"PT1H\" then
            raise Program_Error;
         end if;
      end;
      if Value (Inherited.Step) /= \"PT0.5S\" or else Value (Choice.ByDuration) /= \"-P1D\" then
         raise Program_Error;
      end if;
   end;
   --  Task 040: a default-declared carrier is rejected, for both carriers.
   declare
      Rejected : Natural := 0;
   begin
      begin
         declare
            Bad : XML_Schema_Duration;
         begin
            Valid (Value (Bad), \"\");
         end;
      exception
         when Program_Error => Rejected := Rejected + 1;
      end;
      begin
         declare
            Bad : Span;
         begin
            Valid (Value (Bad), \"\");
         end;
      exception
         when Program_Error => Rejected := Rejected + 1;
      end;
      if Rejected /= 2 then raise Program_Error; end if;
   end;
end Probe;
";

#[test]
fn named_and_direct_duration_share_the_gnat_validator_under_both_policies() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-duration.xsd"),
    )
    .unwrap();
    let files = AdaBackend
        .generate(&schema, GenerationWorld::ClosedSchemaSet)
        .unwrap();
    let file = |name: &str| {
        &files
            .iter()
            .find(|file| file.relative_path.to_string_lossy() == name)
            .unwrap_or_else(|| panic!("missing {name}"))
            .contents
    };
    let spec = file("test-duration.ads");
    let body = file("test-duration.adb");
    assert!(spec.contains("type XML_Schema_Duration is private;"));
    assert!(spec.contains("type Span is private;"));
    assert!(spec.contains("when True  => Value : XML_Schema_Duration;"));
    assert!(!spec.contains("Duration range") && !spec.contains(": Duration;"));
    assert_eq!(
        body.matches("package body XML_Schema_Duration_Parser is")
            .count(),
        1
    );
    assert_eq!(
        body.matches("function Is_Duration (Text : String) return Boolean is")
            .count(),
        1
    );
    assert!(!body.contains("Is_Date_Time"));
    let mut probe = String::from(PROBE_HEAD);
    for case in common::duration_cases() {
        let input = common::ada_literal(&case.input);
        match case.expected {
            Some(expected) => writeln!(
                probe,
                "   Valid ({input}, {});",
                common::ada_literal(&expected)
            )
            .unwrap(),
            None => writeln!(probe, "   Invalid ({input});").unwrap(),
        }
    }
    probe.push_str(PROBE_TAIL);
    let dir = std::env::temp_dir().join("ams-gra-task057-ada");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for file in &files {
        std::fs::write(dir.join(&file.relative_path), &file.contents).unwrap();
    }
    for (flags, policy) in [
        (vec!["-q", "-f", "probe.adb"], ""),
        (
            vec!["-q", "-f", "-gnata", "probe.adb"],
            "pragma Assertion_Policy (Ignore);\n",
        ),
    ] {
        std::fs::write(dir.join("probe.adb"), format!("{policy}{probe}")).unwrap();
        let output = Command::new("gnatmake")
            .current_dir(&dir)
            .args(&flags)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let run = Command::new(dir.join("probe")).output().unwrap();
        assert!(
            run.status.success(),
            "policy {policy:?}: {}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}
