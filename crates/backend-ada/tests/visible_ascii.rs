//! Task 039: the generated Ada visible-ASCII carrier, compiled and run.
//!
//! Generated text alone would not prove the validator behaves as the
//! authoritative XSD requires, so every case in the shared corpus is executed
//! against a real GNAT-compiled program.

mod common;

use ams_gra_oms_backend_ada::{generate, generate_body};
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;

fn visible_ascii_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-visible-ascii.xsd"),
    )
    .expect("visible-ASCII fixture should parse")
}

/// Whether GNAT is available, honouring the strict CI switch.
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

/// The generated API shape, asserted before anything is compiled.
#[test]
fn generated_visible_ascii_api_is_an_opaque_validated_carrier() {
    let spec = generate(&visible_ascii_schema(), CLOSED).expect("fixture must generate");
    let (visible, private_part) = spec
        .split_once("\nprivate\n")
        .expect("a String-profile schema must emit a private part");

    // The visible part is opaque: only the private type and the two
    // operations. No representation is nameable from a client, so no aggregate
    // or conversion can bypass `Create`.
    assert!(visible.contains("type Callsign is private;"));
    assert!(visible.contains("function Create (Value : String) return Callsign;"));
    assert!(visible.contains("function Value (Item : Callsign) return String;"));
    assert!(!visible.contains("Text :"));

    // The representation lives in the private completion, reusing the
    // package's existing owned-string type.
    assert!(private_part.contains("type Callsign is record"));
    // Task 040 gave the component an explicitly failing default, so an
    // ordinary default declaration of the carrier cannot silently produce
    // empty text this profile would itself reject. The storage type is
    // otherwise unchanged.
    assert!(private_part.contains("Text : Standard.Ada.Strings.Unbounded.Unbounded_String :=\n"));
    assert!(private_part.contains("raise Standard.Program_Error"));

    // Predefined "=" IS the correct XML Schema semantics for xs:string, and the
    // generated comment says so. Still no ordering.
    assert!(visible.contains("IS XML Schema value equality"));
    for forbidden in ["function \"<\"", "function \"<=\"", "function Compare"] {
        assert!(!spec.contains(forbidden), "must not declare {forbidden}");
    }

    // No regex engine, and no locale-sensitive classification: the class is
    // tested as the ordinal interval ' ' .. '~'.
    for forbidden in [
        "GNAT.Regpat",
        "Regpat",
        "Regexp",
        "Ada.Characters.Handling",
        "Is_Graphic",
        "Trim",
    ] {
        assert!(!spec.contains(forbidden), "must not reference {forbidden}");
    }

    // Several supported declarations coexist, spanning ALL THREE String
    // profiles, which requires Ada Create/Value overload resolution to work.
    for carrier in [
        "Callsign",
        "ShortLabel",
        "CountryCode",
        "DerivedLabel",
        "SchemaVersion",
        "Uuid",
    ] {
        assert!(visible.contains(&format!("type {carrier} is private;")));
        assert!(visible.contains(&format!(
            "function Create (Value : String) return {carrier};"
        )));
        assert!(visible.contains(&format!("function Value (Item : {carrier}) return String;")));
    }

    // Task 034 composition: the optional named occurrence reuses the existing
    // per-field wrapper, with no optional-visible-string special case.
    assert!(spec.contains("type Payload_Alternate_Optional"));
    assert!(spec.contains("Value : Callsign;"));

    // Ordinary unconstrained String keeps its existing plain representation.
    assert!(spec.contains("Notes : Standard.Ada.Strings.Unbounded.Unbounded_String;"));

    // The profile is PARAMETERIZED, so each body must carry its OWN bounds.
    let body = generate_body(&visible_ascii_schema(), CLOSED)
        .expect("body generation must not fail")
        .expect("a String-profile schema must emit a package body");
    assert!(body.contains("Min_Length : constant := 1;"));
    assert!(body.contains("Max_Length : constant := 256;"));
    assert!(body.contains("Max_Length : constant := 32;"));
    assert!(body.contains("Min_Length : constant := 2;"));
    assert!(body.contains("Max_Length : constant := 4;"));
    // The class is the exact ordinal interval, not a locale-sensitive guess.
    assert!(body.contains("(Item in ' ' .. '~')"));
}

/// Every shared-corpus case, executed against the GNAT-compiled generated
/// package. `Create` must raise `Constraint_Error` on every invalid case.
#[test]
fn generated_visible_ascii_validator_matches_the_shared_corpus_under_gnat() {
    let schema = visible_ascii_schema();
    let spec = generate(&schema, CLOSED).expect("fixture must generate");
    let body = generate_body(&schema, CLOSED)
        .expect("body generation must not fail")
        .expect("a String-profile schema must emit a package body");
    let cases = common::load_corpus(&common::visible_ascii_corpus_path());

    if !gnat_available() {
        return;
    }

    let mut probe = String::from(
        "with Ada.Text_IO;\nwith Test.Callsign;\n\n\
         procedure Probe is\n\
         \x20  use Ada.Text_IO;\n\
         \x20  use type Test.Callsign.Callsign;\n\
         \x20  Failures : Natural := 0;\n\n\
         \x20  procedure Expect_Valid (Input, Stored, Label : String) is\n\
         \x20     Made : constant Test.Callsign.Callsign :=\n\
         \x20       Test.Callsign.Create (Input);\n\
         \x20  begin\n\
         \x20     if Test.Callsign.Value (Made) /= Stored then\n\
         \x20        Put_Line (\"bad storage: \" & Label);\n\
         \x20        Failures := Failures + 1;\n\
         \x20     end if;\n\
         \x20  exception\n\
         \x20     when Constraint_Error =>\n\
         \x20        Put_Line (\"must accept: \" & Label);\n\
         \x20        Failures := Failures + 1;\n\
         \x20  end Expect_Valid;\n\n\
         \x20  procedure Expect_Invalid (Input, Label : String) is\n\
         \x20  begin\n\
         \x20     declare\n\
         \x20        Ignored : constant Test.Callsign.Callsign :=\n\
         \x20          Test.Callsign.Create (Input);\n\
         \x20     begin\n\
         \x20        if Test.Callsign.Value (Ignored)'Length >= 0 then\n\
         \x20           Put_Line (\"must reject: \" & Label);\n\
         \x20           Failures := Failures + 1;\n\
         \x20        end if;\n\
         \x20     end;\n\
         \x20  exception\n\
         \x20     when Constraint_Error => null;\n\
         \x20  end Expect_Invalid;\n\n\
         begin\n",
    );
    for (index, case) in cases.iter().enumerate() {
        let input = common::ada_literal(&case.input);
        let label = format!("case {index}");
        match &case.expected {
            Some(stored) => {
                let stored = common::ada_literal(stored);
                writeln!(probe, "   Expect_Valid ({input}, {stored}, \"{label}\");")
                    .expect("writing to String cannot fail");
            }
            None => {
                writeln!(probe, "   Expect_Invalid ({input}, \"{label}\");")
                    .expect("writing to String cannot fail");
            }
        }
    }
    probe.push_str(&boundary_probe());
    probe.push_str(
        "   if Failures = 0 then\n\
         \x20     Put_Line (\"ok\");\n\
         \x20  else\n\
         \x20     Put_Line (\"failures\");\n\
         \x20  end if;\n\
         end Probe;\n",
    );

    let directory = std::env::temp_dir().join("ams-gra-oms-task039-ada-visible-ascii");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Ada probe directory");
    std::fs::write(directory.join("test.ads"), "package Test is\nend Test;\n")
        .expect("write Ada parent package");
    std::fs::write(directory.join("test-callsign.ads"), &spec).expect("write generated spec");
    std::fs::write(directory.join("test-callsign.adb"), &body).expect("write generated body");
    std::fs::write(directory.join("probe.adb"), &probe).expect("write Ada probe");

    let output = Command::new("gnatmake")
        .current_dir(&directory)
        .args(["-q", "probe.adb"])
        .output()
        .expect("gnatmake must run");
    assert!(
        output.status.success(),
        "generated Ada package must compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run = Command::new(directory.join("probe"))
        .output()
        .expect("compiled probe must run");
    let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
    assert!(
        run.status.success() && stdout.trim() == "ok",
        "generated validator must agree with the shared corpus:\n{stdout}"
    );
    std::fs::remove_dir_all(&directory).expect("remove Ada probe directory");
}

/// The assertions the shared corpus cannot express, appended to the probe.
///
/// The four ordinal boundaries, the per-declaration bounds, and equality under
/// `whiteSpace = preserve`. The `Create` calls here also prove the OVERLOAD
/// model works: several `Create (Value : String) return <different type>`
/// functions are legal because each call has a typed expected result.
fn boundary_probe() -> String {
    let mut probe = String::new();
    probe.push_str(
        "   Expect_Invalid ((1 => Character'Val (31)), \"U+001F\");\n\
         \x20  Expect_Valid ((1 => Character'Val (32)), (1 => Character'Val (32)), \"U+0020\");\n\
         \x20  Expect_Valid ((1 => Character'Val (126)), (1 => Character'Val (126)), \"U+007E\");\n\
         \x20  Expect_Invalid ((1 => Character'Val (127)), \"U+007F\");\n",
    );
    probe.push_str(&overload_probe());
    // whiteSpace = preserve: predefined equality compares the stored text, so
    // a trailing space makes a DISTINCT value and is never trimmed.
    //
    // Each `Create` result is bound to an explicitly typed constant first.
    // That is required, not incidental: with six `Create (Value : String)`
    // overloads differing only in return type, a call with no typed expected
    // result is genuinely ambiguous, and Ada rejects it.
    probe.push_str(
        "   declare\n\
         \x20     Plain    : constant Test.Callsign.Callsign := Test.Callsign.Create (\"abc\");\n\
         \x20     Same     : constant Test.Callsign.Callsign := Test.Callsign.Create (\"abc\");\n\
         \x20     Trailing : constant Test.Callsign.Callsign := Test.Callsign.Create (\"abc \");\n\
         \x20     Leading  : constant Test.Callsign.Callsign := Test.Callsign.Create (\" abc\");\n\
         \x20  begin\n\
         \x20     if Plain /= Same\n\
         \x20       or else Plain = Trailing\n\
         \x20       or else Plain = Leading\n\
         \x20       or else Trailing = Leading\n\
         \x20       or else Test.Callsign.Value (Trailing) /= \"abc \"\n\
         \x20       or else Test.Callsign.Value (Leading) /= \" abc\"\n\
         \x20     then\n\
         \x20        Put_Line (\"bad equality or trimmed spacing\");\n\
         \x20        Failures := Failures + 1;\n\
         \x20     end if;\n\
         \x20  end;\n",
    );
    probe
}

/// Per-declaration bounds, asked through overloaded `Create` calls.
///
/// Each declare block below requests a DIFFERENT return type from the same
/// `Create` spelling, so this compiling at all is the overload evidence.
fn overload_probe() -> String {
    String::from(
        "   declare\n\
         \x20     Long : constant String (1 .. 33) := (others => 'x');\n\
         \x20  begin\n\
         \x20     declare\n\
         \x20        Wide : constant Test.Callsign.Callsign := Test.Callsign.Create (Long);\n\
         \x20     begin\n\
         \x20        if Test.Callsign.Value (Wide)'Length /= 33 then\n\
         \x20           Put_Line (\"33 must fit 1 .. 256\");\n\
         \x20           Failures := Failures + 1;\n\
         \x20        end if;\n\
         \x20     end;\n\
         \x20     begin\n\
         \x20        declare\n\
         \x20           Narrow : constant Test.Callsign.ShortLabel :=\n\
         \x20             Test.Callsign.Create (Long);\n\
         \x20        begin\n\
         \x20           if Test.Callsign.Value (Narrow)'Length > 0 then\n\
         \x20              Put_Line (\"33 must exceed 1 .. 32\");\n\
         \x20              Failures := Failures + 1;\n\
         \x20           end if;\n\
         \x20        end;\n\
         \x20     exception\n\
         \x20        when Constraint_Error => null;\n\
         \x20     end;\n\
         \x20     begin\n\
         \x20        declare\n\
         \x20           Short : constant Test.Callsign.CountryCode :=\n\
         \x20             Test.Callsign.Create (\"x\");\n\
         \x20        begin\n\
         \x20           if Test.Callsign.Value (Short)'Length > 0 then\n\
         \x20              Put_Line (\"1 must be below minLength 2\");\n\
         \x20              Failures := Failures + 1;\n\
         \x20           end if;\n\
         \x20        end;\n\
         \x20     exception\n\
         \x20        when Constraint_Error => null;\n\
         \x20     end;\n\
         \x20     declare\n\
         \x20        Pair : constant Test.Callsign.CountryCode :=\n\
         \x20          Test.Callsign.Create (\"xx\");\n\
         \x20     begin\n\
         \x20        if Test.Callsign.Value (Pair) /= \"xx\" then\n\
         \x20           Put_Line (\"2 must meet minLength 2\");\n\
         \x20           Failures := Failures + 1;\n\
         \x20        end if;\n\
         \x20     end;\n\
         \x20  end;\n",
    )
}

/// Task 039: a client cannot name or aggregate the private representation.
///
/// Only the compiler can prove this, so GNAT is asked rather than asserted at.
#[test]
fn the_private_visible_ascii_representation_is_unreachable_from_a_client_under_gnat() {
    let schema = visible_ascii_schema();
    let spec = generate(&schema, CLOSED).expect("fixture must generate");
    let body = generate_body(&schema, CLOSED)
        .expect("body generation must not fail")
        .expect("a String-profile schema must emit a package body");

    if !gnat_available() {
        return;
    }

    let directory = std::env::temp_dir().join("ams-gra-oms-task039-ada-bypass");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Ada probe directory");
    std::fs::write(directory.join("test.ads"), "package Test is\nend Test;\n")
        .expect("write Ada parent package");
    std::fs::write(directory.join("test-callsign.ads"), &spec).expect("write generated spec");
    std::fs::write(directory.join("test-callsign.adb"), &body).expect("write generated body");

    for (label, declarations) in [
        (
            "aggregate",
            "   Bad : constant Test.Callsign.Callsign :=\n\
             \x20    (Text => Ada.Strings.Unbounded.To_Unbounded_String (\"nope\"));",
        ),
        (
            "component read",
            "   Made : constant Test.Callsign.Callsign :=\n\
             \x20    Test.Callsign.Create (\"ok\");\n\
             \x20  Bad : constant Ada.Strings.Unbounded.Unbounded_String := Made.Text;",
        ),
    ] {
        let probe = format!(
            "with Test.Callsign;\nwith Ada.Strings.Unbounded;\n\n\
             procedure Probe is\n{declarations}\nbegin\n   null;\nend Probe;\n"
        );
        std::fs::write(directory.join("probe.adb"), &probe).expect("write Ada probe");
        let output = Command::new("gnatmake")
            .current_dir(&directory)
            .args(["-q", "probe.adb"])
            .output()
            .expect("gnatmake must run");
        assert!(
            !output.status.success(),
            "the {label} bypass must not compile"
        );
    }
    std::fs::remove_dir_all(&directory).expect("remove Ada probe directory");
}
