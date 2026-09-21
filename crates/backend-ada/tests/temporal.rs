//! Task 036: the generated Ada DateTime Zulu carrier, compiled and run.
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

fn temporal_schema() -> ams_gra_oms_ir::SchemaIr {
    fixture("backend-temporal-datetime.xsd")
}

/// The generated API shape, asserted before anything is compiled.
#[test]
fn generated_date_time_api_is_an_opaque_validated_carrier() {
    let spec = generate(&temporal_schema(), CLOSED).expect("temporal fixture must generate");
    let (visible, private_part) = spec
        .split_once("\nprivate\n")
        .expect("a temporal schema must emit a private part");

    // The visible part is opaque: only the private type and the two
    // operations. No representation is nameable from a client, so no
    // aggregate or conversion can bypass `Create`.
    assert!(visible.contains("type Instant is private;"));
    assert!(visible.contains("function Create (Value : String) return Instant;"));
    assert!(visible.contains("function Value (Item : Instant) return String;"));
    // The carrier's own component name is not visible, so a client cannot
    // write an aggregate or a conversion for it. (`Optional_String` is a
    // pre-existing support type and legitimately mentions Unbounded_String.)
    assert!(!visible.contains("Lexical"));

    // The representation lives in the private completion, based on the
    // package's existing owned-string type -- not Ada.Calendar.Time, whose
    // year range and precision are narrower than XML Schema's.
    assert!(private_part.contains("type Instant is record"));
    assert!(private_part.contains("Lexical : Standard.Ada.Strings.Unbounded.Unbounded_String;"));
    assert!(!spec.contains("Ada.Calendar"));

    // Predefined "=" is documented as lexical, not value-space, equality, and
    // no ordering operator is declared.
    assert!(visible.contains("NOT XML Schema value-space equality"));
    for forbidden in ["function \"<\"", "function \"<=\"", "function Compare"] {
        assert!(!spec.contains(forbidden), "must not declare {forbidden}");
    }

    // Several supported declarations coexist, which requires Ada Create/Value
    // overload resolution to work; GNAT proves it in the runtime test below.
    assert!(visible.contains("type Deadline is private;"));
    assert!(visible.contains("function Create (Value : String) return Deadline;"));

    // Task 034 composition: the optional named occurrence reuses the existing
    // per-field wrapper, with no optional-DateTime special case.
    assert!(spec.contains("type Payload_Timestamp_Optional"));
    assert!(spec.contains("Value : Instant;"));
}

/// A schema with no Task 036 carrier must not gain a package body.
#[test]
fn schemas_without_a_temporal_carrier_emit_no_body() {
    for name in ["track.xsd", "backend-constrained-floating.xsd"] {
        let schema = fixture(name);
        assert_eq!(
            generate_body(&schema, CLOSED).expect("body generation must not fail"),
            None,
            "{name} must not gain an Ada package body"
        );
    }
    // The temporal fixture does need one.
    assert!(
        generate_body(&temporal_schema(), CLOSED)
            .expect("body generation must not fail")
            .is_some()
    );
}

/// Every shared-corpus case, executed against the GNAT-compiled generated
/// package. `Create` must raise `Constraint_Error` on every invalid case.
#[test]
fn generated_date_time_validator_matches_the_shared_corpus_under_gnat() {
    let schema = temporal_schema();
    let spec = generate(&schema, CLOSED).expect("temporal fixture must generate");
    let body = generate_body(&schema, CLOSED)
        .expect("body generation must not fail")
        .expect("a temporal schema must emit a package body");
    let cases = common::load_cases();

    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but gnatmake is unavailable"
        );
        return;
    }

    let mut probe = String::from(
        "with Ada.Text_IO;\nwith Test.Temporal;\n\n\
         procedure Probe is\n\
         \x20  use Ada.Text_IO;\n\
         \x20  Failures : Natural := 0;\n\n\
         \x20  procedure Expect_Valid (Input, Normalized, Label : String) is\n\
         \x20     Made : constant Test.Temporal.Instant := Test.Temporal.Create (Input);\n\
         \x20  begin\n\
         \x20     if Test.Temporal.Value (Made) /= Normalized then\n\
         \x20        Put_Line (\"bad normalization: \" & Label);\n\
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
         \x20        Ignored : constant Test.Temporal.Instant :=\n\
         \x20          Test.Temporal.Create (Input);\n\
         \x20     begin\n\
         \x20        if Test.Temporal.Value (Ignored)'Length >= 0 then\n\
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
            Some(normalized) => {
                let normalized = ada_literal(normalized);
                writeln!(
                    probe,
                    "   Expect_Valid ({input}, {normalized}, \"{label}\");"
                )
                .expect("writing to String cannot fail");
            }
            None => {
                writeln!(probe, "   Expect_Invalid ({input}, \"{label}\");")
                    .expect("writing to String cannot fail");
            }
        }
    }
    probe.push_str(
        "   if Failures = 0 then\n\
         \x20     Put_Line (\"ok\");\n\
         \x20  else\n\
         \x20     Put_Line (\"failures\");\n\
         \x20  end if;\n\
         end Probe;\n",
    );

    let directory = std::env::temp_dir().join("ams-gra-oms-task036-ada-temporal");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Ada probe directory");
    std::fs::write(directory.join("test.ads"), "package Test is\nend Test;\n")
        .expect("write Ada parent package");
    std::fs::write(directory.join("test-temporal.ads"), &spec).expect("write generated spec");
    std::fs::write(directory.join("test-temporal.adb"), &body).expect("write generated body");
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

/// Task 036: constrained-float and DateTime wrappers coexist in one package.
///
/// Each of the four declarations emits a package-level `Create` and `Value`.
/// Two of them are `Create (String) return _`, differing *only* in result
/// type, which is the case the shared generated-callable name model depends
/// on being legal. GNAT is the authority, so it is compiled rather than
/// argued about.
///
/// Skipped only where GNAT is absent, matching this file's probe policy.
#[test]
fn float_and_temporal_create_value_overloads_compile_under_gnat() {
    let schema = fixture("backend-temporal-mixed-callables.xsd");
    let spec = generate(&schema, CLOSED).expect("mixed fixture must generate");
    let body = generate_body(&schema, CLOSED)
        .expect("body generation must not fail")
        .expect("the temporal wrappers require a package body");

    // All four wrappers really do emit the shared callable pair.
    for declaration in ["BurnRate", "AltitudeMeters"] {
        assert!(spec.contains(&format!(
            "function Create (Value : Interfaces.IEEE_Float_64) return {declaration};"
        )));
    }
    for declaration in ["Instant", "Deadline"] {
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
    let directory = std::env::temp_dir().join("ams-gra-oms-task036-ada-overloads");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Ada probe directory");
    std::fs::write(directory.join("test.ads"), "package Test is\nend Test;\n")
        .expect("write Ada parent package");
    std::fs::write(directory.join("test-mixed.ads"), &spec).expect("write generated spec");
    std::fs::write(directory.join("test-mixed.adb"), &body).expect("write generated body");
    // A client that actually calls both `Create (String)` overloads, so the
    // ambiguity would be a hard error rather than merely unexercised.
    std::fs::write(
        directory.join("probe.adb"),
        "with Test.Mixed;\nwith Interfaces;\nuse type Interfaces.IEEE_Float_64;\n\n\
         procedure Probe is\n\
         \x20  Started : constant Test.Mixed.Instant :=\n\
         \x20    Test.Mixed.Create (\"2026-09-20T12:34:56Z\");\n\
         \x20  Ends : constant Test.Mixed.Deadline :=\n\
         \x20    Test.Mixed.Create (\"2026-09-21T00:00:00Z\");\n\
         \x20  Rate : constant Test.Mixed.BurnRate := Test.Mixed.Create (0.5);\n\
         begin\n\
         \x20  if Test.Mixed.Value (Started) = \"\"\n\
         \x20    or else Test.Mixed.Value (Ends) = \"\"\n\
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
