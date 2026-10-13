//! Independent schema-impact v1 output contract.
use ams_gra_oms_codegen_core::{
    DependencyExposure, DependencyImpactError, static_dependency_exposures,
};
use ams_gra_oms_ir::{
    BinaryLexicalEncoding, PrimitiveKind, QualifiedName, SchemaIr, TypeRef, TypeRefTarget,
};
use ams_gra_oms_schema_diff::{Category, Change, ChangeKind, SchemaDiff, compare_schemas};
use serde::Serialize;
use std::collections::BTreeSet;

pub const SCOPE: &str = "STATIC_NAMED_REFERENCES";
pub const HELP: &str = "schema-impact --before PATH --after PATH [--before-overlay PATH]... [--after-overlay PATH]... [--format text|tsv|json]\n\nRead-only STATIC_NAMED_REFERENCES evidence: direct changes and snapshot-isolated\nexposures with shortest deterministic witnesses. Default: text. JSON contract:\nschema-impact-json-v1, compatibility NOT_ASSESSED. No backward/forward, wire,\nruntime, generated API, safe deployment, service readiness or open-extension\ncompleteness verdict; no reverse inheritance expansion. Overlays retain caller\norder independently. Exit codes: success (including findings) 0, execution 1, usage 2.\n";
pub const HEADER: &str = "record\tside\tmessage_namespace\tmessage_local\tseed_namespace\tseed_local\tchange\tstep\ttype_namespace\ttype_local\tvalue\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaImpact {
    pub diff: SchemaDiff,
    pub before: Vec<DependencyExposure>,
    pub after: Vec<DependencyExposure>,
}

impl SchemaImpact {
    /// Loaders must supply semantic IR; invalid snapshots fail before comparison.
    pub fn analyze(before: &SchemaIr, after: &SchemaIr) -> Result<Self, DependencyImpactError> {
        before
            .validate()
            .map_err(DependencyImpactError::InvalidSchema)?;
        after
            .validate()
            .map_err(DependencyImpactError::InvalidSchema)?;
        let diff = compare_schemas(before, after);
        let names: BTreeSet<_> = diff
            .changes
            .iter()
            .filter(|c| c.category == Category::Type)
            .map(|c| c.name.clone())
            .collect();
        let seeds = |s: &SchemaIr| {
            s.types
                .iter()
                .filter(|t| names.contains(&t.name))
                .map(|t| t.name.clone())
                .collect()
        };
        Ok(Self {
            before: static_dependency_exposures(before, &seeds(before))?,
            after: static_dependency_exposures(after, &seeds(after))?,
            diff,
        })
    }

    pub fn direct(&self) -> impl Iterator<Item = &Change> {
        self.diff
            .changes
            .iter()
            .filter(|c| c.category == Category::Message)
    }

    pub fn unique_messages(rows: &[DependencyExposure]) -> usize {
        rows.iter()
            .map(|r| &r.message)
            .collect::<BTreeSet<_>>()
            .len()
    }

    fn seed_change(&self, seed: &QualifiedName) -> &'static str {
        for c in &self.diff.changes {
            if c.category == Category::Type && &c.name == seed {
                match c.kind {
                    ChangeKind::TypeAdded => return "ADDED",
                    ChangeKind::TypeRemoved => return "REMOVED",
                    _ => {}
                }
            }
        }
        "CHANGED"
    }

    fn json_side<'a>(
        &'a self,
        version: Option<&'a str>,
        types: usize,
        messages: usize,
        rows: &'a [DependencyExposure],
    ) -> JsonSide<'a> {
        JsonSide {
            schema_version: version,
            types,
            messages,
            exposed_unique_messages: Self::unique_messages(rows),
            exposure_pairs: rows.len(),
            exposures: rows
                .iter()
                .map(|r| JsonExposure {
                    message: qname(&r.message),
                    seed: qname(&r.seed),
                    seed_change: self.seed_change(&r.seed),
                    witness: r.witness.iter().map(qname).collect(),
                })
                .collect(),
        }
    }

    /// Serialize typed Task 069/078 facts as schema-impact-json-v1, then one LF.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let report = JsonReport {
            format: "schema-impact-json-v1",
            scope: SCOPE,
            compatibility: "NOT_ASSESSED",
            before: self.json_side(
                self.diff.before_version.as_deref(),
                self.diff.types.before,
                self.diff.messages.before,
                &self.before,
            ),
            after: self.json_side(
                self.diff.after_version.as_deref(),
                self.diff.types.after,
                self.diff.messages.after,
                &self.after,
            ),
            direct_unique_messages: self
                .direct()
                .map(|c| &c.name)
                .collect::<BTreeSet<_>>()
                .len(),
            direct_message_changes: self
                .direct()
                .map(|c| {
                    Ok(JsonDirect {
                        message: qname(&c.name),
                        change: c.kind.label(),
                        before: json_payload(&c.before)?,
                        after: json_payload(&c.after)?,
                    })
                })
                .collect::<Result<_, serde_json::Error>>()?,
        };
        let mut output = serde_json::to_string(&report)?;
        output.push('\n');
        Ok(output)
    }

    pub fn to_tsv(&self) -> String {
        let mut out = HEADER.to_owned();
        let mut row = |cells: [&str; 11]| {
            out.push_str(&cells.map(escape).join("\t"));
            out.push('\n');
        };
        row([
            "FORMAT",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "schema-impact-v1",
        ]);
        row(["SCOPE", "", "", "", "", "", "", "", "", "", SCOPE]);
        for (side, version, types, messages, rows) in [
            (
                "BEFORE",
                &self.diff.before_version,
                self.diff.types.before,
                self.diff.messages.before,
                &self.before,
            ),
            (
                "AFTER",
                &self.diff.after_version,
                self.diff.types.after,
                self.diff.messages.after,
                &self.after,
            ),
        ] {
            row([
                "SCHEMA",
                side,
                "",
                "",
                "",
                "",
                "version",
                "",
                "",
                "",
                version.as_deref().unwrap_or(""),
            ]);
            for (key, count) in [
                ("types", types),
                ("messages", messages),
                ("exposed_unique_messages", Self::unique_messages(rows)),
                ("exposure_pairs", rows.len()),
            ] {
                row([
                    "COUNT",
                    side,
                    "",
                    "",
                    "",
                    "",
                    key,
                    "",
                    "",
                    "",
                    &count.to_string(),
                ]);
            }
        }
        row([
            "COUNT",
            "",
            "",
            "",
            "",
            "",
            "direct_unique_messages",
            "",
            "",
            "",
            &self
                .direct()
                .map(|c| &c.name)
                .collect::<BTreeSet<_>>()
                .len()
                .to_string(),
        ]);
        for c in self.direct() {
            row([
                "DIRECT",
                "",
                &c.name.namespace_uri,
                &c.name.local_name,
                "",
                "",
                c.kind.label(),
                "",
                "",
                "",
                &format!("{:?} -> {:?}", c.before, c.after),
            ]);
        }
        for (side, rows) in [("BEFORE", &self.before), ("AFTER", &self.after)] {
            for r in rows {
                row([
                    "EXPOSURE",
                    side,
                    &r.message.namespace_uri,
                    &r.message.local_name,
                    &r.seed.namespace_uri,
                    &r.seed.local_name,
                    self.seed_change(&r.seed),
                    "",
                    "",
                    "",
                    "message payload -> witness step 0",
                ]);
                for (step, name) in r.witness.iter().enumerate() {
                    row([
                        "WITNESS",
                        side,
                        &r.message.namespace_uri,
                        &r.message.local_name,
                        &r.seed.namespace_uri,
                        &r.seed.local_name,
                        "",
                        &step.to_string(),
                        &name.namespace_uri,
                        &name.local_name,
                        "",
                    ]);
                }
            }
        }
        out
    }

    pub fn to_text(&self) -> String {
        let mut out = format!(
            "schema-impact v1\nScope: {SCOPE}\nNot a compatibility or safe-upgrade verdict.\n"
        );
        for (side, version, types, messages, rows) in [
            (
                "BEFORE",
                &self.diff.before_version,
                self.diff.types.before,
                self.diff.messages.before,
                &self.before,
            ),
            (
                "AFTER",
                &self.diff.after_version,
                self.diff.types.after,
                self.diff.messages.after,
                &self.after,
            ),
        ] {
            out.push_str(&format!("{side}: version={version:?}, types={types}, messages={messages}\n{side}: exposed unique messages={}, message/type pairs={}\n", Self::unique_messages(rows), rows.len()));
        }
        out.push_str(&format!(
            "Directly changed unique messages: {}\n",
            self.direct()
                .map(|c| &c.name)
                .collect::<BTreeSet<_>>()
                .len()
        ));
        for c in self.direct() {
            out.push_str(&format!(
                "DIRECT {} {}: {:?} -> {:?}\n",
                c.kind.label(),
                display(&c.name),
                c.before,
                c.after
            ));
        }
        for (side, rows) in [("BEFORE", &self.before), ("AFTER", &self.after)] {
            for r in rows {
                out.push_str(&format!(
                    "{side} {} payload -> {} ({} seed {})\n",
                    display(&r.message),
                    r.witness
                        .iter()
                        .map(display)
                        .collect::<Vec<_>>()
                        .join(" -> "),
                    self.seed_change(&r.seed),
                    display(&r.seed)
                ));
            }
        }
        if self.direct().next().is_none() && self.before.is_empty() && self.after.is_empty() {
            out.push_str("No static dependency impact found.\n");
        }
        out
    }
}

fn json_payload(
    value: &Option<ams_gra_oms_schema_diff::Value>,
) -> Result<Option<JsonReference<'_>>, serde_json::Error> {
    match value {
        None => Ok(None),
        Some(ams_gra_oms_schema_diff::Value::Reference(r)) => Ok(Some(JsonReference::from(r))),
        Some(_) => Err(<serde_json::Error as serde::ser::Error>::custom(
            "direct message payload is not a typed reference",
        )),
    }
}

fn qname(name: &QualifiedName) -> [&str; 2] {
    [&name.namespace_uri, &name.local_name]
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonReport<'a> {
    format: &'static str,
    scope: &'static str,
    compatibility: &'static str,
    before: JsonSide<'a>,
    after: JsonSide<'a>,
    direct_unique_messages: usize,
    direct_message_changes: Vec<JsonDirect<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonSide<'a> {
    schema_version: Option<&'a str>,
    types: usize,
    messages: usize,
    exposed_unique_messages: usize,
    exposure_pairs: usize,
    exposures: Vec<JsonExposure<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonExposure<'a> {
    message: [&'a str; 2],
    seed: [&'a str; 2],
    seed_change: &'static str,
    witness: Vec<[&'a str; 2]>,
}

#[derive(Serialize)]
struct JsonDirect<'a> {
    message: [&'a str; 2],
    change: &'static str,
    before: Option<JsonReference<'a>>,
    after: Option<JsonReference<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonReference<'a> {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    qname: Option<[&'a str; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    primitive: Option<&'static str>,
    binary_encoding: Option<&'static str>,
}

impl<'a> From<&'a TypeRef> for JsonReference<'a> {
    fn from(reference: &'a TypeRef) -> Self {
        let (kind, qname, primitive) = match &reference.target {
            TypeRefTarget::Named(n) => ("named", Some(qname(n)), None),
            TypeRefTarget::Primitive(p) => (
                "primitive",
                None,
                Some(match p {
                    PrimitiveKind::Boolean => "Boolean",
                    PrimitiveKind::SignedInteger => "SignedInteger",
                    PrimitiveKind::UnsignedInteger => "UnsignedInteger",
                    PrimitiveKind::Decimal => "Decimal",
                    PrimitiveKind::Float32 => "Float32",
                    PrimitiveKind::Float64 => "Float64",
                    PrimitiveKind::String => "String",
                    PrimitiveKind::Binary => "Binary",
                    PrimitiveKind::DateTime => "DateTime",
                    PrimitiveKind::Time => "Time",
                    PrimitiveKind::Duration => "Duration",
                }),
            ),
        };
        Self {
            kind,
            qname,
            primitive,
            binary_encoding: reference.binary_encoding.map(|e| match e {
                BinaryLexicalEncoding::HexBinary => "HexBinary",
                BinaryLexicalEncoding::Base64Binary => "Base64Binary",
            }),
        }
    }
}

fn display(name: &QualifiedName) -> String {
    format!(
        "{{{}}}{}",
        escape(&name.namespace_uri),
        escape(&name.local_name)
    )
}
fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_control_character_is_escaped() {
        assert_eq!(escape("a\t\n\r\\b"), "a\\t\\n\\r\\\\b");
    }
}
