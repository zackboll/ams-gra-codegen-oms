//! Task 060 pre-production evidence gate, stacked on the exact Task 059 head.
//! This target intentionally asserts the unsupported parent baseline.
use ams_gra_oms_codegen_core::{
    BackendLanguage, CoverageAnalysis, GenerationWorld, analyze_service_readiness,
    project_service_generation_schema, resolve_service_plan, string_profile,
};
use ams_gra_oms_ir::{PrimitiveKind, TypeKind, TypeRefTarget};
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::{collections::BTreeSet, fmt::Write, path::PathBuf, process::Command};

const PARENT: &str = "263df8be24439f368b64cfde56b9b0c70a93e226";
const MEMBERS: [&str; 19] = [
    "FIPS_CountryCodeType",
    "IPv4_AddressType",
    "IPv6_AddressType",
    "MilitaryGridType",
    "NITF_AIMIDB_MissionNumberType",
    "NITF_CodewordsType",
    "NITF_DateAndTimeType",
    "NITF_DateType",
    "NITF_DeclassificationExemptionType",
    "NITF_DeclassificationType",
    "NITF_IPON_IID2_ProgramCodeType",
    "NITF_MSTGTA_TargetCategoryType",
    "NITF_MSTGTA_TargetLocationType",
    "NITF_MSTGTA_TargetPriorityType",
    "NITF_PATCHB_GravityType",
    "NITF_ReleasingInstructionsType",
    "NITF_UTC_TimeType",
    "NotationType",
    "RecordOriginatorType",
];
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

fn branch_assessment(name: &str) -> &'static str {
    match name {
        "IPv4_AddressType" => {
            "five finite octet branches; four octets with literal dot separators; needs finite nested union/sequence, not a networking parser"
        }
        "IPv6_AddressType" => {
            "finite hex runs (0..4), colon alternatives, 0..5 repeated runs, finite optional suffix, embedded decimal alternatives; model decision pending, no platform parser permitted"
        }
        "MilitaryGridType" => {
            "three zone alternatives plus polar alternative; finite 0..5 digit pairs; release-specific optional suffix in 2.5 versus mandatory letters in 2.6; finite expansion possible"
        }
        "NITF_DateAndTimeType" | "NITF_DateType" | "NITF_MSTGTA_TargetLocationType" => {
            "NOT ASCII-only: XML Schema \\d denotes Unicode decimal digits; do not replace by ASCII digits; excluded from an ASCII-only family unless evidence disproves Unicode reach"
        }
        "NITF_CodewordsType" | "NITF_ReleasingInstructionsType" => {
            "same-level OR of finite fixed-width uppercase pairs and exact literal SPACE padding; deterministic branch sequences"
        }
        "NITF_DeclassificationExemptionType" => {
            "four internal alternatives; length=4 independently rejects the single-letter DNIO branch; retain this branch in semantics rather than invent padding"
        }
        "NotationType" => {
            "upper-alphanumeric exact 5 OR literal UNKN OR literal NONE; independent minLength=4/maxLength=5"
        }
        "RecordOriginatorType" => {
            "two uppercase letters OR literal E OR literal hyphen; independent minLength=1/maxLength=2"
        }
        _ => {
            "finite literal/class fixed-width branch sequences; representable by finite union of Task 059-style bodies"
        }
    }
}

fn contract(message: &str, version: &str) -> ams_gra_oms_service_contract::Contract {
    ams_gra_oms_service_contract::parse_yaml(&format!(
        "contract_version: \"0.1\"\nservice:\n  name: task060\n  version: \"0.1.0\"\n  kind: service\nstandards:\n  oms_version: \"{version}\"\n  uci_schema_version: \"{version}\"\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: output\n        mandate: mandatory\n        message: {message}\n        topic: t\n        timing:\n          kind: asynchronous\n"
    ))
    .unwrap()
}

#[test]
fn task060_parent_inventory_and_baseline() {
    if ROOTS
        .iter()
        .any(|(_, variable, _)| std::env::var_os(variable).is_none())
    {
        assert!(
            std::env::var_os("AMS_GRA_TASK060_EVIDENCE_OUTPUT").is_none(),
            "cannot freeze evidence without both pinned roots"
        );
        eprintln!("SKIPPED Task 060 parent evidence: both pinned roots required");
        return;
    }
    let mut evidence = format!(
        "# Task 060 — frozen pre-production evidence\n\n\
         Starting parent: `{PARENT}`. Branch: \
         `feature/060-alternating-ascii-string-profiles`. Separate worktree: \
         `/home/zboll/git/ams-gra-codegen-oms-task060`.\n\n\
         At isolation, PR #60 was OPEN/unmerged at this exact head; this task is \
         STACKED, not main-based. The worktree was clean before adding this evidence \
         target. No production implementation or support claim is made here.\n\n\
         ## Pattern-group semantics\n\n\
         Internal `|` is union within an expression. Alternatives within one \
         normalized group are OR. Restriction-level groups are AND, in base-to-derived \
         order. The frontend collects local patterns into one group and appends that \
         group to inherited groups; flattening those groups would be incorrect.\n\n\
         ## Inventory\n\n\
         Regenerated from pinned normalized IR using the unchanged Task 059 classifier. \
         The complete unsupported String name set is asserted against all 19 names, \
         not inferred from a prior markdown list. Counts below distinguish semantic \
         message closures from generated-support projections. Projection failures \
         are recorded separately, not treated as absence of support reach. Exact \
         effective constraints include all numeric, length and lexical facets.\n\n"
    );
    let mut ran = 0;
    for (version, variable, digest) in ROOTS {
        let path = PathBuf::from(std::env::var_os(variable).expect("both pinned roots required"));
        let hash = Command::new("sha256sum").arg(&path).output().unwrap();
        assert!(hash.status.success());
        assert_eq!(
            String::from_utf8_lossy(&hash.stdout)
                .split_whitespace()
                .next(),
            Some(digest)
        );
        let schema = load_schema_set(&path).unwrap();
        let rows: Vec<_> = schema
            .types
            .iter()
            .filter(|d| {
                matches!(d.kind, TypeKind::Primitive(PrimitiveKind::String))
                    && string_profile(PrimitiveKind::String, &d.constraints).is_err()
            })
            .collect();
        assert_eq!(
            rows.iter()
                .map(|d| d.name.local_name.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(MEMBERS)
        );
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        let mut reach = vec![BTreeSet::new(); rows.len()];
        let mut support = vec![BTreeSet::new(); rows.len()];
        let mut projection_failures = Vec::new();
        let mut oob_selected = BTreeSet::new();
        let mut oob_support = BTreeSet::new();
        for message in &schema.messages {
            if let TypeRefTarget::Named(payload) = &message.payload_type.target
                && let Ok(closure) = analysis.dependency_closure(payload)
            {
                for (i, row) in rows.iter().enumerate() {
                    if closure.iter().any(|d| d.name == row.name) {
                        reach[i].insert(message.name.local_name.clone());
                    }
                }
            }
            let plan = resolve_service_plan(&contract(&message.name.local_name, version), &schema)
                .unwrap();
            match project_service_generation_schema(
                &plan,
                &schema,
                GenerationWorld::ClosedSchemaSet,
            ) {
                Ok(projection) => {
                    for (i, row) in rows.iter().enumerate() {
                        if projection
                            .generated_support_type_names()
                            .contains(&row.name)
                        {
                            support[i].insert(message.name.local_name.clone());
                        }
                    }
                    if message.name.local_name == "OrderOfBattle" {
                        oob_selected.extend(projection.selected_type_names().iter().cloned());
                        oob_support
                            .extend(projection.generated_support_type_names().iter().cloned());
                    }
                }
                Err(error) => {
                    projection_failures.push(format!("{}: {error}", message.name.local_name))
                }
            }
        }
        writeln!(evidence, "### UCI {version}\n\nRoot SHA-256: `{digest}`.\n").unwrap();
        evidence.push_str("|Qualified name|Groups / alternatives per group|Exact effective constraints|Effective whiteSpace|ASCII gate|Semantic message reach|Generated-support reach|OrderOfBattle selected / support|Branch assessment|\n|---|---|---|---|---|---:|---:|---|---|\n");
        for (i, row) in rows.iter().enumerate() {
            let c = &row.constraints;
            assert_eq!(c.lexical.pattern_groups.len(), 1);
            assert!(c.lexical.white_space.is_none());
            assert!(c.min_inclusive.is_none() && c.max_inclusive.is_none());
            assert!(c.min_exclusive.is_none() && c.max_exclusive.is_none());
            let unicode_digit = c
                .lexical
                .pattern_groups
                .iter()
                .any(|g| g.alternatives.iter().any(|p| p.expression.contains("\\d")));
            let alternatives: Vec<_> = c
                .lexical
                .pattern_groups
                .iter()
                .map(|g| g.alternatives.len())
                .collect();
            writeln!(
                evidence,
                "|`{{{}}}{}`|{} / {alternatives:?}|`{}`|{:?}|{}|{}|{}|{} / {}|{}|",
                row.name.namespace_uri,
                row.name.local_name,
                c.lexical.pattern_groups.len(),
                format!("{c:?}").replace('|', "&#124;"),
                c.lexical.effective_white_space(PrimitiveKind::String),
                if unicode_digit {
                    "NON-ASCII: XSD \\d"
                } else {
                    "finite ASCII classes/literals"
                },
                reach[i].len(),
                support[i].len(),
                oob_selected.contains(&row.name),
                oob_support.contains(&row.name),
                branch_assessment(&row.name.local_name),
            )
            .unwrap();
        }
        for (i, row) in rows.iter().enumerate() {
            writeln!(
                evidence,
                "\n<!-- {} semantic messages: {:?}; support messages: {:?} -->\n",
                row.name.local_name, reach[i], support[i]
            )
            .unwrap();
        }
        writeln!(
            evidence,
            "\nProjection failures ({}): {projection_failures:?}\n",
            projection_failures.len()
        )
        .unwrap();
        evidence.push_str("#### OrderOfBattle parent baseline\n\n");
        let plan = resolve_service_plan(&contract("OrderOfBattle", version), &schema).unwrap();
        for language in BackendLanguage::ALL {
            let readiness = analyze_service_readiness(
                &plan,
                &schema,
                language,
                GenerationWorld::ClosedSchemaSet,
            )
            .unwrap();
            assert!(!readiness.is_ready());
            assert!(readiness.unsupported_types.is_empty());
            assert_eq!(
                readiness
                    .unsupported_generated_support_types
                    .iter()
                    .map(|n| n.local_name.as_str())
                    .collect::<BTreeSet<_>>(),
                BTreeSet::from(["MilitaryGridType", "NotationType", "RecordOriginatorType"])
            );
            writeln!(evidence, "- {language:?}: selected {}/{}, support {}/{}, READY=false; unsupported support: {:?}.",
                readiness.selected_types_renderable, readiness.selected_types_total,
                readiness.generated_support_types_renderable, readiness.generated_support_types_total,
                readiness.unsupported_generated_support_types).unwrap();
        }
        evidence.push_str("\n#### Parent coverage\n\n");
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let coverage = CoverageAnalysis::new(&schema, world).unwrap();
            for language in BackendLanguage::ALL {
                writeln!(
                    evidence,
                    "- {world:?} {language:?}: {:?}",
                    coverage.backend_coverage(language).unwrap()
                )
                .unwrap();
            }
        }
        println!(
            "UCI {version} TASK060 PARENT INVENTORY: 19 exact names; OrderOfBattle three support blockers verified"
        );
        ran += 1;
    }
    assert_eq!(ran, 2);
    evidence.push_str("\n## Scope gate status\n\nThe 19-name baseline is frozen, but no final admitted count is claimed. Three rows contain Unicode decimal-digit escapes and are not proven ASCII-only. IPv6 is finite but the clean semantic representation and branch corpus still require review. IPv4 must remain lexical, never use a platform IP parser. No production changes may precede completion of this freeze.\n\n## Validation status\n\nThis evidence target checks both root hashes, exact unsupported name sets, effective facets, per-message reach, generated-support reach, all six OrderOfBattle parent verdicts, and the 12-cell parent coverage matrix. The separate Task 060 frontend grouping test proves preservation of same-level alternatives and successive restriction groups using an explicit finite-literal intersection oracle. Implementation, after measurements, backend corpus, codecs, byte identity, workspace/MSRV and CI delivery are NOT COMPLETE.\n");
    if let Some(output) = std::env::var_os("AMS_GRA_TASK060_EVIDENCE_OUTPUT") {
        assert!(
            !std::fs::read_to_string(&output)
                .is_ok_and(|existing| existing.contains("## PRE-PRODUCTION FINAL GATE")),
            "refusing to overwrite the finalized admission gate with the parent survey; use a temporary output path"
        );
        std::fs::write(output, &evidence).unwrap();
    }
    println!("{evidence}");
}
