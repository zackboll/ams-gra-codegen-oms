//! Task 061 pinned Time evidence. Both roots and execution markers are required
//! by Deep CI; absent roots are a developer skip, never pinned evidence.
use ams_gra_oms_codegen_core::{
    BackendLanguage, CoverageAnalysis, GenerationWorld, analyze_service_readiness,
    effective_choice_alternatives, effective_record_fields, project_service_generation_schema,
    resolve_service_plan,
};
use ams_gra_oms_ir::{PrimitiveKind, SchemaIr, TypeKind, TypeRefTarget};
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::{collections::BTreeSet, path::PathBuf, process::Command};

const ROOTS: [(&str, &str, &str); 2] = [
    (
        "2.5",
        "AMS_GRA_UCI_2_5_ROOT",
        "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27",
    ),
    (
        "2.6",
        "AMS_GRA_UCI_2_6_ROOT",
        "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b",
    ),
];

fn contract(name: &str, version: &str) -> ams_gra_oms_service_contract::Contract {
    ams_gra_oms_service_contract::parse_yaml(&format!(
        "contract_version: \"0.1\"\nservice:\n  name: task061\n  version: \"0.1.0\"\n  kind: service\nstandards:\n  oms_version: \"{version}\"\n  uci_schema_version: \"{version}\"\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: output\n        mandate: mandatory\n        message: {name}\n        topic: t\n        timing:\n          kind: asynchronous\n"
    )).unwrap()
}

fn time_names(schema: &SchemaIr) -> BTreeSet<ams_gra_oms_ir::QualifiedName> {
    schema
        .types
        .iter()
        .filter(|d| matches!(d.kind, TypeKind::Primitive(PrimitiveKind::Time)))
        .map(|d| d.name.clone())
        .collect()
}

fn stores_time(
    d: &ams_gra_oms_ir::TypeDecl,
    names: &BTreeSet<ams_gra_oms_ir::QualifiedName>,
) -> bool {
    let is_time = |r: &ams_gra_oms_ir::TypeRef| match &r.target {
        TypeRefTarget::Primitive(PrimitiveKind::Time) => true,
        TypeRefTarget::Named(n) => names.contains(n),
        _ => false,
    };
    names.contains(&d.name)
        || match &d.kind {
            TypeKind::Record { fields }
            | TypeKind::Choice {
                alternatives: fields,
            } => fields.iter().any(|f| is_time(&f.type_ref)),
            TypeKind::Alias(r) | TypeKind::List { item_type: r, .. } => is_time(r),
            _ => false,
        }
}

#[test]
fn task061_pinned_time_inventory_and_impact() {
    campaign(false);
}

#[test]
fn task061_pinned_time_inventory_and_coverage() {
    campaign(true);
}

#[test]
fn task061_pinned_ada_full_schema_naming_attribution() {
    for (version, variable, digest) in ROOTS {
        let Some(root) = std::env::var_os(variable).map(PathBuf::from) else {
            continue;
        };
        let hash = Command::new("sha256sum").arg(&root).output().unwrap();
        assert_eq!(
            String::from_utf8_lossy(&hash.stdout)
                .split_whitespace()
                .next(),
            Some(digest)
        );
        let schema = load_schema_set(&root).unwrap();
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        let ada = analysis
            .renderable_message_closure_names(BackendLanguage::Ada)
            .unwrap();
        let rust = analysis
            .renderable_message_closure_names(BackendLanguage::Rust)
            .unwrap();
        let cpp = analysis
            .renderable_message_closure_names(BackendLanguage::Cpp)
            .unwrap();
        assert_eq!(rust, cpp);
        // Historical Task061/062 gaps are preserved in their frozen fixtures.
        // Task065 supersedes the current naming attribution assertion.
        assert_eq!(rust, ada);
        let unsafe_ada = ams_gra_oms_codegen_core::unsafe_named_declarations(
            &schema,
            BackendLanguage::Ada,
            GenerationWorld::ClosedSchemaSet,
        );
        assert!(unsafe_ada.is_empty());
        for name in rust.difference(&ada) {
            println!("ADA-GAP\t{version}\t{}", name.local_name);
            let m = schema.messages.iter().find(|m| m.name == *name).unwrap();
            let TypeRefTarget::Named(payload) = &m.payload_type.target else {
                panic!()
            };
            let closure = analysis.dependency_closure(payload).unwrap();
            assert!(
                closure.iter().any(|d| unsafe_ada.contains(&d.name)),
                "{}",
                name.local_name
            );
            if name.local_name == "Task" {
                let unsafe_names: BTreeSet<_> = closure
                    .iter()
                    .filter(|d| unsafe_ada.contains(&d.name))
                    .map(|d| d.name.local_name.as_str())
                    .collect();
                println!("TASK062 TASK ADA UNSAFE\t{version}\t{unsafe_names:?}");
                let plan = resolve_service_plan(&contract("Task", version), &schema).unwrap();
                let r = analyze_service_readiness(
                    &plan,
                    &schema,
                    BackendLanguage::Ada,
                    GenerationWorld::ClosedSchemaSet,
                )
                .unwrap();
                assert!(r.backend_blocker.is_none());
                assert!(!r.is_ready());
                assert_eq!(
                    r.unsupported_generated_support_types
                        .iter()
                        .map(|n| n.local_name.as_str())
                        .collect::<BTreeSet<_>>(),
                    BTreeSet::from([
                        "NITF_DateAndTimeType",
                        "NITF_DateType",
                        "NITF_MSTGTA_TargetLocationType"
                    ])
                );
            }
        }
        println!("UCI {version} TASK061 ADA FULL-SCHEMA NAMING: PASSED");
    }
}

fn campaign(inventory_only: bool) {
    for (version, variable, digest) in ROOTS {
        let Some(root) = std::env::var_os(variable).map(PathBuf::from) else {
            eprintln!("SKIPPED {variable}");
            continue;
        };
        let hash = Command::new("sha256sum").arg(&root).output().unwrap();
        assert!(hash.status.success());
        assert_eq!(
            String::from_utf8_lossy(&hash.stdout)
                .split_whitespace()
                .next(),
            Some(digest)
        );
        let schema = load_schema_set(&root).unwrap();
        let names = time_names(&schema);
        assert_eq!(names.len(), 1);
        let time = schema
            .types
            .iter()
            .find(|d| names.contains(&d.name))
            .unwrap();
        assert_eq!(
            ams_gra_oms_codegen_core::temporal_profile(PrimitiveKind::Time, &time.constraints),
            Ok(Some(ams_gra_oms_codegen_core::TemporalProfile::TimeZulu))
        );
        let expected_inventory =
            include_str!("../../../tests/fixtures/temporal/task061-pinned-inventory.tsv");
        let mut actual_inventory = Vec::new();
        let relative_source = |source: &ams_gra_oms_ir::SourceRef| ams_gra_oms_ir::SourceRef {
            document: std::path::Path::new(&source.document)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            line: source.line,
        };
        for d in &schema.types {
            if names.contains(&d.name) {
                actual_inventory.push(format!(
                    "TIME\t{version}\tNAMED\t{:?}\t{:?}\tbase={:?}\t{:?}",
                    d.name,
                    relative_source(&d.source),
                    d.base_type,
                    d.constraints
                ));
                println!(
                    "TIME\t{version}\tNAMED\t{:?}\t{:?}\tbase={:?}\t{:?}",
                    d.name, d.source, d.base_type, d.constraints
                );
            }
            let fields = match &d.kind {
                TypeKind::Record { fields }
                | TypeKind::Choice {
                    alternatives: fields,
                } => fields,
                _ => continue,
            };
            for f in fields {
                let representation = match &f.type_ref.target {
                    TypeRefTarget::Primitive(PrimitiveKind::Time) => "DIRECT",
                    TypeRefTarget::Named(n) if names.contains(n) => "NAMED",
                    _ => continue,
                };
                actual_inventory.push(format!("TIME\t{version}\t{representation}\t{}.{}\t{:?}\t{:?}\t{:?}\tchoice={}\tnillable={}\t{:?}", d.name.local_name, f.name, relative_source(&f.source), f.type_ref, f.cardinality, matches!(d.kind,TypeKind::Choice{..}),f.nillable,f.constraints));
                println!(
                    "TIME\t{version}\t{representation}\t{}.{}\t{:?}\t{:?}\t{:?}\tchoice={}\tnillable={}\t{:?}",
                    d.name.local_name,
                    f.name,
                    f.source,
                    f.type_ref,
                    f.cardinality,
                    matches!(d.kind, TypeKind::Choice { .. }),
                    f.nillable,
                    f.constraints
                );
            }
            let effective = match &d.kind {
                TypeKind::Record { .. } => effective_record_fields(&schema, &d.name),
                TypeKind::Choice { .. } => effective_choice_alternatives(&schema, &d.name),
                _ => unreachable!(),
            };
            if let Ok(effective) = effective {
                for f in effective {
                    if !fields.contains(f)
                        && matches!(&f.type_ref.target, TypeRefTarget::Named(n) if names.contains(n))
                    {
                        actual_inventory.push(format!(
                            "TIME\t{version}\tINHERITED\t{}.{}\t{:?}\t{:?}",
                            d.name.local_name,
                            f.name,
                            relative_source(&f.source),
                            f.cardinality
                        ));
                        println!(
                            "TIME\t{version}\tINHERITED\t{}.{}\t{:?}\t{:?}",
                            d.name.local_name, f.name, f.source, f.cardinality
                        );
                    }
                }
            }
        }
        assert_eq!(
            actual_inventory,
            expected_inventory
                .lines()
                .filter(|line| line.starts_with(&format!("TIME\t{version}\t")))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        );
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let coverage = CoverageAnalysis::new(&schema, world).unwrap();
            for language in BackendLanguage::ALL {
                println!(
                    "COVERAGE\t{version}\t{world:?}\t{language:?}\t{:?}",
                    coverage.backend_coverage(language).unwrap()
                );
            }
            if inventory_only {
                continue;
            }
            for m in &schema.messages {
                let plan =
                    resolve_service_plan(&contract(&m.name.local_name, version), &schema).unwrap();
                let selected = plan.selected_type_closure(&schema).unwrap();
                let selected_time = selected.iter().any(|d| stores_time(d, &names))
                    || matches!(
                        m.payload_type.target,
                        TypeRefTarget::Primitive(PrimitiveKind::Time)
                    );
                let projection = project_service_generation_schema(&plan, &schema, world);
                let support_time = projection.as_ref().is_ok_and(|p| {
                    p.schema().types.iter().any(|d| {
                        p.generated_support_type_names().contains(&d.name) && stores_time(d, &names)
                    })
                });
                if !selected_time && !support_time {
                    continue;
                }
                println!(
                    "REACH\t{version}\t{world:?}\t{}\tselected={selected_time}\tsupport={support_time}",
                    m.name.local_name
                );
                // An unavailable projected representation is already a
                // fail-closed projection verdict. Do not run readiness's
                // whole-schema diagnostic fallback 3 times per message: it
                // rescans thousands of unrelated declarations in open world.
                // This is evidence harness scheduling, not a capability change.
                if let Err(error) = &projection {
                    for language in BackendLanguage::ALL {
                        println!(
                            "ERROR\t{version}\t{world:?}\t{language:?}\t{}\t{error}",
                            m.name.local_name
                        );
                    }
                    continue;
                }
                for language in BackendLanguage::ALL {
                    match analyze_service_readiness(&plan, &schema, language, world) {
                        Ok(r) => {
                            let expected = include_str!(
                                "../../../tests/fixtures/string/task062-time-impact-current.tsv"
                            )
                            .lines()
                            .find(|line| {
                                line.starts_with(&format!(
                                    "{version}\t{world:?}\t{language:?}\t{}\t",
                                    m.name.local_name
                                ))
                            })
                            .expect("frozen impact row");
                            let row: Vec<_> = expected.split('\t').collect();
                            assert_eq!(r.is_ready(), row[4] == "ready", "{expected}");
                            assert_eq!(r.selected_types_total, row[5].parse::<usize>().unwrap());
                            assert_eq!(
                                r.generated_support_types_total,
                                row[6].parse::<usize>().unwrap()
                            );
                            println!(
                                "SERVICE\t{version}\t{world:?}\t{language:?}\t{}\t{}\t{}\t{}\t{:?}\t{:?}\t{:?}\t{:?}",
                                m.name.local_name,
                                r.is_ready(),
                                r.selected_types_total,
                                r.generated_support_types_total,
                                r.unsupported_types,
                                r.unsupported_generated_support_types,
                                r.backend_blocker,
                                r.service_api_blocker
                            );
                        }
                        Err(e) => println!(
                            "ERROR\t{version}\t{world:?}\t{language:?}\t{}\t{e}",
                            m.name.local_name
                        ),
                    }
                }
            }
        }
        println!("UCI {version} TASK061 TIME INVENTORY AND IMPACT: PASSED");
    }
}

#[test]
fn task061_smallest_real_service_compilers_and_time_codec() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (version, variable, digest) in ROOTS {
        let Some(root) = std::env::var_os(variable).map(PathBuf::from) else {
            continue;
        };
        let hash = Command::new("sha256sum").arg(&root).output().unwrap();
        assert_eq!(
            String::from_utf8_lossy(&hash.stdout)
                .split_whitespace()
                .next(),
            Some(digest)
        );
        let scratch =
            std::env::temp_dir().join(format!("task061-dlz-{version}-{}", std::process::id()));
        std::fs::create_dir_all(&scratch).unwrap();
        let yaml = format!(
            "contract_version: \"0.1\"\nservice:\n  name: task061\n  version: \"0.1.0\"\n  kind: service\nstandards:\n  oms_version: \"{version}\"\n  uci_schema_version: \"{version}\"\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: output\n        mandate: mandatory\n        message: DLZ\n        topic: t\n        timing:\n          kind: asynchronous\n"
        );
        let contract_path = scratch.join("contract.yaml");
        std::fs::write(&contract_path, yaml).unwrap();
        for language in ["ada", "rust", "cpp"] {
            let out = scratch.join(language);
            let common = [
                "--schema",
                root.to_str().unwrap(),
                "--contract",
                contract_path.to_str().unwrap(),
                "--language",
                language,
                "--world",
                "closed-schema",
            ];
            let check = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
                .arg("service-check")
                .args(common)
                .output()
                .unwrap();
            assert!(
                check.status.success(),
                "{}{}",
                String::from_utf8_lossy(&check.stdout),
                String::from_utf8_lossy(&check.stderr)
            );
            let mut generate = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
            generate
                .arg("service-generate")
                .args(common)
                .arg("--output")
                .arg(&out);
            if language == "rust" {
                generate.arg("--with-codec");
            }
            let result = generate.output().unwrap();
            assert!(
                result.status.success(),
                "{}{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            let result = match language {
                "ada" => Command::new("gnatmake")
                    .current_dir(&out)
                    .args(["-c", "-gnat2022", "-gnatwe", "service_api.ads"])
                    .output()
                    .unwrap(),
                "cpp" => {
                    std::fs::write(
                        out.join("probe.cpp"),
                        "#include \"service_api.hpp\"\nint main(){return 0;}\n",
                    )
                    .unwrap();
                    Command::new("c++")
                        .current_dir(&out)
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
                        .unwrap()
                }
                _ => {
                    let vector = std::fs::read_to_string(
                        repo.join("tests/fixtures/oms-json/task061-dlz.json"),
                    )
                    .unwrap()
                    .replace(
                        "002.5.0",
                        &format!("002.{}.0", version.split('.').nth(1).unwrap()),
                    );
                    std::fs::write(out.join("valid.json"), vector).unwrap();
                    std::fs::write(out.join("Cargo.toml"),format!("[package]\nname=\"task061-dlz\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[workspace]\n[[bin]]\nname=\"probe\"\npath=\"probe.rs\"\n[dependencies]\nams-gra-oms-runtime-api={{path={:?}}}\nams-gra-oms-runtime-rust={{path={:?}}}\nserde_json=\"1\"\n",repo.join("crates/runtime-api-rust"),repo.join("crates/runtime-rust"))).unwrap();
                    std::fs::write(out.join("probe.rs"),r#"
#[path="service_api.rs"] pub mod generated;
use ams_gra_oms_runtime_rust::OmsJsonCodec;
use generated::{model::DLZMT,service_codec::ServiceCodec};
fn main() {
 let body:serde_json::Value=serde_json::from_str(include_str!("valid.json")).unwrap();
 let payload:DLZMT=ServiceCodec.decode_payload(&body).unwrap();
 let encoded=ServiceCodec.encode_payload(&payload).unwrap();
 assert_eq!(encoded["MessageData"]["DLZ_Data"]["TimeOfIntercept"],"12:34:56.5000Z");
 let second:DLZMT=ServiceCodec.decode_payload(&encoded).unwrap();
 assert_eq!(ServiceCodec.encode_payload(&second).unwrap(),encoded);
 for bad in [serde_json::json!("12:34:56+00:00"),serde_json::json!("24:00:00.001Z"),serde_json::json!("12:34:61Z"),serde_json::json!(42)] {
  let mut changed=body.clone();changed["MessageData"]["DLZ_Data"]["TimeOfIntercept"]=bad;
  let rejected:Result<DLZMT,_>=ServiceCodec.decode_payload(&changed);assert!(rejected.is_err());
 }
}
"#).unwrap();
                    Command::new("cargo")
                        .current_dir(&out)
                        .args(["run", "--offline", "--quiet"])
                        .env("CARGO_TARGET_DIR", scratch.join("cargo-target"))
                        .env("RUSTFLAGS", "-Dwarnings")
                        .output()
                        .unwrap()
                }
            };
            assert!(
                result.status.success(),
                "{version} {language}: {}{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            if language == "ada" {
                for file in std::fs::read_dir(&out)
                    .unwrap()
                    .map(Result::unwrap)
                    .filter(|file| file.path().extension().is_some_and(|e| e == "adb"))
                {
                    let compiled = Command::new("gnatmake")
                        .current_dir(&out)
                        .args(["-c", "-gnat2022", "-gnatwe"])
                        .arg(file.file_name())
                        .output()
                        .unwrap();
                    assert!(
                        compiled.status.success(),
                        "{}{}",
                        String::from_utf8_lossy(&compiled.stdout),
                        String::from_utf8_lossy(&compiled.stderr)
                    );
                }
            }
        }
        println!("UCI {version} TASK061 DLZ COMPILER CODEC VERTICAL: PASSED");
    }
}
