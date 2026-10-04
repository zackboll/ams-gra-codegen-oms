//! Task 060: pinned UCI structured ASCII evidence (Deep CI only).
use ams_gra_oms_codegen_core::{
    BackendLanguage, CoverageAnalysis, GenerationWorld, StringProfile, analyze_service_readiness,
    resolve_service_plan, string_profile,
};
use ams_gra_oms_ir::{PrimitiveKind, SchemaIr, TypeKind};
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
fn root(variable: &str, digest: &str) -> Option<(PathBuf, SchemaIr)> {
    let path = PathBuf::from(std::env::var_os(variable)?);
    let hash = Command::new("sha256sum").arg(&path).output().unwrap();
    assert_eq!(
        String::from_utf8_lossy(&hash.stdout)
            .split_whitespace()
            .next(),
        Some(digest)
    );
    let schema = load_schema_set(&path).unwrap();
    Some((path, schema))
}
fn member(d: &ams_gra_oms_ir::TypeDecl) -> bool {
    matches!(d.kind, TypeKind::Primitive(PrimitiveKind::String))
        && matches!(
            string_profile(PrimitiveKind::String, &d.constraints),
            Ok(Some(StringProfile::AlternatingAscii(_)))
        )
}
fn contract(name: &str, version: &str) -> ams_gra_oms_service_contract::Contract {
    let yaml = format!(
        "contract_version: \"0.1\"\nservice:\n  name: task060\n  version: \"0.1.0\"\n  kind: service\nstandards:\n  oms_version: \"{version}\"\n  uci_schema_version: \"{version}\"\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: output\n        mandate: mandatory\n        message: {name}\n        topic: t\n        timing:\n          kind: asynchronous\n"
    );
    ams_gra_oms_service_contract::parse_yaml(&yaml).unwrap()
}
#[test]
fn task060_real_uci_after_coverage_and_order_of_battle() {
    for (version, var, digest) in ROOTS {
        let Some((_, schema)) = root(var, digest) else {
            eprintln!("SKIPPED {var}");
            continue;
        };
        assert_eq!(schema.types.iter().filter(|d| member(d)).count(), 15);
        let deferred: BTreeSet<_> = schema
            .types
            .iter()
            .filter(|d| {
                matches!(d.kind, TypeKind::Primitive(PrimitiveKind::String))
                    && string_profile(PrimitiveKind::String, &d.constraints).is_err()
            })
            .map(|d| d.name.local_name.as_str())
            .collect();
        assert_eq!(
            deferred,
            BTreeSet::from([
                "IPv6_AddressType",
                "NITF_DateAndTimeType",
                "NITF_DateType",
                "NITF_MSTGTA_TargetLocationType"
            ])
        );
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let analysis = CoverageAnalysis::new(&schema, world).unwrap();
            for language in BackendLanguage::ALL {
                println!(
                    "UCI {version} TASK060 AFTER COVERAGE {world:?} {language:?}: {:?}",
                    analysis.backend_coverage(language).unwrap()
                );
            }
        }
        let plan = resolve_service_plan(&contract("OrderOfBattle", version), &schema).unwrap();
        for language in BackendLanguage::ALL {
            let ready = analyze_service_readiness(
                &plan,
                &schema,
                language,
                GenerationWorld::ClosedSchemaSet,
            )
            .unwrap();
            println!("UCI {version} TASK060 OOB {language:?}: {ready:?}");
            assert_eq!(ready.generated_support_types_total, 442);
            assert_eq!(ready.generated_support_types_renderable, 442);
            assert!(ready.unsupported_generated_support_types.is_empty());
            assert!(ready.is_ready());
        }
    }
}

/// Uses the original parent/final measured reaching set, including support-only
/// reach. One load per release; production readiness handles projected naming
/// and topology independently from full-schema closure coverage.
#[test]
fn task060_real_projected_message_impact_and_naming() {
    // Historical Task060 rows remain frozen; Task061 changes current readiness.
    let rows = include_str!("../../../tests/fixtures/temporal/task061-task060-subset-current.tsv");
    for (version, var, digest) in ROOTS {
        let Some((_, schema)) = root(var, digest) else {
            continue;
        };
        let coverage = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        let ada = coverage
            .renderable_message_closure_names(BackendLanguage::Ada)
            .unwrap();
        for language in [BackendLanguage::Rust, BackendLanguage::Cpp] {
            let other = coverage.renderable_message_closure_names(language).unwrap();
            assert_eq!(
                other
                    .difference(&ada)
                    .map(|n| n.local_name.as_str())
                    .collect::<BTreeSet<_>>(),
                include_str!("../../../tests/fixtures/temporal/task061-ada-full-schema-gap.tsv")
                    .lines()
                    .filter_map(|line| line.strip_prefix(&format!("{version}\t")))
                    .collect()
            );
        }
        let mut ready = 0;
        let mut topology = 0;
        let mut blocked = std::collections::BTreeMap::new();
        for line in rows.lines().filter(|line| !line.starts_with('#')) {
            let row: Vec<_> = line.split('\t').collect();
            if row[0] != version {
                continue;
            }
            let plan = resolve_service_plan(&contract(row[1], version), &schema).unwrap();
            for language in BackendLanguage::ALL {
                let result = analyze_service_readiness(
                    &plan,
                    &schema,
                    language,
                    GenerationWorld::ClosedSchemaSet,
                );
                if row[2] == "topology" {
                    assert!(
                        result
                            .unwrap_err()
                            .to_string()
                            .contains("cyclic generated value dependencies")
                    );
                    continue;
                }
                let r = result.unwrap();
                assert_eq!(r.selected_types_total, row[3].parse::<usize>().unwrap());
                assert_eq!(
                    r.generated_support_types_total,
                    row[4].parse::<usize>().unwrap()
                );
                assert_eq!(
                    r.is_ready(),
                    row[2] == "ready"
                        || (version == "2.5"
                            && row[1] == "PrioritizationList"
                            && row[2] == "USMTF_SerialNumberOfQualifierType"),
                    "{version} {} {language:?}",
                    row[1]
                );
                if version == "2.5" && row[1] == "PrioritizationList" {
                    // Keep historical Task 060 counts/ledger intact. The
                    // exact Task 063 gain has its own frozen nine-tuple gate.
                    assert_eq!(row[2], "USMTF_SerialNumberOfQualifierType");
                    assert!(r.is_ready());
                } else if !r.is_ready() {
                    let first = r
                        .unsupported_types
                        .first()
                        .or_else(|| r.unsupported_generated_support_types.first())
                        .unwrap();
                    assert_eq!(first.local_name, row[2]);
                }
            }
            match row[2] {
                "ready" => ready += 1,
                "topology" => topology += 1,
                blocker => *blocked.entry(blocker).or_insert(0) += 1,
            }
        }
        assert_eq!(ready, if version == "2.5" { 80 } else { 81 });
        assert_eq!(topology, 6);
        let mut expected = std::collections::BTreeMap::from([
            ("IPv6_AddressType", 11),
            ("NITF_DateAndTimeType", 5),
        ]);
        if version == "2.5" {
            expected.insert("USMTF_SerialNumberOfQualifierType", 1);
        }
        assert_eq!(blocked, expected);
        println!("\nUCI {version} TASK060 PROJECTED MESSAGE IMPACT: PASSED");
    }
}

#[test]
fn task060_real_service_compiler_and_codec_verticals() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (version, variable, digest) in ROOTS {
        let Some(path) = std::env::var_os(variable).map(PathBuf::from) else {
            continue;
        };
        let hash = Command::new("sha256sum").arg(&path).output().unwrap();
        assert_eq!(
            String::from_utf8_lossy(&hash.stdout)
                .split_whitespace()
                .next(),
            Some(digest)
        );
        for (name, fixture, payload, lexical_path, spelling) in [
            (
                "OrderOfBattle",
                "task060-oob.json",
                "OrderOfBattleMT",
                "/MessageData/ManagedLists/Records/0/RecordData/Identity/Emitter/EmitterCategory/Communications/CENOT_Identifier",
                "UNKN",
            ),
            (
                "SMTI_SettingsCommand",
                "task060-smallest.json",
                "SMTISettingsCommandMT",
                "/MessageData/DefaultPackingPlan/ClassificationSystem",
                "US",
            ),
        ] {
            let scratch = std::env::temp_dir().join(format!(
                "task060-real-{}-{version}-{name}",
                std::process::id()
            ));
            std::fs::create_dir_all(&scratch).unwrap();
            let contract_path = scratch.join("contract.yaml");
            // The normal CLI contract path must be exercised as well as APIs.
            let yaml = format!(
                "contract_version: \"0.1\"\nservice:\n  name: task060\n  version: \"0.1.0\"\n  kind: service\nstandards:\n  oms_version: \"{version}\"\n  uci_schema_version: \"{version}\"\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: output\n        mandate: mandatory\n        message: {name}\n        topic: t\n        timing:\n          kind: asynchronous\n"
            );
            std::fs::write(&contract_path, yaml).unwrap();
            for language in ["ada", "rust", "cpp"] {
                let out = scratch.join(language);
                let common = [
                    "--schema",
                    path.to_str().unwrap(),
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
                    "{}",
                    String::from_utf8_lossy(&check.stdout)
                );
                let mut command = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
                command
                    .arg("service-generate")
                    .args(common)
                    .arg("--output")
                    .arg(&out);
                if language == "rust" {
                    command.arg("--with-codec");
                }
                let generated = command.output().unwrap();
                assert!(
                    generated.status.success(),
                    "{} {}",
                    String::from_utf8_lossy(&generated.stdout),
                    String::from_utf8_lossy(&generated.stderr)
                );
                let built = match language {
                    "ada" => Command::new("gnatmake")
                        .current_dir(&out)
                        .args(["-c", "-gnat2022", "service_api.ads"])
                        .output()
                        .unwrap(),
                    "cpp" => {
                        std::fs::write(
                            out.join("probe.cpp"),
                            "#include \"service_api.hpp\"\nint main(){return 0;}\n",
                        )
                        .unwrap();
                        Command::new("g++")
                            .current_dir(&out)
                            .args([
                                "-std=c++17",
                                "-Wall",
                                "-Wextra",
                                "-Werror",
                                "-pedantic",
                                "probe.cpp",
                                "-o",
                                "probe",
                            ])
                            .output()
                            .unwrap()
                    }
                    _ => {
                        let mut vector = std::fs::read_to_string(
                            repo.join("tests/fixtures/oms-json").join(fixture),
                        )
                        .unwrap();
                        vector = vector.replace(
                            "002.5.0",
                            &format!("002.{}.0", version.split('.').nth(1).unwrap()),
                        );
                        std::fs::write(out.join("valid.json"), vector).unwrap();
                        std::fs::write(
                            out.join("Cargo.toml"),
                            format!(
                                r#"[package]
name="task060-real-generated"
version="0.1.0"
edition="2021"
[workspace]
[[bin]]
name="probe"
path="probe.rs"
[dependencies]
ams-gra-oms-runtime-api={{path={:?}}}
ams-gra-oms-runtime-rust={{path={:?}}}
serde_json="1"
"#,
                                repo.join("crates/runtime-api-rust"),
                                repo.join("crates/runtime-rust")
                            ),
                        )
                        .unwrap();
                        std::fs::write(out.join("probe.rs"),format!(r#"
#[path="service_api.rs"] pub mod generated;
use ams_gra_oms_runtime_rust::OmsJsonCodec;
use generated::{{model::{payload},service_codec::ServiceCodec}};
fn main() {{
let body:serde_json::Value=serde_json::from_str(include_str!("valid.json")).unwrap();
let payload:{payload}=ServiceCodec.decode_payload(&body).unwrap();
let encoded=ServiceCodec.encode_payload(&payload).unwrap();
assert_eq!(encoded.pointer({lexical_path:?}).unwrap(),{spelling:?});
let second:{payload}=ServiceCodec.decode_payload(&encoded).unwrap();assert_eq!(ServiceCodec.encode_payload(&second).unwrap(),encoded);
for bad in [serde_json::json!("bad"),serde_json::json!(42)] {{
let mut changed=body.clone();*changed.pointer_mut({lexical_path:?}).unwrap()=bad;
let rejected:Result<{payload},_>=ServiceCodec.decode_payload(&changed);assert!(rejected.is_err());
}}
}}
"#)).unwrap();
                        // This standalone manifest resolves its own dependencies.
                        // Deep CI's CLI build does not warm the runtime's crate cache.
                        Command::new("cargo")
                            .current_dir(&out)
                            .args(["run", "--quiet"])
                            .env("CARGO_TARGET_DIR", scratch.join("cargo-target"))
                            .env("RUSTFLAGS", "-Dwarnings")
                            .output()
                            .unwrap()
                    }
                };
                assert!(
                    built.status.success(),
                    "{version} {name} {language}: {}",
                    String::from_utf8_lossy(&built.stderr)
                );
            }
            println!("\nUCI {version} TASK060 {name} COMPILER CODEC VERTICAL: PASSED");
            std::fs::remove_dir_all(scratch).unwrap();
        }
    }
}
