//! Exact normalized-IR change inventory. No compatibility or transitive-impact claims.
//! Inputs must be validated SchemaIr models; declaration identity is the full QName.

use ams_gra_oms_ir::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Type,
    Message,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeKind {
    TypeAdded,
    TypeRemoved,
    TypeChanged,
    MessageAdded,
    MessageRemoved,
    MessageChanged,
    MemberAdded,
    MemberRemoved,
    MemberChanged,
    MemberOrderChanged,
    EnumValueAdded,
    EnumValueRemoved,
    EnumOrderChanged,
    ConstraintChanged,
    BaseTypeChanged,
    AbstractChanged,
}

impl ChangeKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::TypeAdded => "TYPE_ADDED",
            Self::TypeRemoved => "TYPE_REMOVED",
            Self::TypeChanged => "TYPE_CHANGED",
            Self::MessageAdded => "MESSAGE_ADDED",
            Self::MessageRemoved => "MESSAGE_REMOVED",
            Self::MessageChanged => "MESSAGE_CHANGED",
            Self::MemberAdded => "MEMBER_ADDED",
            Self::MemberRemoved => "MEMBER_REMOVED",
            Self::MemberChanged => "MEMBER_CHANGED",
            Self::MemberOrderChanged => "MEMBER_ORDER_CHANGED",
            Self::EnumValueAdded => "ENUM_VALUE_ADDED",
            Self::EnumValueRemoved => "ENUM_VALUE_REMOVED",
            Self::EnumOrderChanged => "ENUM_ORDER_CHANGED",
            Self::ConstraintChanged => "CONSTRAINT_CHANGED",
            Self::BaseTypeChanged => "BASE_TYPE_CHANGED",
            Self::AbstractChanged => "ABSTRACT_CHANGED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Property {
    Presence,
    Abstract,
    BaseType,
    Kind,
    Primitive,
    Target,
    Payload,
    ItemType,
    Cardinality,
    WireNamespace,
    Nillable,
    Order,
    MinInclusive,
    MaxInclusive,
    MinExclusive,
    MaxExclusive,
    Length,
    MinLength,
    MaxLength,
    WhiteSpace,
    Patterns,
}

impl Property {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Presence => "presence",
            Self::Abstract => "abstract",
            Self::BaseType => "base_type",
            Self::Kind => "kind",
            Self::Primitive => "primitive",
            Self::Target => "type_ref",
            Self::Payload => "payload",
            Self::ItemType => "item_type",
            Self::Cardinality => "cardinality",
            Self::WireNamespace => "wire_namespace",
            Self::Nillable => "nillable",
            Self::Order => "order",
            Self::MinInclusive => "minInclusive",
            Self::MaxInclusive => "maxInclusive",
            Self::MinExclusive => "minExclusive",
            Self::MaxExclusive => "maxExclusive",
            Self::Length => "length",
            Self::MinLength => "minLength",
            Self::MaxLength => "maxLength",
            Self::WhiteSpace => "whiteSpace",
            Self::Patterns => "pattern_groups",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindFamily {
    Primitive,
    Alias,
    Enumeration,
    Record,
    Choice,
    List,
}

fn family(kind: &TypeKind) -> KindFamily {
    match kind {
        TypeKind::Primitive(_) => KindFamily::Primitive,
        TypeKind::Alias(_) => KindFamily::Alias,
        TypeKind::Enumeration { .. } => KindFamily::Enumeration,
        TypeKind::Record { .. } => KindFamily::Record,
        TypeKind::Choice { .. } => KindFamily::Choice,
        TypeKind::List { .. } => KindFamily::List,
    }
}

/// Typed semantic values, never diagnostic strings or raw TypeDecl snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    Kind(KindFamily),
    Primitive(PrimitiveKind),
    Reference(TypeRef),
    Cardinality(Cardinality),
    Text(String),
    Order(Vec<String>),
    Numeric(NumericValue),
    Unsigned(u64),
    WhiteSpace(WhiteSpacePolicy),
    Patterns(Vec<PatternGroup>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub category: Category,
    pub name: QualifiedName,
    /// A local member name or exact enum wire value; None means declaration-wide.
    pub member: Option<String>,
    pub property: Property,
    pub kind: ChangeKind,
    /// None means absent (distinct from an empty string).
    pub before: Option<Value>,
    pub after: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Counts {
    pub before: usize,
    pub after: usize,
    pub added: usize,
    pub removed: usize,
    pub changed: usize,
    pub unchanged: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDiff {
    pub before_version: Option<String>,
    pub after_version: Option<String>,
    /// Declaration counts: each common declaration counts once, regardless of record count.
    pub types: Counts,
    pub messages: Counts,
    /// Property records; no redundant coarse TYPE_CHANGED record for fine-grained changes.
    pub changes: Vec<Change>,
}

struct Collector<'a> {
    changes: &'a mut Vec<Change>,
    category: Category,
    name: &'a QualifiedName,
    member: Option<String>,
}

impl Collector<'_> {
    fn emit(
        &mut self,
        property: Property,
        kind: ChangeKind,
        before: Option<Value>,
        after: Option<Value>,
    ) {
        if before != after {
            self.changes.push(Change {
                category: self.category,
                name: self.name.clone(),
                member: self.member.clone(),
                property,
                kind,
                before,
                after,
            });
        }
    }

    fn constraints(&mut self, a: &ConstraintSet, b: &ConstraintSet) {
        use Property::*;
        for (p, a, b) in [
            (MinInclusive, a.min_inclusive, b.min_inclusive),
            (MaxInclusive, a.max_inclusive, b.max_inclusive),
            (MinExclusive, a.min_exclusive, b.min_exclusive),
            (MaxExclusive, a.max_exclusive, b.max_exclusive),
        ] {
            self.emit(
                p,
                ChangeKind::ConstraintChanged,
                a.map(Value::Numeric),
                b.map(Value::Numeric),
            );
        }
        for (p, a, b) in [
            (Length, a.length, b.length),
            (MinLength, a.min_length, b.min_length),
            (MaxLength, a.max_length, b.max_length),
        ] {
            self.emit(
                p,
                ChangeKind::ConstraintChanged,
                a.map(Value::Unsigned),
                b.map(Value::Unsigned),
            );
        }
        self.emit(
            WhiteSpace,
            ChangeKind::ConstraintChanged,
            a.lexical.white_space.map(Value::WhiteSpace),
            b.lexical.white_space.map(Value::WhiteSpace),
        );
        self.emit(
            Patterns,
            ChangeKind::ConstraintChanged,
            Some(Value::Patterns(a.lexical.pattern_groups.clone())),
            Some(Value::Patterns(b.lexical.pattern_groups.clone())),
        );
    }

    fn order(&mut self, a: Vec<String>, b: Vec<String>, kind: ChangeKind) {
        let aset: BTreeSet<_> = a.iter().collect();
        let bset: BTreeSet<_> = b.iter().collect();
        let old_common: Vec<_> = a.iter().filter(|v| bset.contains(v)).collect();
        let new_common: Vec<_> = b.iter().filter(|v| aset.contains(v)).collect();
        if old_common != new_common {
            self.emit(
                Property::Order,
                kind,
                Some(Value::Order(a)),
                Some(Value::Order(b)),
            );
        }
    }

    fn members(&mut self, a: &[FieldDecl], b: &[FieldDecl]) {
        let ai: BTreeMap<_, _> = a.iter().map(|f| (&f.name, f)).collect();
        let bi: BTreeMap<_, _> = b.iter().map(|f| (&f.name, f)).collect();
        for name in ai.keys().chain(bi.keys()).copied().collect::<BTreeSet<_>>() {
            self.member = Some(name.clone());
            match (ai.get(name), bi.get(name)) {
                (None, Some(_)) => self.emit(
                    Property::Presence,
                    ChangeKind::MemberAdded,
                    None,
                    Some(Value::Bool(true)),
                ),
                (Some(_), None) => self.emit(
                    Property::Presence,
                    ChangeKind::MemberRemoved,
                    Some(Value::Bool(true)),
                    None,
                ),
                (Some(a), Some(b)) => {
                    self.emit(
                        Property::Target,
                        ChangeKind::MemberChanged,
                        Some(Value::Reference(a.type_ref.clone())),
                        Some(Value::Reference(b.type_ref.clone())),
                    );
                    self.emit(
                        Property::Cardinality,
                        ChangeKind::MemberChanged,
                        Some(Value::Cardinality(a.cardinality)),
                        Some(Value::Cardinality(b.cardinality)),
                    );
                    self.emit(
                        Property::Nillable,
                        ChangeKind::MemberChanged,
                        Some(Value::Bool(a.nillable)),
                        Some(Value::Bool(b.nillable)),
                    );
                    self.emit(
                        Property::WireNamespace,
                        ChangeKind::MemberChanged,
                        a.wire_namespace_uri.clone().map(Value::Text),
                        b.wire_namespace_uri.clone().map(Value::Text),
                    );
                    self.constraints(&a.constraints, &b.constraints);
                }
                (None, None) => unreachable!(),
            }
        }
        self.member = None;
        self.order(
            a.iter().map(|f| f.name.clone()).collect(),
            b.iter().map(|f| f.name.clone()).collect(),
            ChangeKind::MemberOrderChanged,
        );
    }

    fn declaration(&mut self, a: &TypeDecl, b: &TypeDecl) {
        self.emit(
            Property::Abstract,
            ChangeKind::AbstractChanged,
            Some(Value::Bool(a.is_abstract)),
            Some(Value::Bool(b.is_abstract)),
        );
        self.emit(
            Property::BaseType,
            ChangeKind::BaseTypeChanged,
            a.base_type.clone().map(Value::Reference),
            b.base_type.clone().map(Value::Reference),
        );
        self.constraints(&a.constraints, &b.constraints);
        if family(&a.kind) != family(&b.kind) {
            self.emit(
                Property::Kind,
                ChangeKind::TypeChanged,
                Some(Value::Kind(family(&a.kind))),
                Some(Value::Kind(family(&b.kind))),
            );
            return;
        }
        match (&a.kind, &b.kind) {
            (TypeKind::Primitive(a), TypeKind::Primitive(b)) => self.emit(
                Property::Primitive,
                ChangeKind::TypeChanged,
                Some(Value::Primitive(*a)),
                Some(Value::Primitive(*b)),
            ),
            (TypeKind::Alias(a), TypeKind::Alias(b)) => self.emit(
                Property::Target,
                ChangeKind::TypeChanged,
                Some(Value::Reference(a.clone())),
                Some(Value::Reference(b.clone())),
            ),
            (TypeKind::Record { fields: a }, TypeKind::Record { fields: b })
            | (TypeKind::Choice { alternatives: a }, TypeKind::Choice { alternatives: b }) => {
                self.members(a, b)
            }
            (TypeKind::Enumeration { variants: a }, TypeKind::Enumeration { variants: b }) => {
                let aset: BTreeSet<_> = a.iter().map(|v| &v.wire_value).collect();
                let bset: BTreeSet<_> = b.iter().map(|v| &v.wire_value).collect();
                for value in aset.symmetric_difference(&bset) {
                    self.member = Some((*value).clone());
                    let present = Some(Value::Text((*value).clone()));
                    if aset.contains(value) {
                        self.emit(
                            Property::Presence,
                            ChangeKind::EnumValueRemoved,
                            present,
                            None,
                        );
                    } else {
                        self.emit(
                            Property::Presence,
                            ChangeKind::EnumValueAdded,
                            None,
                            present,
                        );
                    }
                }
                self.member = None;
                self.order(
                    a.iter().map(|v| v.wire_value.clone()).collect(),
                    b.iter().map(|v| v.wire_value.clone()).collect(),
                    ChangeKind::EnumOrderChanged,
                );
            }
            (
                TypeKind::List {
                    item_type: a,
                    cardinality: ac,
                },
                TypeKind::List {
                    item_type: b,
                    cardinality: bc,
                },
            ) => {
                self.emit(
                    Property::ItemType,
                    ChangeKind::TypeChanged,
                    Some(Value::Reference(a.clone())),
                    Some(Value::Reference(b.clone())),
                );
                self.emit(
                    Property::Cardinality,
                    ChangeKind::TypeChanged,
                    Some(Value::Cardinality(*ac)),
                    Some(Value::Cardinality(*bc)),
                );
            }
            _ => unreachable!("equal kind families"),
        }
    }
}

/// Compare validated normalized IR. Builds four indexes once, with O(n log n) matching.
#[must_use]
pub fn compare_schemas(before: &SchemaIr, after: &SchemaIr) -> SchemaDiff {
    let bt: BTreeMap<_, _> = before.types.iter().map(|d| (&d.name, d)).collect();
    let at: BTreeMap<_, _> = after.types.iter().map(|d| (&d.name, d)).collect();
    let bm: BTreeMap<_, _> = before.messages.iter().map(|d| (&d.name, d)).collect();
    let am: BTreeMap<_, _> = after.messages.iter().map(|d| (&d.name, d)).collect();
    let mut diff = SchemaDiff {
        before_version: before.schema_version.clone(),
        after_version: after.schema_version.clone(),
        types: Counts {
            before: bt.len(),
            after: at.len(),
            ..Counts::default()
        },
        messages: Counts {
            before: bm.len(),
            after: am.len(),
            ..Counts::default()
        },
        changes: Vec::new(),
    };
    for name in bt.keys().chain(at.keys()).copied().collect::<BTreeSet<_>>() {
        let start = diff.changes.len();
        let mut c = Collector {
            changes: &mut diff.changes,
            category: Category::Type,
            name,
            member: None,
        };
        match (bt.get(name), at.get(name)) {
            (None, Some(b)) => {
                diff.types.added += 1;
                c.emit(
                    Property::Presence,
                    ChangeKind::TypeAdded,
                    None,
                    Some(Value::Kind(family(&b.kind))),
                );
            }
            (Some(a), None) => {
                diff.types.removed += 1;
                c.emit(
                    Property::Presence,
                    ChangeKind::TypeRemoved,
                    Some(Value::Kind(family(&a.kind))),
                    None,
                );
            }
            (Some(a), Some(b)) => {
                c.declaration(a, b);
                if diff.changes.len() == start {
                    diff.types.unchanged += 1;
                } else {
                    diff.types.changed += 1;
                }
            }
            (None, None) => unreachable!(),
        }
    }
    for name in bm.keys().chain(am.keys()).copied().collect::<BTreeSet<_>>() {
        let mut c = Collector {
            changes: &mut diff.changes,
            category: Category::Message,
            name,
            member: None,
        };
        match (bm.get(name), am.get(name)) {
            (None, Some(b)) => {
                diff.messages.added += 1;
                c.emit(
                    Property::Presence,
                    ChangeKind::MessageAdded,
                    None,
                    Some(Value::Reference(b.payload_type.clone())),
                );
            }
            (Some(a), None) => {
                diff.messages.removed += 1;
                c.emit(
                    Property::Presence,
                    ChangeKind::MessageRemoved,
                    Some(Value::Reference(a.payload_type.clone())),
                    None,
                );
            }
            (Some(a), Some(b)) => {
                if a.payload_type == b.payload_type {
                    diff.messages.unchanged += 1;
                } else {
                    diff.messages.changed += 1;
                    c.emit(
                        Property::Payload,
                        ChangeKind::MessageChanged,
                        Some(Value::Reference(a.payload_type.clone())),
                        Some(Value::Reference(b.payload_type.clone())),
                    );
                }
            }
            (None, None) => unreachable!(),
        }
    }
    diff.changes.sort_by(|a, b| {
        (a.category, &a.name, &a.member, a.property.label(), a.kind).cmp(&(
            b.category,
            &b.name,
            &b.member,
            b.property.label(),
            b.kind,
        ))
    });
    diff
}

fn qname(name: &QualifiedName) -> String {
    format!("{{{}}}{}", name.namespace_uri, name.local_name)
}

fn value(value: &Option<Value>) -> String {
    match value {
        None => "absent".into(),
        Some(Value::Reference(r)) => {
            let target = match &r.target {
                TypeRefTarget::Named(n) => format!("named:{}", qname(n)),
                TypeRefTarget::Primitive(p) => format!("primitive:{p:?}"),
            };
            format!("{target};binary={:?}", r.binary_encoding)
        }
        Some(Value::Text(s)) => format!("{s:?}"),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Kind(k)) => format!("{k:?}"),
        Some(Value::Primitive(p)) => format!("{p:?}"),
        Some(Value::Cardinality(c)) => format!(
            "{}..{}",
            c.min_occurs,
            c.max_occurs
                .map_or_else(|| "unbounded".into(), |v| v.to_string())
        ),
        Some(Value::Unsigned(n)) => n.to_string(),
        Some(Value::Numeric(n)) => format!("{n:?}"),
        Some(Value::WhiteSpace(w)) => format!("{w:?}"),
        Some(Value::Order(o)) => format!("{o:?}"),
        Some(Value::Patterns(p)) => format!("{p:?}"),
    }
}

/// Escape every control character, backslash, tab and newline into a single TSV cell.
#[must_use]
pub fn escape_tsv(input: &str) -> String {
    let mut output = String::new();
    for c in input.chars() {
        match c {
            '\\' => output.push_str("\\\\"),
            '\t' => output.push_str("\\t"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            c if c.is_control() => output.push_str(&format!("\\u{{{:x}}}", u32::from(c))),
            c => output.push(c),
        }
    }
    output
}

impl SchemaDiff {
    /// Compact deterministic metadata, declaration counts, and property-record totals.
    #[must_use]
    pub fn summary_tsv(&self) -> String {
        let mut out = String::from(
            "category\tnamespace\tdeclaration\tmember_or_property\tchange\tbefore\tafter\n",
        );
        out.push_str(&format!(
            "METADATA\t\t\tschema_version\tVALUE\t{}\t{}\n",
            escape_tsv(&value(&self.before_version.clone().map(Value::Text))),
            escape_tsv(&value(&self.after_version.clone().map(Value::Text)))
        ));
        for (label, c) in [("TYPE", &self.types), ("MESSAGE", &self.messages)] {
            out.push_str(&format!(
                "SUMMARY\t\t{label}\tcount\tVALUE\t{}\t{}\n",
                c.before, c.after
            ));
            for (key, n) in [
                ("added", c.added),
                ("removed", c.removed),
                ("changed", c.changed),
                ("unchanged", c.unchanged),
            ] {
                out.push_str(&format!("SUMMARY\t\t{label}\t{key}\tCOUNT\t\t{n}\n"));
            }
        }
        let mut totals = BTreeMap::new();
        for c in &self.changes {
            *totals.entry(c.kind.label()).or_insert(0usize) += 1;
        }
        for (kind, n) in totals {
            out.push_str(&format!("TOTAL\t\t\t\t{kind}\t\t{n}\n"));
        }
        out
    }

    #[must_use]
    pub fn to_tsv(&self) -> String {
        let mut out = self.summary_tsv();
        for c in &self.changes {
            let category = match c.category {
                Category::Type => "TYPE",
                Category::Message => "MESSAGE",
            };
            let property = match &c.member {
                Some(m) => format!("{m:?}/{}", c.property.label()),
                None => c.property.label().into(),
            };
            let cells = [
                category.to_owned(),
                c.name.namespace_uri.clone(),
                c.name.local_name.clone(),
                property,
                c.kind.label().into(),
                value(&c.before),
                value(&c.after),
            ];
            out.push_str(
                &cells
                    .iter()
                    .map(|v| escape_tsv(v))
                    .collect::<Vec<_>>()
                    .join("\t"),
            );
            out.push('\n');
        }
        out
    }

    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = format!(
            "Semantic schema diff (exact IR inventory; no compatibility claim)\nBefore schema version: {}\nAfter schema version: {}\n",
            value(&self.before_version.clone().map(Value::Text)),
            value(&self.after_version.clone().map(Value::Text))
        );
        for (label, c) in [("Types", &self.types), ("Messages", &self.messages)] {
            out.push_str(&format!(
                "{label}: before {}, after {}; added {}, removed {}, changed {}, unchanged {}\n",
                c.before, c.after, c.added, c.removed, c.changed, c.unchanged
            ));
        }
        for c in &self.changes {
            out.push_str(&format!(
                "{} {} {}{}: {} -> {}\n",
                c.kind.label(),
                escape_tsv(&qname(&c.name)),
                c.member
                    .as_ref()
                    .map_or_else(String::new, |m| format!("{m:?}/")),
                c.property.label(),
                escape_tsv(&value(&c.before)),
                escape_tsv(&value(&c.after))
            ));
        }
        out
    }
}
