//! Task 062 evidence: unchanged-parent BEFORE and eventual production AFTER.
//! No declaration name participates in the inventory profile comparison.
use ams_gra_oms_codegen_core::{
    BackendLanguage, CoverageAnalysis, GenerationWorld, analyze_service_readiness,
    effective_choice_alternatives, effective_record_fields, project_service_generation_schema,
    resolve_service_plan,
};
use ams_gra_oms_ir::{
    ConstraintSet, LexicalConstraintSet, PatternExpression, PatternGroup, PrimitiveKind, SchemaIr,
    TypeKind, TypeRefTarget, WhiteSpacePolicy,
};
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::{collections::BTreeSet, path::PathBuf, process::Command};

#[path = "../../../tests/task063_integrated_coverage.rs"]
mod task063_integrated_coverage;

// Cross-checked against both raw pinned XSD roots before writing this harness.
const PATTERN: &str = r"((:|[0-9a-fA-F]{0,4}):)([0-9a-fA-F]{0,4}:){0,5}((([0-9a-fA-F]{0,4}:)?(:|[0-9a-fA-F]{0,4}))|(((25[0-5]|2[0-4][0-9]|[01]?[0-9]?[0-9])\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9]?[0-9])))";
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

fn exact_constraints() -> ConstraintSet {
    ConstraintSet {
        min_length: Some(2),
        max_length: Some(45),
        lexical: LexicalConstraintSet {
            pattern_groups: vec![PatternGroup {
                alternatives: vec![PatternExpression::xml_schema(PATTERN)],
            }],
            white_space: None,
        },
        ..ConstraintSet::default()
    }
}

fn contract(message: &str, version: &str) -> ams_gra_oms_service_contract::Contract {
    ams_gra_oms_service_contract::parse_yaml(&format!(
        "contract_version: \"0.1\"\nservice:\n  name: task062\n  version: \"0.1.0\"\n  kind: service\nstandards:\n  oms_version: \"{version}\"\n  uci_schema_version: \"{version}\"\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: output\n        mandate: mandatory\n        message: {message}\n        topic: t\n        timing:\n          kind: asynchronous\n"
    )).unwrap()
}

fn inventory(schema: &SchemaIr, version: &str) -> BTreeSet<ams_gra_oms_ir::QualifiedName> {
    let mut authored = 0;
    let mut inherited = 0;
    let mut optional = 0;
    let names: BTreeSet<_> = schema
        .types
        .iter()
        .filter(|d| {
            d.kind == TypeKind::Primitive(PrimitiveKind::String)
                && d.constraints == exact_constraints()
        })
        .map(|d| d.name.clone())
        .collect();
    assert_eq!(names.len(), 1, "complete exact-profile inventory");
    for d in &schema.types {
        if names.contains(&d.name) {
            assert!(matches!(
                d.base_type.as_ref().map(|r| &r.target),
                Some(TypeRefTarget::Primitive(PrimitiveKind::String))
            ));
            assert_eq!(
                d.constraints
                    .lexical
                    .effective_white_space(PrimitiveKind::String),
                WhiteSpacePolicy::Preserve
            );
            println!(
                "DECL\t{version}\t{:?}\t{:?}\tbase={:?}\t{:?}",
                d.name, d.source, d.base_type, d.constraints
            );
        }
        let fields = match &d.kind {
            TypeKind::Record { fields }
            | TypeKind::Choice {
                alternatives: fields,
            } => fields,
            TypeKind::Alias(r) | TypeKind::List { item_type: r, .. } => {
                if matches!(&r.target, TypeRefTarget::Named(n) if names.contains(n)) {
                    println!("OTHER_REF\t{version}\t{:?}\t{:?}", d.name, d.kind);
                }
                continue;
            }
            _ => continue,
        };
        for f in fields {
            if matches!(&f.type_ref.target, TypeRefTarget::Named(n) if names.contains(n)) {
                authored += 1;
                optional += usize::from(f.cardinality.min_occurs == 0);
                assert_eq!(f.cardinality.max_occurs, Some(1));
                assert!(!matches!(d.kind, TypeKind::Choice { .. }));
                println!(
                    "MEMBER\t{version}\t{}.{}\t{:?}\t{:?}\t{:?}\tchoice={}\tnillable={}\t{:?}",
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
        }
        let effective = match d.kind {
            TypeKind::Record { .. } => effective_record_fields(schema, &d.name),
            _ => effective_choice_alternatives(schema, &d.name),
        };
        match effective {
            Ok(effective) => {
                for f in effective {
                    if matches!(&f.type_ref.target, TypeRefTarget::Named(n) if names.contains(n)) {
                        inherited += usize::from(!fields.contains(f));
                        println!(
                            "EFFECTIVE_MEMBER\t{version}\t{}.{}\tinherited={}\t{:?}\t{:?}",
                            d.name.local_name,
                            f.name,
                            !fields.contains(f),
                            f.source,
                            f.cardinality
                        );
                    }
                }
            }
            Err(error) => println!(
                "MEMBER_PROJECTION\t{version}\t{}\t{error}",
                d.name.local_name
            ),
        }
    }
    assert_eq!((authored, inherited, optional), (3, 3, 1));
    names
}

#[test]
fn task062_pinned_ipv6_inventory_coverage_and_impact() {
    if ROOTS
        .iter()
        .any(|(_, variable, _)| std::env::var_os(variable).is_none())
    {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_TASK062_PINNED").is_none(),
            "both pinned roots required"
        );
        eprintln!("SKIPPED TASK062 pinned inventory: both roots required");
        return;
    }
    for (version, variable, digest) in ROOTS {
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
        let names = inventory(&schema, version);
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let coverage = CoverageAnalysis::new(&schema, world).unwrap();
            for language in BackendLanguage::ALL {
                let actual = coverage.backend_coverage(language).unwrap();
                let row = format!("COVERAGE\t{version}\t{world:?}\t{language:?}\t{:?}", actual);
                // Task 062's AFTER campaign stays historical. The newest
                // evidence layer owns exact combined-current capability counts.
                assert_eq!(
                    vec![
                        actual.declaration_kinds_renderable,
                        actual.declarations_fully_renderable,
                        actual.field_type_references_renderable,
                        actual.field_occurrences_renderable,
                        actual.message_closures_renderable,
                    ],
                    task063_integrated_coverage::integrated_coverage(
                        version,
                        world,
                        &format!("{language:?}"),
                    ),
                    "{row}"
                );
                let baseline = include_str!("../../../tests/fixtures/string/task062-after.tsv")
                    .lines()
                    .find(|line| {
                        line.starts_with(&format!("COVERAGE\t{version}\t{world:?}\t{language:?}\t"))
                    })
                    .unwrap();
                for (metric, value) in [
                    ("declarations_total", actual.declarations_total),
                    ("fields_total", actual.fields_total),
                    ("messages_total", actual.messages_total),
                ] {
                    assert!(baseline.contains(&format!("{metric}: {value},")), "{row}");
                }
                println!(
                    "COVERAGE\t{version}\t{world:?}\t{language:?}\t{:?}",
                    coverage.backend_coverage(language).unwrap()
                );
                println!(
                    "FULL_MESSAGES\t{version}\t{world:?}\t{language:?}\t{:?}",
                    coverage.renderable_message_closure_names(language).unwrap()
                );
            }
            for m in &schema.messages {
                let plan =
                    resolve_service_plan(&contract(&m.name.local_name, version), &schema).unwrap();
                let selected = plan.selected_type_closure(&schema).unwrap();
                let selected_ipv6 = selected.iter().any(|d| names.contains(&d.name));
                let projection = project_service_generation_schema(&plan, &schema, world);
                let support_ipv6 = projection.as_ref().is_ok_and(|p| {
                    p.generated_support_type_names()
                        .iter()
                        .any(|n| names.contains(n))
                });
                if let Err(error) = &projection {
                    println!(
                        "PROJECTION\t{version}\t{world:?}\t{}\tselected={selected_ipv6}\t{error}",
                        m.name.local_name
                    );
                    continue;
                }
                if !selected_ipv6 && !support_ipv6 {
                    continue;
                }
                println!(
                    "REACH\t{version}\t{world:?}\t{}\tselected={selected_ipv6}\tsupport={support_ipv6}",
                    m.name.local_name
                );
                for language in BackendLanguage::ALL {
                    let r = analyze_service_readiness(&plan, &schema, language, world).unwrap();
                    let row = format!(
                        "SERVICE\t{version}\t{world:?}\t{language:?}\t{}\t{}\t{}\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}",
                        m.name.local_name,
                        r.is_ready(),
                        r.selected_types_total,
                        r.generated_support_types_total,
                        r.unsupported_types,
                        r.unsupported_generated_support_types,
                        r.blocked_messages,
                        r.backend_blocker,
                        r.service_api_blocker
                    );
                    assert!(
                        include_str!("../../../tests/fixtures/string/task062-after.tsv")
                            .lines()
                            .any(|expected| expected == row),
                        "{row}"
                    );
                    println!(
                        "SERVICE\t{version}\t{world:?}\t{language:?}\t{}\t{}\t{}\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}",
                        m.name.local_name,
                        r.is_ready(),
                        r.selected_types_total,
                        r.generated_support_types_total,
                        r.unsupported_types,
                        r.unsupported_generated_support_types,
                        r.blocked_messages,
                        r.backend_blocker,
                        r.service_api_blocker
                    );
                }
            }
            println!("UCI {version} TASK062 {world:?} INVENTORY COVERAGE IMPACT: PASSED");
        }
    }
}
