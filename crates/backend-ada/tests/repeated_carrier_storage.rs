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

    // Bounded: opaque storage. The physical slot is still discriminated, with
    // no payload component in its unused variant, and the backing array is
    // still finite and tied to maxOccurs -- but both now live in the private
    // completion, because a publicly writable count beside public slots could
    // not preserve actual occupancy.
    assert!(
        spec.contains("type EmptyBounded_Slots_Sequence is private;"),
        "bounded storage must be opaque:\n{spec}"
    );
    assert!(
        spec.contains("type EmptyBounded_Slots_Slot (Is_Used : Boolean := False) is record"),
        "{spec}"
    );
    assert!(spec.contains("when False => null;"), "{spec}");
    assert!(
        spec.contains("array (EmptyBounded_Slots_Index) of EmptyBounded_Slots_Slot;"),
        "{spec}"
    );
    assert!(
        spec.contains("subtype EmptyBounded_Slots_Index is Positive range 1 .. 3;"),
        "maxOccurs must still bound the backing storage:\n{spec}"
    );
    assert!(
        spec.contains("Count : Natural range 0 .. 3 := 0;"),
        "cardinality semantics must be preserved:\n{spec}"
    );
    // The slot array and the count appear only AFTER `private`, so no client
    // can reach either.
    let private_at = spec.find("\nprivate\n").expect("a private part must exist");
    assert!(
        spec.find("Count : Natural").expect("a count must exist") > private_at,
        "occupancy must not be publicly writable:\n{spec}"
    );
    assert!(
        spec.find("EmptyBounded_Slots_Array").expect("array") > private_at,
        "the slot array must not be publicly reachable:\n{spec}"
    );
    // A positive-minimum bounded sequence has no valid default at all.
    assert!(
        spec.contains("Count : Natural range 2 .. 3 :=\n        raise Standard.Program_Error"),
        "a positive-minimum bounded default must reject, not claim:\n{spec}"
    );
    // A fixed-count 2..2 field is representable and equally protected.
    assert!(
        spec.contains("Count : Natural range 2 .. 2 :=\n        raise Standard.Program_Error"),
        "a fixed-count field must reject a default too:\n{spec}"
    );
    // `Clear` exists for the zero-minimum shape and not for a positive one.
    assert!(
        spec.contains("procedure Clear (Container : in out EmptyBounded_Slots_Sequence);"),
        "{spec}"
    );
    assert!(
        !spec.contains("procedure Clear (Container : in out Payload_Span_Sequence);"),
        "Clear must not exist where the empty sequence is illegal:\n{spec}"
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
      Expect (Test.Carriers.Length (Empty.Slots), 0, "empty bounded");
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

   --  Bounded 0..3: empty, partial, full, and the cardinality limit enforced
   --  by the GENERATED SEQUENCE itself -- not by a substitute local subtype.
   --  The previous probe declared `Over : Natural range 0 .. 3` and observed
   --  its Constraint_Error, which proved nothing about the generated API.
   declare
      B : Test.Carriers.Payload_Bounded_Sequence;
   begin
      Expect (Test.Carriers.Length (B), 0, "bounded empty");

      Test.Carriers.Append (B, Test.Carriers.Create ("B-1"));
      Expect (Test.Carriers.Length (B), 1, "bounded partial");
      Check (Test.Carriers.Value (Test.Carriers.Element (B, 1)), "B-1",
        "bounded partial value");

      Test.Carriers.Append (B, Test.Carriers.Create ("B-2"));
      Test.Carriers.Append (B, Test.Carriers.Create ("B-3"));
      Expect (Test.Carriers.Length (B), 3, "bounded full");
      Check (Test.Carriers.Value (Test.Carriers.Element (B, 3)), "B-3",
        "bounded full value");

      --  Appending at capacity is rejected by the generated Append.
      begin
    Test.Carriers.Append (B, Test.Carriers.Create ("B-4"));
    Put_Line ("append past maxOccurs was accepted");
    Failures := Failures + 1;
      exception
    when Constraint_Error => null;
      end;

      --  Reading outside the logical length is rejected rather than handing
      --  back spare capacity.
      begin
    declare
       Seen : constant String :=
         Test.Carriers.Value (Test.Carriers.Element (B, 4));
    begin
       Put_Line ("out-of-range read returned: " & Seen);
       Failures := Failures + 1;
    end;
      exception
    when Constraint_Error => null;
      end;

      --  Copy and assignment preserve a coherent sequence.
      declare
    Copied   : constant Test.Carriers.Payload_Bounded_Sequence := B;
    Assigned : Test.Carriers.Payload_Bounded_Sequence;
      begin
    Assigned := Copied;
    Expect (Test.Carriers.Length (Assigned), 3, "bounded assigned");
    Check (Test.Carriers.Value (Test.Carriers.Element (Copied, 2)), "B-2",
      "bounded copy");
    Check (Test.Carriers.Value (Test.Carriers.Element (Assigned, 2)), "B-2",
      "bounded assignment");
    Test.Carriers.Clear (Assigned);
    Expect (Test.Carriers.Length (Assigned), 0, "bounded cleared");
    begin
       declare
          Seen : constant String :=
        Test.Carriers.Value (Test.Carriers.Element (Assigned, 1));
       begin
          Put_Line ("cleared sequence still yielded: " & Seen);
          Failures := Failures + 1;
       end;
    exception
       when Constraint_Error => null;
    end;
      end;
   end;

   --  Bounded 2..3 (POSITIVE minimum). Explicit construction from enough
   --  valid values succeeds; too few and too many are both rejected.
   declare
      S : constant Test.Carriers.Payload_Span_Sequence :=
    Test.Carriers.To_Sequence
      ((Test.Carriers.Create ("S-1"), Test.Carriers.Create ("S-2")));
   begin
      Expect (Test.Carriers.Length (S), 2, "span minimum");
      Check (Test.Carriers.Value (Test.Carriers.Element (S, 1)), "S-1",
        "span first");
      Check (Test.Carriers.Value (Test.Carriers.Element (S, 2)), "S-2",
        "span second");

      declare
    Grown : Test.Carriers.Payload_Span_Sequence := S;
      begin
    Test.Carriers.Append (Grown, Test.Carriers.Create ("S-3"));
    Expect (Test.Carriers.Length (Grown), 3, "span grown");
    Check (Test.Carriers.Value (Test.Carriers.Element (Grown, 3)), "S-3",
      "span appended");
    begin
       Test.Carriers.Append (Grown, Test.Carriers.Create ("S-4"));
       Put_Line ("span append past maxOccurs was accepted");
       Failures := Failures + 1;
    exception
       when Constraint_Error => null;
    end;
      end;

      --  Too few: the checked constructor raises rather than fabricating a
      --  second element or silently truncating.
      begin
    declare
       Short : constant Test.Carriers.Payload_Span_Sequence :=
         Test.Carriers.To_Sequence ((1 => Test.Carriers.Create ("S-1")));
    begin
       Put_Line
         ("too few values accepted:" & Test.Carriers.Length (Short)'Image);
       Failures := Failures + 1;
    end;
      exception
    when Constraint_Error => null;
      end;

      --  Too many: rejected rather than silently truncated.
      begin
    declare
       Long : constant Test.Carriers.Payload_Span_Sequence :=
         Test.Carriers.To_Sequence
           ((Test.Carriers.Create ("1"), Test.Carriers.Create ("2"),
             Test.Carriers.Create ("3"), Test.Carriers.Create ("4")));
    begin
       Put_Line
         ("too many values accepted:" & Test.Carriers.Length (Long)'Image);
       Failures := Failures + 1;
    end;
      exception
    when Constraint_Error => null;
      end;
   end;

   --  A default-declared positive-minimum sequence claims nothing: it fails
   --  explicitly instead of reporting two elements nobody supplied. This is
   --  an initialization expression, so it holds without -gnata.
   --  The handler is OUTSIDE the declaring block: an exception raised while
   --  elaborating a declarative part propagates past that block's own
   --  handler, so catching it here is what actually observes the rejection.
   begin
      declare
    Unset : Test.Carriers.Payload_Span_Sequence;
      begin
    Put_Line
      ("a positive-minimum default claimed"
       & Test.Carriers.Length (Unset)'Image);
    Failures := Failures + 1;
      end;
   exception
      when Program_Error => null;
   end;

   --  Fixed count 2..2: required occupancy cannot be replaced by unused
   --  slots, and there is no spare capacity at all.
   declare
      P : constant Test.Carriers.Payload_Pair_Sequence :=
    Test.Carriers.To_Sequence
      ((Test.Carriers.Create ("P-1"), Test.Carriers.Create ("P-2")));
   begin
      Expect (Test.Carriers.Length (P), 2, "fixed pair");
      Check (Test.Carriers.Value (Test.Carriers.Element (P, 2)), "P-2",
        "fixed pair second");
      declare
    Grown : Test.Carriers.Payload_Pair_Sequence := P;
      begin
    Test.Carriers.Append (Grown, Test.Carriers.Create ("P-3"));
    Put_Line ("a fixed-count sequence accepted a third element");
    Failures := Failures + 1;
      exception
    when Constraint_Error => null;
      end;
   end;

   begin
      declare
    Unset : Test.Carriers.Payload_Pair_Sequence;
      begin
    Put_Line
      ("a fixed-count default claimed" & Test.Carriers.Length (Unset)'Image);
    Failures := Failures + 1;
      end;
   exception
      when Program_Error => null;
   end;

   --  A positive-minimum bounded field of COMPOSED records, each of which
   --  transitively contains a required validated carrier.
   declare
      M : constant Test.Carriers.Payload_Marks_Sequence :=
    Test.Carriers.To_Sequence
      (((Label => Test.Carriers.Create ("M-1")),
        (Label => Test.Carriers.Create ("M-2"))));
   begin
      Expect (Test.Carriers.Length (M), 2, "composed bounded");
      Check
   (Test.Carriers.Value (Test.Carriers.Element (M, 2).Label), "M-2",
    "composed bounded value");
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
      Expect (Test.Carriers.Length (M.Marked), 0, "choice bounded empty");
      Test.Carriers.Append
   (M.Marked, (Label => Test.Carriers.Create ("M-1")));
      Check
   (Test.Carriers.Value (Test.Carriers.Element (M.Marked, 1).Label), "M-1",
    "choice bounded partial");
      --  The Choice emission path gets the identical protections.
      Test.Carriers.Append
   (M.Marked, (Label => Test.Carriers.Create ("M-2")));
      begin
    Test.Carriers.Append
      (M.Marked, (Label => Test.Carriers.Create ("M-3")));
    Put_Line ("a Choice bounded alternative exceeded maxOccurs");
    Failures := Failures + 1;
      exception
    when Constraint_Error => null;
      end;
   end;

   if Failures /= 0 then
      Put_Line ("failures");
   else
      Put_Line ("ok");
   end if;
end Probe;
"##;
