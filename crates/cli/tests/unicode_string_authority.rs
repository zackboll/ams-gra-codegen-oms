//! Task066 pinned authority/current impact, loading each root once.
use ams_gra_oms_codegen_core::{
    BackendLanguage, CoverageAnalysis, GenerationWorld, direct_named_dependencies,
    effective_choice_alternatives, effective_record_fields, project_service_generation_schema,
    resolve_service_plan, string_profile,
};
use ams_gra_oms_ir::{PrimitiveKind, TypeKind, TypeRefTarget, WhiteSpacePolicy};
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::{collections::BTreeSet, path::PathBuf, process::Command};

const NAMES: [&str; 3] = [
    "NITF_DateAndTimeType",
    "NITF_DateType",
    "NITF_MSTGTA_TargetLocationType",
];

fn contract(name: &str, version: &str) -> ams_gra_oms_service_contract::Contract {
    ams_gra_oms_service_contract::parse_yaml(&format!(
        "contract_version: \"0.1\"\nservice:\n  name: task066\n  version: \"0.1.0\"\n  kind: service\nstandards:\n  oms_version: \"{version}\"\n  uci_schema_version: \"{version}\"\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: output\n        mandate: mandatory\n        message: {name}\n        topic: t\n        timing:\n          kind: asynchronous\n"
    )).unwrap()
}

#[test]
fn task066_pinned_unicode_authority_inventory() {
    let roots = [
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
    if roots
        .iter()
        .any(|(_, variable, _)| std::env::var_os(variable).is_none())
    {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_TASK066_PINNED").is_none(),
            "both pinned roots required"
        );
        eprintln!("SKIPPED Task066: both pinned roots required");
        return;
    }
    for (release, variable, digest) in roots {
        let root = PathBuf::from(std::env::var_os(variable).unwrap());
        let hash = Command::new("sha256sum").arg(&root).output().unwrap();
        assert!(hash.status.success());
        assert_eq!(
            String::from_utf8_lossy(&hash.stdout)
                .split_whitespace()
                .next(),
            Some(digest)
        );
        let schema = load_schema_set(&root).unwrap();
        let raw = std::fs::read_to_string(&root).unwrap();
        let rows: Vec<_> = NAMES
            .iter()
            .map(|name| {
                schema
                    .types
                    .iter()
                    .find(|d| &d.name.local_name == name)
                    .unwrap()
            })
            .collect();
        let expected_rows = include_str!("../../../tests/fixtures/string/task060-pinned-rows.tsv");
        for d in &rows {
            assert_eq!(d.kind, TypeKind::Primitive(PrimitiveKind::String));
            assert!(matches!(
                string_profile(PrimitiveKind::String, &d.constraints),
                Ok(Some(ams_gra_oms_codegen_core::StringProfile::Unicode(_)))
            ));
            assert_eq!(
                d.base_type.as_ref().unwrap().target,
                TypeRefTarget::Primitive(PrimitiveKind::String)
            );
            assert!(direct_named_dependencies(d).is_empty());
            assert_eq!(d.constraints.lexical.pattern_groups.len(), 1);
            assert_eq!(
                d.constraints.lexical.pattern_groups[0].alternatives.len(),
                2
            );
            assert!(d.constraints.lexical.white_space.is_none());
            let expected = expected_rows
                .lines()
                .find(|line| line.starts_with(&format!("{release}\t{}\t", d.name.local_name)))
                .unwrap();
            let cells: Vec<_> = expected.split('\t').collect();
            assert_eq!(d.constraints.length, Some(cells[2].parse().unwrap()));
            assert!(d.constraints.min_length.is_none() && d.constraints.max_length.is_none());
            assert!(d.constraints.min_inclusive.is_none() && d.constraints.max_inclusive.is_none());
            assert!(d.constraints.min_exclusive.is_none() && d.constraints.max_exclusive.is_none());
            for (p, expected) in d.constraints.lexical.pattern_groups[0]
                .alternatives
                .iter()
                .zip(&cells[5..])
            {
                assert_eq!(&p.expression, expected);
                assert_eq!(p.dialect, ams_gra_oms_ir::PatternDialect::XmlSchema);
            }
            let start = raw
                .find(&format!("<xs:simpleType name=\"{}\"", d.name.local_name))
                .unwrap();
            let end =
                start + raw[start..].find("</xs:simpleType>").unwrap() + "</xs:simpleType>".len();
            println!(
                "RAW\t{release}\t{}\tline={}\n{}",
                d.name.local_name,
                raw[..start].bytes().filter(|b| *b == b'\n').count() + 1,
                &raw[start..end]
            );
            println!(
                "PROFILE\t{release}\t{:?}\t{:?}\tkind={:?}\tbase={:?}\tdepth=1\texplicit_whitespace=None\teffective_whitespace={:?}\t{:?}",
                d.name,
                d.source,
                d.kind,
                d.base_type,
                WhiteSpacePolicy::Preserve,
                d.constraints
            );
        }
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let analysis = CoverageAnalysis::new(&schema, world).unwrap();
            for language in BackendLanguage::ALL {
                let row = format!(
                    "COVERAGE\t{release}\t{world:?}\t{language:?}\t{:?}",
                    analysis.backend_coverage(language).unwrap()
                );
                assert!(
                    include_str!("../../../tests/fixtures/string/task066-after.tsv")
                        .lines()
                        .any(|l| l == row),
                    "{row}"
                );
                println!("{row}");
            }
            let mut selected = vec![BTreeSet::new(); 3];
            let mut support = vec![BTreeSet::new(); 3];
            let mut ready = Vec::new();
            for m in &schema.messages {
                let plan =
                    resolve_service_plan(&contract(&m.name.local_name, release), &schema).unwrap();
                let closure = plan.selected_type_closure(&schema).unwrap();
                for (i, d) in rows.iter().enumerate() {
                    if closure.iter().any(|s| s.name == d.name) {
                        selected[i].insert(&m.name.local_name);
                    }
                }
                match project_service_generation_schema(&plan, &schema, world) {
                    Ok(p) => {
                        let reaches = rows.iter().any(|d| {
                            p.generated_support_type_names().contains(&d.name)
                                || p.selected_type_names().contains(&d.name)
                        });
                        if reaches {
                            for language in BackendLanguage::ALL {
                                let r = ams_gra_oms_codegen_core::analyze_service_readiness(
                                    &plan, &schema, language, world,
                                )
                                .unwrap();
                                println!(
                                    "SERVICE\t{release}\t{world:?}\t{language:?}\t{}\tready={}\tselected={}\tsupport={}\tselected_blockers={:?}\tsupport_blockers={:?}\tbackend={:?}\tapi={:?}",
                                    m.name.local_name,
                                    r.is_ready(),
                                    r.selected_types_total,
                                    r.generated_support_types_total,
                                    r.unsupported_types,
                                    r.unsupported_generated_support_types,
                                    r.backend_blocker,
                                    r.service_api_blocker
                                );
                                if r.is_ready() {
                                    ready.push((
                                        r.selected_types_total + r.generated_support_types_total,
                                        m.name.local_name.clone(),
                                        language,
                                    ));
                                }
                            }
                        }
                        for (i, d) in rows.iter().enumerate() {
                            if p.generated_support_type_names().contains(&d.name) {
                                support[i].insert(&m.name.local_name);
                            }
                        }
                    }
                    Err(e) => println!(
                        "PROJECTION\t{release}\t{world:?}\t{}\t{e}",
                        m.name.local_name
                    ),
                }
            }
            for (i, d) in rows.iter().enumerate() {
                println!(
                    "REACH\t{release}\t{world:?}\t{}\tselected={}\t{:?}\tsupport={}\t{:?}",
                    d.name.local_name,
                    selected[i].len(),
                    selected[i],
                    support[i].len(),
                    support[i]
                );
            }
            ready.sort();
            for (_, message, language) in &ready {
                let tuple = format!("{release}\t{world:?}\t{language:?}\t{message}");
                assert!(
                    include_str!("../../../tests/fixtures/string/task066-new-ready.tsv")
                        .lines()
                        .any(|l| l == tuple),
                    "{tuple}"
                );
            }
            assert_eq!(
                ready.len(),
                include_str!("../../../tests/fixtures/string/task066-new-ready.tsv")
                    .lines()
                    .filter(|l| l.starts_with(&format!("{release}\t{world:?}\t")))
                    .count()
            );
            println!("NEW-READY\t{release}\t{world:?}\t{ready:?}");
            println!("SMALLEST\t{release}\t{world:?}\t{:?}", ready.first());
            for d in &schema.types {
                let authored = match &d.kind {
                    TypeKind::Record { fields }
                    | TypeKind::Choice {
                        alternatives: fields,
                    } => fields,
                    _ => continue,
                };
                let effective = match &d.kind {
                    TypeKind::Record { .. } => effective_record_fields(&schema, &d.name),
                    _ => effective_choice_alternatives(&schema, &d.name),
                };
                if let Ok(fields) = effective {
                    for f in fields {
                        if matches!(&f.type_ref.target, TypeRefTarget::Named(n) if rows.iter().any(|r| r.name == *n))
                        {
                            println!(
                                "REFERENCE\t{release}\t{world:?}\t{}.{}\tinherited={}\t{:?}\t{:?}",
                                d.name.local_name,
                                f.name,
                                !authored.contains(f),
                                f.type_ref,
                                f.source
                            );
                        }
                    }
                }
            }
        }
        println!("UCI {release} TASK066 AUTHORITY INVENTORY: PASSED");
    }
}
