//! Task 058: generated Ada bounded-ASCII carriers under GNAT.
//!
//! The generated package is compiled with `-gnatwa -gnatwe`; the probe runs
//! the shared corpus for EVERY admitted alphabet, then exercises zero-length
//! values in every storage form and Task 040 default rejection, twice: with
//! the default policy and with `-gnata` plus `pragma Assertion_Policy
//! (Ignore)`, proving the rejection does not depend on assertions.

mod common;

use ams_gra_oms_backend_ada::AdaBackend;
use ams_gra_oms_codegen_core::{Backend, GenerationWorld};
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn files() -> Vec<ams_gra_oms_codegen_core::GeneratedFile> {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-bounded-ascii.xsd"),
    )
    .expect("fixture parses");
    AdaBackend
        .generate(&schema, GenerationWorld::ClosedSchemaSet)
        .expect("fixture generates")
}

fn gnat_available() -> bool {
    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but gnatmake is unavailable"
        );
        return false;
    }
    true
}

#[test]
fn bounded_ascii_carriers_are_private_with_rejecting_defaults() {
    let files = files();
    let spec = &files
        .iter()
        .find(|f| f.relative_path.to_string_lossy() == "test-bounded.ads")
        .expect("spec")
        .contents;
    let body = &files
        .iter()
        .find(|f| f.relative_path.to_string_lossy() == "test-bounded.adb")
        .expect("body")
        .contents;
    let (visible, private_part) = spec.split_once("\nprivate\n").expect("private part");
    for carrier in ["Blank", "Tail8", "Serial7", "Fixed10", "DerivedTail"] {
        assert!(visible.contains(&format!("type {carrier} is private;")));
        assert!(visible.contains(&format!(
            "function Create (Value : String) return {carrier};"
        )));
        assert!(visible.contains(&format!("function Value (Item : {carrier}) return String;")));
        assert!(private_part.contains(&format!(
            "with \"{carrier} requires initialization from Create\";"
        )));
    }
    assert!(!visible.contains("Text :"));
    assert!(body.contains("Length_Facet : constant := 0;"));
    assert!(body.contains("Value'Length /= Length_Facet"));
    assert!(body.contains("Value'Length not in Min_Length .. Max_Length"));
    assert!(body.contains(
        "(Character'Pos (Item) in 16#20#..16#2E# | 16#30# .. 16#39# | 16#3B# .. 16#60# | 16#7B# .. 16#7E#);"
    ) || body.contains(
        "(Character'Pos (Item) in 16#20# .. 16#2E# | 16#30# .. 16#39# | 16#3B# .. 16#60# | 16#7B# .. 16#7E#);"
    ));
}

#[test]
fn generated_bounded_ascii_validators_match_the_shared_corpus_under_both_policies() {
    if !gnat_available() {
        return;
    }
    let groups = common::bounded_ascii_cases();
    assert_eq!(groups.len(), 23);
    let mut probe = String::from(PROBE_HEAD);
    let mut total = 0;
    for group in &groups {
        let carrier = &group.carrier;
        for (index, case) in group.cases.iter().enumerate() {
            let input = common::ada_literal(&case.input);
            let label = format!("\"{carrier} case {index}\"");
            match &case.expected {
                Some(stored) => writeln!(
                    probe,
                    "   declare V : constant {carrier} := Create ({input}); begin Check (Value (V), {}, {label}); end;",
                    common::ada_literal(stored)
                ),
                None => writeln!(
                    probe,
                    "   begin declare V : constant {carrier} := Create ({input}); begin Fail ({label} & Value (V)); end; exception when Constraint_Error => null; end;"
                ),
            }
            .unwrap();
            total += 1;
        }
    }
    assert!(total >= 700, "{total}");
    probe.push_str(PROBE_TAIL);

    let dir = std::env::temp_dir().join("ams-gra-task058-ada");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for file in files() {
        std::fs::write(dir.join(&file.relative_path), &file.contents).unwrap();
    }
    // The generated package alone under `-gnatwa`. The ONLY tolerated
    // diagnostic is the pre-existing Task 040 bounded-sequence `Append` guard
    // (`Count >= Max` on a `0 .. Max` subtype, -gnatwc), which main's Task 057
    // duration fixture emits identically. Any warning from a Task 058 carrier
    // body fails this test.
    let strict = Command::new("gnatmake")
        .current_dir(&dir)
        .args(["-q", "-f", "-c", "-gnatwa", "test-bounded.adb"])
        .output()
        .unwrap();
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&strict.stdout),
        String::from_utf8_lossy(&strict.stderr)
    );
    assert!(strict.status.success(), "{diagnostics}");
    let unexpected: Vec<&str> = diagnostics
        .lines()
        .filter(|line| line.contains("warning"))
        .filter(|line| !line.contains("can never be greater than, could replace by \"=\""))
        .collect();
    assert!(unexpected.is_empty(), "{unexpected:#?}");
    for (flags, policy) in [
        (vec!["-q", "-f", "probe.adb"], ""),
        (
            vec!["-q", "-f", "-gnata", "probe.adb"],
            "pragma Assertion_Policy (Ignore);\n",
        ),
    ] {
        std::fs::write(dir.join("probe.adb"), format!("{policy}{probe}")).unwrap();
        let built = Command::new("gnatmake")
            .current_dir(&dir)
            .args(&flags)
            .output()
            .unwrap();
        assert!(
            built.status.success(),
            "{}{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
        let run = Command::new(dir.join("probe")).output().unwrap();
        let stdout = String::from_utf8_lossy(&run.stdout);
        assert!(
            run.status.success() && stdout.trim_end().ends_with("ok"),
            "policy {policy:?}: {stdout}{}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}

const PROBE_HEAD: &str = "with Ada.Strings.Unbounded;
with Ada.Text_IO; use Ada.Text_IO;
with Test.Bounded; use Test.Bounded;
procedure Probe is
   Failures : Natural := 0;
   procedure Fail (Label : String) is
   begin
      Put_Line (\"FAIL: \" & Label);
      Failures := Failures + 1;
   end Fail;
   procedure Check (Got, Want, Label : String) is
   begin
      if Got /= Want then
         Fail (Label);
      end if;
   end Check;
begin
";

/// Zero-length values in every storage form, spare capacity, inheritance,
/// Choice, and Task 040 default rejection of every kind of carrier.
const PROBE_TAIL: &str = "   declare
      Empty : constant Blank := Create (\"\");
      Present : constant Payload_OptionalMarker_Optional :=
        (Is_Present => True, Value => Empty);
      Absent : Payload_OptionalWord_Optional;
      Markers : Payload_Markers_Sequence := To_Sequence ((1 => Empty, 2 => Empty));
      Codes : Payload_Codes_Sequence;
      Choice : constant Pick := (Kind => ByEmpty_Kind, ByEmpty => Empty);
      Inherited : constant Concrete :=
        (Launch => Create (\"ABC\"), Serial => Create (\"\"\"'\\[]{}\"));
   begin
      Check (Value (Empty), \"\", \"empty\");
      Check (Value (Present.Value), \"\", \"optional empty\");
      if Absent.Is_Present then Fail (\"absent\"); end if;
      Append (Markers, Empty);
      if Length (Markers) /= 3 or else Value (Element (Markers, 3)) /= \"\" then
         Fail (\"bounded\");
      end if;
      begin
         Append (Markers, Empty);
         Fail (\"bounded overflow\");
      exception
         when Constraint_Error => null;
      end;
      Reserve_Capacity (Codes, 64);
      for I in 1 .. 40 loop
         Append (Codes, Create (\"aB3z\"));
      end loop;
      if Length (Codes) /= 40 then Fail (\"unbounded\"); end if;
      Check (Value (Choice.ByEmpty), \"\", \"choice\");
      Check (Value (Inherited.Serial), \"\"\"'\\[]{}\", \"serial\");
      declare
         Whole : constant Payload :=
           (Marker => Empty, Aircraft => Create (\"AB12 XY \"),
            OptionalMarker => Present, OptionalWord => Absent,
            Markers => Markers, Codes => Codes,
            Derived => Create (\"        \"),
            Notes => Ada.Strings.Unbounded.To_Unbounded_String (\" x \"));
      begin
         Check (Value (Whole.Aircraft), \"AB12 XY \", \"aircraft\");
         Check (Value (Whole.Derived), \"        \", \"derived\");
      end;
   end;
   --  Task 040: default construction raises even for the zero-length
   --  profile, independently of assertion policy.
   begin
      declare
         Bad : Blank;
      begin
         Fail (\"default Blank \" & Value (Bad));
      end;
   exception
      when Program_Error => null;
   end;
   begin
      declare
         Bad : Tail8;
      begin
         Fail (\"default Tail8 \" & Value (Bad));
      end;
   exception
      when Program_Error => null;
   end;
   begin
      declare
         Bad : Payload;
      begin
         Fail (\"default Payload \" & Value (Bad.Marker));
      end;
   exception
      when Program_Error => null;
   end;
   if Failures = 0 then
      Put_Line (\"ok\");
   else
      Put_Line (\"failures\");
   end if;
end Probe;
";
