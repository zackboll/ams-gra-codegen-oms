//! Task 053: generated Ada constrained named Binary carriers, compiled and
//! EXECUTED under GNAT.
//!
//! * `Create` / `Value` at both boundaries; `Constraint_Error` one octet
//!   either side -- an explicit check, never an assertion;
//! * Task 040 lifecycle: an ordinary default declaration raises
//!   `Program_Error`, INCLUDING `ZeroBlob` whose domain admits the empty
//!   sequence ("empty is a valid value" is not "unchecked default
//!   construction is allowed");
//! * repeated storage reuses the Task 040 machinery: 0..N and 1..N bounded
//!   slot sequences, 0.. and 1.. indefinite unbounded sequences -- append,
//!   element access, clear, spare capacity never creating a live carrier,
//!   and an invalid length rejected by `Create` before insertion;
//! * the Task 034 optional wrapper: absent constructible, present only from
//!   an already valid carrier;
//! * every probe is built WITHOUT `-gnata`, and again under
//!   `pragma Assertion_Policy (Ignore)`.

use ams_gra_oms_backend_ada::{generate, generate_body};
use ams_gra_oms_codegen_core::GenerationWorld;
use std::path::{Path, PathBuf};
use std::process::Command;

const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;

fn schema() -> ams_gra_oms_ir::SchemaIr {
    ams_gra_oms_xsd_frontend::load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/service-generate/constrained-binary.xsd"),
    )
    .expect("constrained-binary fixture parses")
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

fn stage(label: &str) -> PathBuf {
    let schema = schema();
    let spec = generate(&schema, CLOSED).expect("spec generates");
    let body = generate_body(&schema, CLOSED)
        .expect("body generation")
        .expect("repeated storage needs a body");
    let directory = std::env::temp_dir().join(format!(
        "ams-gra-oms-task053-ada-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create probe directory");
    // The OAM namespace URI selects package `Programs.Oam`.
    std::fs::write(
        directory.join("programs.ads"),
        "package Programs is\nend Programs;\n",
    )
    .expect("parent");
    std::fs::write(directory.join("programs-oam.ads"), spec).expect("spec");
    std::fs::write(directory.join("programs-oam.adb"), body).expect("body");
    std::fs::write(
        directory.join("no_assertions.adc"),
        "pragma Assertion_Policy (Ignore);\n",
    )
    .expect("policy");
    directory
}

/// Build (never with `-gnata`) and run; returns (built, succeeded, output).
fn build_and_run(directory: &Path, probe: &str, extra: &[&str]) -> (bool, bool, String) {
    std::fs::write(directory.join("probe.adb"), probe).expect("write probe");
    let built = Command::new("gnatmake")
        .current_dir(directory)
        .args(["-q", "-f"])
        .args(extra)
        .arg("probe.adb")
        .output()
        .expect("gnatmake runs");
    if !built.status.success() {
        return (
            false,
            false,
            String::from_utf8_lossy(&built.stderr).into_owned(),
        );
    }
    let run = Command::new(directory.join("probe"))
        .output()
        .expect("probe runs");
    let mut text = String::from_utf8_lossy(&run.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&run.stderr));
    (true, run.status.success(), text)
}

const POLICIES: [&[&str]; 2] = [&[], &["-gnatec=no_assertions.adc"]];

#[test]
fn task053_ada_carrier_shape() {
    let spec = generate(&schema(), CLOSED).expect("spec");
    for carrier in [
        "Exact4",
        "Min2",
        "Max6",
        "Between2And6",
        "ZeroBlob",
        "BlobExact",
    ] {
        for required in [
            format!("   type {carrier} is private;"),
            format!("   function Create (Value : Binary_Vectors.Vector) return {carrier};"),
            format!("   function Value (Item : {carrier}) return Binary_Vectors.Vector;"),
            format!("with \"{carrier} requires initialization from Create\";"),
        ] {
            assert!(spec.contains(&required), "{carrier}: {required}\n{spec}");
        }
    }
    // The storage is a private component, not a visible record field.
    let private_at = spec.find("\nprivate\n").expect("private part");
    assert!(
        spec.find("      Data : Binary_Vectors.Vector :=")
            .expect("component")
            > private_at
    );
    // The check is explicit, never an assertion or predicate.
    assert!(spec.contains("else raise Standard.Constraint_Error"));
    // The unconstrained control keeps the Task 025 record.
    assert!(spec.contains(
        "   type PlainBytes is record\n      Value : Binary_Vectors.Vector;\n   end record;\n"
    ));
}

const RUNTIME_PROBE: &str = r##"with Ada.Text_IO;
with Interfaces;
with Programs.Oam;
procedure Probe is
   use Ada.Text_IO;
   package B renames Programs.Oam;
   use type B.Binary_Vectors.Vector;
   Failures : Natural := 0;

   procedure Fail (Label : String) is
   begin
      Put_Line ("FAIL: " & Label);
      Failures := Failures + 1;
   end Fail;

   function Octets (Count : Natural) return B.Binary_Vectors.Vector is
      Result : B.Binary_Vectors.Vector;
   begin
      for Index in 1 .. Count loop
         Result.Append (Interfaces.Unsigned_8 (Index mod 256));
      end loop;
      return Result;
   end Octets;

   generic
      type Carrier is private;
      with function Create (Value : B.Binary_Vectors.Vector) return Carrier;
      with function Value (Item : Carrier) return B.Binary_Vectors.Vector;
   procedure Bounds (Label : String; Min : Natural; Max : Integer);
   --  Max < 0 means unbounded above.

   procedure Bounds (Label : String; Min : Natural; Max : Integer) is
      procedure Rejects (Count : Natural) is
      begin
         declare
            Item : constant Carrier := Create (Octets (Count));
         begin
            if Value (Item) = Octets (Count) then
               Fail (Label & " accepted" & Count'Image);
            end if;
         end;
      exception
         when Constraint_Error => null;
      end Rejects;

      procedure Accepts (Count : Natural) is
      begin
         if Value (Create (Octets (Count))) /= Octets (Count) then
            Fail (Label & " stored" & Count'Image);
         end if;
      exception
         when Constraint_Error => Fail (Label & " rejected" & Count'Image);
      end Accepts;
   begin
      if Min > 0 then
         Rejects (Min - 1);
      end if;
      Accepts (Min);
      if Max >= 0 then
         Accepts (Max);
         Rejects (Max + 1);
      else
         Accepts (1000);
      end if;
   end Bounds;

   procedure Exact4_Bounds is new Bounds (B.Exact4, B.Create, B.Value);
   procedure Min2_Bounds is new Bounds (B.Min2, B.Create, B.Value);
   procedure Max6_Bounds is new Bounds (B.Max6, B.Create, B.Value);
   procedure Between_Bounds is new Bounds (B.Between2And6, B.Create, B.Value);
   procedure Derived_Bounds is new Bounds (B.DerivedExact4, B.Create, B.Value);
   procedure Zero_Bounds is new Bounds (B.ZeroBlob, B.Create, B.Value);
   procedure Base_Bounds is new Bounds (B.BlobBase, B.Create, B.Value);
   procedure Middle_Bounds is new Bounds (B.BlobMiddle, B.Create, B.Value);
   procedure Exact_Bounds is new Bounds (B.BlobExact, B.Create, B.Value);

   --  A default declaration must fail. It is declared inside a nested
   --  procedure because an exception raised while elaborating a declarative
   --  part propagates past that block's own handler (Task 040 note).
   generic
      type Carrier is private;
   procedure Default_Fails (Label : String);

   procedure Default_Fails (Label : String) is
      procedure Declare_One is
         Item : Carrier;
         pragma Unreferenced (Item);
      begin
         null;
      end Declare_One;
   begin
      Declare_One;
      Fail (Label & " default declaration succeeded");
   exception
      when Program_Error => null;
   end Default_Fails;

   procedure Exact4_Default is new Default_Fails (B.Exact4);
   procedure Zero_Default is new Default_Fails (B.ZeroBlob);
   procedure Max6_Default is new Default_Fails (B.Max6);

   Four : constant B.Exact4 := B.Create (Octets (4));
begin
   Exact4_Bounds ("Exact4", 4, 4);
   Min2_Bounds ("Min2", 2, -1);
   Max6_Bounds ("Max6", 0, 6);
   Between_Bounds ("Between2And6", 2, 6);
   Derived_Bounds ("DerivedExact4", 4, 4);
   Zero_Bounds ("ZeroBlob", 0, 0);
   Base_Bounds ("BlobBase", 4, -1);
   Middle_Bounds ("BlobMiddle", 4, 16);
   Exact_Bounds ("BlobExact", 8, 8);

   --  Exact length zero: the empty VALUE is valid through Create...
   if not B.Binary_Vectors.Is_Empty
     (B.Value (B.ZeroBlob'(B.Create (B.Binary_Vectors.Empty_Vector))))
   then
      Fail ("ZeroBlob Create (empty)");
   end if;
   --  ...but default declaration still fails by API policy.
   Exact4_Default ("Exact4");
   Zero_Default ("ZeroBlob");
   Max6_Default ("Max6");

   --  Copy and assignment preserve a valid carrier.
   declare
      Copy   : constant B.Exact4 := Four;
      Target : B.Exact4 := B.Create (Octets (4));
   begin
      Target := Copy;
      if B.Value (Target) /= Octets (4) or else B.Value (Four) /= Octets (4) then
         Fail ("copy/assignment");
      end if;
   end;

   --  Task 034 optional: absent needs no carrier; present only from a valid one.
   declare
      Absent  : B.ConstrainedBlobPayload_MaybeMin_Optional;
      Present : constant B.ConstrainedBlobPayload_MaybeMin_Optional :=
        (Is_Present => True, Value => B.Create (Octets (2)));
   begin
      if Absent.Is_Present or else B.Value (Present.Value) /= Octets (2) then
         Fail ("optional");
      end if;
   end;

   --  Bounded 0..3 (Max6): default is the empty sequence over unused slots.
   declare
      S : B.ConstrainedBlobPayload_Bounded_Sequence;
   begin
      if B.Length (S) /= 0 then
         Fail ("bounded empty");
      end if;
      B.Append (S, B.Create (Octets (6)));
      B.Append (S, B.Create (B.Binary_Vectors.Empty_Vector));
      if B.Length (S) /= 2 or else B.Value (B.Element (S, 1)) /= Octets (6) then
         Fail ("bounded append/element");
      end if;
      --  An invalid length is rejected by Create BEFORE insertion.
      begin
         B.Append (S, B.Create (Octets (7)));
         Fail ("bounded accepted 7 octets");
      exception
         when Constraint_Error => null;
      end;
      if B.Length (S) /= 2 then
         Fail ("bounded length after rejected append");
      end if;
      B.Clear (S);
      if B.Length (S) /= 0 then
         Fail ("bounded clear");
      end if;
   end;

   --  Bounded 1..3 (Between2And6): occupancy only from supplied values.
   declare
      S : B.ConstrainedBlobPayload_BoundedRequired_Sequence :=
        B.To_Sequence ((1 => B.Create (Octets (2))));
   begin
      B.Append (S, B.Create (Octets (6)));
      if B.Length (S) /= 2 or else B.Value (B.Element (S, 2)) /= Octets (6) then
         Fail ("bounded required");
      end if;
   end;

   --  Unbounded 0.. (DerivedExact4): indefinite storage, many growths.
   declare
      S : B.ConstrainedBlobPayload_Stream_Sequence;
   begin
      B.Reserve_Capacity (S, 3);
      for Index in 1 .. 100 loop
         B.Append (S, B.Create (Octets (4)));
      end loop;
      if B.Length (S) /= 100 or else B.Value (B.Element (S, 100)) /= Octets (4) then
         Fail ("unbounded append/element");
      end if;
      begin
         B.Append (S, B.Create (Octets (3)));
         Fail ("unbounded accepted 3 octets");
      exception
         when Constraint_Error => null;
      end;
      B.Clear (S);
      if B.Length (S) /= 0 then
         Fail ("unbounded clear");
      end if;
   end;

   --  Unbounded 1.. (Exact4): required prefix plus additional storage.
   declare
      S : B.ConstrainedBlobPayload_StreamRequired_Sequence :=
        (Required => (1 => Four), Additional => <>);
   begin
      B.Append (S.Additional, B.Create (Octets (4)));
      if B.Length (S.Additional) /= 1 then
         Fail ("unbounded required");
      end if;
   end;

   --  Choice alternative.
   declare
      C : constant B.BlobChoice :=
        (Kind => B.Fixed_Kind, Fixed => B.Create (Octets (4)));
   begin
      if B.Value (C.Fixed) /= Octets (4) then
         Fail ("choice");
      end if;
   end;

   if Failures = 0 then
      Put_Line ("ADA CONSTRAINED BINARY RUNTIME: PASSED");
   else
      Put_Line ("failures:" & Failures'Image);
   end if;
end Probe;
"##;

#[test]
fn task053_ada_constrained_binary_runtime_lifecycle_and_storage() {
    if !gnat_available() {
        return;
    }
    let directory = stage("runtime");
    for extra in POLICIES {
        let (built, ran, output) = build_and_run(&directory, RUNTIME_PROBE, extra);
        assert!(built, "probe must compile ({extra:?}):\n{output}");
        assert!(ran, "probe must not abort ({extra:?}):\n{output}");
        assert!(
            output.contains("ADA CONSTRAINED BINARY RUNTIME: PASSED"),
            "({extra:?}):\n{output}"
        );
        println!("{extra:?}: {output}");
    }
    std::fs::remove_dir_all(&directory).expect("remove probe directory");
}
