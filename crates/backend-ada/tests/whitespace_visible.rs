//! Task 041: the generated Ada whitespace-visible carriers, compiled and run.
//!
//! Generated text alone would not prove the validators behave as the
//! authoritative XSD requires, so both shared corpora are executed against real
//! GNAT-compiled programs -- under ordinary client settings AND under
//! `Assertion_Policy (Ignore)`, because the carriers' enforcement must not
//! depend on assertions being enabled.

mod common;

use ams_gra_oms_backend_ada::{generate, generate_body};
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;

fn whitespace_visible_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-whitespace-visible.xsd"),
    )
    .expect("whitespace-visible fixture should parse")
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
fn generated_whitespace_visible_api_is_an_opaque_private_carrier() {
    let schema = whitespace_visible_schema();
    let spec = generate(&schema, CLOSED).expect("fixture must generate");
    let body = generate_body(&schema, CLOSED)
        .expect("fixture must generate a body")
        .expect("the validators need a body");

    for name in [
        "CollapsedRemarks",
        "CollapsedNarrative",
        "PreservedRemarks",
        "PreservedNarrative",
        "PreservedQuery",
    ] {
        // An opaque handle: the representation is not nameable by a client, so
        // no aggregate, component read, or conversion can bypass Create.
        assert!(
            spec.contains(&format!("type {name} is private;")),
            "{name} must be private"
        );
        assert!(
            spec.contains(&format!("function Create (Value : String) return {name};")),
            "{name} must expose Create"
        );
        assert!(
            spec.contains(&format!("function Value (Item : {name}) return String;")),
            "{name} must expose Value"
        );
        // Task 040: explicit-initialization policy is preserved, via a failing
        // component default rather than an assertion.
        assert!(
            spec.contains(&format!("{name} requires initialization from Create")),
            "{name} must keep its failing component default"
        );
    }

    // Task 041: the documented POLICY must not claim the empty string is an
    // invalid value. It is valid for the collapse profiles, whose minLength is 0.
    assert!(
        spec.contains("Create (\"\") SUCCEEDS"),
        "the private part must document the policy/validity distinction"
    );
    assert!(
        !spec.contains("empty string, which this profile's own validator rejects"),
        "the old over-broad claim must be gone"
    );

    // Documentation says "normalized" for collapse and keeps "exactly as
    // supplied" for preserve. Reusing one phrase for both would be wrong.
    assert!(
        spec.contains(
            "The stored representation, which is the collapse-normalized form of Create's argument."
        ),
        "a collapse carrier must not claim its value is exactly as supplied"
    );
    assert!(
        spec.contains("The stored representation, exactly as supplied."),
        "a preserve carrier must keep the original wording"
    );
    assert!(spec.contains("stored collapse-normalized"));
    assert!(spec.contains("stored with whitespace preserved"));

    // Per-declaration bounds, over whole evidenced triples.
    assert!(body.contains("Min_Length : constant := 0;"));
    assert!(body.contains("Min_Length : constant := 1;"));
    assert!(body.contains("Max_Length : constant := 1024;"));
    assert!(body.contains("Max_Length : constant := 4096;"));

    // The collapse half normalizes into a BOUNDED buffer, sized by the
    // declaration's own maxLength, never by the untrusted input's length.
    assert!(body.contains("Normalized : String (1 .. Max_Length);"));
    assert!(
        !body.contains("String (1 .. Value'Length)"),
        "the buffer must not be sized by client input"
    );
    // Exactly three collapse carriers in this fixture.
    assert_eq!(
        body.matches("Normalized : String (1 .. Max_Length);")
            .count(),
        3
    );

    // No regex engine, and nothing locale-sensitive.
    // Checked as `with` clauses and calls: the bare unit names appear in the
    // carriers' own documentation, which records them as non-dependencies.
    for forbidden in [
        "with GNAT.Regpat",
        "Regpat.",
        "with Ada.Characters.Handling",
        "Characters.Handling.",
        "Is_Graphic",
        "Trim (",
    ] {
        assert!(
            !body.contains(forbidden),
            "generated Ada must not reference {forbidden}"
        );
    }

    // Older profiles coexist in the same generated unit, which puts several
    // `Create (Value : String) return <different type>` overloads together.
    assert!(spec.contains("type Callsign is private;"));
    assert!(spec.contains("type SchemaVersion is private;"));
}

/// Both shared corpora, executed against GNAT-compiled programs.
///
/// Run TWICE: once with ordinary client settings, and once under
/// `Assertion_Policy (Ignore)`. The carriers raise `Constraint_Error` from
/// explicit code rather than from a `pragma Assert`, so disabling assertions
/// must not weaken validation. A carrier that leaned on assertions would pass
/// the first run and fail the second.
#[test]
fn generated_whitespace_visible_validators_match_the_shared_corpora_under_both_policies() {
    if !gnat_available() {
        return;
    }
    let schema = whitespace_visible_schema();
    let spec = generate(&schema, CLOSED).expect("fixture must generate");
    let body = generate_body(&schema, CLOSED)
        .expect("fixture must generate a body")
        .expect("the validators need a body");
    let cases = corpus_probe();

    for (label, prelude) in [
        ("ordinary", ""),
        ("assertions-ignored", "pragma Assertion_Policy (Ignore);\n"),
    ] {
        let probe = format!("{prelude}{cases}");
        let directory =
            std::env::temp_dir().join(format!("ams-gra-oms-task041-ada-corpus-{label}"));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("create Ada probe directory");
        std::fs::write(directory.join("test.ads"), "package Test is\nend Test;\n")
            .expect("write Ada parent package");
        std::fs::write(directory.join("test-remarks.ads"), &spec).expect("write generated spec");
        std::fs::write(directory.join("test-remarks.adb"), &body).expect("write generated body");
        std::fs::write(directory.join("probe.adb"), &probe).expect("write Ada probe");

        let output = Command::new("gnatmake")
            .current_dir(&directory)
            .args(["-q", "probe.adb"])
            .output()
            .expect("gnatmake must run");
        assert!(
            output.status.success(),
            "generated Ada package must compile ({label}):\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let run = Command::new(directory.join("probe"))
            .output()
            .expect("compiled probe must run");
        let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
        assert!(
            run.status.success() && stdout.trim() == "ok",
            "generated validators must agree with the shared corpora ({label}):\n{stdout}"
        );
        std::fs::remove_dir_all(&directory).expect("remove Ada probe directory");
    }
}

/// Build the corpus-driven Ada probe body.
///
/// Every `Create` call is a qualified expression, because the generated package
/// legitimately holds several `Create (Value : String) return <different type>`
/// overloads and only the expected type disambiguates them.
fn corpus_probe() -> String {
    let mut probe = String::from(
        "with Ada.Text_IO; use Ada.Text_IO;\n\
         with Ada.Strings.Fixed;\n\
         with Test.Remarks;\n\
         --  `use type` makes the predefined \"=\" of the private carriers directly\n\
         --  visible. The equality is the generated package's own; this only\n\
         --  brings the operator into scope so the probe can name it infix.\n\
         use type Test.Remarks.CollapsedRemarks;\n\
         use type Test.Remarks.PreservedRemarks;\n\
         procedure Probe is\n\
         \x20  Failures : Natural := 0;\n\
         \x20  function Rep (N : Natural; C : Character) return String is\n\
         \x20    (Ada.Strings.Fixed.\"*\" (N, C));\n",
    );
    // One Expect_Valid / Expect_Invalid pair per carrier, so each corpus is
    // checked against the carrier whose policy it documents.
    for carrier in ["CollapsedRemarks", "PreservedRemarks"] {
        writeln!(
            probe,
            "\x20  procedure Expect_Valid_{carrier}\n\
             \x20    (Input, Expected, Label : String) is\n\
             \x20  begin\n\
             \x20     declare\n\
             \x20        Made : constant Test.Remarks.{carrier} :=\n\
             \x20          Test.Remarks.Create (Input);\n\
             \x20        Got : constant String := Test.Remarks.Value (Made);\n\
             \x20     begin\n\
             \x20        if Got /= Expected then\n\
             \x20           Put_Line (\"bad storage: \" & Label);\n\
             \x20           Failures := Failures + 1;\n\
             \x20        end if;\n\
             \x20        --  Reconstruction: the stored form is itself a valid\n\
             \x20        --  input mapping to the same stored form.\n\
             \x20        declare\n\
             \x20           Again : constant Test.Remarks.{carrier} :=\n\
             \x20             Test.Remarks.Create (Got);\n\
             \x20        begin\n\
             \x20           if Test.Remarks.Value (Again) /= Expected then\n\
             \x20              Put_Line (\"not idempotent: \" & Label);\n\
             \x20              Failures := Failures + 1;\n\
             \x20           end if;\n\
             \x20        end;\n\
             \x20     end;\n\
             \x20  exception\n\
             \x20     when Constraint_Error =>\n\
             \x20        Put_Line (\"must accept: \" & Label);\n\
             \x20        Failures := Failures + 1;\n\
             \x20  end Expect_Valid_{carrier};\n\n\
             \x20  procedure Expect_Invalid_{carrier} (Input, Label : String) is\n\
             \x20  begin\n\
             \x20     declare\n\
             \x20        Ignored : constant Test.Remarks.{carrier} :=\n\
             \x20          Test.Remarks.Create (Input);\n\
             \x20     begin\n\
             \x20        if Test.Remarks.Value (Ignored)'Length >= 0 then\n\
             \x20           Put_Line (\"must reject: \" & Label);\n\
             \x20           Failures := Failures + 1;\n\
             \x20        end if;\n\
             \x20     end;\n\
             \x20  exception\n\
             \x20     when Constraint_Error => null;\n\
             \x20  end Expect_Invalid_{carrier};\n"
        )
        .expect("writing to String cannot fail");
    }
    probe.push_str("begin\n");
    for (carrier, path) in [
        (
            "CollapsedRemarks",
            common::whitespace_visible_collapse_corpus_path(),
        ),
        (
            "PreservedRemarks",
            common::whitespace_visible_preserve_corpus_path(),
        ),
    ] {
        let cases = common::load_corpus(&path);
        assert!(!cases.is_empty(), "{carrier} corpus must not be empty");
        for (index, case) in cases.iter().enumerate() {
            let input = common::ada_literal(&case.input);
            let label = format!("{carrier} case {index}");
            match &case.expected {
                Some(stored) => {
                    let stored = common::ada_literal(stored);
                    writeln!(
                        probe,
                        "   Expect_Valid_{carrier} ({input}, {stored}, \"{label}\");"
                    )
                    .expect("writing to String cannot fail");
                }
                None => {
                    writeln!(probe, "   Expect_Invalid_{carrier} ({input}, \"{label}\");")
                        .expect("writing to String cannot fail");
                }
            }
        }
    }
    probe.push_str(&semantics_probe());
    probe.push_str(
        "   if Failures = 0 then\n\
         \x20     Put_Line (\"ok\");\n\
         \x20  else\n\
         \x20     Put_Line (\"failures\");\n\
         \x20  end if;\n\
         end Probe;\n",
    );
    probe
}

/// The assertions the line-oriented corpora cannot express.
///
/// Long inputs, the two differing maxima, null and non-1-based input slices, and
/// the empty-accepted versus default-prohibited distinction.
fn semantics_probe() -> String {
    let mut probe = String::new();
    // The raw/normalized length distinction: a RAW argument far longer than
    // maxLength is ACCEPTED when its NORMALIZED form fits. This is what the
    // bounded output buffer must get right without an input-sized stack object.
    probe.push_str(
        "   Expect_Valid_CollapsedRemarks\n\
         \x20    (\"a\" & Rep (4000, ' ') & \"b\", \"a b\", \"raw exceeds max, normalized fits\");\n",
    );
    // Normalized length exactly N, and N+1 rejected, for the 1024 members.
    probe.push_str(
        "   Expect_Valid_CollapsedRemarks\n\
         \x20    (Rep (1024, 'x'), Rep (1024, 'x'), \"collapse 1024 fits\");\n\
         \x20  Expect_Invalid_CollapsedRemarks (Rep (1025, 'x'), \"collapse 1025 exceeds\");\n\
         \x20  Expect_Valid_PreservedRemarks\n\
         \x20    (Rep (1024, 'x'), Rep (1024, 'x'), \"preserve 1024 fits\");\n\
         \x20  Expect_Invalid_PreservedRemarks (Rep (1025, 'x'), \"preserve 1025 exceeds\");\n",
    );
    // A length BETWEEN 1024 and 4096, proving the two maxima really differ.
    probe.push_str(
        "   Expect_Invalid_CollapsedRemarks (Rep (2000, 'x'), \"2000 exceeds 1024\");\n\
         \x20  Expect_Invalid_PreservedRemarks (Rep (2000, 'x'), \"2000 exceeds 1024\");\n\
         \x20  declare\n\
         \x20     Wide : constant Test.Remarks.CollapsedNarrative :=\n\
         \x20       Test.Remarks.Create (Rep (2000, 'x'));\n\
         \x20     Long_Preserved : constant Test.Remarks.PreservedNarrative :=\n\
         \x20       Test.Remarks.Create (Rep (4096, 'x'));\n\
         \x20  begin\n\
         \x20     if Test.Remarks.Value (Wide)'Length /= 2000 then\n\
         \x20        Put_Line (\"2000 must fit the 4096 member\");\n\
         \x20        Failures := Failures + 1;\n\
         \x20     end if;\n\
         \x20     if Test.Remarks.Value (Long_Preserved)'Length /= 4096 then\n\
         \x20        Put_Line (\"4096 must fit the 4096 member\");\n\
         \x20        Failures := Failures + 1;\n\
         \x20     end if;\n\
         \x20  end;\n",
    );
    // NULL and NON-1-BASED input String ranges. Create must never assume
    // Value'First = 1, and a null slice must behave as the empty string.
    probe.push_str(
        "   declare\n\
         \x20     Offset : constant String (7 .. 12) := \"  a b \";\n\
         \x20     From_Offset : constant Test.Remarks.CollapsedRemarks :=\n\
         \x20       Test.Remarks.Create (Offset);\n\
         \x20     From_Null : constant Test.Remarks.CollapsedRemarks :=\n\
         \x20       Test.Remarks.Create (Offset (7 .. 6));\n\
         \x20     Preserved_Offset : constant Test.Remarks.PreservedRemarks :=\n\
         \x20       Test.Remarks.Create (Offset);\n\
         \x20  begin\n\
         \x20     if Test.Remarks.Value (From_Offset) /= \"a b\" then\n\
         \x20        Put_Line (\"non-1-based slice must normalize\");\n\
         \x20        Failures := Failures + 1;\n\
         \x20     end if;\n\
         \x20     if Test.Remarks.Value (From_Null) /= \"\" then\n\
         \x20        Put_Line (\"null slice must normalize to empty\");\n\
         \x20        Failures := Failures + 1;\n\
         \x20     end if;\n\
         \x20     if Test.Remarks.Value (Preserved_Offset) /= \"  a b \" then\n\
         \x20        Put_Line (\"non-1-based slice must be preserved verbatim\");\n\
         \x20        Failures := Failures + 1;\n\
         \x20     end if;\n\
         \x20  end;\n",
    );
    probe.push_str(&empty_and_equality_probe());
    probe
}

/// Accepted-empty construction, equality, and the older-profile overloads.
///
/// Separated from [`semantics_probe`] only to keep each function readable.
fn empty_and_equality_probe() -> String {
    let mut probe = String::new();
    // ACCEPTED-EMPTY construction, exercised SEPARATELY from the prohibited
    // default construction (which has its own test below). `Create ("")`
    // succeeding is a fact about the VALUE SPACE; default construction being
    // prohibited is an API POLICY. Both hold at once, and conflating them is
    // exactly the documentation error Task 041 corrects.
    probe.push_str(
        "   declare\n\
         \x20     Empty : constant Test.Remarks.CollapsedRemarks :=\n\
         \x20       Test.Remarks.Create (\"\");\n\
         \x20     Query_Empty : constant Test.Remarks.PreservedQuery :=\n\
         \x20       Test.Remarks.Create (\"\");\n\
         \x20  begin\n\
         \x20     if Test.Remarks.Value (Empty) /= \"\" then\n\
         \x20        Put_Line (\"minLength 0 must accept the empty string\");\n\
         \x20        Failures := Failures + 1;\n\
         \x20     end if;\n\
         \x20     if Test.Remarks.Value (Query_Empty) /= \"\" then\n\
         \x20        Put_Line (\"the 2.5 query shape must accept empty\");\n\
         \x20        Failures := Failures + 1;\n\
         \x20     end if;\n\
         \x20  end;\n\
         \x20  Expect_Invalid_PreservedRemarks (\"\", \"minLength 1 rejects empty\");\n",
    );
    // TAB: rejected under preserve, normalized to SPACE under collapse.
    probe.push_str(
        "   Expect_Valid_CollapsedRemarks\n\
         \x20    (\"a\" & ASCII.HT & \"b\", \"a b\", \"collapse maps TAB to SPACE\");\n\
         \x20  Expect_Invalid_PreservedRemarks\n\
         \x20    (\"a\" & ASCII.HT & \"b\", \"the preserved class excludes TAB\");\n",
    );
    // Equality on the STORED value: normalized spellings become identical and
    // compare equal under predefined "="; preserved whitespace stays
    // significant.
    probe.push_str(
        "   declare\n\
         \x20     Tidy : constant Test.Remarks.CollapsedRemarks :=\n\
         \x20       Test.Remarks.Create (\"a b\");\n\
         \x20     Messy : constant Test.Remarks.CollapsedRemarks :=\n\
         \x20       Test.Remarks.Create (\"  a   b  \");\n\
         \x20     Single : constant Test.Remarks.PreservedRemarks :=\n\
         \x20       Test.Remarks.Create (\"a b\");\n\
         \x20     Double : constant Test.Remarks.PreservedRemarks :=\n\
         \x20       Test.Remarks.Create (\"a  b\");\n\
         \x20  begin\n\
         \x20     if Tidy /= Messy then\n\
         \x20        Put_Line (\"normalized spellings must compare equal\");\n\
         \x20        Failures := Failures + 1;\n\
         \x20     end if;\n\
         \x20     if Single = Double then\n\
         \x20        Put_Line (\"preserved whitespace must stay significant\");\n\
         \x20        Failures := Failures + 1;\n\
         \x20     end if;\n\
         \x20  end;\n",
    );
    // MIXED Create/Value overloads with the OLDER profiles, in one declarative
    // part. This is what proves the overload model still resolves once a fourth
    // profile family joins the same generated package.
    probe.push_str(
        "   declare\n\
         \x20     Sign : constant Test.Remarks.Callsign :=\n\
         \x20       Test.Remarks.Create (\"abc \");\n\
         \x20     Version : constant Test.Remarks.SchemaVersion :=\n\
         \x20       Test.Remarks.Create (\"001.1.0\");\n\
         \x20     Derived : constant Test.Remarks.DerivedCollapsedRemarks :=\n\
         \x20       Test.Remarks.Create (\" a  b \");\n\
         \x20  begin\n\
         \x20     if Test.Remarks.Value (Sign) /= \"abc \" then\n\
         \x20        Put_Line (\"the visible-ASCII profile must be unchanged\");\n\
         \x20        Failures := Failures + 1;\n\
         \x20     end if;\n\
         \x20     if Test.Remarks.Value (Version) /= \"001.1.0\" then\n\
         \x20        Put_Line (\"the schema-version profile must be unchanged\");\n\
         \x20        Failures := Failures + 1;\n\
         \x20     end if;\n\
         \x20     if Test.Remarks.Value (Derived) /= \"a b\" then\n\
         \x20        Put_Line (\"the chain member must keep the collapse profile\");\n\
         \x20        Failures := Failures + 1;\n\
         \x20     end if;\n\
         \x20  end;\n",
    );
    probe
}
