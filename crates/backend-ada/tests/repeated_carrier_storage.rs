//! Task 040 corrective: repeated storage of validated lexical carriers,
//! compiled and executed under GNAT.
//!
//! # The defect this file exists to prevent
//!
//! Task 040 made an unchecked default carrier fail explicitly, which is
//! correct. But repeated storage default-initializes **physical capacity**
//! before any live element is assigned, and the original change did not
//! account for that. Two storage paths regressed.
//!
//! **Unbounded.** The helper instantiated `Ada.Containers.Vectors` -- the
//! *definite* vector -- over the generated element type. GNAT's definite
//! vector allocates a default-initialized replacement array when it grows,
//! before copying the existing elements and `New_Item`. Appending two
//! perfectly valid carriers therefore raised:
//!
//! ```text
//! empty vector created
//! append 1 ok, length = 1
//! raised PROGRAM_ERROR : finalize/adjust raised exception
//! ```
//!
//! **Bounded.** The representation was a logical `Length` initialized to the
//! schema minimum plus an `Items` array sized to the schema maximum. For a
//! `0..N` field, `Length = 0` did not prevent default initialization of all
//! `N` physical carrier slots, so a valid *empty* sequence raised.
//!
//! # The required invariant
//!
//! *Unused capacity is not a live validated value.*
//!
//! Unbounded storage is now an `Indefinite_Vectors` instantiation behind an
//! opaque sequence type: each live element is constructed from the supplied
//! value and spare capacity is an access value, not a carrier. Bounded storage
//! keeps its bounded array and its `min .. max` logical length, but each
//! physical slot is a discriminated record whose unused variant has no payload
//! component at all.
//!
//! Nothing is fabricated to fill unused capacity, and no client is ever asked
//! to supply a value merely to populate it.
//!
//! # Scope
//!
//! These probes are built **without** `-gnata` and again under
//! `Assertion_Policy (Ignore)`, for the same reason the scalar lifecycle
//! probes are: the guarantee must not be a client assertion setting.

use ams_gra_oms_backend_ada::{generate, generate_body};
use ams_gra_oms_codegen_core::GenerationWorld;
use std::path::{Path, PathBuf};
use std::process::Command;

const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;

/// The repository fixture whose repeated fields reach validated carriers.
const FIXTURE: &str = "backend-ada-repeated-carrier.xsd";

fn fixture(name: &str) -> ams_gra_oms_ir::SchemaIr {
    ams_gra_oms_xsd_frontend::load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../xsd-frontend/tests/fixtures/{name}")),
    )
    .unwrap_or_else(|error| panic!("{name} fixture should parse: {error:?}"))
}

/// Whether GNAT is available, honoring the CI hard gate.
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

/// Stage the generated child package plus its parent into a probe directory.
fn stage(label: &str) -> PathBuf {
    let schema = fixture(FIXTURE);
    let spec = generate(&schema, CLOSED).expect("fixture must generate a spec");
    let body = generate_body(&schema, CLOSED)
        .expect("body generation must not fail")
        .expect("a repeated-carrier schema must emit a package body");

    let directory = std::env::temp_dir().join(format!("ams-gra-oms-task040-repeated-{label}"));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Ada probe directory");
    std::fs::write(directory.join("test.ads"), "package Test is\nend Test;\n")
        .expect("write Ada parent package");
    std::fs::write(directory.join("test-carriers.ads"), &spec).expect("write spec");
    std::fs::write(directory.join("test-carriers.adb"), &body).expect("write body");
    directory
}

/// Build and run one probe, returning (compiled, ran successfully, output).
fn build_and_run(directory: &Path, probe: &str, extra: &[&str]) -> (bool, bool, String) {
    std::fs::write(directory.join("probe.adb"), probe).expect("write Ada probe");
    let mut args: Vec<&str> = vec!["-q", "-f"];
    args.extend_from_slice(extra);
    args.push("probe.adb");
    let built = Command::new("gnatmake")
        .current_dir(directory)
        .args(&args)
        .output()
        .expect("gnatmake must run");
    if !built.status.success() {
        return (
            false,
            false,
            String::from_utf8_lossy(&built.stderr).into_owned(),
        );
    }
    let run = Command::new(directory.join("probe"))
        .output()
        .expect("compiled probe must run");
    let mut text = String::from_utf8_lossy(&run.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&run.stderr));
    (true, run.status.success(), text)
}

/// Write a configuration pragma file disabling assertions, and return its path.
fn write_ignore_policy(directory: &Path) -> String {
    let path = directory.join("no_assertions.adc");
    std::fs::write(&path, "pragma Assertion_Policy (Ignore);\n")
        .expect("write configuration pragma file");
    path.to_string_lossy().into_owned()
}

/// The emitted storage representation, asserted as text before anything runs.
#[test]
fn repeated_storage_never_declares_a_live_carrier_for_spare_capacity() {
    let spec = generate(&fixture(FIXTURE), CLOSED).expect("repeated-carrier fixture must generate");

    // Unbounded: the INDEFINITE vector, which constructs each live element
    // from the supplied value rather than default-initializing capacity.
    assert!(
        spec.contains("with Ada.Containers.Indefinite_Vectors;"),
        "{spec}"
    );
    assert!(
        !spec.contains("new Standard.Ada.Containers.Vectors\n"),
        "the definite vector must not store validated carriers:\n{spec}"
    );
    // The storage type is opaque, so no client can reach a raw vector whose
    // capacity semantics are not part of the generated contract.
    assert!(
        spec.contains("type Payload_Any_Sequence is private;"),
        "{spec}"
    );

    // Bounded: a discriminated physical slot whose unused variant has no
    // payload component at all. The bounded array and the schema's logical
    // `min .. max` length are both preserved.
    assert!(
        spec.contains("type EmptyBounded_Slots_Slot (Is_Used : Boolean := False) is record"),
        "{spec}"
    );
    assert!(spec.contains("when False => null;"), "{spec}");
    assert!(
        spec.contains("array (Positive range 1 .. 3) of EmptyBounded_Slots_Slot;"),
        "{spec}"
    );
    assert!(
        spec.contains("Length : Natural range 0 .. 3 := 0;"),
        "cardinality semantics must be preserved:\n{spec}"
    );

    // The positive-minimum shape is preserved rather than collapsed into an
    // unconstrained vector: a required prefix plus an additional portion.
    assert!(
        spec.contains("array (Positive range 1 .. 2) of Payload_Required_Item;"),
        "{spec}"
    );
    assert!(
        spec.contains("Additional : Payload_Required_Additional;"),
        "{spec}"
    );

    // No fabricated placeholder of any kind is introduced to fill capacity.
    for forbidden in ["00000000-0000", "1970-01-01", "000.0.0"] {
        assert!(
            !spec.contains(forbidden),
            "no value may be invented to fill unused capacity: {forbidden}"
        );
    }
}

/// The runtime reproduction, and the corrected behavior, under GNAT.
///
/// Every live value is inspected for its exact expected text after the
/// operation, and the DateTime carrier is compared against its normalized
/// representation rather than the supplied lexical form.
#[test]
fn repeated_carrier_storage_is_usable_under_gnat() {
    if !gnat_available() {
        return;
    }

    let directory = stage("usable");

    for policy in [
        vec![],
        vec!["-gnatec=".to_owned() + &write_ignore_policy(&directory)],
    ] {
        let extra: Vec<&str> = policy.iter().map(String::as_str).collect();
        let (compiled, ran, output) = build_and_run(&directory, REPEATED_PROBE, &extra);
        assert!(
            compiled,
            "the repeated-storage probe must compile:\n{output}"
        );
        assert!(ran, "the repeated-storage probe must not abort:\n{output}");
        assert!(
            output.contains("ok") && !output.contains("failures"),
            "every repeated-storage path must work:\n{output}"
        );
    }

    std::fs::remove_dir_all(&directory).expect("remove Ada probe directory");
}

/// The executable probe.
///
/// This is the permanent expression of the required *usable API behavior*. It
/// deliberately does not assert any particular allocation strategy: another
/// compiler could grow a container differently, but every implementation must
/// let a client build and read back these sequences without supplying a value
/// merely to populate unused capacity.
const REPEATED_PROBE: &str = r##"with Test.Carriers;
with Ada.Text_IO;

procedure Probe is
   use Ada.Text_IO;
   Failures : Natural := 0;

   procedure Check (Got, Want, Label : String) is
   begin
      if Got /= Want then
    Put_Line ("wrong " & Label & ": [" & Got & "]");
    Failures := Failures + 1;
      end if;
   end Check;

   procedure Expect (Got, Want : Natural; Label : String) is
   begin
      if Got /= Want then
    Put_Line ("wrong length for " & Label & ":" & Got'Image);
    Failures := Failures + 1;
      end if;
   end Expect;
begin
   --  A valid EMPTY zero-minimum bounded sequence, in a record with NO other
   --  required field, so a failure here is attributable to unused repeated
   --  storage and to nothing else.
   declare
      Empty : Test.Carriers.EmptyBounded;
   begin
      Expect (Empty.Slots.Length, 0, "empty bounded");
   end;

   --  Unbounded 0..N: empty, reserve, growth through many capacity
   --  boundaries, copy, assignment, clear, and reuse.
   declare
      V : Test.Carriers.Payload_Any_Sequence;
   begin
      Expect (Test.Carriers.Length (V), 0, "empty unbounded");
      Test.Carriers.Reserve_Capacity (V, 4);
      Expect (Test.Carriers.Length (V), 0, "reserve on empty");

      for I in 1 .. 64 loop
    Test.Carriers.Append (V, Test.Carriers.Create ("N" & I'Image));
      end loop;
      Expect (Test.Carriers.Length (V), 64, "after growth");
      Check (Test.Carriers.Value (Test.Carriers.Element (V, 1)), "N 1", "first");
      Check (Test.Carriers.Value (Test.Carriers.Element (V, 64)), "N 64", "last");

      Test.Carriers.Reserve_Capacity (V, 512);
      Check
   (Test.Carriers.Value (Test.Carriers.Element (V, 64)), "N 64",
    "after reserve on populated");

      declare
    Copied   : constant Test.Carriers.Payload_Any_Sequence := V;
    Assigned : Test.Carriers.Payload_Any_Sequence;
      begin
    Assigned := Copied;
    Expect (Test.Carriers.Length (Assigned), 64, "assigned");
    Check
      (Test.Carriers.Value (Test.Carriers.Element (Copied, 7)), "N 7",
       "copy construction");
    Check
      (Test.Carriers.Value (Test.Carriers.Element (Assigned, 7)), "N 7",
       "assignment");

    Test.Carriers.Clear (Assigned);
    Expect (Test.Carriers.Length (Assigned), 0, "cleared");
    Test.Carriers.Append (Assigned, Test.Carriers.Create ("REUSE"));
    Check
      (Test.Carriers.Value (Test.Carriers.Element (Assigned, 1)), "REUSE",
       "reuse after clear");
      end;
   end;

   --  The named Zulu DateTime carrier, compared against its NORMALIZED
   --  representation.
   declare
      S : Test.Carriers.Payload_Stamps_Sequence;
   begin
      Test.Carriers.Append (S, Test.Carriers.Create ("2026-09-24T12:00:00Z"));
      Test.Carriers.Append (S, Test.Carriers.Create ("2026-09-24T13:30:00Z"));
      Check
   (Test.Carriers.Value (Test.Carriers.Element (S, 1)),
    "2026-09-24T12:00:00Z", "datetime 1");
      Check
   (Test.Carriers.Value (Test.Carriers.Element (S, 2)),
    "2026-09-24T13:30:00Z", "datetime 2");
   end;

   --  A COMPOSED element: a record whose required field is a validated
   --  carrier. Storage must be chosen from the element's structure, not from
   --  any declaration name.
   declare
      G : Test.Carriers.Payload_Tags_Sequence;
   begin
      for I in 1 .. 20 loop
    Test.Carriers.Append
      (G, (Label => Test.Carriers.Create ("T" & I'Image)));
      end loop;
      Check
   (Test.Carriers.Value (Test.Carriers.Element (G, 20).Label), "T 20",
    "composed element");
   end;

   --  Positive-minimum unbounded: the required portion is supplied
   --  explicitly, and the additional portion starts EMPTY without the client
   --  inventing anything.
   declare
      R : Test.Carriers.Payload_Required_Sequence :=
   (Required   =>
      (Test.Carriers.Create ("R-1"), Test.Carriers.Create ("R-2")),
    Additional => <>);
   begin
      Expect (Test.Carriers.Length (R.Additional), 0, "additional starts empty");
      Check (Test.Carriers.Value (R.Required (1)), "R-1", "required 1");
      Check (Test.Carriers.Value (R.Required (2)), "R-2", "required 2");
      Test.Carriers.Append (R.Additional, Test.Carriers.Create ("R-3"));
      Check
   (Test.Carriers.Value (Test.Carriers.Element (R.Additional, 1)), "R-3",
    "additional element");
   end;

   --  Bounded 0..3: empty, partial, full, and the cardinality limit.
   declare
      B : Test.Carriers.Payload_Bounded_Sequence;
   begin
      Expect (B.Length, 0, "bounded empty");

      B.Items (1) := (Is_Used => True, Value => Test.Carriers.Create ("B-1"));
      B.Length := 1;
      Check (Test.Carriers.Value (B.Items (1).Value), "B-1", "bounded partial");

      B.Items (2) := (Is_Used => True, Value => Test.Carriers.Create ("B-2"));
      B.Items (3) := (Is_Used => True, Value => Test.Carriers.Create ("B-3"));
      B.Length := 3;
      Check (Test.Carriers.Value (B.Items (3).Value), "B-3", "bounded full");

      --  The schema's maxOccurs is still enforced by the representation.
      declare
    Over : Natural range 0 .. 3 := 0;
      begin
    Over := B.Length + 1;
    Put_Line ("cardinality limit not enforced:" & Over'Image);
    Failures := Failures + 1;
      exception
    when Constraint_Error => null;
      end;
   end;

   --  Repeated CHOICE alternatives, both shapes.
   declare
      S : Test.Carriers.Selection :=
   (Kind     => Test.Carriers.Repeated_Kind,
    Repeated =>
      (Required   => (1 => Test.Carriers.Create ("C-1")),
       Additional => <>));
   begin
      Test.Carriers.Append
   (S.Repeated.Additional, Test.Carriers.Create ("C-2"));
      Check
   (Test.Carriers.Value (S.Repeated.Required (1)), "C-1",
    "choice required");
      Check
   (Test.Carriers.Value
      (Test.Carriers.Element (S.Repeated.Additional, 1)),
    "C-2", "choice additional");
   end;

   declare
      M : Test.Carriers.Selection (Kind => Test.Carriers.Marked_Kind);
   begin
      Expect (M.Marked.Length, 0, "choice bounded empty");
      M.Marked.Items (1) :=
   (Is_Used => True,
    Value   => (Label => Test.Carriers.Create ("M-1")));
      M.Marked.Length := 1;
      Check
   (Test.Carriers.Value (M.Marked.Items (1).Value.Label), "M-1",
    "choice bounded partial");
   end;

   if Failures /= 0 then
      Put_Line ("failures");
   else
      Put_Line ("ok");
   end if;
end Probe;
"##;
