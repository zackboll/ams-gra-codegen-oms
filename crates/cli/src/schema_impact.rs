//! Independent schema-impact v1 output contract.
use ams_gra_oms_codegen_core::{
    DependencyExposure, DependencyImpactError, static_dependency_exposures,
};
use ams_gra_oms_ir::{QualifiedName, SchemaIr};
use ams_gra_oms_schema_diff::{Category, Change, ChangeKind, SchemaDiff, compare_schemas};
use std::collections::BTreeSet;

pub const SCOPE: &str = "STATIC_NAMED_REFERENCES";
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
