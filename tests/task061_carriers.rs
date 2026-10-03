// Shared compiler-backed Time oracle. Included by each backend integration test.
use ams_gra_oms_codegen_core::{Backend, GenerationWorld};
use std::{fmt::Write as _, path::Path, process::Command};

fn checked(output: std::process::Output) {
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn task061_time_corpus_lifecycle_and_storage_compile() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema = ams_gra_oms_xsd_frontend::load_schema_document(
        &root.join("tests/fixtures/temporal/time-zulu.xsd"),
    )
    .unwrap();
    let files = backend()
        .generate(&schema, GenerationWorld::ClosedSchemaSet)
        .unwrap();
    let cases = common::load_corpus(&root.join("tests/fixtures/temporal/time-zulu.txt"));
    assert!(cases.len() >= 50);
    let dir = std::env::temp_dir().join(format!(
        "task061-carriers-{LANGUAGE}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    for file in &files {
        std::fs::write(dir.join(&file.relative_path), &file.contents).unwrap();
    }
    match LANGUAGE {
        "rust" => {
            let source = files
                .iter()
                .find(|f| f.relative_path.extension().is_some_and(|e| e == "rs"))
                .unwrap();
            assert!(
                source
                    .contents
                    .contains("#[derive(Debug, Clone, PartialEq, Eq)]\npub struct AbsentHolder")
            );
            assert!(!source.contents.contains("XmlSchemaTimeParser"));
            std::fs::write(dir.join("generated.rs"), &source.contents).unwrap();
            let mut probe = String::from("include!(\"generated.rs\");\nfn main() {\n");
            for case in &cases {
                writeln!(
                    probe,
                    "assert_eq!(Clock::new({:?}).as_ref().map(|v| v.as_str()), {:?});",
                    case.input, case.expected
                )
                .unwrap();
            }
            probe.push_str("for h in 0..100 { for m in 0..100 { for s in 0..100 { let text = format!(\"{h:02}:{m:02}:{s:02}Z\"); let expected = (h < 24 && m < 60 && s <= 60) || (h == 24 && m == 0 && s == 0); assert_eq!(Clock::new(&text).is_some(), expected); } } }\nlet value = Clock::new(\"12:34:56Z\").unwrap(); let copy = value.clone(); assert_eq!(value.as_str(), copy.as_str());\nlet payload = Payload { required: value.clone(), optional: Some(value.clone()), bounded: BoundedVec::new(vec![value.clone()]).unwrap(), unbounded: UnboundedVec::new(vec![value.clone(); 100]).unwrap(), selection: Pick::Time(value.clone()), item: AbstractClock::ConcreteClock(ConcreteClock { time: value.clone() }) }; assert_eq!(payload.required.as_str(), payload.optional.as_ref().unwrap().as_str()); assert_eq!(payload.bounded.as_slice().len(), 1); assert_eq!(payload.unbounded.as_slice().len(),100);\nfn comparable<T: PartialEq + Eq>() {} comparable::<Unrelated>();\nif std::env::args().len() > 1 { assert!(Clock::new(\"00:00:00Z\").is_none()); }\n}\n");
            std::fs::write(dir.join("probe.rs"), probe).unwrap();
            checked(
                Command::new("rustc")
                    .current_dir(&dir)
                    .args(["--edition=2024", "-Dwarnings", "probe.rs", "-o", "probe"])
                    .output()
                    .unwrap(),
            );
            for (label, code, diagnostic) in [
                ("default", "let _ = generated::Clock::default();", "default"),
                (
                    "privacy",
                    "let _ = generated::Clock { lexical: String::new() };",
                    "private",
                ),
                (
                    "equality",
                    "fn eq<T: PartialEq>() {} eq::<generated::Clock>();",
                    "PartialEq",
                ),
                (
                    "record-equality",
                    "fn eq<T: PartialEq>() {} eq::<generated::Payload>();",
                    "PartialEq",
                ),
                (
                    "choice-equality",
                    "fn eq<T: PartialEq>() {} eq::<generated::Pick>();",
                    "PartialEq",
                ),
                (
                    "sum-equality",
                    "fn eq<T: PartialEq>() {} eq::<generated::AbstractClock>();",
                    "PartialEq",
                ),
                (
                    "optional-equality",
                    "fn eq<T: PartialEq>() {} eq::<Option<generated::Clock>>();",
                    "PartialEq",
                ),
                (
                    "bounded-equality",
                    "fn eq<T: PartialEq>() {} eq::<generated::BoundedVec<generated::Clock,0,3>>();",
                    "PartialEq",
                ),
                (
                    "unbounded-equality",
                    "fn eq<T: PartialEq>() {} eq::<generated::UnboundedVec<generated::Clock,0>>();",
                    "PartialEq",
                ),
                (
                    "inherited-equality",
                    "fn eq<T: PartialEq>() {} eq::<generated::Base>();",
                    "PartialEq",
                ),
                (
                    "concrete-equality",
                    "fn eq<T: PartialEq>() {} eq::<generated::ConcreteClock>();",
                    "PartialEq",
                ),
            ] {
                std::fs::write(
                    dir.join("negative.rs"),
                    format!(
                        "mod generated {{ include!(\"generated.rs\"); }} fn main() {{ {code} }}"
                    ),
                )
                .unwrap();
                let result = Command::new("rustc")
                    .current_dir(&dir)
                    .args(["--edition=2024", "negative.rs"])
                    .output()
                    .unwrap();
                assert!(
                    !result.status.success()
                        && String::from_utf8_lossy(&result.stderr).contains(diagnostic),
                    "{label}: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
            }
        }
        "cpp" => {
            let source = files
                .iter()
                .find(|f| f.relative_path.extension().is_some_and(|e| e == "hpp"))
                .unwrap();
            std::fs::write(dir.join("generated.hpp"), &source.contents).unwrap();
            let mut probe = String::from(
                "#include \"generated.hpp\"\n#include <cassert>\n#include <cstdio>\n#include <type_traits>\nusing namespace test::time;\nint main(int argc, char**) {\nstatic_assert(!std::is_default_constructible_v<Clock>);\n",
            );
            for case in &cases {
                let input = common::escape_for_cpp_source(&case.input);
                writeln!(probe, "{{ auto v = Clock::create(std::string_view(\"{input}\", {})); assert(v.has_value() == {});", case.input.len(), case.expected.is_some()).unwrap();
                if let Some(expected) = &case.expected {
                    writeln!(
                        probe,
                        "assert(v->value() == \"{}\");",
                        common::escape_for_cpp_source(expected)
                    )
                    .unwrap();
                }
                probe.push_str("}\n");
            }
            probe.push_str("for (unsigned h=0; h<100; ++h) { for (unsigned m=0; m<100; ++m) { for (unsigned s=0; s<100; ++s) { char text[10]; std::snprintf(text, sizeof text, \"%02u:%02u:%02uZ\", h,m,s); bool expected=(h<24 && m<60 && s<=60)||(h==24 && m==0 && s==0); assert(Clock::create(text).has_value()==expected); } } }\nauto v = Clock::create(\"12:34:56Z\").value(); auto copy = v; auto moved = std::move(v); assert(v.value()==copy.value() && moved.value()==v.value()); copy = std::move(v); assert(v.value()==copy.value()); std::swap(v, copy); assert(v.value()==copy.value());\nPayload payload {v, std::optional<Clock>(v), BoundedVector<Clock,0,3>::create({v}).value(), UnboundedVector<Clock,0>::create(std::vector<Clock>(100,v)).value(), Pick{Pick::Time{v}}, AbstractClock{ConcreteClock{v}}}; assert(payload.bounded.values().size()==1); assert(payload.unbounded.values().size()==100); assert(payload.required.value()==payload.optional->value());\nif (argc > 1) { assert(!Clock::create(\"00:00:00Z\")); }\n}\n");
            std::fs::write(dir.join("probe.cpp"), probe).unwrap();
            checked(
                Command::new("c++")
                    .current_dir(&dir)
                    .args([
                        "-std=c++17",
                        "-Wall",
                        "-Wextra",
                        "-Werror",
                        "-pedantic-errors",
                        "probe.cpp",
                        "-o",
                        "probe",
                    ])
                    .output()
                    .unwrap(),
            );
            for code in [
                "Clock v(std::string(\"x\"));",
                "auto v = Clock::create(\"00:00:00Z\"); (void)v->value_;",
                "auto v = Clock::create(\"00:00:00Z\"); (void)(*v == *v);",
            ] {
                std::fs::write(dir.join("negative.cpp"), format!("#include \"generated.hpp\"\nusing namespace test::time; int main() {{ {code} }}")).unwrap();
                let result = Command::new("c++")
                    .current_dir(&dir)
                    .args(["-std=c++17", "-c", "negative.cpp"])
                    .output()
                    .unwrap();
                assert!(!result.status.success(), "unchecked/equality path compiled");
            }
        }
        "ada" => {
            if Command::new("gnatmake").arg("--version").output().is_err() {
                assert!(
                    std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                    "GNAT required"
                );
                return;
            }
            let mut probe = String::from(
                "with Test.Time; use Test.Time;\nwith Ada.Command_Line;\nprocedure Probe is\nprocedure Valid (Text, Expected : String) is\n V : constant Clock := Create (Text);\nbegin\n if Value (V) /= Expected then raise Program_Error; end if;\nend Valid;\nprocedure Invalid (Text : String) is\nbegin\n declare V : constant Clock := Create (Text); begin Valid (Value(V), Value(V)); end;\n raise Program_Error with \"accepted invalid Time\";\nexception when Constraint_Error => null;\nend Invalid;\nbegin\n",
            );
            for case in &cases {
                let input = common::ada_literal(&case.input);
                match &case.expected {
                    Some(expected) => {
                        writeln!(probe, "Valid ({input}, {});", common::ada_literal(expected))
                            .unwrap()
                    }
                    None => writeln!(probe, "Invalid ({input});").unwrap(),
                }
            }
            probe.push_str("declare\n Slice : String (11 .. 19) := \"12:34:56Z\";\n Edge : String (Positive'Last - 8 .. Positive'Last) := \"12:34:56Z\";\n Empty : String (2 .. 1);\nbegin Valid (Slice, \"12:34:56Z\"); Valid (Edge, \"12:34:56Z\"); Invalid (Empty); end;\nbegin declare Bad : Clock; begin Valid (Value(Bad), \"\"); end; raise Constraint_Error with \"default did not reject\"; exception when Program_Error => null; end;\ndeclare\n V : constant Clock := Create (\"12:34:56Z\");\n B : Payload_Bounded_Sequence;\n U : Payload_Unbounded_Sequence;\nbegin\n if Length(B) /= 0 or else Length(U) /= 0 then raise Program_Error; end if;\n Reserve_Capacity(U,100);\n for I in 1 .. 100 loop Append(U,V); end loop;\n for I in 1 .. 3 loop Append(B,V); end loop;\n declare P : constant Payload := (Required=>V, Optional=>(Is_Present=>True,Value=>V), Bounded=>B, Unbounded=>U, Selection=>(Kind=>Time_Kind,Time=>V), Item=>(Kind=>ConcreteClock_Kind,ConcreteClock_Value=>(Time=>V))); begin\n Valid(Value(P.Required),\"12:34:56Z\"); Valid(Value(P.Optional.Value),\"12:34:56Z\"); Valid(Value(Element(P.Bounded,3)),\"12:34:56Z\"); Valid(Value(Element(P.Unbounded,100)),\"12:34:56Z\"); Valid(Value(P.Selection.Time),\"12:34:56Z\"); Valid(Value(P.Item.ConcreteClock_Value.Time),\"12:34:56Z\"); end;\n Clear(B); Clear(U); if Length(B)/=0 or else Length(U)/=0 then raise Program_Error; end if;\nend;\nif Ada.Command_Line.Argument_Count > 0 then Invalid (\"00:00:00Z\"); end if;\nend Probe;\n");
            for (policy, extra) in [("", ""), ("pragma Assertion_Policy (Ignore);\n", "-gnata")] {
                std::fs::write(dir.join("probe.adb"), format!("{policy}{probe}")).unwrap();
                let mut command = Command::new("gnatmake");
                command
                    .current_dir(&dir)
                    .args(["-q", "-f", "-gnat2022", "-gnatwe", "probe.adb"]);
                if !extra.is_empty() {
                    command.arg(extra);
                }
                checked(command.output().unwrap());
                checked(Command::new(dir.join("probe")).output().unwrap());
                let wrong = Command::new(dir.join("probe"))
                    .arg("sabotage")
                    .output()
                    .unwrap();
                assert!(
                    !wrong.status.success(),
                    "Ada deliberately wrong oracle escaped"
                );
            }
        }
        _ => unreachable!(),
    }
    checked(Command::new(dir.join("probe")).output().unwrap());
    let wrong = Command::new(dir.join("probe"))
        .arg("sabotage")
        .output()
        .unwrap();
    assert!(!wrong.status.success(), "deliberately wrong oracle escaped");
    println!("TASK061 {LANGUAGE} TIME CORPUS/LIFECYCLE/STORAGE: PASSED (sabotage detected)");
}
