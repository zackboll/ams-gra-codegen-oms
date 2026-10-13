//! Explicit opt-in, bounded schema-only qualification. Missing inputs are errors.
use ams_gra_codegen_oms::schema_impact::SchemaImpact;
use ams_gra_oms_codegen_core::{DependencyExposure, direct_named_dependencies};
use ams_gra_oms_ir::*;
use ams_gra_oms_schema_diff::*;
use ams_gra_oms_xsd_frontend::load_schema_set_with_overlays;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn verify(path: &Path, expected: &str) {
    let out = std::process::Command::new("sha256sum")
        .arg(path)
        .output()
        .expect("sha256sum available");
    assert!(
        out.status.success(),
        "missing pinned input {}",
        path.display()
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .next(),
        Some(expected),
        "pin mismatch {}",
        path.display()
    );
}
fn load(key: &str, release: &str, digest: &str, dependency_digest: &str) -> SchemaIr {
    let path =
        PathBuf::from(std::env::var_os(key).unwrap_or_else(|| {
            panic!("{key} required for explicitly requested pinned qualification")
        }));
    verify(&path, digest);
    verify(
        &path
            .parent()
            .unwrap()
            .join(format!("UCI_SecurityMarkings_v{release}_0.xsd")),
        dependency_digest,
    );
    load_schema_set_with_overlays(&path, &[]).expect("verified schema closure loads")
}
fn check(schema: &SchemaIr, rows: &[DependencyExposure], seeds: &BTreeSet<QualifiedName>) {
    let index: BTreeMap<_, _> = schema.types.iter().map(|t| (&t.name, t)).collect();
    let actual: BTreeSet<_> = rows
        .iter()
        .map(|r| (r.message.clone(), r.seed.clone()))
        .collect();
    assert_eq!(actual.len(), rows.len());
    // Independent forward, per-message stack traversal, not reverse BFS.
    let mut expected = BTreeSet::new();
    for m in &schema.messages {
        let TypeRefTarget::Named(root) = &m.payload_type.target else {
            continue;
        };
        let mut stack = vec![root];
        let mut visited = BTreeSet::new();
        while let Some(n) = stack.pop() {
            if !visited.insert(n) {
                continue;
            }
            if seeds.contains(n) {
                expected.insert((m.name.clone(), n.clone()));
            }
            stack.extend(direct_named_dependencies(index[n]));
        }
    }
    assert_eq!(actual, expected);
    for r in rows {
        let m = schema
            .messages
            .iter()
            .find(|m| m.name == r.message)
            .unwrap();
        assert_eq!(
            m.payload_type.target,
            TypeRefTarget::Named(r.witness[0].clone())
        );
        assert_eq!(r.witness.last(), Some(&r.seed));
        for edge in r.witness.windows(2) {
            assert!(direct_named_dependencies(index[&edge[0]]).contains(&&edge[1]));
        }
    }
}
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
#[ignore = "requires hash-verified cached UCI 2.5 and 2.6 roots"]
fn task078_pinned_static_impact() {
    let before = load(
        "AMS_GRA_UCI_2_5_ROOT",
        "2_5",
        "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27",
        "4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b",
    );
    let after = load(
        "AMS_GRA_UCI_2_6_ROOT",
        "2_6",
        "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b",
        "ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88",
    );
    let forward = SchemaImpact::analyze(&before, &after).unwrap();
    let reverse = SchemaImpact::analyze(&after, &before).unwrap();
    assert_eq!(forward.before, reverse.after);
    assert_eq!(forward.after, reverse.before);
    for c in &forward.diff.changes {
        if c.kind == ChangeKind::TypeRemoved {
            assert!(!forward.after.iter().any(|r| r.seed == c.name));
        }
        if c.kind == ChangeKind::TypeAdded {
            assert!(!forward.before.iter().any(|r| r.seed == c.name));
        }
    }
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    assert_eq!(
        compact(&forward.diff),
        std::fs::read_to_string(dir.join("schema-diff/task069-uci25-to-uci26-summary.tsv"))
            .unwrap()
    );
    for (schema, rows) in [(&before, &forward.before), (&after, &forward.after)] {
        let changed: BTreeSet<_> = forward
            .diff
            .changes
            .iter()
            .filter(|c| c.category == Category::Type)
            .map(|c| c.name.clone())
            .collect();
        let seeds = schema
            .types
            .iter()
            .filter(|t| changed.contains(&t.name))
            .map(|t| t.name.clone())
            .collect();
        check(schema, rows, &seeds);
    }
    let json = forward.to_json().unwrap();
    assert_eq!(json, forward.to_json().unwrap());
    let j: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(j["directUniqueMessages"], 3);
    assert_eq!(
        j["directMessageChanges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["message"][1].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "SystemSchedule",
            "SystemScheduleDataRequest",
            "SystemScheduleDataRequestStatus"
        ]
    );
    for c in j["directMessageChanges"].as_array().unwrap() {
        assert_eq!(c["message"][0], "https://www.vdl.afrl.af.mil/programs/oam");
        assert_eq!(c["change"], "MESSAGE_ADDED");
        assert!(c["before"].is_null());
    }
    for (side, rows, unique, pairs, types, messages) in [
        ("before", &forward.before, 722, 3000, 5557, 722),
        ("after", &forward.after, 725, 3475, 5570, 725),
    ] {
        assert_eq!(j[side]["exposedUniqueMessages"], unique);
        assert_eq!(j[side]["exposurePairs"], pairs);
        assert_eq!(j[side]["types"], types);
        assert_eq!(j[side]["messages"], messages);
        let exposures = j[side]["exposures"].as_array().unwrap();
        assert_eq!(exposures.len(), rows.len());
        for (e, row) in exposures.iter().zip(rows) {
            let q = |n: &QualifiedName| serde_json::json!([n.namespace_uri, n.local_name]);
            assert_eq!(e["message"], q(&row.message));
            assert_eq!(e["seed"], q(&row.seed));
            assert_eq!(
                e["witness"],
                serde_json::Value::Array(row.witness.iter().map(q).collect())
            );
        }
        assert!(exposures.iter().any(|e| e["message"][1] == "OrderOfBattle"
            && e["witness"]
                == serde_json::json!([
                    [
                        "https://www.vdl.afrl.af.mil/programs/oam",
                        "OrderOfBattleMT"
                    ],
                    ["https://www.vdl.afrl.af.mil/programs/oam", "MessageType"],
                    [
                        "https://www.vdl.afrl.af.mil/programs/oam",
                        "SecurityInformationType"
                    ]
                ])));
    }
    let forward_tsv = forward.to_tsv();
    let reverse_text = reverse.to_text();
    assert_eq!(forward_tsv, forward.clone().to_tsv());
    assert_eq!(reverse_text, reverse.clone().to_text());
    let mut summary = format!(
        "scope\tSTATIC_NAMED_REFERENCES\ndirect_unique_messages\t{}\n",
        forward.direct().count()
    );
    for (side, rows) in [("BEFORE", &forward.before), ("AFTER", &forward.after)] {
        summary.push_str(&format!(
            "{side}\tunique_messages\t{}\npairs\t{side}\t{}\n",
            SchemaImpact::unique_messages(rows),
            rows.len()
        ));
        let r = rows
            .iter()
            .find(|r| r.message.local_name == "OrderOfBattle" && r.witness.len() > 2)
            .expect("unchanged-payload nested exposure");
        let old = before
            .messages
            .iter()
            .find(|m| m.name == r.message)
            .unwrap();
        let new = after.messages.iter().find(|m| m.name == r.message).unwrap();
        assert_eq!(old.payload_type, new.payload_type);
        summary.push_str(&format!(
            "WITNESS\t{side}\t{}\t{}\t{}\n",
            r.message.local_name,
            r.seed.local_name,
            r.witness
                .iter()
                .map(|n| format!("{{{}}}{}", n.namespace_uri, n.local_name))
                .collect::<Vec<_>>()
                .join(" -> ")
        ));
    }
    if let Some(path) = std::env::var_os("TASK078_BOOTSTRAP_EVIDENCE") {
        std::fs::write(path, &summary).unwrap();
    } else {
        assert_eq!(
            summary,
            std::fs::read_to_string(dir.join("schema-impact/uci-summary.tsv")).unwrap()
        );
    }
    println!("{summary}TASK078 PINNED STATIC IMPACT: PASSED");
}
