//! Bounded opt-in real-schema qualification; never silently skip missing pins.
use ams_gra_codegen_oms::{schema_impact::SchemaImpact, service_impact::ServiceImpact};
use ams_gra_oms_codegen_core::{direct_named_dependencies, resolve_service_plan};
use ams_gra_oms_ir::{SchemaIr, TypeRefTarget};
use ams_gra_oms_service_contract::parse_yaml;
use ams_gra_oms_xsd_frontend::load_schema_set_with_overlays;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-impact")
        .join(name)
}
fn verify(path: &Path, digest: &str) {
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
        Some(digest),
        "pin mismatch {}",
        path.display()
    );
}
fn load(key: &str, release: &str, root: &str, security: &str) -> SchemaIr {
    let path =
        PathBuf::from(std::env::var_os(key).unwrap_or_else(|| {
            panic!("{key} required for explicitly requested pinned qualification")
        }));
    verify(&path, root);
    verify(
        &path
            .parent()
            .unwrap()
            .join(format!("UCI_SecurityMarkings_v{release}_0.xsd")),
        security,
    );
    load_schema_set_with_overlays(&path, &[]).expect("verified closure loads")
}
fn evidence() -> String {
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
    let single = parse_yaml(&std::fs::read_to_string(fixture("uci-single.yaml")).unwrap()).unwrap();
    let repeated =
        parse_yaml(&std::fs::read_to_string(fixture("uci-repeated.yaml")).unwrap()).unwrap();
    // Production typed batch computes one Task 078 report; schemas loaded once.
    let reports = ServiceImpact::analyze_many(&[&single, &repeated], &before, &after).unwrap();
    let whole = &reports[0].schema_impact;
    assert_eq!(whole, &reports[1].schema_impact);
    let mut summary = "scope\tSTATIC_NAMED_REFERENCES\nbasis\tBEFORE_CONTRACT\n".to_owned();
    for (case, contract, r, expected_occurrences, expected_unique) in [
        ("single", &single, &reports[0], 1, 1),
        ("repeated", &repeated, &reports[1], 3, 2),
    ] {
        let independent = resolve_service_plan(contract, &before).unwrap();
        assert_eq!(r.plan, independent);
        assert_eq!(
            (r.oms_occurrences(), r.selected.len()),
            (expected_occurrences, expected_unique)
        );
        assert_eq!(r.total_occurrences(), r.oms_occurrences());
        assert!(r.direct.is_empty());
        assert!(r.selected.iter().all(|s| s.status == "SAME_PAYLOAD"));
        assert_eq!(r.to_text(), r.to_text());
        assert_eq!(r.to_tsv(), r.to_tsv());
        let text = r.to_text();
        assert!(text.contains("2.5") && text.contains("002.5.0") && text.contains("002.6.0"));
        summary.push_str(&format!(
            "CASE\t{case}\toccurrences\t{}\tselected\t{}\tdirect\t{}\n",
            r.oms_occurrences(),
            r.selected.len(),
            r.direct_unique_messages()
        ));
        summary.push_str(&format!(
            "VERSIONS\t{case}\t{}\t{:?}\t{:?}\n",
            r.plan.standards.uci_schema_version,
            whole.diff.before_version,
            whole.diff.after_version
        ));
        for f in &r.plan.functions {
            for e in &f.exchanges {
                let e = e.as_oms_message().unwrap();
                summary.push_str(&format!(
                    "BINDING\t{case}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    f.id,
                    e.id,
                    e.direction,
                    e.topic,
                    e.message_name.namespace_uri,
                    e.message_name.local_name
                ));
            }
        }
        for (side, schema, selected, all) in [
            ("BEFORE", &before, &r.before, &whole.before),
            ("AFTER", &after, &r.after, &whole.after),
        ] {
            let expected: Vec<_> = all
                .iter()
                .filter(|e| {
                    independent
                        .selected_messages()
                        .iter()
                        .any(|m| m.name == e.message)
                })
                .cloned()
                .collect();
            assert_eq!(selected, &expected);
            assert_eq!(SchemaImpact::unique_messages(selected), expected_unique);
            summary.push_str(&format!(
                "EXPOSURE\t{case}\t{side}\tunique\t{}\tpairs\t{}\n",
                SchemaImpact::unique_messages(selected),
                selected.len()
            ));
            for e in selected {
                let message = schema
                    .messages
                    .iter()
                    .find(|m| m.name == e.message)
                    .unwrap();
                assert_eq!(
                    message.payload_type.target,
                    TypeRefTarget::Named(e.witness[0].clone())
                );
                assert_eq!(e.witness.last(), Some(&e.seed));
                for edge in e.witness.windows(2) {
                    let t = schema.types.iter().find(|t| t.name == edge[0]).unwrap();
                    assert!(direct_named_dependencies(t).contains(&&edge[1]));
                }
            }
            for message in independent.selected_messages() {
                let e = selected
                    .iter()
                    .find(|e| {
                        e.message == message.name && e.seed.local_name == "SecurityInformationType"
                    })
                    .expect("shared nested witness");
                summary.push_str(&format!(
                    "WITNESS\t{case}\t{side}\t{}\t{}\t{}\n",
                    e.message.local_name,
                    e.seed.local_name,
                    e.witness
                        .iter()
                        .map(|n| format!("{{{}}}{}", n.namespace_uri, n.local_name))
                        .collect::<Vec<_>>()
                        .join(" -> ")
                ));
            }
        }
    }
    // Repetition affects occurrences, never the PositionReport evidence subset.
    for (single, repeated) in [
        (&reports[0].before, &reports[1].before),
        (&reports[0].after, &reports[1].after),
    ] {
        assert_eq!(
            single,
            &repeated
                .iter()
                .filter(|r| r.message.local_name == "PositionReport")
                .cloned()
                .collect::<Vec<_>>()
        );
    }
    summary
}

#[test]
#[ignore = "preparation only: inspect evidence, no qualification pass marker"]
fn task079_prepare_pinned_evidence() {
    println!(
        "TASK079 PREPARATION BEGIN\n{}TASK079 PREPARATION END",
        evidence()
    );
}

#[test]
#[ignore = "requires hash-verified cached UCI 2.5 and 2.6 closures"]
fn task079_pinned_service_impact() {
    let summary = evidence();
    assert_eq!(
        summary,
        std::fs::read_to_string(fixture("uci-summary.tsv"))
            .expect("inspected frozen evidence required")
    );
    println!("{summary}TASK079 PINNED SERVICE IMPACT: PASSED");
}
