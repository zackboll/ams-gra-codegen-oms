//! Task 038: the generated Ada UUID carrier, compiled and run.
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

fn uuid_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-uuid.xsd"),
    )
    .expect("uuid fixture should parse")
}

/// The generated API shape, asserted before anything is compiled.
#[test]
fn generated_uuid_api_is_an_opaque_validated_carrier() {
    let spec = generate(&uuid_schema(), CLOSED).expect("uuid fixture must generate");
    let (visible, private_part) = spec
        .split_once("\nprivate\n")
        .expect("a String-profile schema must emit a private part");

    // The visible part is opaque: only the private type and the two
    // operations. No representation is nameable from a client, so no aggregate
    // or conversion can bypass `Create`.
    assert!(visible.contains("type Uuid is private;"));
    assert!(visible.contains("function Create (Value : String) return Uuid;"));
    assert!(visible.contains("function Value (Item : Uuid) return String;"));
    assert!(!visible.contains("Text :"));

    // The representation lives in the private completion, reusing the
    // package's existing owned-string type.
    assert!(private_part.contains("type Uuid is record"));
    assert!(private_part.contains("Text : Standard.Ada.Strings.Unbounded.Unbounded_String;"));

    // Predefined "=" IS the correct XML Schema semantics for xs:string, and the
    // generated comment says so. Still no ordering.
    assert!(visible.contains("IS XML Schema value equality"));
    for forbidden in ["function \"<\"", "function \"<=\"", "function Compare"] {
        assert!(!spec.contains(forbidden), "must not declare {forbidden}");
    }

    // No regex engine.
    for forbidden in ["GNAT.Regpat", "Regpat", "Regexp"] {
        assert!(!spec.contains(forbidden), "must not reference {forbidden}");
    }

    // Several supported declarations coexist, spanning BOTH String profiles,
    // which requires Ada Create/Value overload resolution to work.
    assert!(visible.contains("type PeerUuid is private;"));
    assert!(visible.contains("function Create (Value : String) return PeerUuid;"));
    assert!(visible.contains("type SchemaVersion is private;"));
    assert!(visible.contains("function Create (Value : String) return SchemaVersion;"));

    // Task 034 composition: the optional named occurrence reuses the existing
    // per-field wrapper, with no optional-UUID special case.
    assert!(spec.contains("type Payload_Correlation_Optional"));
    assert!(spec.contains("Value : Uuid;"));

    // Ordinary unconstrained String keeps its existing plain representation.
    assert!(spec.contains("Label : Standard.Ada.Strings.Unbounded.Unbounded_String;"));
}

/// Every shared-corpus case, executed against the GNAT-compiled generated
/// package. `Create` must raise `Constraint_Error` on every invalid case.
#[test]
fn generated_uuid_validator_matches_the_shared_corpus_under_gnat() {
    let schema = uuid_schema();
    let spec = generate(&schema, CLOSED).expect("uuid fixture must generate");
    let body = generate_body(&schema, CLOSED)
        .expect("body generation must not fail")
        .expect("a String-profile schema must emit a package body");
    let cases = common::load_corpus(&common::uuid_corpus_path());

    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but gnatmake is unavailable"
        );
        return;
    }

    let mut probe = String::from(
        "with Ada.Text_IO;\nwith Test.Identity;\n\n\
         procedure Probe is\n\
         \x20  use Ada.Text_IO;\n\
         \x20  use type Test.Identity.Uuid;\n\
         \x20  Failures : Natural := 0;\n\n\
         \x20  procedure Expect_Valid (Input, Stored, Label : String) is\n\
         \x20     Made : constant Test.Identity.Uuid :=\n\
         \x20       Test.Identity.Create (Input);\n\
         \x20  begin\n\
         \x20     if Test.Identity.Value (Made) /= Stored then\n\
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
         \x20        Ignored : constant Test.Identity.Uuid :=\n\
         \x20          Test.Identity.Create (Input);\n\
         \x20     begin\n\
         \x20        if Test.Identity.Value (Ignored)'Length >= 0 then\n\
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
    // Predefined equality is a claimed property of the carrier, and it must
    // remain CASE-SENSITIVE: two otherwise-valid UUIDs differing only in
    // letter case are distinct xs:string values.
    probe.push_str(
        "   if Test.Identity.Create (\"123e4567-e89b-12d3-a456-42661417400f\")\n\
         \x20       /= Test.Identity.Create (\"123e4567-e89b-12d3-a456-42661417400f\")\n\
         \x20    or else Test.Identity.Create (\"123e4567-e89b-12d3-a456-42661417400f\")\n\
         \x20          = Test.Identity.Create (\"123E4567-E89B-12D3-A456-42661417400F\")\n\
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

    let directory = std::env::temp_dir().join("ams-gra-oms-task038-ada-uuid");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Ada probe directory");
    std::fs::write(directory.join("test.ads"), "package Test is\nend Test;\n")
        .expect("write Ada parent package");
    std::fs::write(directory.join("test-identity.ads"), &spec).expect("write generated spec");
    std::fs::write(directory.join("test-identity.adb"), &body).expect("write generated body");
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

/// Task 038: a client cannot name or aggregate the private representation.
///
/// Only the compiler can prove this, so GNAT is asked rather than asserted at.
#[test]
fn the_private_uuid_representation_is_unreachable_from_a_client_under_gnat() {
    let schema = uuid_schema();
    let spec = generate(&schema, CLOSED).expect("uuid fixture must generate");
    let body = generate_body(&schema, CLOSED)
        .expect("body generation must not fail")
        .expect("a String-profile schema must emit a package body");

    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but gnatmake is unavailable"
        );
        return;
    }

    let directory = std::env::temp_dir().join("ams-gra-oms-task038-ada-bypass");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Ada probe directory");
    std::fs::write(directory.join("test.ads"), "package Test is\nend Test;\n")
        .expect("write Ada parent package");
    std::fs::write(directory.join("test-identity.ads"), &spec).expect("write generated spec");
    std::fs::write(directory.join("test-identity.adb"), &body).expect("write generated body");

    for (label, declarations) in [
        (
            "aggregate",
            "   Bad : constant Test.Identity.Uuid :=\n\
             \x20    (Text => Ada.Strings.Unbounded.To_Unbounded_String (\"nope\"));",
        ),
        (
            "component read",
            "   Made : constant Test.Identity.Uuid :=\n\
             \x20    Test.Identity.Create (\"00000000-0000-0000-0000-000000000000\");\n\
             \x20  Bad : constant Ada.Strings.Unbounded.Unbounded_String := Made.Text;",
        ),
    ] {
        let probe = format!(
            "with Test.Identity;\nwith Ada.Strings.Unbounded;\n\n\
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
