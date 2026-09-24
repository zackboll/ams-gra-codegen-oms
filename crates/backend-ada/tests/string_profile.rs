//! Task 037: the generated Ada schema-version carrier, compiled and run.
//!
//! Generated text alone would not prove the validator behaves as XML Schema
//! requires, so every case in the shared corpus is executed against a real
//! GNAT-compiled program.

mod common;

use ams_gra_oms_backend_ada::{generate, generate_body};
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;

fn fixture(name: &str) -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../xsd-frontend/tests/fixtures/{name}")),
    )
    .unwrap_or_else(|error| panic!("fixture {name} should parse: {error:?}"))
}

fn string_schema() -> ams_gra_oms_ir::SchemaIr {
    fixture("backend-string-schema-version.xsd")
}

/// The generated API shape, asserted before anything is compiled.
#[test]
fn generated_schema_version_api_is_an_opaque_validated_carrier() {
    let spec = generate(&string_schema(), CLOSED).expect("string fixture must generate");
    let (visible, private_part) = spec
        .split_once("\nprivate\n")
        .expect("a String-profile schema must emit a private part");

    // The visible part is opaque: only the private type and the two
    // operations. No representation is nameable from a client, so no aggregate
    // or conversion can bypass `Create`.
    assert!(visible.contains("type SchemaVersion is private;"));
    assert!(visible.contains("function Create (Value : String) return SchemaVersion;"));
    assert!(visible.contains("function Value (Item : SchemaVersion) return String;"));
    // The carrier's own component name is not visible, so a client cannot write
    // an aggregate or a conversion for it.
    assert!(!visible.contains("Text :"));

    // The representation lives in the private completion, reusing the
    // package's existing owned-string type.
    assert!(private_part.contains("type SchemaVersion is record"));
    // Task 040 gave the component an explicitly failing default, so an
    // ordinary default declaration of the carrier cannot silently produce
    // empty text this profile would itself reject. The storage type is
    // otherwise unchanged.
    assert!(private_part.contains("Text : Standard.Ada.Strings.Unbounded.Unbounded_String :=\n"));
    assert!(private_part.contains("raise Standard.Program_Error"));

    // Unlike the Task 036 carrier, predefined "=" IS the correct XML Schema
    // semantics here, and the generated comment says so. Still no ordering:
    // XML Schema defines no order relation on `string`.
    assert!(visible.contains("IS XML Schema value equality"));
    for forbidden in ["function \"<\"", "function \"<=\"", "function Compare"] {
        assert!(!spec.contains(forbidden), "must not declare {forbidden}");
    }

    // No regex engine.
    for forbidden in ["GNAT.Regpat", "Regpat", "Regexp"] {
        assert!(!spec.contains(forbidden), "must not reference {forbidden}");
    }

    // Several supported declarations coexist, which requires Ada Create/Value
    // overload resolution to work; GNAT proves it in the runtime test below.
    assert!(visible.contains("type PeerVersion is private;"));
    assert!(visible.contains("function Create (Value : String) return PeerVersion;"));

    // Task 034 composition: the optional named occurrence reuses the existing
    // per-field wrapper, with no optional-String-profile special case.
    assert!(spec.contains("type Payload_Negotiated_Optional"));
    assert!(spec.contains("Value : SchemaVersion;"));

    // Ordinary unconstrained String keeps its existing plain representation.
    assert!(spec.contains("Label : Standard.Ada.Strings.Unbounded.Unbounded_String;"));
}

/// A schema with no body-requiring feature at all must not gain a body.
///
/// Task 037 generalized the existing body predicate rather than adding a
/// second mechanism, so this must still hold for unrelated schemas.
///
/// The Task 040 corrective second pass changed which fixtures qualify, and the
/// change is real rather than cosmetic: bounded sequence storage is now an
/// opaque private type whose operations have bodies, so `track.xsd` -- whose
/// `Sensor_Ids` field is `0 .. 8` -- legitimately acquires one. It is asserted
/// below as a positive control instead of being quietly dropped.
/// `backend-constrained-floating.xsd` has no repeated member and no carrier,
/// so it still emits no `.adb` at all, which is what keeps this a real check.
#[test]
fn schemas_without_a_validated_carrier_emit_no_body() {
    let name = "backend-constrained-floating.xsd";
    assert_eq!(
        generate_body(&fixture(name), CLOSED).expect("body generation must not fail"),
        None,
        "{name} must not gain an Ada package body"
    );
    // A bounded repeated member now needs a body, because its storage is
    // opaque. The body must define exactly that storage's operations and
    // nothing else -- no carrier validator appears, since this fixture has no
    // validated carrier.
    let track = generate_body(&fixture("track.xsd"), CLOSED)
        .expect("body generation must not fail")
        .expect("a bounded repeated member needs an Ada package body");
    assert!(track.contains("function To_Sequence"), "{track}");
    assert!(track.contains("procedure Append"), "{track}");
    assert!(
        !track.contains("Constraint_Error\n           with \"Create"),
        "{track}"
    );
    // The String-profile fixture does need one.
    assert!(
        generate_body(&string_schema(), CLOSED)
            .expect("body generation must not fail")
            .is_some()
    );
}

/// Every shared-corpus case, executed against the GNAT-compiled generated
/// package. `Create` must raise `Constraint_Error` on every invalid case.
#[test]
fn generated_schema_version_validator_matches_the_shared_corpus_under_gnat() {
    let schema = string_schema();
    let spec = generate(&schema, CLOSED).expect("string fixture must generate");
    let body = generate_body(&schema, CLOSED)
        .expect("body generation must not fail")
        .expect("a String-profile schema must emit a package body");
    let cases = common::load_corpus(&common::string_corpus_path());

    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but gnatmake is unavailable"
        );
        return;
    }

    let mut probe = String::from(
        "with Ada.Text_IO;\nwith Test.Versioning;\n\n\
         procedure Probe is\n\
         \x20  use Ada.Text_IO;\n\
         \x20  use type Test.Versioning.SchemaVersion;\n\
         \x20  Failures : Natural := 0;\n\n\
         \x20  procedure Expect_Valid (Input, Stored, Label : String) is\n\
         \x20     Made : constant Test.Versioning.SchemaVersion :=\n\
         \x20       Test.Versioning.Create (Input);\n\
         \x20  begin\n\
         \x20     if Test.Versioning.Value (Made) /= Stored then\n\
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
         \x20        Ignored : constant Test.Versioning.SchemaVersion :=\n\
         \x20          Test.Versioning.Create (Input);\n\
         \x20     begin\n\
         \x20        if Test.Versioning.Value (Ignored)'Length >= 0 then\n\
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
        let input = ada_literal(&case.input);
        let label = format!("case {index}");
        match &case.expected {
            Some(stored) => {
                let stored = ada_literal(stored);
                writeln!(probe, "   Expect_Valid ({input}, {stored}, \"{label}\");")
                    .expect("writing to String cannot fail");
            }
            None => {
                writeln!(probe, "   Expect_Invalid ({input}, \"{label}\");")
                    .expect("writing to String cannot fail");
            }
        }
    }
    // Predefined equality is a claimed property of the carrier, so it is
    // exercised rather than merely asserted in the generated text.
    probe.push_str(
        "   if Test.Versioning.Create (\"002.5.0\") /= Test.Versioning.Create (\"002.5.0\")\n\
         \x20    or else Test.Versioning.Create (\"002.5.0\") = Test.Versioning.Create (\"002.5.1\")\n\
         \x20  then\n\
         \x20     Put_Line (\"bad equality\");\n\
         \x20     Failures := Failures + 1;\n\
         \x20  end if;\n",
    );
    probe.push_str(
        "   if Failures = 0 then\n\
         \x20     Put_Line (\"ok\");\n\
         \x20  else\n\
         \x20     Put_Line (\"failures\");\n\
         \x20  end if;\n\
         end Probe;\n",
    );

    let directory = std::env::temp_dir().join("ams-gra-oms-task037-ada-string");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Ada probe directory");
    std::fs::write(directory.join("test.ads"), "package Test is\nend Test;\n")
        .expect("write Ada parent package");
    std::fs::write(directory.join("test-versioning.ads"), &spec).expect("write generated spec");
    std::fs::write(directory.join("test-versioning.adb"), &body).expect("write generated body");
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

/// Task 037: float, DateTime, and String-profile wrappers in one package.
///
/// Each of the three declarations emits a package-level `Create` and `Value`.
/// Two of them are `Create (String) return _`, differing *only* in result
/// type, which is the case the shared generated-callable name model depends on
/// being legal. GNAT is the authority, so it is compiled rather than argued
/// about. This is the Task 037 extension of the Task 036 overload evidence.
///
/// Skipped only where GNAT is absent, matching this file's probe policy.
#[test]
fn float_temporal_and_string_create_value_overloads_compile_under_gnat() {
    let schema = fixture("backend-string-mixed-callables.xsd");
    let spec = generate(&schema, CLOSED).expect("mixed fixture must generate");
    let body = generate_body(&schema, CLOSED)
        .expect("body generation must not fail")
        .expect("the validated carriers require a package body");

    // All four wrappers really do emit the shared callable pair. `Identifier`
    // is the Task 038 UUID carrier, making a THIRD `Create (String)` overload.
    assert!(spec.contains("function Create (Value : Interfaces.IEEE_Float_64) return BurnRate;"));
    for declaration in ["Instant", "SchemaVersion", "Identifier"] {
        assert!(spec.contains(&format!(
            "function Create (Value : String) return {declaration};"
        )));
    }

    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but gnatmake is unavailable"
        );
        return;
    }
    let directory = std::env::temp_dir().join("ams-gra-oms-task037-ada-overloads");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Ada probe directory");
    std::fs::write(directory.join("test.ads"), "package Test is\nend Test;\n")
        .expect("write Ada parent package");
    std::fs::write(directory.join("test-mixed.ads"), &spec).expect("write generated spec");
    std::fs::write(directory.join("test-mixed.adb"), &body).expect("write generated body");
    // A client that actually calls both `Create (String)` overloads, so an
    // ambiguity would be a hard error rather than merely unexercised.
    std::fs::write(
        directory.join("probe.adb"),
        "with Test.Mixed;\nwith Interfaces;\nuse type Interfaces.IEEE_Float_64;\n\n\
         procedure Probe is\n\
         \x20  Started : constant Test.Mixed.Instant :=\n\
         \x20    Test.Mixed.Create (\"2026-09-20T12:34:56Z\");\n\
         \x20  Version : constant Test.Mixed.SchemaVersion :=\n\
         \x20    Test.Mixed.Create (\"002.5.0\");\n\
         \x20  Ident : constant Test.Mixed.Identifier :=\n\
         \x20    Test.Mixed.Create (\"123e4567-e89b-12d3-a456-426614174000\");\n\
         \x20  Rate : constant Test.Mixed.BurnRate := Test.Mixed.Create (0.5);\n\
         begin\n\
         \x20  if Test.Mixed.Value (Started) = \"\"\n\
         \x20    or else Test.Mixed.Value (Version) /= \"002.5.0\"\n\
         \x20    or else Test.Mixed.Value (Ident)\n\
         \x20         /= \"123e4567-e89b-12d3-a456-426614174000\"\n\
         \x20    or else Test.Mixed.Value (Rate) < 0.0\n\
         \x20  then\n\
         \x20     raise Program_Error;\n\
         \x20  end if;\n\
         end Probe;\n",
    )
    .expect("write Ada probe");

    let output = Command::new("gnatmake")
        .current_dir(&directory)
        .args(["-q", "probe.adb"])
        .output()
        .expect("gnatmake must run");
    assert!(
        output.status.success(),
        "overloaded Create/Value across wrapper families must compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        Command::new(directory.join("probe"))
            .status()
            .expect("probe must run")
            .success()
    );
    std::fs::remove_dir_all(&directory).expect("remove Ada probe directory");
}

/// Render a Rust string as an Ada `String` expression.
///
/// Ada string literals cannot carry control characters, so tabs, line feeds,
/// and carriage returns are spliced in as `ASCII` constants. This keeps the
/// whitespace cases byte-identical to the ones Rust and C++ receive.
fn ada_literal(text: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut literal = String::new();
    for character in text.chars() {
        match character {
            '\t' | '\n' | '\r' => {
                if !literal.is_empty() {
                    parts.push(format!("\"{literal}\""));
                    literal.clear();
                }
                parts.push(
                    match character {
                        '\t' => "ASCII.HT",
                        '\n' => "ASCII.LF",
                        _ => "ASCII.CR",
                    }
                    .to_owned(),
                );
            }
            // Ada doubles an embedded quotation mark.
            '"' => literal.push_str("\"\""),
            other => literal.push(other),
        }
    }
    if !literal.is_empty() || parts.is_empty() {
        parts.push(format!("\"{literal}\""));
    }
    parts.join(" & ")
}
