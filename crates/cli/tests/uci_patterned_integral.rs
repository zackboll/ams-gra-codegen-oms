//! Pinned Task 063 inventory, coverage and production projection/readiness.
use ams_gra_oms_codegen_core::{
    BackendLanguage, CoverageAnalysis, GenerationWorld, analyze_service_readiness,
    effective_choice_alternatives, effective_record_fields, patterned_integral_profile,
    project_service_generation_schema, resolve_service_plan,
};
use ams_gra_oms_ir::{PrimitiveKind, TypeKind, TypeRefTarget};
use std::{path::PathBuf, process::Command};

#[path = "../../../tests/task063_integrated_coverage.rs"]
mod task063_integrated_coverage;
use task063_integrated_coverage::integrated_coverage;

#[test]
fn task063_pinned_inventory_coverage_and_service_impact() {
    for (release, variable, digest) in [
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
    ] {
        let Some(root) = std::env::var_os(variable).map(PathBuf::from) else {
            eprintln!("SKIPPED {variable}: not pinned evidence");
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
        let schema = ams_gra_oms_xsd_frontend::load_schema_set(&root).unwrap();
        let integral = |d: &&ams_gra_oms_ir::TypeDecl| {
            matches!(
                d.kind,
                TypeKind::Primitive(PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger)
            )
        };
        let patterned: Vec<_> = schema
            .types
            .iter()
            .filter(integral)
            .filter(|d| !d.constraints.lexical.pattern_groups.is_empty())
            .collect();
        assert_eq!(patterned.len(), usize::from(release == "2.5"));
        let serial = schema
            .types
            .iter()
            .find(|d| d.name.local_name == "USMTF_SerialNumberOfQualifierType")
            .unwrap();
        assert_eq!(
            serial.kind,
            TypeKind::Primitive(PrimitiveKind::SignedInteger)
        );
        assert_eq!(
            serial.constraints.min_inclusive,
            Some(ams_gra_oms_ir::NumericValue::Integer(1))
        );
        assert_eq!(
            serial.constraints.max_inclusive,
            Some(ams_gra_oms_ir::NumericValue::Integer(999))
        );
        assert_eq!(
            serial.base_type,
            Some(ams_gra_oms_ir::TypeRef::primitive(
                PrimitiveKind::SignedInteger
            ))
        );
        assert_eq!(
            serial.source.line,
            Some(if release == "2.5" { 145855 } else { 146103 })
        );
        assert_eq!(serial.constraints.min_exclusive, None);
        assert_eq!(serial.constraints.max_exclusive, None);
        assert_eq!(serial.constraints.length, None);
        assert_eq!(serial.constraints.min_length, None);
        assert_eq!(serial.constraints.max_length, None);
        assert_eq!(serial.constraints.lexical.white_space, None);
        assert_eq!(
            serial
                .constraints
                .lexical
                .effective_white_space(PrimitiveKind::SignedInteger),
            ams_gra_oms_ir::WhiteSpacePolicy::Collapse
        );
        assert_eq!(
            patterned_integral_profile(PrimitiveKind::SignedInteger, &serial.constraints).is_some(),
            release == "2.5"
        );
        println!(
            "INVENTORY\t{release}\t{:?}\tbase={:?}\tconstraints={:?}",
            serial.source, serial.base_type, serial.constraints
        );
        for d in &schema.types {
            let fields = match &d.kind {
                TypeKind::Record { fields }
                | TypeKind::Choice {
                    alternatives: fields,
                } => fields.as_slice(),
                _ => &[],
            };
            for f in fields {
                if matches!(&f.type_ref.target, TypeRefTarget::Named(n) if n == &serial.name) {
                    println!(
                        "REFERENCE\t{release}\t{}.{}\t{:?}\t{:?}\t{:?}",
                        d.name.local_name, f.name, f.source, f.cardinality, d.kind
                    );
                }
            }
            let effective = match &d.kind {
                TypeKind::Record { .. } => effective_record_fields(&schema, &d.name).unwrap(),
                TypeKind::Choice { .. } => effective_choice_alternatives(&schema, &d.name).unwrap(),
                _ => vec![],
            };
            for f in effective {
                if matches!(&f.type_ref.target, TypeRefTarget::Named(n) if n == &serial.name) {
                    println!(
                        "EFFECTIVE_REFERENCE\t{release}\t{}.{}\t{:?}\t{:?}",
                        d.name.local_name, f.name, f.source, f.cardinality
                    );
                }
            }
        }
        let yaml = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/integral/patterned.yaml"),
        )
        .unwrap();
        let mut reaching = Vec::new();
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let coverage = CoverageAnalysis::new(&schema, world).unwrap();
            for lang in BackendLanguage::ALL {
                let actual = coverage.backend_coverage(lang).unwrap();
                let backend_key = match lang {
                    BackendLanguage::Ada => "Ada",
                    BackendLanguage::Rust => "Rust",
                    BackendLanguage::Cpp => "Cpp",
                };
                let expected = integrated_coverage(release, world, backend_key);
                assert_eq!(
                    vec![
                        actual.declaration_kinds_renderable,
                        actual.declarations_fully_renderable,
                        actual.field_type_references_renderable,
                        actual.field_occurrences_renderable,
                        actual.message_closures_renderable
                    ],
                    expected
                );
                println!("COVERAGE\t{release}\t{world:?}\t{lang:?}\t{:?}", actual);
            }
            for message in &schema.messages {
                let contract = ams_gra_oms_service_contract::parse_yaml(
                    &yaml
                        .replace(
                            "message: Message",
                            &format!("message: {}", message.name.local_name),
                        )
                        .replace("\"2.5\"", &format!("\"{release}\"")),
                )
                .unwrap();
                let plan = resolve_service_plan(&contract, &schema).unwrap();
                let selected = plan.selected_type_closure(&schema).unwrap();
                let selected_reach = selected.iter().any(|d| d.name == serial.name);
                let projection = project_service_generation_schema(&plan, &schema, world);
                let support_reach = projection
                    .as_ref()
                    .is_ok_and(|p| p.generated_support_type_names().contains(&serial.name));
                if !selected_reach && !support_reach {
                    continue;
                }
                reaching.push((
                    world,
                    message.name.local_name.clone(),
                    selected_reach,
                    support_reach,
                ));
                let Ok(projection) = projection else {
                    println!(
                        "PROJECTION_FAILURE\t{release}\t{world:?}\t{}\t{:?}",
                        message.name.local_name,
                        projection.unwrap_err()
                    );
                    continue;
                };
                for lang in BackendLanguage::ALL {
                    let r = analyze_service_readiness(&plan, &schema, lang, world).unwrap();
                    assert!(
                        r.is_ready(),
                        "{release} {world:?} {lang:?} {}: {r:?}",
                        message.name.local_name
                    );
                    println!(
                        "SERVICE\t{release}\t{world:?}\t{lang:?}\t{}\tselected={}\tsupport={}\tready={}\tcost={}\t{:?}",
                        message.name.local_name,
                        selected_reach,
                        support_reach,
                        r.is_ready(),
                        projection.selected_type_names().len()
                            + projection.generated_support_type_names().len(),
                        r
                    );
                }
            }
        }
        assert_eq!(
            reaching,
            vec![
                (
                    GenerationWorld::ClosedSchemaSet,
                    "OrdersMetadata".into(),
                    true,
                    false
                ),
                (
                    GenerationWorld::ClosedSchemaSet,
                    "PrioritizationList".into(),
                    true,
                    false
                ),
                (
                    GenerationWorld::OpenExtensions,
                    "OrdersMetadata".into(),
                    true,
                    false
                ),
                (
                    GenerationWorld::OpenExtensions,
                    "PrioritizationList".into(),
                    true,
                    false
                ),
            ]
        );
        println!("UCI {release} TASK063 PINNED INVENTORY AND IMPACT: PASSED");
    }
}

#[test]
fn task063_smallest_real_service_compiler_codec_vertical() {
    fn success(output: std::process::Output) {
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (release, variable) in [
        ("2.5", "AMS_GRA_UCI_2_5_ROOT"),
        ("2.6", "AMS_GRA_UCI_2_6_ROOT"),
    ] {
        let Some(root) = std::env::var_os(variable).map(PathBuf::from) else {
            eprintln!("SKIPPED {variable}: not pinned evidence");
            continue;
        };
        let digest = if release == "2.5" {
            "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27"
        } else {
            "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b"
        };
        let hash = Command::new("sha256sum").arg(&root).output().unwrap();
        assert!(hash.status.success());
        assert_eq!(
            String::from_utf8_lossy(&hash.stdout)
                .split_whitespace()
                .next(),
            Some(digest)
        );
        let scratch =
            std::env::temp_dir().join(format!("task063-orders-{release}-{}", std::process::id()));
        std::fs::create_dir_all(&scratch).unwrap();
        let contract = scratch.join("contract.yaml");
        let yaml =
            std::fs::read_to_string(repo.join("tests/fixtures/integral/patterned.yaml")).unwrap();
        std::fs::write(
            &contract,
            yaml.replace("message: Message", "message: OrdersMetadata")
                .replace("\"2.5\"", &format!("\"{release}\"")),
        )
        .unwrap();
        for lang in ["ada", "rust", "cpp"] {
            let out = scratch.join(lang);
            let mut check = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
            check
                .arg("service-check")
                .arg("--schema")
                .arg(&root)
                .arg("--contract")
                .arg(&contract)
                .args(["--language", lang, "--world", "closed-schema"]);
            if lang == "rust" {
                check.arg("--with-codec");
            }
            success(check.output().unwrap());
            let mut generate = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
            generate
                .arg("service-generate")
                .arg("--schema")
                .arg(&root)
                .arg("--contract")
                .arg(&contract)
                .args(["--language", lang, "--world", "closed-schema"])
                .arg("--output")
                .arg(&out);
            if lang == "rust" {
                generate.arg("--with-codec");
            }
            success(generate.output().unwrap());
            if release == "2.6" {
                for row in include_str!(
                    "../../../tests/fixtures/integral/task063-uci26-source-digests.tsv"
                )
                .lines()
                .filter(|line| line.starts_with(&format!("{lang}\t")))
                {
                    let parts: Vec<_> = row.split('\t').collect();
                    let hash = Command::new("sha256sum")
                        .arg(out.join(parts[1]))
                        .output()
                        .unwrap();
                    assert!(hash.status.success());
                    assert_eq!(
                        String::from_utf8_lossy(&hash.stdout)
                            .split_whitespace()
                            .next(),
                        Some(parts[2]),
                        "UCI2.6 source changed: {}",
                        parts[1]
                    );
                }
            }
            match lang {
                "ada" => {
                    success(
                        Command::new("gnatmake")
                            .current_dir(&out)
                            .args(["-c", "-gnat2022", "-gnatwe", "service_api.ads"])
                            .output()
                            .unwrap(),
                    );
                    for file in std::fs::read_dir(&out)
                        .unwrap()
                        .map(Result::unwrap)
                        .filter(|f| f.path().extension().is_some_and(|e| e == "adb"))
                    {
                        success(
                            Command::new("gnatmake")
                                .current_dir(&out)
                                .args(["-c", "-gnat2022", "-gnatwe"])
                                .arg(file.path())
                                .output()
                                .unwrap(),
                        );
                    }
                }
                "cpp" => {
                    std::fs::write(
                        out.join("probe.cpp"),
                        "#include \"service_api.hpp\"\nint main(){return 0;}\n",
                    )
                    .unwrap();
                    success(
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
                            .unwrap(),
                    );
                }
                "rust" => {
                    std::fs::copy(
                        repo.join("tests/fixtures/oms-json/task063-orders-metadata.json"),
                        out.join("valid.json"),
                    )
                    .unwrap();
                    std::fs::write(out.join("Cargo.toml"), format!("[package]\nname=\"task063-orders\"\nversion=\"0.0.0\"\nedition=\"2024\"\n[[bin]]\nname=\"probe\"\npath=\"probe.rs\"\n[dependencies]\nserde_json=\"1\"\nams-gra-oms-runtime-api={{path={:?}}}\nams-gra-oms-runtime-rust={{path={:?}}}\n",repo.join("crates/runtime-api-rust"),repo.join("crates/runtime-rust"))).unwrap();
                    std::fs::write(out.join("probe.rs"), r#"
#[path="service_api.rs"] pub mod generated;
use ams_gra_oms_runtime_rust::OmsJsonCodec;
use generated::{model::OrdersMetadataMT,service_codec::ServiceCodec};
fn main() {
 let base:serde_json::Value=serde_json::from_str(include_str!("valid.json")).unwrap();
 for n in [1,999] {
  let mut body=base.clone(); body["MessageData"]["OrdersAssociation"][0]["Orders"]["ATO"][0]["Identifier"]["QualifierSerialNumber"]=serde_json::json!(n);
  let value:OrdersMetadataMT=ServiceCodec.decode_payload(&body).unwrap();
  let encoded=ServiceCodec.encode_payload(&value).unwrap(); assert_eq!(encoded,body);
  let second:OrdersMetadataMT=ServiceCodec.decode_payload(&encoded).unwrap(); assert_eq!(ServiceCodec.encode_payload(&second).unwrap(),encoded);
 }
 for text in ["0","1000","-1","1.0","1e0","-0"] {
  let mut body=base.clone();body["MessageData"]["OrdersAssociation"][0]["Orders"]["ATO"][0]["Identifier"]["QualifierSerialNumber"]=serde_json::from_str(text).unwrap();
  let rejected:Result<OrdersMetadataMT,_>=ServiceCodec.decode_payload(&body);assert!(rejected.is_err(),"{text}");
 }
}
"#).unwrap();
                    success(
                        Command::new("cargo")
                            .current_dir(&out)
                            .args(["run", "--offline", "--quiet"])
                            .env("CARGO_TARGET_DIR", scratch.join("cargo-target"))
                            .env("RUSTFLAGS", "-Dwarnings")
                            .output()
                            .unwrap(),
                    );
                }
                _ => unreachable!(),
            }
            println!("UCI {release} TASK063 {lang} ORDERS METADATA VERTICAL: PASSED");
        }
        println!("UCI {release} TASK063 SMALLEST SERVICE VERTICAL: PASSED");
    }
}
