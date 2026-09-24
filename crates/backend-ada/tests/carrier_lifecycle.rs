//! Task 040: the generated Ada validated lexical carriers' initialization
//! lifecycle, compiled and executed under GNAT.
//!
//! # The defect this file exists to prevent
//!
//! Before Task 040 the private completion of each validated lexical carrier
//! was a plain record whose `Unbounded_String` component had the predefined
//! null-string default. An ordinary default declaration --
//!
//! ```ada
//! Item : Test.Callsign.Callsign;   --  no Create anywhere
//! ```
//!
//! -- therefore produced a usable, fully unchecked instance whose `Value`
//! returned the empty string. Every one of these profiles rejects the empty
//! string, so the carrier's own advertised invariant did not hold for a value
//! a client could obtain without writing anything unusual.
//!
//! # The selected policy
//!
//! The private component's default is now a `raise` expression. Enforcement is
//! part of Ada's initialization semantics, so it does **not** depend on the
//! client compiling with `-gnata`, on any `Assertion_Policy`, on a caller-side
//! precondition, or on a type invariant a client setting could disable. The
//! probes below are deliberately built **without** `-gnata` and additionally
//! with `Assertion_Policy (Ignore)` to demonstrate exactly that.
//!
//! `Create` builds its result with an explicit named aggregate after
//! validation succeeds, so legitimate construction never evaluates the
//! rejecting default.
//!
//! # Scope
//!
//! No new package-scope name is introduced: the change is a component default
//! inside an existing private record, so the shared generated-name model owes
//! nothing new.

use ams_gra_oms_backend_ada::{generate, generate_body};
use ams_gra_oms_codegen_core::GenerationWorld;
use std::path::{Path, PathBuf};
use std::process::Command;

const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;

fn fixture(name: &str) -> ams_gra_oms_ir::SchemaIr {
    ams_gra_oms_xsd_frontend::load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../xsd-frontend/tests/fixtures/{name}")),
    )
    .unwrap_or_else(|error| panic!("{name} fixture should parse: {error:?}"))
}

/// Whether GNAT is available, honoring the CI hard gate.
///
/// `AMS_GRA_REQUIRE_GNAT` turns the developer-machine skip into a hard failure,
/// so a runner without GNAT cannot produce a silently green result.
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

/// Write the generated child package plus its parent into a probe directory.
fn stage(label: &str, fixture_name: &str, child: &str) -> PathBuf {
    let schema = fixture(fixture_name);
    let spec = generate(&schema, CLOSED).expect("fixture must generate a spec");
    let body = generate_body(&schema, CLOSED)
        .expect("body generation must not fail")
        .expect("a validated-carrier schema must emit a package body");

    let directory = std::env::temp_dir().join(format!("ams-gra-oms-task040-ada-{label}"));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Ada probe directory");
    std::fs::write(directory.join("test.ads"), "package Test is\nend Test;\n")
        .expect("write Ada parent package");
    std::fs::write(directory.join(format!("test-{child}.ads")), &spec).expect("write spec");
    std::fs::write(directory.join(format!("test-{child}.adb")), &body).expect("write body");
    directory
}

/// Build and run one probe, returning (compiled, ran successfully, output).
///
/// `extra` carries the assertion-policy flags under test. The probes are
/// deliberately NOT built with `-gnata`.
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

/// The emitted private representation, asserted as text before anything runs.
#[test]
fn every_validated_carrier_defaults_to_an_explicit_failure() {
    let string_spec = generate(&fixture("backend-string-visible-ascii.xsd"), CLOSED)
        .expect("visible-ASCII fixture must generate");
    let temporal_spec = generate(&fixture("backend-temporal-datetime.xsd"), CLOSED)
        .expect("temporal fixture must generate");

    for (spec, component, carriers) in [
        (
            &string_spec,
            "Text",
            // Visible-ASCII at several bounds (CountryCode is the 2..4
            // profile), plus the schema-version and UUID families.
            vec![
                "Callsign",
                "ShortLabel",
                "CountryCode",
                "DerivedLabel",
                "SchemaVersion",
                "Uuid",
            ],
        ),
        (&temporal_spec, "Lexical", vec!["Instant", "Deadline"]),
    ] {
        let (visible, private_part) = spec
            .split_once("\nprivate\n")
            .expect("a validated-carrier schema must emit a private part");
        for carrier in carriers {
            // The public interface is unchanged: still opaque, still the same
            // Create/Value signatures.
            assert!(
                visible.contains(&format!("type {carrier} is private;")),
                "{carrier} must stay private"
            );
            assert!(
                visible.contains(&format!(
                    "function Create (Value : String) return {carrier};"
                )),
                "{carrier} must keep its Create signature"
            );
            assert!(
                visible.contains(&format!("function Value (Item : {carrier}) return String;")),
                "{carrier} must keep its Value signature"
            );
            // The rejecting default lives in the private completion.
            assert!(
                private_part.contains(&format!(
                    "{component} : Standard.Ada.Strings.Unbounded.Unbounded_String :=\n\
                 \x20       raise Standard.Program_Error\n\
                 \x20         with \"{carrier} requires initialization from Create\";"
                )),
                "{carrier} must default to an explicit failure:\n{private_part}"
            );
        }
    }

    // Enforcement must not be a client assertion effect, so no aspect a client
    // setting could disable is used for this.
    for forbidden in ["Type_Invariant", "with Pre =>"] {
        assert!(
            !string_spec.contains(forbidden),
            "carrier enforcement must not rely on {forbidden}"
        );
    }
}

/// The negative control: an ordinary default declaration must fail explicitly.
///
/// This is the exact reproduction of the Task 040 Ada defect. Before the
/// correction this probe printed the empty string as a usable `Value`; it must
/// now raise instead. The probe is built **without** `-gnata`, and is repeated
/// under `Assertion_Policy (Ignore)`, so a client cannot turn the enforcement
/// off.
#[test]
fn a_default_declared_carrier_cannot_silently_hold_invalid_text_under_gnat() {
    if !gnat_available() {
        return;
    }

    // Every affected family, including the DateTime carrier and the 2..4
    // visible-ASCII profile whose minimum is above one character.
    for (label, fixture_name, child, carriers) in [
        (
            "string-default",
            "backend-string-visible-ascii.xsd",
            "callsign",
            vec![
                "Callsign",
                "ShortLabel",
                "CountryCode",
                "DerivedLabel",
                "SchemaVersion",
                "Uuid",
            ],
        ),
        (
            "temporal-default",
            "backend-temporal-datetime.xsd",
            "temporal",
            vec!["Instant", "Deadline"],
        ),
    ] {
        let directory = stage(label, fixture_name, child);
        let unit = format!("Test.{}", child[..1].to_uppercase() + &child[1..]);

        for carrier in carriers {
            // An ordinary default declaration, then an attempt to observe it.
            // If the default ever produced a usable object, `Value` would
            // return the empty string and the probe would report it.
            let probe = format!(
                "with {unit};\nwith Ada.Text_IO;\n\n\
                 procedure Probe is\n\
                 begin\n\
                 \x20  declare\n\
                 \x20     Item : {unit}.{carrier};\n\
                 \x20  begin\n\
                 \x20     Ada.Text_IO.Put_Line\n\
                 \x20       (\"UNCHECKED: [\" & {unit}.Value (Item) & \"]\");\n\
                 \x20  end;\n\
                 exception\n\
                 \x20  when Program_Error =>\n\
                 \x20     Ada.Text_IO.Put_Line (\"rejected\");\n\
                 end Probe;\n"
            );

            // Two assertion policies, neither of which enables assertions.
            for policy in [
                vec![],
                vec!["-gnatec=".to_owned() + &write_ignore_policy(&directory)],
            ] {
                let extra: Vec<&str> = policy.iter().map(String::as_str).collect();
                let (compiled, ran, output) = build_and_run(&directory, &probe, &extra);
                assert!(compiled, "{carrier} default probe must compile:\n{output}");
                assert!(ran, "{carrier} default probe must not abort:\n{output}");
                assert!(
                    output.contains("rejected"),
                    "{carrier}: a default declaration must fail explicitly, got:\n{output}"
                );
                assert!(
                    !output.contains("UNCHECKED"),
                    "{carrier}: a default declaration must not yield a usable value:\n{output}"
                );
            }
        }
        std::fs::remove_dir_all(&directory).expect("remove Ada probe directory");
    }
}

/// Write a configuration pragma file that disables assertions, and return it.
///
/// Running the negative control under `Assertion_Policy (Ignore)` as well as
/// with no `-gnata` demonstrates the enforcement is a language initialization
/// effect rather than a client assertion setting.
fn write_ignore_policy(directory: &Path) -> String {
    let path = directory.join("no_assertions.adc");
    std::fs::write(&path, "pragma Assertion_Policy (Ignore);\n")
        .expect("write configuration pragma file");
    path.to_string_lossy().into_owned()
}

/// The positive controls: every legitimate construction path still works.
///
/// The rejecting default must not make valid code fail. `Create` builds its
/// result with an explicit named aggregate after validation succeeds, so the
/// default is never evaluated on a legitimate path. This probe is likewise
/// built without `-gnata`.
#[test]
fn legitimate_construction_paths_still_work_under_gnat() {
    if !gnat_available() {
        return;
    }

    let directory = stage(
        "string-positive",
        "backend-string-visible-ascii.xsd",
        "callsign",
    );
    let mut probe = String::from(
        "with Test.Callsign;\nwith Ada.Strings.Unbounded;\nwith Ada.Text_IO;\n\n\
     procedure Probe is\n\
     \x20  use Ada.Text_IO;\n\
     \x20  Failures : Natural := 0;\n\n\
     \x20  --  Explicit initialization from Create.\n\
     \x20  Made : constant Test.Callsign.Callsign :=\n\
     \x20    Test.Callsign.Create (\"ALPHA-1\");\n\n\
     \x20  --  Copy initialization from an already valid carrier.\n\
     \x20  Copied : constant Test.Callsign.Callsign := Made;\n\n\
     \x20  --  The 2..4 profile, whose minimum is above one character.\n\
     \x20  Country : constant Test.Callsign.CountryCode :=\n\
     \x20    Test.Callsign.Create (\"US\");\n\n\
     \x20  --  Assignment between valid carriers.\n\
     \x20  Assigned : Test.Callsign.Callsign :=\n\
     \x20    Test.Callsign.Create (\"BRAVO-2\");\n\n\
     \x20  --  An explicitly initialized ARRAY aggregate stays usable.\n\
     \x20  type Callsign_Array is array (1 .. 2) of Test.Callsign.Callsign;\n\
     \x20  Items : constant Callsign_Array :=\n\
     \x20    (1 => Test.Callsign.Create (\"ONE-1\"),\n\
     \x20     2 => Test.Callsign.Create (\"TWO-2\"));\n\n\
     \x20  procedure Check (Got, Want, Label : String) is\n\
     \x20  begin\n\
     \x20     if Got /= Want then\n\
     \x20        Put_Line (\"wrong text for \" & Label & \": [\" & Got & \"]\");\n\
     \x20        Failures := Failures + 1;\n\
     \x20     end if;\n\
     \x20  end Check;\n\
     begin\n\
     \x20  Assigned := Made;\n\n\
     \x20  Check (Test.Callsign.Value (Made), \"ALPHA-1\", \"Create\");\n\
     \x20  Check (Test.Callsign.Value (Copied), \"ALPHA-1\", \"copy init\");\n\
     \x20  Check (Test.Callsign.Value (Assigned), \"ALPHA-1\", \"assignment\");\n\
     \x20  Check (Test.Callsign.Value (Country), \"US\", \"2..4 profile\");\n\
     \x20  Check (Test.Callsign.Value (Items (1)), \"ONE-1\", \"array (1)\");\n\
     \x20  Check (Test.Callsign.Value (Items (2)), \"TWO-2\", \"array (2)\");\n",
    );
    probe.push_str(POSITIVE_PROBE_TAIL);

    let (compiled, ran, output) = build_and_run(&directory, &probe, &[]);
    assert!(compiled, "the positive controls must compile:\n{output}");
    assert!(ran, "the positive controls must not abort:\n{output}");
    assert!(
        output.contains("ok") && !output.contains("failures"),
        "every legitimate construction path must still work:\n{output}"
    );
    assert!(
        !output.contains("UNCHECKED"),
        "a default-created present payload must not hold invalid text:\n{output}"
    );

    /// The remainder of the positive-control probe: record aggregates and the two
    /// optional-wrapper cases.
    ///
    /// Kept as a separate constant purely so neither piece of generated Ada source
    /// becomes unreadably long.
    const POSITIVE_PROBE_TAIL: &str = r##"
   --  An explicitly initialized RECORD aggregate stays usable. This is the
   --  generated Payload, whose optional members use the existing Task 034
   --  wrappers rather than any carrier-specific optional path.
   declare
  Record_Value : constant Test.Callsign.Payload :=
    (Primary    => Test.Callsign.Create ("PRIMARY-1"),
     Alternate  =>
       (Is_Present => True,
        Value      => Test.Callsign.Create ("ALTERNATE-2")),
     Label      => (Is_Present => False),
     Country    => Test.Callsign.Create ("NATO"),
     Derived    => Test.Callsign.Create ("derived"),
     Version    => Test.Callsign.Create ("002.5.0"),
     Identifier =>
       Test.Callsign.Create ("123e4567-e89b-12d3-a456-426614174000"),
     Notes      =>
       Ada.Strings.Unbounded.To_Unbounded_String ("notes"));
   begin
  Check
    (Test.Callsign.Value (Record_Value.Primary), "PRIMARY-1",
     "record primary");
  Check
    (Test.Callsign.Value (Record_Value.Country), "NATO",
     "record country");

  --  A PRESENT optional wrapper initialized with Create succeeds.
  Check
    (Test.Callsign.Value (Record_Value.Alternate.Value), "ALTERNATE-2",
     "present optional");
   end;

   --  An ABSENT optional wrapper can still be DEFAULT-created: its absent
   --  variant has no payload component, so the rejecting default is never
   --  elaborated and nothing tries to construct an absent payload.
   declare
  Absent : Test.Callsign.Payload_Label_Optional;
   begin
  if Absent.Is_Present then
     Put_Line ("absent wrapper must default to absent");
     Failures := Failures + 1;
  end if;
   end;

   --  A DEFAULT-created PRESENT payload must NOT silently contain invalid
   --  text: constraining the discriminant to True forces the payload
   --  component to be elaborated, which must raise.
   declare
  procedure Try_Present is
     Present : Test.Callsign.Payload_Label_Optional (Is_Present => True);
  begin
     Put_Line
       ("UNCHECKED present payload: ["
        & Test.Callsign.Value (Present.Value) & "]");
     Failures := Failures + 1;
  end Try_Present;
   begin
  Try_Present;
  Put_Line ("UNCHECKED present payload did not raise");
  Failures := Failures + 1;
   exception
  when Program_Error => null;
   end;

   if Failures /= 0 then
  Put_Line ("failures");
   else
  Put_Line ("ok");
   end if;
end Probe;
"##;

    std::fs::remove_dir_all(&directory).expect("remove Ada probe directory");
}
