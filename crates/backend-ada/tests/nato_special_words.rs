//! Task 042: the generated Ada NATO special-words carrier, compiled and run.
//!
//! The shared corpus, a 256-value `Character` sweep, boundary lengths, and
//! null / non-1-based / high-index input slices are executed against real
//! GNAT-compiled programs, under ordinary settings AND under
//! `Assertion_Policy (Ignore)`. The sweep's expected answers come from an
//! alphabet stated in this file, never from the production validator.

mod common;

use ams_gra_oms_backend_ada::{generate, generate_body};
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;

fn nato_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-nato-special-words.xsd"),
    )
    .expect("NATO special-words fixture should parse")
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

/// The suffix alphabet `[a-zA-Z\-_]` over `Character` positions, stated
/// independently of the generator.
fn expected_suffix_position(position: u8) -> bool {
    matches!(position, b'A'..=b'Z' | b'a'..=b'z' | b'-' | b'_')
}

/// Stage the generated child package plus its parent in a fresh directory.
fn stage(label: &str) -> PathBuf {
    let schema = nato_schema();
    let spec = generate(&schema, CLOSED).expect("fixture must generate");
    let body = generate_body(&schema, CLOSED)
        .expect("body generation must not fail")
        .expect("the NATO carrier needs a body");
    let directory = std::env::temp_dir().join(format!("ams-gra-oms-task042-ada-{label}"));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create Ada probe directory");
    std::fs::write(directory.join("test.ads"), "package Test is\nend Test;\n")
        .expect("write Ada parent package");
    std::fs::write(directory.join("test-markings.ads"), &spec).expect("write generated spec");
    std::fs::write(directory.join("test-markings.adb"), &body).expect("write generated body");
    directory
}

/// Build (never with `-gnata`) and run one probe: (compiled, ran ok, output).
fn build_and_run(directory: &Path, probe: &str) -> (bool, bool, String) {
    std::fs::write(directory.join("probe.adb"), probe).expect("write Ada probe");
    let built = Command::new("gnatmake")
        .current_dir(directory)
        .args(["-q", "-f", "probe.adb"])
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

/// The generated API shape and body, asserted before anything is compiled.
#[test]
fn generated_nato_special_words_api_is_an_opaque_private_carrier() {
    let schema = nato_schema();
    let spec = generate(&schema, CLOSED).expect("fixture must generate");
    let body = generate_body(&schema, CLOSED)
        .expect("fixture must generate a body")
        .expect("the NATO carrier needs a body, through the shared predicate");

    for name in ["SpecialWord", "DerivedSpecialWord"] {
        assert!(spec.contains(&format!("type {name} is private;")));
        assert!(spec.contains(&format!("function Create (Value : String) return {name};")));
        assert!(spec.contains(&format!("function Value (Item : {name}) return String;")));
        assert!(
            spec.contains(&format!("{name} requires initialization from Create")),
            "{name} must keep the Task 040 failing component default"
        );
    }
    assert!(spec.contains("A validated NATO special-words string (lexical form only)."));
    assert!(spec.contains("NATO:[a-zA-Z\\-_]{1,256} pattern and both length facets"));
    assert!(spec.contains("The stored representation, exactly as supplied."));

    assert_eq!(body.matches("Min_Length : constant := 6;").count(), 2);
    assert_eq!(body.matches("Max_Length : constant := 261;").count(), 2);
    assert_eq!(
        body.matches("Prefix : constant String := \"NATO:\";")
            .count(),
        2
    );
    // No assumption that Value'First = 1, and no input-sized buffer.
    assert!(body.contains("Text (Text'First + Offset)"));
    assert!(!body.contains("Value (1)"));
    assert!(!body.contains("String (1 .. Value'Length)"));
    for forbidden in [
        "with GNAT.Regpat",
        "Regpat.",
        "with Ada.Characters.Handling",
        "Characters.Handling.",
        "Is_Letter",
        "Is_Alphanumeric",
        "Trim (",
        "To_Lower",
        "To_Upper",
    ] {
        assert!(
            !body.contains(forbidden),
            "generated Ada must not reference {forbidden}"
        );
    }
    // Older profiles coexist, and the ordinary String control is unchanged.
    assert!(spec.contains("type Callsign is private;"));
    assert!(spec.contains("type SchemaVersion is private;"));
    assert!(spec.contains("type PreservedRemarks is private;"));
    assert!(spec.contains("Notes : Standard.Ada.Strings.Unbounded.Unbounded_String;"));
    // The positive-minimum bounded member still has no Clear.
    assert!(!spec.contains("procedure Clear (Container : in out Marking_Required_Sequence)"));
    assert!(spec.contains("procedure Clear (Container : in out Marking_History_Sequence)"));
}

/// The corpus, sweep, boundaries, and slices, under BOTH assertion policies.
#[test]
fn generated_nato_special_words_validator_matches_the_shared_corpus_under_both_policies() {
    if !gnat_available() {
        return;
    }
    let cases = corpus_probe();
    for (label, prelude) in [
        ("ordinary", ""),
        ("assertions-ignored", "pragma Assertion_Policy (Ignore);\n"),
    ] {
        let directory = stage(&format!("corpus-{label}"));
        let (compiled, ran, output) = build_and_run(&directory, &format!("{prelude}{cases}"));
        assert!(compiled, "generated Ada must compile ({label}):\n{output}");
        assert!(
            ran && output.trim() == "ok",
            "generated validator must agree with the shared corpus ({label}):\n{output}"
        );
        std::fs::remove_dir_all(&directory).expect("remove Ada probe directory");
    }
}

/// Build the corpus-driven Ada probe.
///
/// Every `Create` call is resolved by its expected type, because the package
/// legitimately holds several `Create (Value : String) return <type>`
/// overloads.
fn corpus_probe() -> String {
    let mut probe = String::from(
        "with Ada.Text_IO; use Ada.Text_IO;\n\
         with Ada.Strings.Fixed;\n\
         with Test.Markings;\n\
         use type Test.Markings.SpecialWord;\n\
         procedure Probe is\n\
         \x20  Failures : Natural := 0;\n\
         \x20  function Rep (N : Natural; C : Character) return String is\n\
         \x20    (Ada.Strings.Fixed.\"*\" (N, C));\n\
         \x20  procedure Expect_Valid (Input, Expected, Label : String) is\n\
         \x20  begin\n\
         \x20     declare\n\
         \x20        Made : constant Test.Markings.SpecialWord :=\n\
         \x20          Test.Markings.Create (Input);\n\
         \x20        Got : constant String := Test.Markings.Value (Made);\n\
         \x20        Again : constant Test.Markings.SpecialWord :=\n\
         \x20          Test.Markings.Create (Got);\n\
         \x20     begin\n\
         \x20        if Got /= Expected or else Again /= Made then\n\
         \x20           Put_Line (\"bad storage: \" & Label);\n\
         \x20           Failures := Failures + 1;\n\
         \x20        end if;\n\
         \x20     end;\n\
         \x20  exception\n\
         \x20     when Constraint_Error =>\n\
         \x20        Put_Line (\"must accept: \" & Label);\n\
         \x20        Failures := Failures + 1;\n\
         \x20  end Expect_Valid;\n\
         \x20  procedure Expect_Invalid (Input, Label : String) is\n\
         \x20  begin\n\
         \x20     declare\n\
         \x20        Ignored : constant Test.Markings.SpecialWord :=\n\
         \x20          Test.Markings.Create (Input);\n\
         \x20     begin\n\
         \x20        if Test.Markings.Value (Ignored)'Length >= 0 then\n\
         \x20           Put_Line (\"must reject: \" & Label);\n\
         \x20           Failures := Failures + 1;\n\
         \x20        end if;\n\
         \x20     end;\n\
         \x20  exception\n\
         \x20     --  ONLY Constraint_Error from the checked API counts as a\n\
         \x20     --  rejection; any other exception propagates and fails.\n\
         \x20     when Constraint_Error => null;\n\
         \x20  end Expect_Invalid;\n",
    );
    // The independently computed sweep expectation, as an Ada aggregate.
    let accepted: Vec<String> = (0_u16..=255)
        .map(|position| u8::try_from(position).expect("byte range"))
        .filter(|&position| expected_suffix_position(position))
        .map(|position| format!("{position} => True"))
        .collect();
    assert_eq!(
        accepted.len(),
        54,
        "the independent alphabet has 54 members"
    );
    writeln!(
        probe,
        "   Accepted : constant array (0 .. 255) of Boolean :=\n     ({}, others => False);",
        accepted.join(", ")
    )
    .expect("writing to String cannot fail");
    probe.push_str("begin\n");

    let cases = common::load_corpus(&common::nato_special_words_corpus_path());
    assert!(cases.len() > 100, "the corpus must be substantial");
    for (index, case) in cases.iter().enumerate() {
        let input = common::ada_literal(&case.input);
        match &case.expected {
            Some(stored) => writeln!(
                probe,
                "   Expect_Valid ({input}, {}, \"case {index}\");",
                common::ada_literal(stored)
            ),
            None => writeln!(probe, "   Expect_Invalid ({input}, \"case {index}\");"),
        }
        .expect("writing to String cannot fail");
    }
    probe.push_str(ADA_SEMANTICS_PROBE);
    probe.push_str(ADA_COMPOSITION_PROBE);
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

/// Sweep, boundaries, and slices.
const ADA_SEMANTICS_PROBE: &str = r#"   --  All 256 Character positions as the single suffix character.
   for Position in 0 .. 255 loop
      declare
         Text : constant String := "NATO:" & Character'Val (Position);
         Got  : Boolean;
      begin
         begin
            declare
               Made : constant Test.Markings.SpecialWord :=
                 Test.Markings.Create (Text);
            begin
               Got := Test.Markings.Value (Made) = Text;
            end;
         exception
            when Constraint_Error => Got := False;
         end;
         if Got /= Accepted (Position) then
            Put_Line ("alphabet position" & Position'Image);
            Failures := Failures + 1;
         end if;
      end;
   end loop;

   --  Suffix 0/1/255/256/257 == total 5/6/260/261/262.
   Expect_Invalid ("NATO:", "suffix 0 / total 5");
   Expect_Valid ("NATO:" & Rep (1, 'a'), "NATO:" & Rep (1, 'a'), "suffix 1");
   Expect_Valid ("NATO:" & Rep (255, 'b'), "NATO:" & Rep (255, 'b'), "suffix 255");
   Expect_Valid ("NATO:" & Rep (256, 'C'), "NATO:" & Rep (256, 'C'), "suffix 256");
   Expect_Invalid ("NATO:" & Rep (257, 'a'), "suffix 257 / total 262");
   --  Long syntactically invalid inputs, and a late digit.
   Expect_Invalid (Rep (10_000, 'x'), "long x");
   Expect_Invalid ("NATO:" & Rep (10_000, 'a'), "long suffix");
   Expect_Invalid ("NATO:" & Rep (255, 'a') & "1", "late digit");

   --  Non-1-based, null, and HIGH-INDEX input slices.
   declare
      Offset : constant String (7 .. 17) := "NATO:aBc-X_";
      Short  : constant String (40 .. 44) := "NATO:";
      High   : constant String (Positive'Last - 9 .. Positive'Last) :=
        "NATO:Ab-_z";
      High_Short : constant String (Positive'Last - 3 .. Positive'Last) :=
        "NATO";
      High_Long  : constant String
        (Positive'Last - 260 .. Positive'Last) := "NATO:" & Rep (256, 'q');
   begin
      Expect_Valid (Offset, "NATO:aBc-X_", "non-1-based slice");
      Expect_Valid (Offset (7 .. 12), "NATO:a", "non-1-based sub-slice");
      Expect_Invalid (Offset (7 .. 6), "null slice");
      Expect_Invalid (Offset (8 .. 17), "slice without the leading N");
      Expect_Invalid (Short, "non-1-based bare prefix");
      Expect_Valid (High, "NATO:Ab-_z", "slice ending at Positive'Last");
      Expect_Invalid (High_Short, "short slice ending at Positive'Last");
      Expect_Valid (High_Long, "NATO:" & Rep (256, 'q'),
        "maximum-length slice ending at Positive'Last");
      Expect_Invalid (High (Positive'Last - 4 .. Positive'Last),
        "high slice missing the prefix");
   end;
"#;

/// Mixed overloads, equality, the derived carrier, and every occurrence shape.
const ADA_COMPOSITION_PROBE: &str = r#"   declare
      Sign    : constant Test.Markings.Callsign :=
        Test.Markings.Create ("abc ");
      Version : constant Test.Markings.SchemaVersion :=
        Test.Markings.Create ("001.1.0");
      Remarks : constant Test.Markings.PreservedRemarks :=
        Test.Markings.Create ("a" & ASCII.LF & "b");
      Word    : constant Test.Markings.SpecialWord :=
        Test.Markings.Create ("NATO:Word");
      Derived : constant Test.Markings.DerivedSpecialWord :=
        Test.Markings.Create ("NATO:Derived_X");
      Lower   : constant Test.Markings.SpecialWord :=
        Test.Markings.Create ("NATO:word");
   begin
      if Test.Markings.Value (Sign) /= "abc "
        or else Test.Markings.Value (Version) /= "001.1.0"
        or else Test.Markings.Value (Remarks) /= "a" & ASCII.LF & "b"
        or else Test.Markings.Value (Word) /= "NATO:Word"
        or else Test.Markings.Value (Derived) /= "NATO:Derived_X"
      then
         Put_Line ("mixed overloads");
         Failures := Failures + 1;
      end if;
      if Word = Lower then
         Put_Line ("suffix case must be significant");
         Failures := Failures + 1;
      end if;
      begin
         declare
            Bad : constant Test.Markings.DerivedSpecialWord :=
              Test.Markings.Create ("NATO:1");
         begin
            Put_Line ("derived keeps the profile" & Test.Markings.Value (Bad));
            Failures := Failures + 1;
         end;
      exception
         when Constraint_Error => null;
      end;
   end;

   declare
      Word : constant Test.Markings.SpecialWord :=
        Test.Markings.Create ("NATO:Word");
      History : Test.Markings.Marking_History_Sequence :=
        Test.Markings.To_Sequence ((1 => Word));
      Required : constant Test.Markings.Marking_Required_Sequence :=
        Test.Markings.To_Sequence
          ((1 => Word, 2 => Test.Markings.Create ("NATO:r_b")));
      Log : Test.Markings.Marking_Log_Sequence;
      Absent : constant Test.Markings.Marking_Alternate_Optional :=
        (Is_Present => False);
      Present : constant Test.Markings.Marking_Alternate_Optional :=
        (Is_Present => True, Value => Test.Markings.Create ("NATO:alt-one"));
   begin
      for Index in 1 .. 300 loop
         Test.Markings.Append (Log, Word);
      end loop;
      Test.Markings.Append (History, Test.Markings.Create ("NATO:h-two"));
      if Test.Markings.Length (Log) /= 300
        or else Test.Markings.Length (History) /= 2
        or else Test.Markings.Value
          (Test.Markings.Element (Required, 2)) /= "NATO:r_b"
        or else Test.Markings.Value
          (Test.Markings.Element (History, 2)) /= "NATO:h-two"
        or else Absent.Is_Present
        or else Test.Markings.Value (Present.Value) /= "NATO:alt-one"
      then
         Put_Line ("composition");
         Failures := Failures + 1;
      end if;
      Test.Markings.Clear (History);
      begin
         declare
            Short : constant Test.Markings.Marking_Required_Sequence :=
              Test.Markings.To_Sequence ((1 => Word));
         begin
            Put_Line ("positive minimum" & Test.Markings.Length (Short)'Image);
            Failures := Failures + 1;
         end;
      exception
         when Constraint_Error => null;
      end;
   end;
"#;

/// Task 040 policy: a default-declared carrier can never hold unchecked text,
/// with or without assertions, while an absent optional wrapper remains
/// constructible and an unchecked PRESENT payload stays rejected.
#[test]
fn a_default_declared_nato_carrier_is_rejected_under_both_policies() {
    if !gnat_available() {
        return;
    }
    let directory = stage("lifecycle");
    std::fs::write(
        directory.join("no_assertions.adc"),
        "pragma Assertion_Policy (Ignore);\n",
    )
    .expect("write configuration pragma file");
    let default_probe = |carrier: &str| {
        format!(
            "with Test.Markings;\nwith Ada.Text_IO;\n\n\
             procedure Probe is\n\
             begin\n\
             \x20  declare\n\
             \x20     Item : Test.Markings.{carrier};\n\
             \x20  begin\n\
             \x20     Ada.Text_IO.Put_Line (\"UNCHECKED\");\n\
             \x20  end;\n\
             exception\n\
             \x20  when Program_Error =>\n\
             \x20     Ada.Text_IO.Put_Line (\"rejected\");\n\
             end Probe;\n"
        )
    };
    let absent_probe = "with Test.Markings;\nwith Ada.Text_IO;\n\n\
         procedure Probe is\n\
         \x20  Absent : constant Test.Markings.Marking_Alternate_Optional :=\n\
         \x20    (Is_Present => False);\n\
         begin\n\
         \x20  if not Absent.Is_Present then\n\
         \x20     Ada.Text_IO.Put_Line (\"absent ok\");\n\
         \x20  end if;\n\
         \x20  declare\n\
         \x20     Present : Test.Markings.Marking_Alternate_Optional (Is_Present => True);\n\
         \x20  begin\n\
         \x20     Ada.Text_IO.Put_Line (\"UNCHECKED present\");\n\
         \x20  end;\n\
         exception\n\
         \x20  when Program_Error =>\n\
         \x20     Ada.Text_IO.Put_Line (\"present rejected\");\n\
         end Probe;\n"
        .to_owned();
    for config in ["", "no_assertions.adc"] {
        let prelude = if config.is_empty() {
            String::new()
        } else {
            "pragma Assertion_Policy (Ignore);\n".to_owned()
        };
        for carrier in ["SpecialWord", "DerivedSpecialWord"] {
            let (compiled, ran, output) =
                build_and_run(&directory, &format!("{prelude}{}", default_probe(carrier)));
            assert!(compiled, "{carrier} default probe must compile:\n{output}");
            assert!(ran, "{carrier} default probe must not abort:\n{output}");
            assert!(
                output.contains("rejected") && !output.contains("UNCHECKED"),
                "{carrier}: a default declaration must fail explicitly ({config}):\n{output}"
            );
        }
        let (compiled, ran, output) =
            build_and_run(&directory, &format!("{prelude}{absent_probe}"));
        assert!(
            compiled && ran,
            "optional probe must build and run:\n{output}"
        );
        assert!(
            output.contains("absent ok"),
            "absent optional must construct:\n{output}"
        );
        assert!(
            output.contains("present rejected") && !output.contains("UNCHECKED"),
            "an unchecked present payload must be rejected ({config}):\n{output}"
        );
    }
    std::fs::remove_dir_all(&directory).expect("remove Ada probe directory");
}

/// A client cannot name or aggregate the private representation. Each
/// negative is paired with a positive control built by the same harness.
#[test]
fn the_private_nato_representation_is_unreachable_from_a_client_under_gnat() {
    if !gnat_available() {
        return;
    }
    let directory = stage("privacy");
    let wrap = |declarations: &str| {
        format!(
            "with Test.Markings;\nwith Ada.Strings.Unbounded;\n\n\
             procedure Probe is\n{declarations}\nbegin\n   null;\nend Probe;\n"
        )
    };
    let (compiled, _, output) = build_and_run(
        &directory,
        &wrap(
            "   Made : constant Test.Markings.SpecialWord :=\n\
             \x20    Test.Markings.Create (\"NATO:ok\");\n\
             \x20  Seen : constant String := Test.Markings.Value (Made);\n\
             \x20  Unused : Ada.Strings.Unbounded.Unbounded_String;\n\
             \x20  pragma Unreferenced (Seen, Unused);",
        ),
    );
    assert!(compiled, "the positive control must compile:\n{output}");

    for (label, declarations, diagnostic) in [
        (
            "aggregate",
            "   Bad : constant Test.Markings.SpecialWord :=\n\
             \x20    (Text => Ada.Strings.Unbounded.To_Unbounded_String (\"nope\"));",
            // The aggregate's expected type is the partial view. Matched on the
            // stable part of GNAT's message (observed with GNAT 14.2 as
            // `expected private type "SpecialWord"`), so a CI GNAT of another
            // release still proves the intended restriction.
            "private type",
        ),
        (
            "component read",
            "   Made : constant Test.Markings.SpecialWord :=\n\
             \x20    Test.Markings.Create (\"NATO:ok\");\n\
             \x20  Bad : constant Ada.Strings.Unbounded.Unbounded_String := Made.Text;",
            // The private view has no visible component (GNAT 14.2:
            // `invalid prefix in selected component "Made"`).
            "selected component",
        ),
    ] {
        let (compiled, _, output) = build_and_run(&directory, &wrap(declarations));
        assert!(!compiled, "the {label} bypass must not compile");
        assert!(
            output.contains(diagnostic),
            "the {label} bypass must fail on the private representation, got:\n{output}"
        );
        // And not for an unrelated reason such as a missing unit or syntax.
        for unrelated in ["not found", "file", "syntax error", "missing \";\""] {
            assert!(
                !output.contains(unrelated),
                "the {label} bypass failed for an unrelated reason:\n{output}"
            );
        }
    }
    std::fs::remove_dir_all(&directory).expect("remove Ada probe directory");
}
