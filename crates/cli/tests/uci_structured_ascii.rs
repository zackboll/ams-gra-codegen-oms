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
        assert_eq!(excluded.len(), 4);
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
            "IPv6_AddressType",
            "NITF_DateType",
            "NITF_DateAndTimeType",
            "NITF_MSTGTA_TargetLocationType",
        ] {
            assert!(names.contains(name));
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
