//! One independent frozen corpus executed against all production renderers.
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::{path::PathBuf, process::Command};

fn run(command: &mut Command) {
    let r = command.output().unwrap();
    assert!(
        r.status.success(),
        "{:?}\n{}{}",
        command,
        String::from_utf8_lossy(&r.stdout),
        String::from_utf8_lossy(&r.stderr)
    );
}

#[test]
fn task066_production_compiler_corpus_lifecycle_and_planted_failure() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema =
        load_schema_set(&repo.join("tests/fixtures/service-generate/codec-unicode31.xsd")).unwrap();
    let scratch = std::env::temp_dir().join(format!("task066-compiler-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    let rows: Vec<_> = include_str!("../../../tests/fixtures/string/task066-corpus.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let c: Vec<_> = l.split('\t').collect();
            let bytes: Vec<_> = c[2]
                .as_bytes()
                .chunks(2)
                .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
                .collect();
            (c[0].parse::<usize>().unwrap(), c[1] == "1", bytes)
        })
        .collect();
    let rust =
        ams_gra_oms_backend_rust::generate(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
    let cpp = ams_gra_oms_backend_cpp::generate(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
    let ada = ams_gra_oms_backend_ada::generate(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
    let body = ams_gra_oms_backend_ada::generate_body(&schema, GenerationWorld::ClosedSchemaSet)
        .unwrap()
        .unwrap();
    assert_eq!(rust.matches("struct Unicode31StringValidator;").count(), 1);
    assert_eq!(cpp.matches("class Unicode31StringValidator {").count(), 1);
    assert_eq!(body.matches("function Unicode31_String_Valid").count(), 1);
    assert!(!rust.contains("impl Default for StampValue"));
    assert!(!rust.contains("pub value: String"));
    assert!(ada.contains("type StampValue is private;"));
    assert!(ada.contains("raise Standard.Program_Error"));
    assert!(cpp.contains("private:\n    explicit StampValue"));
    for (language, source) in [("rust", &rust), ("cpp", &cpp), ("ada", &ada)] {
        println!(
            "SIZE\t{language}\tbytes={}\tlines={}",
            source.len(),
            source.lines().count()
        );
    }
    println!(
        "SIZE\tada-body\tbytes={}\tlines={}",
        body.len(),
        body.lines().count()
    );
    let names = ["StampValue", "DayValue", "LocationValue"];
    for language in ["rust", "cpp", "ada"] {
        if language == "ada" && Command::new("gnatmake").arg("--version").output().is_err() {
            assert!(
                std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                "GNAT required"
            );
            continue;
        }
        let out = scratch.join(language);
        std::fs::create_dir_all(&out).unwrap();
        if language == "cpp" {
            std::fs::write(out.join("model.hpp"), &cpp).unwrap();
        }
        if language == "ada" {
            std::fs::write(
                out.join("programs.ads"),
                "package Programs is end Programs;\n",
            )
            .unwrap();
            std::fs::write(out.join("programs-oam.ads"), &ada).unwrap();
            std::fs::write(out.join("programs-oam.adb"), &body).unwrap();
        }
        for planted in [true, false] {
            let mut probe=match language {
                "rust"=>format!("{rust}\nfn main() {{\n"),
                "cpp" => "#include \"model.hpp\"\n#include <cstdlib>\n#include <type_traits>\nstatic_assert(!std::is_default_constructible_v<programs::oam::StampValue>);\nint main() {\n".into(),
                _ => "with Programs.Oam; use Programs.Oam;\nprocedure Probe is\n procedure Check (Id : Natural; Text : String; Expected : Boolean) is\n  Accepted : Boolean := True;\n begin\n  begin\n   case Id is\n    when 0 => declare V : constant StampValue := Create(Text); begin if Value(V) /= Text then raise Program_Error; end if; end;\n    when 1 => declare V : constant DayValue := Create(Text); begin if Value(V) /= Text then raise Program_Error; end if; end;\n    when others => declare V : constant LocationValue := Create(Text); begin if Value(V) /= Text then raise Program_Error; end if; end;\n   end case;\n  exception when Constraint_Error => Accepted := False; end;\n  if Accepted /= Expected then raise Program_Error with \"corpus disagreement\"; end if;\n end Check;\nbegin\n".into(),
            };
            for (i, (id, expected, bytes)) in rows.iter().enumerate() {
                let expected = if planted && i == 0 {
                    !*expected
                } else {
                    *expected
                };
                let name = names[*id];
                match language {
                    "rust" => {
                        let bytes = bytes
                            .iter()
                            .map(u8::to_string)
                            .collect::<Vec<_>>()
                            .join(",");
                        probe.push_str(&format!("let b: &[u8] = std::hint::black_box(&[{bytes}]); let v=std::str::from_utf8(b).ok().and_then({name}::new); assert_eq!(v.is_some(), {expected}); if let Some(v)=v {{ assert_eq!(v.as_str().as_bytes(),b); assert_eq!(v.clone().as_str(),v.as_str()); }}\n"));
                    }
                    "cpp" => {
                        let text = bytes
                            .iter()
                            .map(|b| format!("\\x{b:02X}"))
                            .collect::<String>();
                        probe.push_str(&format!("{{ const std::string text(\"{text}\", {}); const auto v=programs::oam::{name}::create(text); if(v.has_value() != {expected}) return 1; if(v) {{ const auto copy=*v; if(copy.value()!=text) return 2; }} }}\n",bytes.len()));
                    }
                    _ => {
                        let text = if bytes.is_empty() {
                            "\"\"".into()
                        } else {
                            bytes
                                .iter()
                                .map(|b| format!("Character'Val ({b})"))
                                .collect::<Vec<_>>()
                                .join(" & ")
                        };
                        let expected = if expected { "True" } else { "False" };
                        probe.push_str(&format!(" Check ({id}, {text}, {expected});\n"));
                    }
                }
            }
            if language == "ada" {
                probe.push_str(" declare\n  Tail : String (Positive'Last - 21 .. Positive'Last) := \"+12.123456+012.12345\" & Character'Val(217) & Character'Val(161);\n  Wide_Tail : String (Positive'Last - 23 .. Positive'Last) := \"+12.123456+012.12345\" & Character'Val(240) & Character'Val(157) & Character'Val(159) & Character'Val(142);\n begin Check (2, Tail, True); Check (2, Wide_Tail, True); end;\n");
                // Every byte offset is legal even when final absolute index is
                // Positive'Last. No assertion policy is needed for safety.
                probe.push_str(" declare\n  Valid : String (Positive'Last - 8 .. Positive'Last) := \"2\" & Character'Val(217) & Character'Val(161) & \"000101\";\n  Shifted : String (7 .. 15) := Valid;\n  Bad : String (Positive'Last - 7 .. Positive'Last) := \"2000010\" & Character'Val(194);\n begin Check (1, Valid, True); Check (1, Shifted, True); Check (1, Bad, False); Check (1, \"\", False); end;\n declare\n  Rejected : Boolean := False;\n begin\n  begin declare V : DayValue; begin Check (1, Value(V), True); end; exception when Program_Error => Rejected := True; end;\n  if not Rejected then raise Program_Error; end if;\n end;\nend Probe;\n");
            } else {
                probe.push_str("}\n");
            }
            let file = match language {
                "rust" => "probe.rs",
                "cpp" => "probe.cpp",
                _ => "probe.adb",
            };
            std::fs::write(out.join(file), &probe).unwrap();
            let policies = if language == "ada" {
                vec!["check", "ignore"]
            } else {
                vec!["check"]
            };
            for policy in policies {
                match language {
                    "rust" => run(Command::new("rustc").current_dir(&out).args([
                        "--edition=2024",
                        "-Dwarnings",
                        "probe.rs",
                        "-o",
                        "probe",
                    ])),
                    "cpp" => run(Command::new("g++").current_dir(&out).args([
                        "-std=c++17",
                        "-Wall",
                        "-Wextra",
                        "-Werror",
                        "-pedantic-errors",
                        "probe.cpp",
                        "-o",
                        "probe",
                    ])),
                    _ => {
                        std::fs::write(
                            out.join("policy.adc"),
                            if policy == "check" {
                                "pragma Assertion_Policy (Check);\n"
                            } else {
                                "pragma Assertion_Policy (Ignore);\n"
                            },
                        )
                        .unwrap();
                        run(Command::new("gnatmake").current_dir(&out).args([
                            "-q",
                            "-f",
                            "-gnat2022",
                            "-gnatwe",
                            "-gnato",
                            "-gnatec=policy.adc",
                            "probe.adb",
                        ]));
                    }
                }
                let result = Command::new(out.join("probe"))
                    .current_dir(&out)
                    .output()
                    .unwrap();
                assert_eq!(
                    result.status.success(),
                    !planted,
                    "{language} {policy}: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
                println!(
                    "TASK066 {language} {policy} {}: VERIFIED",
                    if planted {
                        "PLANTED FAILURE"
                    } else {
                        "CORPUS PASS"
                    }
                );
            }
        }
    }
    println!(
        "TASK066 PRODUCTION COMPILER CORPUS: PASSED ({} cases)",
        rows.len()
    );
}

#[test]
fn task066_generated_support_neighbor_fails_closed_everywhere() {
    use ams_gra_oms_codegen_core::{
        BackendLanguage, GeneratedSupportChange, PlanBindingMismatch, ServiceReadinessError,
        analyze_service_codec, analyze_service_readiness, build_service_api_model,
        project_service_generation_schema, resolve_service_plan,
    };
    use ams_gra_oms_ir::{QualifiedName, TypeKind, TypeRef};
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut schema =
        load_schema_set(&repo.join("tests/fixtures/service-generate/codec-unicode31.xsd")).unwrap();
    let namespace = "https://www.vdl.afrl.af.mil/programs/oam";
    let payload = schema
        .types
        .iter()
        .find(|d| d.name.local_name == "UnicodePayload")
        .unwrap()
        .clone();
    let mut base = payload.clone();
    base.name = QualifiedName::new(namespace, "Base");
    base.is_abstract = true;
    base.kind = TypeKind::Record { fields: vec![] };
    let mut child = payload.clone();
    child.name = QualifiedName::new(namespace, "Concrete");
    child.base_type = Some(TypeRef::named(base.name.clone()));
    let TypeKind::Record { mut fields } = payload.kind.clone() else {
        panic!("record")
    };
    fields.truncate(1);
    fields[0].name = "Item".into();
    fields[0].type_ref = TypeRef::named(base.name.clone());
    let mut holder = payload;
    holder.name = QualifiedName::new(namespace, "Holder");
    holder.kind = TypeKind::Record { fields };
    schema.messages[0].payload_type = TypeRef::named(holder.name.clone());
    schema.types.extend([base, child, holder]);
    let contract = ams_gra_oms_service_contract::parse_yaml(
        &std::fs::read_to_string(repo.join("tests/fixtures/service-generate/codec-unicode31.yaml"))
            .unwrap(),
    )
    .unwrap();
    let plan = resolve_service_plan(&contract, &schema).unwrap();
    for language in BackendLanguage::ALL {
        assert!(
            analyze_service_readiness(&plan, &schema, language, GenerationWorld::ClosedSchemaSet)
                .unwrap()
                .is_ready()
        );
    }
    let projection =
        project_service_generation_schema(&plan, &schema, GenerationWorld::ClosedSchemaSet)
            .unwrap();
    let api = build_service_api_model(&plan, projection.schema(), GenerationWorld::ClosedSchemaSet)
        .unwrap();
    assert!(
        analyze_service_codec(
            &api,
            projection.schema(),
            BackendLanguage::Rust,
            GenerationWorld::ClosedSchemaSet
        )
        .unwrap()
        .is_ready()
    );
    schema
        .types
        .iter_mut()
        .find(|d| d.name.local_name == "DayValue")
        .unwrap()
        .constraints
        .length = Some(9);
    // Task064 binds generated support: a stale plan must fail before classification.
    for language in BackendLanguage::ALL {
        assert!(matches!(
            analyze_service_readiness(&plan, &schema, language, GenerationWorld::ClosedSchemaSet),
            Err(ServiceReadinessError::PlanBinding(PlanBindingMismatch::GeneratedSupport {
                name,
                change: GeneratedSupportChange::Changed,
            })) if name.local_name == "DayValue"
        ));
    }
    // A newly resolved plan still proves Task066's neighboring facets fail closed.
    let plan = resolve_service_plan(&contract, &schema).unwrap();
    for language in BackendLanguage::ALL {
        let r =
            analyze_service_readiness(&plan, &schema, language, GenerationWorld::ClosedSchemaSet)
                .unwrap();
        assert!(!r.is_ready());
        assert!(r.unsupported_types.is_empty());
        assert!(
            r.unsupported_generated_support_types
                .iter()
                .any(|n| n.local_name == "DayValue")
        );
    }
    let projection =
        project_service_generation_schema(&plan, &schema, GenerationWorld::ClosedSchemaSet)
            .unwrap();
    let api = build_service_api_model(&plan, projection.schema(), GenerationWorld::ClosedSchemaSet)
        .unwrap();
    assert!(
        !analyze_service_codec(
            &api,
            projection.schema(),
            BackendLanguage::Rust,
            GenerationWorld::ClosedSchemaSet
        )
        .unwrap()
        .is_ready()
    );
    println!("TASK066 GENERATED SUPPORT FAIL-CLOSED: PASSED");
}
