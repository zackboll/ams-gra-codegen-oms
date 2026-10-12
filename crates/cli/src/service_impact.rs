//! Baseline-contract join of the unchanged Task 078 analysis.
use crate::schema_impact::{SCOPE, SchemaImpact};
use ams_gra_oms_codegen_core::{DependencyExposure, ServicePlan, resolve_service_plan};
use ams_gra_oms_ir::{QualifiedName, SchemaIr, TypeRef, TypeRefTarget};
use ams_gra_oms_schema_diff::{Category, Change, ChangeKind};
use ams_gra_oms_service_contract::Contract;
use std::collections::BTreeSet;

pub const BASIS: &str = "BEFORE_CONTRACT";
pub const HELP: &str = "ams-gra-codegen-oms service-impact\n\nUSAGE:\n    ams-gra-codegen-oms service-impact --contract SERVICE.yaml --before OLD_ROOT.xsd --after NEW_ROOT.xsd [--before-extension ID=PATH]... [--after-extension ID=PATH]... [--format text|tsv]\n\nDefault format: text. Read-only STATIC_NAMED_REFERENCES analysis.\nBEFORE_CONTRACT: resolve exactly once against BEFORE; track full QNames in AFTER.\nRemoved baseline messages are findings, not resolution failures.\nEvery declared extension requires an explicit mapping on EACH side, in contract order.\nNo AFTER planning, readiness, compatibility, wire compatibility or safe-upgrade certification.\n";
pub const LIMITATION: &str = "Static named references only; no compatibility, wire compatibility, safe-upgrade or service-readiness verdict. No AFTER contract planning: the unmodified contract may resolve ambiguously against AFTER. No reverse inheritance/closed-sum descendant expansion or runtime/backend analysis. Non-OMS occurrences are excluded and not assessed against UCI.";
pub const EMPTY: &str = "No service-selected static dependency impact found; this is not a compatibility or readiness verdict.";
pub const HEADER: &str = "record\tside\tfunction_id\texchange_id\tdirection\tmandate\ttopic\thas_subscription_group\tsubscription_group\tmessage_namespace\tmessage_local\tseed_namespace\tseed_local\tstep\ttype_namespace\ttype_local\tkey\thas_value\tvalue\thas_after_value\tafter_value\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedStatus {
    pub message: QualifiedName,
    pub before_payload: TypeRef,
    pub after_payload: Option<TypeRef>,
    /// MISSING, SAME_PAYLOAD or CHANGED_PAYLOAD (direct reference only).
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServiceImpact {
    pub plan: ServicePlan,
    /// Whole-schema inventories remain whole-schema, never rewritten.
    pub schema_impact: SchemaImpact,
    pub selected: Vec<SelectedStatus>,
    pub direct: Vec<Change>,
    pub before: Vec<DependencyExposure>,
    pub after: Vec<DependencyExposure>,
}

impl ServiceImpact {
    /// The contract must come from the portable validated parser. Selection and
    /// comparison are derived together, never joined from an unchecked plan.
    pub fn analyze(
        contract: &Contract,
        before: &SchemaIr,
        after: &SchemaIr,
    ) -> Result<Self, String> {
        Ok(Self::analyze_many(&[contract], before, after)?.remove(0))
    }

    /// Bounded qualification/interactive batches share one comparison of the
    /// same validated snapshots. Each contract is resolved only against BEFORE.
    pub fn analyze_many(
        contracts: &[&Contract],
        before: &SchemaIr,
        after: &SchemaIr,
    ) -> Result<Vec<Self>, String> {
        before.validate().map_err(|e| e.to_string())?;
        after.validate().map_err(|e| e.to_string())?;
        let plans = contracts
            .iter()
            .map(|c| resolve_service_plan(c, before))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let impact = SchemaImpact::analyze(before, after).map_err(|e| e.to_string())?;
        Ok(plans
            .into_iter()
            .map(|plan| {
                let names: BTreeSet<_> = plan
                    .selected_messages()
                    .iter()
                    .map(|m| m.name.clone())
                    .collect();
                let selected = plan
                    .selected_messages()
                    .iter()
                    .map(|m| SelectedStatus {
                        message: m.name.clone(),
                        before_payload: m.payload_type.clone(),
                        after_payload: after
                            .messages
                            .iter()
                            .find(|a| a.name == m.name)
                            .map(|a| a.payload_type.clone()),
                        status: match after.messages.iter().find(|a| a.name == m.name) {
                            None => "MISSING",
                            Some(a) if a.payload_type == m.payload_type => "SAME_PAYLOAD",
                            Some(_) => "CHANGED_PAYLOAD",
                        },
                    })
                    .collect();
                Self {
                    direct: impact
                        .direct()
                        .filter(|c| names.contains(&c.name))
                        .cloned()
                        .collect(),
                    before: impact
                        .before
                        .iter()
                        .filter(|r| names.contains(&r.message))
                        .cloned()
                        .collect(),
                    after: impact
                        .after
                        .iter()
                        .filter(|r| names.contains(&r.message))
                        .cloned()
                        .collect(),
                    selected,
                    plan,
                    schema_impact: impact.clone(),
                }
            })
            .collect())
    }

    pub fn total_occurrences(&self) -> usize {
        self.plan.functions.iter().map(|f| f.exchanges.len()).sum()
    }
    pub fn oms_occurrences(&self) -> usize {
        self.plan
            .functions
            .iter()
            .flat_map(|f| &f.exchanges)
            .filter(|e| e.as_oms_message().is_some())
            .count()
    }
    pub fn direct_unique_messages(&self) -> usize {
        self.direct
            .iter()
            .map(|c| &c.name)
            .collect::<BTreeSet<_>>()
            .len()
    }

    fn rows(&self) -> Vec<Vec<String>> {
        let mut rows = Vec::new();
        let mut meta = |record: &str, side: &str, key: &str, value: Option<&str>| {
            let mut r = row(record, side);
            r[16] = key.into();
            r[17] = value.is_some().to_string();
            r[18] = value.unwrap_or("").into();
            rows.push(r);
        };
        for (key, value) in [
            ("format", "service-impact-v1"),
            ("analysis_scope", SCOPE),
            ("selection_basis", BASIS),
            ("limitations", LIMITATION),
            ("service_name", &self.plan.service.name),
            ("service_version", &self.plan.service.version),
            ("service_kind", self.plan.service.kind.as_str()),
            ("contract_version", &self.plan.contract_version),
            ("oms_version", &self.plan.standards.oms_version),
            (
                "uci_schema_version",
                &self.plan.standards.uci_schema_version,
            ),
        ] {
            meta("META", "", key, Some(value));
        }
        meta(
            "META",
            "",
            "ams_gra_version",
            self.plan.standards.ams_gra_version.as_deref(),
        );
        for id in &self.plan.standards.uci_extension_schemas {
            meta("META", "", "uci_extension_id", Some(id));
        }
        for (side, version, types, messages) in [
            (
                "BEFORE",
                &self.schema_impact.diff.before_version,
                self.schema_impact.diff.types.before,
                self.schema_impact.diff.messages.before,
            ),
            (
                "AFTER",
                &self.schema_impact.diff.after_version,
                self.schema_impact.diff.types.after,
                self.schema_impact.diff.messages.after,
            ),
        ] {
            meta("META", side, "schema_root_version", version.as_deref());
            meta(
                "COUNT",
                side,
                "whole_schema_types",
                Some(&types.to_string()),
            );
            meta(
                "COUNT",
                side,
                "whole_schema_messages",
                Some(&messages.to_string()),
            );
        }
        let missing = self
            .selected
            .iter()
            .filter(|s| s.status == "MISSING")
            .count();
        for (key, count) in [
            ("total_exchange_occurrences", self.total_occurrences()),
            ("oms_exchange_occurrences", self.oms_occurrences()),
            (
                "excluded_non_oms_occurrences",
                self.total_occurrences() - self.oms_occurrences(),
            ),
            ("selected_unique_messages", self.selected.len()),
            ("selected_present_in_after", self.selected.len() - missing),
            ("selected_missing_in_after", missing),
            (
                "direct_changed_selected_unique_messages",
                self.direct_unique_messages(),
            ),
        ] {
            meta("COUNT", "", key, Some(&count.to_string()));
        }
        for (side, exposures) in [("BEFORE", &self.before), ("AFTER", &self.after)] {
            meta(
                "COUNT",
                side,
                "exposed_selected_unique_messages",
                Some(&SchemaImpact::unique_messages(exposures).to_string()),
            );
            meta(
                "COUNT",
                side,
                "selected_message_type_pairs",
                Some(&exposures.len().to_string()),
            );
        }
        if self.selected.is_empty() {
            meta(
                "EMPTY_SELECTION",
                "",
                "selection",
                Some(
                    "No baseline-selected OMS messages; required inputs validated. Non-OMS exchanges were not assessed against UCI.",
                ),
            );
        }
        if self.direct.is_empty() && self.before.is_empty() && self.after.is_empty() {
            meta("EMPTY_IMPACT", "", "result", Some(EMPTY));
        }
        for f in &self.plan.functions {
            for e in &f.exchanges {
                if let Some(e) = e.as_oms_message() {
                    let mut r = row("OCCURRENCE", "BEFORE");
                    r[2] = f.id.clone();
                    r[3] = e.id.clone();
                    r[4] = e.direction.to_string();
                    r[5] = e.mandate.to_string();
                    r[6] = e.topic.clone();
                    r[7] = e.subscription_group.is_some().to_string();
                    r[8] = e.subscription_group.clone().unwrap_or_default();
                    qname(&mut r, 9, &e.message_name);
                    rows.push(r);
                }
            }
        }
        for s in &self.selected {
            let mut r = row("SELECTED_STATUS", "AFTER");
            qname(&mut r, 9, &s.message);
            r[16] = "direct_payload_status".into();
            r[17] = "true".into();
            r[18] = s.status.into();
            rows.push(r);
            for (side, payload) in [
                ("BEFORE", Some(&s.before_payload)),
                ("AFTER", s.after_payload.as_ref()),
            ] {
                let mut r = row("SELECTED_PAYLOAD", side);
                qname(&mut r, 9, &s.message);
                r[17] = payload.is_some().to_string();
                if let Some(payload) = payload {
                    match &payload.target {
                        TypeRefTarget::Named(name) => {
                            r[16] = "NAMED".into();
                            qname(&mut r, 14, name);
                        }
                        TypeRefTarget::Primitive(kind) => {
                            r[16] = "PRIMITIVE".into();
                            r[18] = format!("{kind:?}");
                        }
                    }
                    r[19] = payload.binary_encoding.is_some().to_string();
                    r[20] = payload
                        .binary_encoding
                        .map(|v| format!("{v:?}"))
                        .unwrap_or_default();
                }
                rows.push(r);
            }
        }
        for c in &self.direct {
            let mut r = row("DIRECT", "");
            qname(&mut r, 9, &c.name);
            r[16] = c.kind.label().into();
            r[17] = c.before.is_some().to_string();
            r[18] = c
                .before
                .as_ref()
                .map(|v| format!("{v:?}"))
                .unwrap_or_default();
            r[19] = c.after.is_some().to_string();
            r[20] = c
                .after
                .as_ref()
                .map(|v| format!("{v:?}"))
                .unwrap_or_default();
            rows.push(r);
        }
        for (side, exposures) in [("BEFORE", &self.before), ("AFTER", &self.after)] {
            for e in exposures {
                let mut r = row("EXPOSURE", side);
                qname(&mut r, 9, &e.message);
                qname(&mut r, 11, &e.seed);
                r[16] = "payload_to_seed".into();
                r[17] = "true".into();
                r[18] = self
                    .schema_impact
                    .diff
                    .changes
                    .iter()
                    .find(|c| c.category == Category::Type && c.name == e.seed)
                    .map(|c| match c.kind {
                        ChangeKind::TypeAdded => "ADDED",
                        ChangeKind::TypeRemoved => "REMOVED",
                        _ => "CHANGED",
                    })
                    .unwrap_or("CHANGED")
                    .into();
                rows.push(r);
                for (step, name) in e.witness.iter().enumerate() {
                    let mut r = row("WITNESS", side);
                    qname(&mut r, 9, &e.message);
                    qname(&mut r, 11, &e.seed);
                    r[13] = step.to_string();
                    qname(&mut r, 14, name);
                    rows.push(r);
                }
            }
        }
        rows
    }

    pub fn to_tsv(&self) -> String {
        let mut out = HEADER.to_owned();
        for mut r in self.rows() {
            // Explicit empty final cell keeps all columns without trailing tabs.
            if r[20].is_empty() {
                r[20] = "\"\"".into();
            }
            out.push_str(&r.iter().map(|v| escape(v)).collect::<Vec<_>>().join("\t"));
            out.push('\n');
        }
        out
    }
    pub fn to_text(&self) -> String {
        let columns: Vec<_> = HEADER.trim_end().split('\t').collect();
        let mut out = "service-impact v1\n".to_owned();
        for r in self.rows() {
            out.push_str(
                &r.iter()
                    .enumerate()
                    .filter(|(i, v)| {
                        !v.is_empty()
                            || (*i == 18 && r[17] == "true")
                            || (*i == 20 && r[19] == "true")
                            || (*i == 8 && r[7] == "true")
                    })
                    .map(|(i, v)| format!("{}={}", columns[i], escape(v)))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            out.push('\n');
        }
        out
    }
}
fn row(record: &str, side: &str) -> Vec<String> {
    let mut r = vec![String::new(); 21];
    r[0] = record.into();
    r[1] = side.into();
    r
}
fn qname(row: &mut [String], at: usize, name: &QualifiedName) {
    row[at] = name.namespace_uri.clone();
    row[at + 1] = name.local_name.clone();
}
fn escape(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '\\' => "\\\\".into(),
            '\t' => "\\t".into(),
            '\n' => "\\n".into(),
            '\r' => "\\r".into(),
            c if c.is_control() => format!("\\u{{{:x}}}", c as u32),
            c => c.to_string(),
        })
        .collect()
}
