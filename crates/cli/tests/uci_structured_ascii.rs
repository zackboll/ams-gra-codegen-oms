//! Task 059: pinned UCI structured ASCII evidence (Deep CI only).
use ams_gra_oms_codegen_core::{
    BackendLanguage, CoverageAnalysis, GenerationWorld, StringProfile, analyze_service_readiness,
    resolve_service_plan, string_profile,
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
            Ok(Some(StringProfile::StructuredAscii(_)))
        )
}
fn contract(name: &str, version: &str) -> ams_gra_oms_service_contract::Contract {
    let yaml = format!(
        "contract_version: \"0.1\"\nservice:\n  name: task059\n  version: \"0.1.0\"\n  kind: service\nstandards:\n  oms_version: \"{version}\"\n  uci_schema_version: \"{version}\"\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: output\n        mandate: mandatory\n        message: {name}\n        topic: t\n        timing:\n          kind: asynchronous\n"
    );
    ams_gra_oms_service_contract::parse_yaml(&yaml).unwrap()
}
#[test]
fn task059_real_uci_inventory_and_coverage() {
    for (version, var, digest) in ROOTS {
        let Some((_, schema)) = root(var, digest) else {
            eprintln!("SKIPPED {var}");
            continue;
        };
        let matched: Vec<_> = schema.types.iter().filter(|d| member(d)).collect();
        let excluded: Vec<_> = schema
            .types
            .iter()
            .filter(|d| {
                matches!(d.kind, TypeKind::Primitive(PrimitiveKind::String))
                    && string_profile(PrimitiveKind::String, &d.constraints).is_err()
            })
            .collect();
        assert_eq!(matched.len(), 27);
        assert!(excluded.is_empty());
        for d in &matched {
            let c = &d.constraints;
            assert!(c.lexical.white_space.is_none());
            assert_eq!(c.lexical.pattern_groups.len(), 1);
            assert_eq!(c.lexical.pattern_groups[0].alternatives.len(), 1);
            println!(
                "UCI {version} STRUCTURED ASCII: {} {:?} length={:?} minLength={:?} maxLength={:?}",
                d.name.local_name, c.lexical.pattern_groups, c.length, c.min_length, c.max_length
            );
        }
        let names: BTreeSet<_> = excluded
            .iter()
            .map(|d| d.name.local_name.as_str())
            .collect();
        for name in [
            "NITF_DateType",
            "NITF_DateAndTimeType",
            "NITF_MSTGTA_TargetLocationType",
        ] {
            // Still not in the structured-ASCII family; Task066 now admits
            // these via its separate Unicode classifier.
            assert!(!names.contains(name));
        }
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let analysis = CoverageAnalysis::new(&schema, world).unwrap();
            for lang in BackendLanguage::ALL {
                let c = analysis.backend_coverage(lang).unwrap();
                println!(
                    "UCI {version} STRUCTURED ASCII COVERAGE {world:?} {lang:?}: kinds {}/{} declarations {}/{} field-types {}/{} field-occurrences {}/{} messages {}/{}",
                    c.declaration_kinds_renderable,
                    c.declarations_total,
                    c.declarations_fully_renderable,
                    c.declarations_total,
                    c.field_type_references_renderable,
                    c.fields_total,
                    c.field_occurrences_renderable,
                    c.fields_total,
                    c.message_closures_renderable,
                    c.messages_total
                );
            }
        }
        println!("UCI {version} STRUCTURED ASCII INVENTORY: PASSED");
    }
}
#[test]
fn task059_real_uci_message_impact() {
    for (version, var, digest) in ROOTS {
        let Some((_, schema)) = root(var, digest) else {
            continue;
        };
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        let mut reaching = 0;
        let mut ready = Vec::new();
        let mut blockers = std::collections::BTreeMap::<String, usize>::new();
        for msg in &schema.messages {
            let TypeRefTarget::Named(payload) = &msg.payload_type.target else {
                continue;
            };
            let Ok(closure) = analysis.dependency_closure(payload) else {
                continue;
            };
            if !closure.iter().any(|d| member(d)) {
                continue;
            }
            reaching += 1;
            let Ok(plan) = resolve_service_plan(&contract(&msg.name.local_name, version), &schema)
            else {
                continue;
            };
            let mut verdicts = Vec::new();
            for lang in BackendLanguage::ALL {
                let verdict = match analyze_service_readiness(
                    &plan,
                    &schema,
                    lang,
                    GenerationWorld::ClosedSchemaSet,
                ) {
                    Ok(r) if r.is_ready() => "A".to_owned(),
                    Ok(r) => r.blocked_messages.first().map_or_else(
                        || {
                            r.unsupported_generated_support_types.first().map_or_else(
                                || "C(preflight/API)".to_owned(),
                                |n| format!("B(support {})", n.local_name),
                            )
                        },
                        |b| format!("B({:?})", b.blocker),
                    ),
                    Err(e) => format!("C({e:?})"),
                };
                verdicts.push(verdict);
            }
            assert_eq!(
                verdicts[0], verdicts[1],
                "{version} {} {verdicts:?}",
                msg.name.local_name
            );
            assert_eq!(
                verdicts[1], verdicts[2],
                "{version} {} {verdicts:?}",
                msg.name.local_name
            );
            if verdicts[0] == "A" {
                ready.push(msg.name.local_name.clone())
            }
            *blockers.entry(verdicts[0].clone()).or_default() += 1;
            println!(
                "UCI {version} STRUCTURED ASCII MESSAGE: {} {}",
                msg.name.local_name, verdicts[0]
            );
        }
        println!(
            "UCI {version} STRUCTURED ASCII MESSAGE SUMMARY: reaching={reaching} READY={} distribution={blockers:?} ready={ready:?}",
            ready.len()
        );
        println!("UCI {version} STRUCTURED ASCII MESSAGE IMPACT: RECORDED");
    }
}

/// CPP-OMS-006: production C++ `--with-codec` readiness on both pinned
/// releases, through `service-check-batch` (one schema load per release).
///
/// * The six messages whose only C++ codec blocker was a Task 059 structured
///   ASCII declaration are codec READY, with their exact closures and
///   declaration counts.
/// * CPP-OMS-007 completes `FileMetadata` through the checked constrained
///   hexBinary carrier. This regression now requires full readiness, not the
///   historical CPP-OMS-006 boundary. Behavioral qualification is separate.
#[test]
fn cpp_oms_006_real_uci_cpp_codec_readiness() {
    let cli = env!("CARGO_BIN_EXE_ams-gra-codegen-oms");
    for (version, var, digest) in ROOTS {
        let Some(path) = std::env::var_os(var).map(PathBuf::from) else {
            eprintln!("SKIPPED {var}");
            continue;
        };
        let hash = Command::new("sha256sum").arg(&path).output().unwrap();
        assert_eq!(
            String::from_utf8_lossy(&hash.stdout)
                .split_whitespace()
                .next(),
            Some(digest)
        );
        let strike_command = if version == "2.5" {
            ("77", "74/74")
        } else {
            ("78", "75/75")
        };
        let ready: [(&str, &str, &str); 6] = [
            ("AO_Capability", "71", "69/69"),
            ("CounterSpaceReport", "55", "54/54"),
            ("DMPI_DesignationRequest", "56", "54/54"),
            ("OrdersMetadata", "82", "80/80"),
            ("StrikeCapability", "84", "82/82"),
            ("StrikeCommand", strike_command.0, strike_command.1),
        ];
        let (file_closure, file_codec) = if version == "2.5" {
            ("52", "50/50")
        } else {
            ("54", "52/52")
        };
        let dir = std::env::temp_dir().join(format!(
            "ams-gra-cpp-oms-006-{version}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let names: Vec<&str> = ready
            .iter()
            .map(|(name, _, _)| *name)
            .chain(["FileMetadata"])
            .collect();
        let mut requests = Vec::new();
        for name in &names {
            let file = dir.join(format!("{name}.yaml"));
            std::fs::write(
                &file,
                format!(
                    "contract_version: \"0.1\"\nservice:\n  name: cpp-oms-006\n  version: \"0.1.0\"\n  kind: service\nstandards:\n  oms_version: \"{version}\"\n  uci_schema_version: \"{version}\"\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: output\n        mandate: mandatory\n        message: {name}\n        topic: t\n        timing:\n          kind: asynchronous\n"
                ),
            )
            .unwrap();
            requests.push(serde_json::json!({
                "id": name, "contract": file.to_str().unwrap(), "language": "cpp",
                "world": "closed-schema", "with_codec": true,
            }));
        }
        let manifest = dir.join("manifest.json");
        std::fs::write(
            &manifest,
            serde_json::json!({
                "format": "service-check-batch-v1",
                "schema_version": format!("002.{}.0", &version[2..]),
                "requests": requests,
            })
            .to_string(),
        )
        .unwrap();
        let output = Command::new(cli)
            .args(["service-check-batch", "--schema"])
            .arg(&path)
            .arg("--manifest")
            .arg(&manifest)
            .output()
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["completed"], names.len(), "{report}");
        let results = report["results"].as_array().unwrap();
        for ((name, closure, codec), result) in ready.iter().zip(results) {
            assert_eq!(result["request"]["id"], *name);
            let stdout = result["stdout"].as_str().unwrap();
            assert_eq!(result["exit_code"], 0, "{version} {name}: {stdout}");
            assert!(
                stdout.contains(&format!("selected type closure: {closure}\n")),
                "{version} {name}: {stdout}"
            );
            assert!(stdout.contains("\nstatus: READY\n"), "{stdout}");
            assert!(
                stdout.contains(&format!("codec renderable emitted declarations: {codec}\n")),
                "{version} {name}: {stdout}"
            );
            assert!(stdout.ends_with("codec status: READY\n"), "{stdout}");
            println!("UCI {version} CPP-OMS-006 C++ codec READY: {name} {codec}");
        }
        let file = &results[6];
        assert_eq!(file["request"]["id"], "FileMetadata");
        let stdout = file["stdout"].as_str().unwrap();
        assert_eq!(file["exit_code"], 0, "{stdout}");
        assert!(
            stdout.contains(&format!("selected type closure: {file_closure}\n")),
            "{stdout}"
        );
        assert!(stdout.contains("\nstatus: READY\n"), "{stdout}");
        assert!(
            stdout.contains(&format!(
                "codec renderable emitted declarations: {file_codec}\n"
            )),
            "{version}: {stdout}"
        );
        assert!(stdout.ends_with("codec status: READY\n"), "{stdout}");
        assert!(!stdout.contains("FileNameType"), "{stdout}");
        println!("UCI {version} CPP-OMS-007 FileMetadata C++ codec READY {file_codec}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
