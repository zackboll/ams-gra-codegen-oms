//! Deep-only campaign: load each verified root once, then compare both directions.
use ams_gra_oms_ir::*;
use ams_gra_oms_schema_diff::*;
use ams_gra_oms_xsd_frontend::load_schema_set_with_overlays;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn compact(d: &SchemaDiff) -> String {
    let mut out = d.summary_tsv();
    let mut properties = BTreeMap::new();
    for c in &d.changes {
        *properties
            .entry((c.kind.label(), c.property.label()))
            .or_insert(0usize) += 1;
        if c.kind == ChangeKind::MessageAdded {
            out.push_str(&format!(
                "IDENTITY\t{}\t{}\tpresence\tMESSAGE_ADDED\t\tpresent\n",
                c.name.namespace_uri, c.name.local_name
            ));
        }
    }
    for ((kind, property), count) in properties {
        out.push_str(&format!(
            "PROPERTY_TOTAL\t\t\t{property}\t{kind}\t\t{count}\n"
        ));
    }
    out
}

#[test]
#[ignore = "requires externally fetched, digest-verified pinned UCI roots"]
fn task069_real_uci_semantic_diff() {
    let root = |key| PathBuf::from(std::env::var_os(key).expect("pinned root required"));
    let start = std::time::Instant::now();
    let before = load_schema_set_with_overlays(&root("AMS_GRA_UCI_2_5_ROOT"), &[]).unwrap();
    let after = load_schema_set_with_overlays(&root("AMS_GRA_UCI_2_6_ROOT"), &[]).unwrap();
    before.validate().unwrap();
    after.validate().unwrap();
    assert_eq!((before.types.len(), before.messages.len()), (5557, 722));
    assert_eq!((after.types.len(), after.messages.len()), (5570, 725));
    let d = compare_schemas(&before, &after);
    let reverse = compare_schemas(&after, &before);
    assert_eq!(
        (
            d.types.added,
            d.types.removed,
            d.types.changed,
            d.types.unchanged
        ),
        (
            reverse.types.removed,
            reverse.types.added,
            reverse.types.changed,
            reverse.types.unchanged
        )
    );
    assert_eq!(
        (
            d.messages.added,
            d.messages.removed,
            d.messages.changed,
            d.messages.unchanged
        ),
        (
            reverse.messages.removed,
            reverse.messages.added,
            reverse.messages.changed,
            reverse.messages.unchanged
        )
    );
    let names: Vec<_> = d
        .changes
        .iter()
        .filter(|c| c.kind == ChangeKind::MessageAdded)
        .map(|c| c.name.clone())
        .collect();
    let ns = "https://www.vdl.afrl.af.mil/programs/oam";
    assert_eq!(
        names,
        [
            "SystemSchedule",
            "SystemScheduleDataRequest",
            "SystemScheduleDataRequestStatus"
        ]
        .map(|n| QualifiedName::new(ns, n))
    );
    // Every reverse record must independently match the forward semantic values.
    let reversed: BTreeMap<_, _> = reverse
        .changes
        .iter()
        .map(|c| ((c.category, &c.name, &c.member, c.property), c))
        .collect();
    for c in &d.changes {
        let r = reversed[&(c.category, &c.name, &c.member, c.property)];
        assert_eq!(c.before, r.after);
        assert_eq!(c.after, r.before);
        let kind = match c.kind {
            ChangeKind::TypeAdded => ChangeKind::TypeRemoved,
            ChangeKind::TypeRemoved => ChangeKind::TypeAdded,
            ChangeKind::MessageAdded => ChangeKind::MessageRemoved,
            ChangeKind::MessageRemoved => ChangeKind::MessageAdded,
            ChangeKind::MemberAdded => ChangeKind::MemberRemoved,
            ChangeKind::MemberRemoved => ChangeKind::MemberAdded,
            ChangeKind::EnumValueAdded => ChangeKind::EnumValueRemoved,
            ChangeKind::EnumValueRemoved => ChangeKind::EnumValueAdded,
            k => k,
        };
        assert_eq!(kind, r.kind);
    }
    // Independent spot checks against the actual normalized declaration fields,
    // not a hand-authored XSD difference list or the summary fixture.
    let bi: BTreeMap<_, _> = before.types.iter().map(|t| (&t.name, t)).collect();
    let ai: BTreeMap<_, _> = after.types.iter().map(|t| (&t.name, t)).collect();
    let mut checked = 0;
    for c in &d.changes {
        if c.category != Category::Type {
            continue;
        }
        match (c.kind, c.property) {
            (ChangeKind::BaseTypeChanged, _) => {
                assert_eq!(
                    c.before,
                    bi[&c.name].base_type.clone().map(Value::Reference)
                );
                assert_eq!(c.after, ai[&c.name].base_type.clone().map(Value::Reference));
                checked += 1;
            }
            (
                ChangeKind::MemberChanged,
                Property::Target
                | Property::Cardinality
                | Property::WireNamespace
                | Property::Nillable,
            ) => {
                let get = |t: &TypeDecl| {
                    let members = match &t.kind {
                        TypeKind::Record { fields } => fields,
                        TypeKind::Choice { alternatives } => alternatives,
                        _ => panic!("member owner"),
                    };
                    let f = members
                        .iter()
                        .find(|f| Some(&f.name) == c.member.as_ref())
                        .unwrap();
                    match c.property {
                        Property::Target => Some(Value::Reference(f.type_ref.clone())),
                        Property::Cardinality => Some(Value::Cardinality(f.cardinality)),
                        Property::WireNamespace => f.wire_namespace_uri.clone().map(Value::Text),
                        Property::Nillable => Some(Value::Bool(f.nillable)),
                        _ => unreachable!(),
                    }
                };
                assert_eq!(c.before, get(bi[&c.name]));
                assert_eq!(c.after, get(ai[&c.name]));
                checked += 1;
            }
            _ => {}
        }
        if checked <= 6 && checked > 0 {
            println!(
                "IR SPOT CHECK: {} {} {:?}",
                c.name.local_name,
                c.kind.label(),
                c.property
            );
        }
    }
    assert!(
        checked >= 3,
        "at least three independent normalized IR checks"
    );
    let tsv = d.to_tsv();
    assert_eq!(tsv, compare_schemas(&before, &after).to_tsv());
    let summary = compact(&d);
    if let Some(dir) = std::env::var_os("TASK069_EVIDENCE_DIR") {
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("forward.tsv"), &tsv).unwrap();
        std::fs::write(dir.join("reverse.tsv"), reverse.to_tsv()).unwrap();
        std::fs::write(dir.join("summary.tsv"), &summary).unwrap();
    }
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/schema-diff/task069-uci25-to-uci26-summary.tsv");
    // Bootstrap is local-only and never used by the CI wrapper. It produces evidence
    // for inspection; it does not write or update the repository fixture.
    if std::env::var_os("TASK069_BOOTSTRAP_EVIDENCE").is_none() {
        assert_eq!(summary, std::fs::read_to_string(fixture).unwrap());
    }
    println!(
        "TASK069 complete pinned comparison seconds: {:.3}",
        start.elapsed().as_secs_f64()
    );
    println!("TASK069 IR spot checks: {checked}");
    println!("TASK069 REAL UCI SEMANTIC DIFF: PASSED");
}
