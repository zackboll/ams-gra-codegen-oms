//! Language-neutral semantic schema IR for OMS/UCI code generation.
//!
//! This crate intentionally contains no XML/XSD parser logic and no
//! language-specific code-generation policy.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QualifiedName {
    pub namespace_uri: String,
    pub local_name: String,
}

impl QualifiedName {
    #[must_use]
    pub fn new(namespace_uri: impl Into<String>, local_name: impl Into<String>) -> Self {
        Self {
            namespace_uri: namespace_uri.into(),
            local_name: local_name.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaIr {
    pub schema_version: Option<String>,
    pub namespaces: Vec<NamespaceDecl>,
    pub types: Vec<TypeDecl>,
    pub messages: Vec<MessageDecl>,
}

impl SchemaIr {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            schema_version: None,
            namespaces: Vec::new(),
            types: Vec::new(),
            messages: Vec::new(),
        }
    }

    /// Validate the language-neutral semantic invariants of this schema.
    ///
    /// # Errors
    ///
    /// Returns the first semantic inconsistency in deterministic IR order.
    pub fn validate(&self) -> Result<(), ValidationError> {
        let mut namespaces = BTreeSet::new();
        for namespace in &self.namespaces {
            if !namespaces.insert(namespace.uri.clone()) {
                return Err(ValidationError::DuplicateNamespace {
                    uri: namespace.uri.clone(),
                });
            }
        }

        let mut declared_types = BTreeSet::new();
        for declaration in &self.types {
            if !namespaces.contains(&declaration.name.namespace_uri) {
                return Err(ValidationError::UndeclaredNamespace {
                    declaration: "type",
                    name: declaration.name.clone(),
                });
            }
            if !declared_types.insert(declaration.name.clone()) {
                return Err(ValidationError::DuplicateType {
                    name: declaration.name.clone(),
                    source: declaration.source.clone(),
                });
            }
        }
        for message in &self.messages {
            if !namespaces.contains(&message.name.namespace_uri) {
                return Err(ValidationError::UndeclaredNamespace {
                    declaration: "message",
                    name: message.name.clone(),
                });
            }
        }

        let mut declared_messages = BTreeSet::new();
        for message in &self.messages {
            if !declared_messages.insert(message.name.clone()) {
                return Err(ValidationError::DuplicateMessage {
                    name: message.name.clone(),
                    source: message.source.clone(),
                });
            }
        }

        for declaration in &self.types {
            let owner = format_name(&declaration.name);
            let primitive = match declaration.kind {
                TypeKind::Primitive(kind) => Some(kind),
                _ => None,
            };
            validate_constraints(&declaration.constraints, &owner, primitive)?;
            if let Some(base_type) = &declaration.base_type {
                validate_reference(base_type, &declared_types, "base type", &declaration.source)?;
                if matches!(
                    declaration.kind,
                    TypeKind::Record { .. } | TypeKind::Choice { .. }
                ) && !matches!(base_type.target, TypeRefTarget::Named(_))
                {
                    return Err(ValidationError::InvalidStructuralBase {
                        name: declaration.name.clone(),
                    });
                }
            }
            match &declaration.kind {
                TypeKind::Primitive(_) => {}
                TypeKind::Alias(target) => validate_reference(
                    target,
                    &declared_types,
                    "alias target",
                    &declaration.source,
                )?,
                TypeKind::Enumeration { variants } => {
                    if variants.is_empty() {
                        return Err(ValidationError::EmptyEnumeration {
                            name: declaration.name.clone(),
                        });
                    }
                }
                TypeKind::Record { fields } => {
                    validate_fields(fields, &declared_types, "record field")?;
                }
                TypeKind::Choice { alternatives } => {
                    if alternatives.is_empty() {
                        return Err(ValidationError::EmptyChoice {
                            name: declaration.name.clone(),
                        });
                    }
                    validate_fields(alternatives, &declared_types, "choice alternative")?;
                }
                TypeKind::List {
                    item_type,
                    cardinality,
                } => {
                    validate_reference(
                        item_type,
                        &declared_types,
                        "list item",
                        &declaration.source,
                    )?;
                    validate_cardinality(*cardinality, &format!("list {owner}"))?;
                }
            }
        }

        validate_structural_inheritance(self)?;
        validate_named_simple_restrictions(self)?;

        for message in &self.messages {
            validate_reference(
                &message.payload_type,
                &declared_types,
                "message payload",
                &message.source,
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    DuplicateNamespace {
        uri: String,
    },
    UndeclaredNamespace {
        declaration: &'static str,
        name: QualifiedName,
    },
    DuplicateType {
        name: QualifiedName,
        source: SourceRef,
    },
    DuplicateMessage {
        name: QualifiedName,
        source: SourceRef,
    },
    UnresolvedTypeReference {
        target: QualifiedName,
        location: &'static str,
        source: SourceRef,
    },
    InvalidCardinality {
        owner: String,
        min_occurs: u64,
        max_occurs: u64,
    },
    ContradictoryNumericRange {
        owner: String,
    },
    IncomparableNumericRange {
        owner: String,
    },
    NonFiniteNumericRange {
        owner: String,
    },
    ContradictoryLengthConstraints {
        owner: String,
    },
    EmptyPatternGroup {
        owner: String,
    },
    InvalidWhiteSpacePolicy {
        owner: String,
        primitive: PrimitiveKind,
        policy: WhiteSpacePolicy,
    },
    EmptyEnumeration {
        name: QualifiedName,
    },
    EmptyChoice {
        name: QualifiedName,
    },
    InvalidStructuralBase {
        name: QualifiedName,
    },
    NonStructuralBase {
        name: QualifiedName,
        base: QualifiedName,
    },
    InheritanceCycle {
        names: Vec<QualifiedName>,
    },
    InvalidNamedRestrictionBase {
        name: QualifiedName,
        base: QualifiedName,
    },
    NamedRestrictionWeakensBase {
        name: QualifiedName,
        base: QualifiedName,
    },
    NamedRestrictionCycle {
        names: Vec<QualifiedName>,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateNamespace { uri } => write!(f, "duplicate namespace URI {uri}"),
            Self::UndeclaredNamespace { declaration, name } => write!(
                f,
                "{declaration} {} uses undeclared namespace {}",
                format_name(name),
                name.namespace_uri
            ),
            Self::DuplicateType { name, source } => write!(
                f,
                "duplicate type declaration {} at {}",
                format_name(name),
                format_source(source)
            ),
            Self::DuplicateMessage { name, source } => write!(
                f,
                "duplicate message declaration {} at {}",
                format_name(name),
                format_source(source)
            ),
            Self::UnresolvedTypeReference {
                target,
                location,
                source,
            } => write!(
                f,
                "unresolved type reference {} in {location} at {}",
                format_name(target),
                format_source(source)
            ),
            Self::InvalidCardinality {
                owner,
                min_occurs,
                max_occurs,
            } => write!(
                f,
                "invalid cardinality on {owner}: min {min_occurs} exceeds max {max_occurs}"
            ),
            Self::ContradictoryNumericRange { owner } => {
                write!(f, "contradictory numeric constraints on {owner}")
            }
            Self::IncomparableNumericRange { owner } => {
                write!(f, "incomparable numeric range domains on {owner}")
            }
            Self::NonFiniteNumericRange { owner } => {
                write!(f, "non-finite floating bound on {owner}")
            }
            Self::ContradictoryLengthConstraints { owner } => {
                write!(f, "contradictory length constraints on {owner}")
            }
            Self::EmptyPatternGroup { owner } => {
                write!(f, "empty lexical pattern group on {owner}")
            }
            Self::InvalidWhiteSpacePolicy {
                owner,
                primitive,
                policy,
            } => write!(
                f,
                "explicit whiteSpace policy {policy:?} on {owner} is incompatible with the intrinsic XSD semantics of {primitive:?}"
            ),
            Self::EmptyEnumeration { name } => {
                write!(f, "enumeration {} has no variants", format_name(name))
            }
            Self::EmptyChoice { name } => {
                write!(f, "choice {} has no alternatives", format_name(name))
            }
            Self::InvalidStructuralBase { name } => write!(
                f,
                "structural type {} must have a named structural base",
                format_name(name)
            ),
            Self::NonStructuralBase { name, base } => write!(
                f,
                "structural type {} has non-structural base {}",
                format_name(name),
                format_name(base)
            ),
            Self::InheritanceCycle { names } => write!(
                f,
                "complex-type inheritance cycle: {}",
                names
                    .iter()
                    .map(format_name)
                    .collect::<Vec<_>>()
                    .join(" -> ")
            ),
            Self::InvalidNamedRestrictionBase { name, base } => write!(
                f,
                "named simple restriction {} has invalid base {}",
                format_name(name),
                format_name(base)
            ),
            Self::NamedRestrictionWeakensBase { name, base } => write!(
                f,
                "named simple restriction {} has effective constraints that do not preserve base {}",
                format_name(name),
                format_name(base)
            ),
            Self::NamedRestrictionCycle { names } => write!(
                f,
                "named simple-restriction cycle: {}",
                names
                    .iter()
                    .map(format_name)
                    .collect::<Vec<_>>()
                    .join(" -> ")
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

fn validate_structural_inheritance(schema: &SchemaIr) -> Result<(), ValidationError> {
    let declarations = schema
        .types
        .iter()
        .map(|declaration| (&declaration.name, declaration))
        .collect::<BTreeMap<_, _>>();
    let mut bases = BTreeMap::new();
    for declaration in &schema.types {
        if !matches!(
            declaration.kind,
            TypeKind::Record { .. } | TypeKind::Choice { .. }
        ) {
            continue;
        }
        let Some(TypeRef {
            target: TypeRefTarget::Named(base),
        }) = &declaration.base_type
        else {
            continue;
        };
        let base_declaration = declarations[base];
        if !matches!(
            base_declaration.kind,
            TypeKind::Record { .. } | TypeKind::Choice { .. }
        ) {
            return Err(ValidationError::NonStructuralBase {
                name: declaration.name.clone(),
                base: base.clone(),
            });
        }
        bases.insert(declaration.name.clone(), base.clone());
    }

    let mut completed = BTreeSet::new();
    for declaration in &schema.types {
        if completed.contains(&declaration.name) {
            continue;
        }
        let mut positions = BTreeMap::new();
        let mut path = Vec::new();
        let mut current = declaration.name.clone();
        while let Some(base) = bases.get(&current) {
            if completed.contains(&current) {
                break;
            }
            positions.insert(current.clone(), path.len());
            path.push(current.clone());
            if let Some(&start) = positions.get(base) {
                let mut names = path[start..].to_vec();
                names.push(base.clone());
                return Err(ValidationError::InheritanceCycle { names });
            }
            current = base.clone();
        }
        completed.extend(path);
    }
    Ok(())
}

fn validate_named_simple_restrictions(schema: &SchemaIr) -> Result<(), ValidationError> {
    let declarations = schema
        .types
        .iter()
        .map(|declaration| (&declaration.name, declaration))
        .collect::<BTreeMap<_, _>>();
    let mut bases = BTreeMap::new();
    for declaration in &schema.types {
        let TypeKind::Primitive(kind) = declaration.kind else {
            continue;
        };
        let Some(TypeRef {
            target: TypeRefTarget::Named(base),
        }) = &declaration.base_type
        else {
            continue;
        };
        let base_declaration = declarations[base];
        if !matches!(base_declaration.kind, TypeKind::Primitive(base_kind) if base_kind == kind) {
            return Err(ValidationError::InvalidNamedRestrictionBase {
                name: declaration.name.clone(),
                base: base.clone(),
            });
        }
        bases.insert(declaration.name.clone(), base.clone());
    }

    for declaration in &schema.types {
        let mut positions = BTreeMap::new();
        let mut path = Vec::new();
        let mut current = declaration.name.clone();
        while let Some(base) = bases.get(&current) {
            positions.insert(current.clone(), path.len());
            path.push(current.clone());
            if let Some(&start) = positions.get(base) {
                let mut names = path[start..].to_vec();
                names.push(base.clone());
                return Err(ValidationError::NamedRestrictionCycle { names });
            }
            current = base.clone();
        }
    }

    for declaration in &schema.types {
        let Some(base) = bases.get(&declaration.name) else {
            continue;
        };
        let base_declaration = declarations[base];
        let TypeKind::Primitive(kind) = declaration.kind else {
            unreachable!("only primitive named restrictions are collected")
        };
        if !constraints_imply(
            &declaration.constraints,
            &base_declaration.constraints,
            kind,
        ) {
            return Err(ValidationError::NamedRestrictionWeakensBase {
                name: declaration.name.clone(),
                base: base.clone(),
            });
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct NumericBound {
    value: NumericValue,
    exclusive: bool,
}

fn constraints_imply(
    derived: &ConstraintSet,
    base: &ConstraintSet,
    primitive: PrimitiveKind,
) -> bool {
    numeric_lower_implies(derived, base)
        && numeric_upper_implies(derived, base)
        && length_constraints_imply(derived, base)
        && derived
            .lexical
            .pattern_groups
            .starts_with(&base.lexical.pattern_groups)
        && derived.lexical.effective_white_space(primitive)
            >= base.lexical.effective_white_space(primitive)
}

fn numeric_lower_implies(derived: &ConstraintSet, base: &ConstraintSet) -> bool {
    let derived_bounds = [
        derived.min_inclusive.map(|value| NumericBound {
            value,
            exclusive: false,
        }),
        derived.min_exclusive.map(|value| NumericBound {
            value,
            exclusive: true,
        }),
    ];
    [
        base.min_inclusive.map(|value| NumericBound {
            value,
            exclusive: false,
        }),
        base.min_exclusive.map(|value| NumericBound {
            value,
            exclusive: true,
        }),
    ]
    .into_iter()
    .flatten()
    .all(|base_bound| {
        derived_bounds
            .into_iter()
            .flatten()
            .any(|derived_bound| lower_bound_implies(derived_bound, base_bound))
    })
}

fn numeric_upper_implies(derived: &ConstraintSet, base: &ConstraintSet) -> bool {
    let derived_bounds = [
        derived.max_inclusive.map(|value| NumericBound {
            value,
            exclusive: false,
        }),
        derived.max_exclusive.map(|value| NumericBound {
            value,
            exclusive: true,
        }),
    ];
    [
        base.max_inclusive.map(|value| NumericBound {
            value,
            exclusive: false,
        }),
        base.max_exclusive.map(|value| NumericBound {
            value,
            exclusive: true,
        }),
    ]
    .into_iter()
    .flatten()
    .all(|base_bound| {
        derived_bounds
            .into_iter()
            .flatten()
            .any(|derived_bound| upper_bound_implies(derived_bound, base_bound))
    })
}

fn lower_bound_implies(derived: NumericBound, base: NumericBound) -> bool {
    match derived.value.semantic_cmp(&base.value) {
        Some(Ordering::Greater) => true,
        Some(Ordering::Equal) => derived.exclusive || !base.exclusive,
        Some(Ordering::Less) | None => false,
    }
}

fn upper_bound_implies(derived: NumericBound, base: NumericBound) -> bool {
    match derived.value.semantic_cmp(&base.value) {
        Some(Ordering::Less) => true,
        Some(Ordering::Equal) => derived.exclusive || !base.exclusive,
        Some(Ordering::Greater) | None => false,
    }
}

fn length_constraints_imply(derived: &ConstraintSet, base: &ConstraintSet) -> bool {
    let derived_min = derived.length.or(derived.min_length);
    let derived_max = derived.length.or(derived.max_length);
    let base_min = base.length.or(base.min_length);
    let base_max = base.length.or(base.max_length);

    base_min.is_none_or(|minimum| derived_min.is_some_and(|value| value >= minimum))
        && base_max.is_none_or(|maximum| derived_max.is_some_and(|value| value <= maximum))
}

fn validate_fields(
    fields: &[FieldDecl],
    declared_types: &BTreeSet<QualifiedName>,
    location: &'static str,
) -> Result<(), ValidationError> {
    for field in fields {
        validate_reference(&field.type_ref, declared_types, location, &field.source)?;
        validate_cardinality(field.cardinality, &format!("field {}", field.name))?;
        let primitive = match field.type_ref.target {
            TypeRefTarget::Primitive(kind) => Some(kind),
            TypeRefTarget::Named(_) => None,
        };
        validate_constraints(
            &field.constraints,
            &format!("field {}", field.name),
            primitive,
        )?;
    }
    Ok(())
}

fn validate_reference(
    type_ref: &TypeRef,
    declared_types: &BTreeSet<QualifiedName>,
    location: &'static str,
    source: &SourceRef,
) -> Result<(), ValidationError> {
    if let TypeRefTarget::Named(target) = &type_ref.target {
        if !declared_types.contains(target) {
            return Err(ValidationError::UnresolvedTypeReference {
                target: target.clone(),
                location,
                source: source.clone(),
            });
        }
    }
    Ok(())
}

fn validate_cardinality(cardinality: Cardinality, owner: &str) -> Result<(), ValidationError> {
    if let Some(max_occurs) = cardinality.max_occurs {
        if cardinality.min_occurs > max_occurs {
            return Err(ValidationError::InvalidCardinality {
                owner: owner.to_owned(),
                min_occurs: cardinality.min_occurs,
                max_occurs,
            });
        }
    }
    Ok(())
}

fn validate_constraints(
    constraints: &ConstraintSet,
    owner: &str,
    primitive: Option<PrimitiveKind>,
) -> Result<(), ValidationError> {
    if constraints
        .lexical
        .pattern_groups
        .iter()
        .any(|group| group.alternatives.is_empty())
    {
        return Err(ValidationError::EmptyPatternGroup {
            owner: owner.to_owned(),
        });
    }
    if let (Some(primitive), Some(policy)) = (primitive, constraints.lexical.white_space) {
        let intrinsic = primitive.intrinsic_white_space_policy();
        if (primitive.intrinsic_white_space_is_fixed() && policy != intrinsic) || policy < intrinsic
        {
            return Err(ValidationError::InvalidWhiteSpacePolicy {
                owner: owner.to_owned(),
                primitive,
                policy,
            });
        }
    }
    let numeric_values = [
        constraints.min_inclusive,
        constraints.min_exclusive,
        constraints.max_inclusive,
        constraints.max_exclusive,
    ];
    let mut domain = None;
    for value in numeric_values.into_iter().flatten() {
        if matches!(
            value,
            NumericValue::Float32(value) if !value.value().is_finite()
        ) || matches!(
            value,
            NumericValue::Float64(value) if !value.value().is_finite()
        ) {
            return Err(ValidationError::NonFiniteNumericRange {
                owner: owner.to_owned(),
            });
        }
        let current = match value {
            NumericValue::Integer(_) => 0,
            NumericValue::Float32(_) => 1,
            NumericValue::Float64(_) => 2,
        };
        if domain
            .replace(current)
            .is_some_and(|previous| previous != current)
        {
            return Err(ValidationError::IncomparableNumericRange {
                owner: owner.to_owned(),
            });
        }
    }
    let lower_bounds = [
        constraints.min_inclusive.map(|value| (value, false)),
        constraints.min_exclusive.map(|value| (value, true)),
    ];
    let upper_bounds = [
        constraints.max_inclusive.map(|value| (value, false)),
        constraints.max_exclusive.map(|value| (value, true)),
    ];
    for (lower, lower_exclusive) in lower_bounds.into_iter().flatten() {
        for (upper, upper_exclusive) in upper_bounds.into_iter().flatten() {
            let ordering = lower.semantic_cmp(&upper).ok_or_else(|| {
                ValidationError::IncomparableNumericRange {
                    owner: owner.to_owned(),
                }
            })?;
            if ordering == Ordering::Greater
                || (ordering == Ordering::Equal && (lower_exclusive || upper_exclusive))
            {
                return Err(ValidationError::ContradictoryNumericRange {
                    owner: owner.to_owned(),
                });
            }
        }
    }

    if constraints
        .min_length
        .zip(constraints.max_length)
        .is_some_and(|(min, max)| min > max)
        || constraints
            .length
            .zip(constraints.min_length)
            .is_some_and(|(length, min)| length < min)
        || constraints
            .length
            .zip(constraints.max_length)
            .is_some_and(|(length, max)| length > max)
    {
        return Err(ValidationError::ContradictoryLengthConstraints {
            owner: owner.to_owned(),
        });
    }
    Ok(())
}

fn format_name(name: &QualifiedName) -> String {
    format!("{{{}}}{}", name.namespace_uri, name.local_name)
}

fn format_source(source: &SourceRef) -> String {
    source.line.map_or_else(
        || source.document.clone(),
        |line| format!("{}:{line}", source.document),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceDecl {
    pub uri: String,
    pub preferred_prefix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDecl {
    pub name: QualifiedName,
    pub is_abstract: bool,
    pub base_type: Option<TypeRef>,
    pub kind: TypeKind,
    pub constraints: ConstraintSet,
    pub documentation: Option<String>,
    pub source: SourceRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind {
    Primitive(PrimitiveKind),
    Alias(TypeRef),
    Enumeration {
        variants: Vec<EnumVariant>,
    },
    Record {
        fields: Vec<FieldDecl>,
    },
    Choice {
        alternatives: Vec<FieldDecl>,
    },
    List {
        item_type: TypeRef,
        cardinality: Cardinality,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveKind {
    Boolean,
    SignedInteger,
    UnsignedInteger,
    Decimal,
    Float32,
    Float64,
    String,
    Binary,
    DateTime,
    Time,
    Duration,
}

impl PrimitiveKind {
    /// XML Schema `whiteSpace` policy intrinsic to this primitive value space.
    #[must_use]
    pub const fn intrinsic_white_space_policy(self) -> WhiteSpacePolicy {
        match self {
            Self::String => WhiteSpacePolicy::Preserve,
            Self::Boolean
            | Self::SignedInteger
            | Self::UnsignedInteger
            | Self::Decimal
            | Self::Float32
            | Self::Float64
            | Self::Binary
            | Self::DateTime
            | Self::Time
            | Self::Duration => WhiteSpacePolicy::Collapse,
        }
    }

    /// Whether XML Schema fixes this primitive's intrinsic `whiteSpace` facet.
    #[must_use]
    pub const fn intrinsic_white_space_is_fixed(self) -> bool {
        !matches!(self, Self::String)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Float32Value(u32);

impl Float32Value {
    #[must_use]
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    #[must_use]
    pub fn from_value(value: f32) -> Self {
        Self(value.to_bits())
    }

    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn value(self) -> f32 {
        f32::from_bits(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Float64Value(u64);

impl Float64Value {
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    #[must_use]
    pub fn from_value(value: f64) -> Self {
        Self(value.to_bits())
    }

    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    #[must_use]
    pub fn value(self) -> f64 {
        f64::from_bits(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericValue {
    Integer(i128),
    Float32(Float32Value),
    Float64(Float64Value),
}

impl NumericValue {
    #[must_use]
    pub fn semantic_cmp(&self, other: &Self) -> Option<Ordering> {
        match (*self, *other) {
            (Self::Integer(left), Self::Integer(right)) => Some(left.cmp(&right)),
            (Self::Float32(left), Self::Float32(right)) => left.value().partial_cmp(&right.value()),
            (Self::Float64(left), Self::Float64(right)) => left.value().partial_cmp(&right.value()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub wire_value: String,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDecl {
    pub name: String,
    pub type_ref: TypeRef,
    pub cardinality: Cardinality,
    pub nillable: bool,
    pub constraints: ConstraintSet,
    pub documentation: Option<String>,
    pub source: SourceRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRef {
    pub target: TypeRefTarget,
}

impl TypeRef {
    #[must_use]
    pub const fn primitive(kind: PrimitiveKind) -> Self {
        Self {
            target: TypeRefTarget::Primitive(kind),
        }
    }

    #[must_use]
    pub fn named(name: QualifiedName) -> Self {
        Self {
            target: TypeRefTarget::Named(name),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRefTarget {
    Primitive(PrimitiveKind),
    Named(QualifiedName),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cardinality {
    pub min_occurs: u64,
    /// `None` means unbounded.
    pub max_occurs: Option<u64>,
}

impl Cardinality {
    pub const REQUIRED_ONE: Self = Self {
        min_occurs: 1,
        max_occurs: Some(1),
    };

    pub const OPTIONAL_ONE: Self = Self {
        min_occurs: 0,
        max_occurs: Some(1),
    };

    #[must_use]
    pub fn is_valid(self) -> bool {
        match self.max_occurs {
            Some(max) => self.min_occurs <= max,
            None => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConstraintSet {
    pub min_inclusive: Option<NumericValue>,
    pub max_inclusive: Option<NumericValue>,
    pub min_exclusive: Option<NumericValue>,
    pub max_exclusive: Option<NumericValue>,
    pub length: Option<u64>,
    pub min_length: Option<u64>,
    pub max_length: Option<u64>,
    pub lexical: LexicalConstraintSet,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LexicalConstraintSet {
    /// One group per restriction level, in base-to-derived order. XML Schema
    /// evaluates these expressions after effective whitespace normalization.
    pub pattern_groups: Vec<PatternGroup>,
    /// Explicit derived `whiteSpace` facet; `None` retains the primitive policy.
    pub white_space: Option<WhiteSpacePolicy>,
}

impl LexicalConstraintSet {
    /// Return the explicit policy or the primitive's intrinsic XSD baseline.
    #[must_use]
    pub fn effective_white_space(&self, primitive: PrimitiveKind) -> WhiteSpacePolicy {
        self.white_space
            .unwrap_or_else(|| primitive.intrinsic_white_space_policy())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternGroup {
    /// XML Schema patterns declared together at one restriction level are alternatives.
    pub alternatives: Vec<PatternExpression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternExpression {
    pub dialect: PatternDialect,
    pub expression: String,
}

impl PatternExpression {
    #[must_use]
    pub fn xml_schema(expression: impl Into<String>) -> Self {
        Self {
            dialect: PatternDialect::XmlSchema,
            expression: expression.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternDialect {
    XmlSchema,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WhiteSpacePolicy {
    Preserve,
    Replace,
    Collapse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageDecl {
    pub name: QualifiedName,
    pub payload_type: TypeRef,
    pub documentation: Option<String>,
    pub source: SourceRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRef {
    pub document: String,
    pub line: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const NS: &str = "urn:test";

    fn source() -> SourceRef {
        SourceRef {
            document: "test.ir".to_owned(),
            line: Some(1),
        }
    }

    fn declaration(name: &str, kind: TypeKind) -> TypeDecl {
        TypeDecl {
            name: QualifiedName::new(NS, name),
            is_abstract: false,
            base_type: None,
            kind,
            constraints: ConstraintSet::default(),
            documentation: None,
            source: source(),
        }
    }

    fn schema(types: Vec<TypeDecl>) -> SchemaIr {
        SchemaIr {
            schema_version: None,
            namespaces: vec![NamespaceDecl {
                uri: NS.to_owned(),
                preferred_prefix: None,
            }],
            types,
            messages: Vec::new(),
        }
    }

    fn named(name: &str) -> TypeRef {
        TypeRef::named(QualifiedName::new(NS, name))
    }

    fn field(type_ref: TypeRef) -> FieldDecl {
        FieldDecl {
            name: "value".to_owned(),
            type_ref,
            cardinality: Cardinality::REQUIRED_ONE,
            nillable: false,
            constraints: ConstraintSet::default(),
            documentation: None,
            source: source(),
        }
    }

    fn assert_unresolved(schema: &SchemaIr, location: &'static str) {
        assert!(matches!(
            schema.validate(),
            Err(ValidationError::UnresolvedTypeReference {
                location: actual,
                ..
            }) if actual == location
        ));
    }

    #[test]
    fn cardinality_rejects_inverted_bounds() {
        assert!(
            !Cardinality {
                min_occurs: 2,
                max_occurs: Some(1),
            }
            .is_valid()
        );
    }

    #[test]
    fn unbounded_cardinality_is_valid() {
        assert!(
            Cardinality {
                min_occurs: 1,
                max_occurs: None,
            }
            .is_valid()
        );
    }

    #[test]
    fn validates_minimal_schema() {
        schema(vec![declaration(
            "Value",
            TypeKind::Primitive(PrimitiveKind::String),
        )])
        .validate()
        .expect("minimal schema should be valid");
    }

    #[test]
    fn rejects_duplicate_namespace_uri() {
        let mut schema = schema(Vec::new());
        schema.namespaces.push(schema.namespaces[0].clone());
        assert!(matches!(
            schema.validate(),
            Err(ValidationError::DuplicateNamespace { .. })
        ));
    }

    #[test]
    fn rejects_missing_type_namespace() {
        let schema = schema(vec![declaration(
            "Value",
            TypeKind::Primitive(PrimitiveKind::String),
        )]);
        let mut invalid = schema;
        invalid.types[0].name.namespace_uri = "urn:absent".to_owned();
        assert!(matches!(
            invalid.validate(),
            Err(ValidationError::UndeclaredNamespace {
                declaration: "type",
                ..
            })
        ));
    }

    #[test]
    fn rejects_missing_message_namespace() {
        let mut schema = schema(Vec::new());
        schema.messages.push(MessageDecl {
            name: QualifiedName::new("urn:absent", "Message"),
            payload_type: TypeRef::primitive(PrimitiveKind::String),
            documentation: None,
            source: source(),
        });
        assert!(matches!(
            schema.validate(),
            Err(ValidationError::UndeclaredNamespace {
                declaration: "message",
                ..
            })
        ));
    }

    #[test]
    fn rejects_duplicate_qualified_type() {
        let value = declaration("Value", TypeKind::Primitive(PrimitiveKind::String));
        assert!(matches!(
            schema(vec![value.clone(), value]).validate(),
            Err(ValidationError::DuplicateType { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_qualified_message_but_allows_matching_type_name() {
        let message = MessageDecl {
            name: QualifiedName::new(NS, "Value"),
            payload_type: named("Value"),
            documentation: None,
            source: source(),
        };
        let mut schema = schema(vec![declaration(
            "Value",
            TypeKind::Primitive(PrimitiveKind::String),
        )]);
        schema.messages = vec![message.clone(), message];
        assert!(matches!(
            schema.validate(),
            Err(ValidationError::DuplicateMessage { .. })
        ));
    }

    #[test]
    fn rejects_unresolved_base_type() {
        let mut value = declaration("Value", TypeKind::Primitive(PrimitiveKind::String));
        value.base_type = Some(named("Absent"));
        assert_unresolved(&schema(vec![value]), "base type");
    }

    #[test]
    fn rejects_unresolved_alias_target() {
        assert_unresolved(
            &schema(vec![declaration("Value", TypeKind::Alias(named("Absent")))]),
            "alias target",
        );
    }

    #[test]
    fn rejects_unresolved_record_field_target() {
        assert_unresolved(
            &schema(vec![declaration(
                "Value",
                TypeKind::Record {
                    fields: vec![field(named("Absent"))],
                },
            )]),
            "record field",
        );
    }

    #[test]
    fn rejects_unresolved_choice_target() {
        assert_unresolved(
            &schema(vec![declaration(
                "Value",
                TypeKind::Choice {
                    alternatives: vec![field(named("Absent"))],
                },
            )]),
            "choice alternative",
        );
    }

    #[test]
    fn rejects_unresolved_list_item_target() {
        assert_unresolved(
            &schema(vec![declaration(
                "Value",
                TypeKind::List {
                    item_type: named("Absent"),
                    cardinality: Cardinality::REQUIRED_ONE,
                },
            )]),
            "list item",
        );
    }

    #[test]
    fn rejects_unresolved_message_payload() {
        let mut schema = schema(Vec::new());
        schema.messages.push(MessageDecl {
            name: QualifiedName::new(NS, "Message"),
            payload_type: named("Absent"),
            documentation: None,
            source: source(),
        });
        assert_unresolved(&schema, "message payload");
    }

    #[test]
    fn rejects_invalid_finite_cardinality() {
        let invalid = Cardinality {
            min_occurs: 2,
            max_occurs: Some(1),
        };
        assert!(matches!(
            schema(vec![declaration(
                "Values",
                TypeKind::List {
                    item_type: TypeRef::primitive(PrimitiveKind::String),
                    cardinality: invalid,
                },
            )])
            .validate(),
            Err(ValidationError::InvalidCardinality { .. })
        ));
    }

    #[test]
    fn rejects_contradictory_numeric_range() {
        let mut value = declaration("Value", TypeKind::Primitive(PrimitiveKind::SignedInteger));
        value.constraints.min_inclusive = Some(NumericValue::Integer(10));
        value.constraints.max_exclusive = Some(NumericValue::Integer(10));
        assert!(matches!(
            schema(vec![value]).validate(),
            Err(ValidationError::ContradictoryNumericRange { .. })
        ));
    }

    #[test]
    fn floating_numeric_values_are_width_aware_and_deterministic() {
        let float_a = NumericValue::Float32(Float32Value::from_value(1.5));
        let float_b = NumericValue::Float32(Float32Value::from_value(1.5));
        let double = NumericValue::Float64(Float64Value::from_value(1.5));
        assert_eq!(float_a, float_b);
        assert_ne!(float_a, double);
        assert_eq!(
            NumericValue::Float64(Float64Value::from_value(-1.0))
                .semantic_cmp(&NumericValue::Float64(Float64Value::from_value(2.0))),
            Some(Ordering::Less)
        );
        assert_eq!(float_a.semantic_cmp(&double), None);
    }

    #[test]
    fn rejects_contradictory_floating_ranges_and_mixed_domains() {
        for (kind, lower, upper) in [
            (
                PrimitiveKind::Float64,
                NumericValue::Float64(Float64Value::from_value(10.0)),
                NumericValue::Float64(Float64Value::from_value(5.0)),
            ),
            (
                PrimitiveKind::Float32,
                NumericValue::Float32(Float32Value::from_value(2.0)),
                NumericValue::Float32(Float32Value::from_value(2.0)),
            ),
        ] {
            let mut value = declaration("Value", TypeKind::Primitive(kind));
            value.constraints.min_inclusive = Some(lower);
            value.constraints.max_exclusive = Some(upper);
            assert!(matches!(
                schema(vec![value]).validate(),
                Err(ValidationError::ContradictoryNumericRange { .. })
            ));
        }

        let mut mixed = declaration("Mixed", TypeKind::Primitive(PrimitiveKind::Float64));
        mixed.constraints.min_inclusive = Some(NumericValue::Integer(0));
        mixed.constraints.max_inclusive =
            Some(NumericValue::Float64(Float64Value::from_value(1.0)));
        assert!(matches!(
            schema(vec![mixed]).validate(),
            Err(ValidationError::IncomparableNumericRange { .. })
        ));

        let mut nan = declaration("NaN", TypeKind::Primitive(PrimitiveKind::Float64));
        nan.constraints.min_inclusive =
            Some(NumericValue::Float64(Float64Value::from_value(f64::NAN)));
        assert!(matches!(
            schema(vec![nan]).validate(),
            Err(ValidationError::NonFiniteNumericRange { .. })
        ));
    }

    #[test]
    fn rejects_contradictory_length_constraints() {
        let mut value = declaration("Value", TypeKind::Primitive(PrimitiveKind::String));
        value.constraints.length = Some(2);
        value.constraints.min_length = Some(3);
        assert!(matches!(
            schema(vec![value]).validate(),
            Err(ValidationError::ContradictoryLengthConstraints { .. })
        ));

        let mut value = declaration("Value", TypeKind::Primitive(PrimitiveKind::Binary));
        value.constraints.length = Some(6);
        value.constraints.max_length = Some(5);
        assert!(matches!(
            schema(vec![value]).validate(),
            Err(ValidationError::ContradictoryLengthConstraints { .. })
        ));

        let mut value = declaration("Value", TypeKind::Primitive(PrimitiveKind::String));
        value.constraints.min_length = Some(8);
        value.constraints.max_length = Some(7);
        assert!(matches!(
            schema(vec![value]).validate(),
            Err(ValidationError::ContradictoryLengthConstraints { .. })
        ));
    }

    #[test]
    fn rejects_empty_pattern_groups() {
        let mut value = declaration("Value", TypeKind::Primitive(PrimitiveKind::String));
        value.constraints.lexical.pattern_groups.push(PatternGroup {
            alternatives: Vec::new(),
        });
        assert!(matches!(
            schema(vec![value]).validate(),
            Err(ValidationError::EmptyPatternGroup { .. })
        ));
    }

    #[test]
    fn primitive_intrinsic_white_space_semantics_cover_every_kind() {
        for (primitive, policy, fixed) in [
            (PrimitiveKind::Boolean, WhiteSpacePolicy::Collapse, true),
            (
                PrimitiveKind::SignedInteger,
                WhiteSpacePolicy::Collapse,
                true,
            ),
            (
                PrimitiveKind::UnsignedInteger,
                WhiteSpacePolicy::Collapse,
                true,
            ),
            (PrimitiveKind::Decimal, WhiteSpacePolicy::Collapse, true),
            (PrimitiveKind::Float32, WhiteSpacePolicy::Collapse, true),
            (PrimitiveKind::Float64, WhiteSpacePolicy::Collapse, true),
            (PrimitiveKind::String, WhiteSpacePolicy::Preserve, false),
            (PrimitiveKind::Binary, WhiteSpacePolicy::Collapse, true),
            (PrimitiveKind::DateTime, WhiteSpacePolicy::Collapse, true),
            (PrimitiveKind::Time, WhiteSpacePolicy::Collapse, true),
            (PrimitiveKind::Duration, WhiteSpacePolicy::Collapse, true),
        ] {
            assert_eq!(primitive.intrinsic_white_space_policy(), policy);
            assert_eq!(primitive.intrinsic_white_space_is_fixed(), fixed);
            assert_eq!(
                LexicalConstraintSet::default().effective_white_space(primitive),
                policy
            );
        }
    }

    #[test]
    fn string_white_space_may_tighten_and_patterns_retain_effective_context() {
        for policy in [WhiteSpacePolicy::Replace, WhiteSpacePolicy::Collapse] {
            let mut value = declaration("Value", TypeKind::Primitive(PrimitiveKind::String));
            value.constraints.lexical.white_space = Some(policy);
            value.constraints.lexical.pattern_groups.push(PatternGroup {
                alternatives: vec![PatternExpression::xml_schema("[A-Z]+")],
            });
            assert_eq!(
                value
                    .constraints
                    .lexical
                    .effective_white_space(PrimitiveKind::String),
                policy
            );
            schema(vec![value])
                .validate()
                .expect("String whitespace tightening should be valid");
        }
    }

    #[test]
    fn primitive_patterns_are_evaluated_with_effective_white_space() {
        for primitive in [
            PrimitiveKind::SignedInteger,
            PrimitiveKind::DateTime,
            PrimitiveKind::Time,
        ] {
            let mut value = declaration("Value", TypeKind::Primitive(primitive));
            value.constraints.lexical.pattern_groups.push(PatternGroup {
                alternatives: vec![PatternExpression::xml_schema(".+Z")],
            });
            assert_eq!(
                value.constraints.lexical.effective_white_space(primitive),
                WhiteSpacePolicy::Collapse
            );
            assert_eq!(
                value.constraints.lexical.pattern_groups[0].alternatives[0].dialect,
                PatternDialect::XmlSchema
            );
            schema(vec![value])
                .validate()
                .expect("pattern retains its primitive whitespace context");
        }
    }

    #[test]
    fn fixed_collapse_primitives_reject_contradictory_explicit_policies() {
        for (primitive, policy) in [
            (PrimitiveKind::DateTime, WhiteSpacePolicy::Preserve),
            (PrimitiveKind::DateTime, WhiteSpacePolicy::Replace),
            (PrimitiveKind::SignedInteger, WhiteSpacePolicy::Preserve),
            (PrimitiveKind::Float64, WhiteSpacePolicy::Replace),
        ] {
            let mut value = declaration("Value", TypeKind::Primitive(primitive));
            value.constraints.lexical.white_space = Some(policy);
            assert!(matches!(
                schema(vec![value]).validate(),
                Err(ValidationError::InvalidWhiteSpacePolicy {
                    primitive: actual_primitive,
                    policy: actual_policy,
                    ..
                }) if actual_primitive == primitive && actual_policy == policy
            ));
        }
    }

    #[test]
    fn named_white_space_restrictions_compare_effective_policies() {
        for (base_policy, derived_policy, valid) in [
            (None, Some(WhiteSpacePolicy::Collapse), true),
            (Some(WhiteSpacePolicy::Collapse), None, false),
            (
                Some(WhiteSpacePolicy::Replace),
                Some(WhiteSpacePolicy::Collapse),
                true,
            ),
            (
                Some(WhiteSpacePolicy::Collapse),
                Some(WhiteSpacePolicy::Replace),
                false,
            ),
        ] {
            let constraints = |policy| ConstraintSet {
                lexical: LexicalConstraintSet {
                    white_space: policy,
                    ..LexicalConstraintSet::default()
                },
                ..ConstraintSet::default()
            };
            let result = schema(vec![
                restricted_primitive(
                    "Base",
                    PrimitiveKind::String,
                    None,
                    constraints(base_policy),
                ),
                restricted_primitive(
                    "Derived",
                    PrimitiveKind::String,
                    Some("Base"),
                    constraints(derived_policy),
                ),
            ])
            .validate();
            assert_eq!(result.is_ok(), valid);
        }

        schema(vec![
            restricted_primitive(
                "Base",
                PrimitiveKind::DateTime,
                None,
                ConstraintSet::default(),
            ),
            restricted_primitive(
                "Derived",
                PrimitiveKind::DateTime,
                Some("Base"),
                ConstraintSet::default(),
            ),
        ])
        .validate()
        .expect("an omitted policy retains fixed intrinsic collapse");
    }

    #[test]
    fn rejects_empty_enumeration() {
        assert!(matches!(
            schema(vec![declaration(
                "Value",
                TypeKind::Enumeration {
                    variants: Vec::new()
                },
            )])
            .validate(),
            Err(ValidationError::EmptyEnumeration { .. })
        ));
    }

    #[test]
    fn rejects_empty_choice() {
        assert!(matches!(
            schema(vec![declaration(
                "Value",
                TypeKind::Choice {
                    alternatives: Vec::new()
                },
            )])
            .validate(),
            Err(ValidationError::EmptyChoice { .. })
        ));
    }

    #[test]
    fn permits_primitive_base_only_for_non_structural_types() {
        let mut simple = declaration("Simple", TypeKind::Primitive(PrimitiveKind::SignedInteger));
        simple.base_type = Some(TypeRef::primitive(PrimitiveKind::SignedInteger));
        schema(vec![simple])
            .validate()
            .expect("simple restriction ancestry should remain valid");

        let mut structural = declaration("Structural", TypeKind::Record { fields: Vec::new() });
        structural.base_type = Some(TypeRef::primitive(PrimitiveKind::String));
        assert!(matches!(
            schema(vec![structural]).validate(),
            Err(ValidationError::InvalidStructuralBase { .. })
        ));
    }

    #[test]
    fn rejects_named_non_structural_base() {
        let base = declaration("Base", TypeKind::Primitive(PrimitiveKind::String));
        let mut derived = declaration("Derived", TypeKind::Record { fields: Vec::new() });
        derived.base_type = Some(named("Base"));
        assert!(matches!(
            schema(vec![base, derived]).validate(),
            Err(ValidationError::NonStructuralBase { .. })
        ));
    }

    #[test]
    fn rejects_structural_base_for_named_simple_restriction() {
        let base = declaration("Base", TypeKind::Record { fields: Vec::new() });
        let mut derived = declaration("Derived", TypeKind::Primitive(PrimitiveKind::String));
        derived.base_type = Some(named("Base"));
        assert!(matches!(
            schema(vec![base, derived]).validate(),
            Err(ValidationError::InvalidNamedRestrictionBase { .. })
        ));
    }

    #[test]
    fn rejects_named_simple_restriction_cycle_deterministically() {
        let mut a = declaration("A", TypeKind::Primitive(PrimitiveKind::String));
        let mut b = declaration("B", TypeKind::Primitive(PrimitiveKind::String));
        a.base_type = Some(named("B"));
        b.base_type = Some(named("A"));
        let error = schema(vec![a, b]).validate().unwrap_err();
        assert_eq!(
            error.to_string(),
            "named simple-restriction cycle: {urn:test}A -> {urn:test}B -> {urn:test}A"
        );
    }

    fn restricted_primitive(
        name: &str,
        kind: PrimitiveKind,
        base: Option<&str>,
        constraints: ConstraintSet,
    ) -> TypeDecl {
        let mut declaration = declaration(name, TypeKind::Primitive(kind));
        declaration.base_type = base.map(named);
        declaration.constraints = constraints;
        declaration
    }

    fn integer(value: i128) -> NumericValue {
        NumericValue::Integer(value)
    }

    fn assert_named_restriction_weakens_base(types: Vec<TypeDecl>, name: &str, base: &str) {
        assert_eq!(
            schema(types).validate(),
            Err(ValidationError::NamedRestrictionWeakensBase {
                name: QualifiedName::new(NS, name),
                base: QualifiedName::new(NS, base),
            })
        );
    }

    #[test]
    fn named_numeric_restrictions_require_effective_bounds_to_tighten() {
        let base_constraints = ConstraintSet {
            min_inclusive: Some(integer(0)),
            max_inclusive: Some(integer(100)),
            ..ConstraintSet::default()
        };
        let valid = ConstraintSet {
            min_inclusive: Some(integer(10)),
            max_inclusive: Some(integer(90)),
            ..ConstraintSet::default()
        };
        schema(vec![
            restricted_primitive(
                "Base",
                PrimitiveKind::SignedInteger,
                None,
                base_constraints.clone(),
            ),
            restricted_primitive("Derived", PrimitiveKind::SignedInteger, Some("Base"), valid),
        ])
        .validate()
        .expect("tighter integer bounds should preserve the base restriction");

        for constraints in [
            ConstraintSet {
                min_inclusive: Some(integer(-1)),
                max_inclusive: Some(integer(100)),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                min_inclusive: Some(integer(0)),
                max_inclusive: Some(integer(101)),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                max_inclusive: Some(integer(100)),
                ..ConstraintSet::default()
            },
        ] {
            assert_named_restriction_weakens_base(
                vec![
                    restricted_primitive(
                        "Base",
                        PrimitiveKind::SignedInteger,
                        None,
                        base_constraints.clone(),
                    ),
                    restricted_primitive(
                        "Derived",
                        PrimitiveKind::SignedInteger,
                        Some("Base"),
                        constraints,
                    ),
                ],
                "Derived",
                "Base",
            );
        }
    }

    #[test]
    fn named_numeric_restrictions_apply_inclusive_exclusive_tie_strength() {
        for (base_constraints, derived_constraints, valid) in [
            (
                ConstraintSet {
                    min_exclusive: Some(integer(0)),
                    ..ConstraintSet::default()
                },
                ConstraintSet {
                    min_inclusive: Some(integer(0)),
                    ..ConstraintSet::default()
                },
                false,
            ),
            (
                ConstraintSet {
                    min_inclusive: Some(integer(0)),
                    ..ConstraintSet::default()
                },
                ConstraintSet {
                    min_exclusive: Some(integer(0)),
                    ..ConstraintSet::default()
                },
                true,
            ),
            (
                ConstraintSet {
                    max_exclusive: Some(integer(100)),
                    ..ConstraintSet::default()
                },
                ConstraintSet {
                    max_inclusive: Some(integer(100)),
                    ..ConstraintSet::default()
                },
                false,
            ),
            (
                ConstraintSet {
                    max_inclusive: Some(integer(100)),
                    ..ConstraintSet::default()
                },
                ConstraintSet {
                    max_exclusive: Some(integer(100)),
                    ..ConstraintSet::default()
                },
                true,
            ),
        ] {
            let result = schema(vec![
                restricted_primitive("Base", PrimitiveKind::SignedInteger, None, base_constraints),
                restricted_primitive(
                    "Derived",
                    PrimitiveKind::SignedInteger,
                    Some("Base"),
                    derived_constraints,
                ),
            ])
            .validate();
            assert_eq!(result.is_ok(), valid, "unexpected tie-strength result");
        }
    }

    #[test]
    fn named_floating_restrictions_compare_at_their_declared_width() {
        let f64_value = |value| NumericValue::Float64(Float64Value::from_value(value));
        schema(vec![
            restricted_primitive(
                "Base",
                PrimitiveKind::Float64,
                None,
                ConstraintSet {
                    min_inclusive: Some(f64_value(0.0)),
                    max_inclusive: Some(f64_value(100.0)),
                    ..ConstraintSet::default()
                },
            ),
            restricted_primitive(
                "Derived",
                PrimitiveKind::Float64,
                Some("Base"),
                ConstraintSet {
                    min_inclusive: Some(f64_value(10.0)),
                    max_exclusive: Some(f64_value(100.0)),
                    ..ConstraintSet::default()
                },
            ),
        ])
        .validate()
        .expect("tighter Float64 bounds should preserve the base restriction");

        assert_named_restriction_weakens_base(
            vec![
                restricted_primitive(
                    "Base",
                    PrimitiveKind::Float64,
                    None,
                    ConstraintSet {
                        min_inclusive: Some(f64_value(0.0)),
                        ..ConstraintSet::default()
                    },
                ),
                restricted_primitive(
                    "Derived",
                    PrimitiveKind::Float64,
                    Some("Base"),
                    ConstraintSet {
                        min_inclusive: Some(f64_value(-1.0)),
                        ..ConstraintSet::default()
                    },
                ),
            ],
            "Derived",
            "Base",
        );

        let f32_value = |value| NumericValue::Float32(Float32Value::from_value(value));
        schema(vec![
            restricted_primitive(
                "Base",
                PrimitiveKind::Float32,
                None,
                ConstraintSet {
                    min_inclusive: Some(f32_value(0.0)),
                    ..ConstraintSet::default()
                },
            ),
            restricted_primitive(
                "Derived",
                PrimitiveKind::Float32,
                Some("Base"),
                ConstraintSet {
                    min_exclusive: Some(f32_value(0.0)),
                    ..ConstraintSet::default()
                },
            ),
        ])
        .validate()
        .expect("Float32 exclusive tie should tighten an inclusive base");
    }

    #[test]
    fn named_numeric_restrictions_reject_incomparable_domains() {
        assert_named_restriction_weakens_base(
            vec![
                restricted_primitive(
                    "Base",
                    PrimitiveKind::Float64,
                    None,
                    ConstraintSet {
                        min_inclusive: Some(NumericValue::Float64(Float64Value::from_value(0.0))),
                        ..ConstraintSet::default()
                    },
                ),
                restricted_primitive(
                    "Derived",
                    PrimitiveKind::Float64,
                    Some("Base"),
                    ConstraintSet {
                        min_inclusive: Some(NumericValue::Float32(Float32Value::from_value(1.0))),
                        ..ConstraintSet::default()
                    },
                ),
            ],
            "Derived",
            "Base",
        );
    }

    #[test]
    fn named_length_restrictions_require_effective_interval_subset() {
        for (base_constraints, derived_constraints, valid) in [
            (
                ConstraintSet {
                    max_length: Some(32),
                    ..ConstraintSet::default()
                },
                ConstraintSet {
                    max_length: Some(16),
                    ..ConstraintSet::default()
                },
                true,
            ),
            (
                ConstraintSet {
                    max_length: Some(32),
                    ..ConstraintSet::default()
                },
                ConstraintSet {
                    max_length: Some(64),
                    ..ConstraintSet::default()
                },
                false,
            ),
            (
                ConstraintSet {
                    min_length: Some(4),
                    ..ConstraintSet::default()
                },
                ConstraintSet {
                    min_length: Some(8),
                    ..ConstraintSet::default()
                },
                true,
            ),
            (
                ConstraintSet {
                    min_length: Some(4),
                    ..ConstraintSet::default()
                },
                ConstraintSet {
                    min_length: Some(2),
                    ..ConstraintSet::default()
                },
                false,
            ),
            (
                ConstraintSet {
                    min_length: Some(4),
                    max_length: Some(16),
                    ..ConstraintSet::default()
                },
                ConstraintSet {
                    length: Some(8),
                    ..ConstraintSet::default()
                },
                true,
            ),
            (
                ConstraintSet {
                    length: Some(8),
                    ..ConstraintSet::default()
                },
                ConstraintSet {
                    length: Some(7),
                    ..ConstraintSet::default()
                },
                false,
            ),
            (
                ConstraintSet {
                    length: Some(8),
                    ..ConstraintSet::default()
                },
                ConstraintSet {
                    min_length: Some(8),
                    max_length: Some(8),
                    ..ConstraintSet::default()
                },
                true,
            ),
        ] {
            let result = schema(vec![
                restricted_primitive("Base", PrimitiveKind::String, None, base_constraints),
                restricted_primitive(
                    "Derived",
                    PrimitiveKind::String,
                    Some("Base"),
                    derived_constraints,
                ),
            ])
            .validate();
            assert_eq!(
                result.is_ok(),
                valid,
                "unexpected length implication result"
            );
        }
    }

    #[test]
    fn named_pattern_restrictions_preserve_inherited_group_prefix() {
        fn lexical(groups: Vec<Vec<&str>>) -> LexicalConstraintSet {
            LexicalConstraintSet {
                pattern_groups: groups
                    .into_iter()
                    .map(|alternatives| PatternGroup {
                        alternatives: alternatives
                            .into_iter()
                            .map(PatternExpression::xml_schema)
                            .collect(),
                    })
                    .collect(),
                white_space: None,
            }
        }
        for (base_patterns, derived_patterns, valid) in [
            (vec![vec!["A", "B"]], vec![vec!["A", "B"]], true),
            (vec![vec!["A", "B"]], vec![vec!["A", "B"], vec!["C"]], true),
            (vec![vec!["A", "B"]], Vec::new(), false),
            (vec![vec!["A", "B"]], vec![vec!["X", "B"]], false),
            (
                vec![vec!["A"], vec!["B"]],
                vec![vec!["B"], vec!["A"]],
                false,
            ),
            (Vec::new(), vec![vec!["B"]], true),
        ] {
            let result = schema(vec![
                restricted_primitive(
                    "Base",
                    PrimitiveKind::String,
                    None,
                    ConstraintSet {
                        lexical: lexical(base_patterns),
                        ..ConstraintSet::default()
                    },
                ),
                restricted_primitive(
                    "Derived",
                    PrimitiveKind::String,
                    Some("Base"),
                    ConstraintSet {
                        lexical: lexical(derived_patterns),
                        ..ConstraintSet::default()
                    },
                ),
            ])
            .validate();
            assert_eq!(
                result.is_ok(),
                valid,
                "unexpected pattern implication result"
            );
        }
    }

    #[test]
    fn named_multilevel_restrictions_validate_against_immediate_base() {
        let bounds = |min, max| ConstraintSet {
            min_inclusive: Some(integer(min)),
            max_inclusive: Some(integer(max)),
            ..ConstraintSet::default()
        };
        let base = restricted_primitive("Base", PrimitiveKind::SignedInteger, None, bounds(0, 100));
        let middle = restricted_primitive(
            "Middle",
            PrimitiveKind::SignedInteger,
            Some("Base"),
            bounds(10, 90),
        );
        schema(vec![
            base.clone(),
            middle.clone(),
            restricted_primitive(
                "Leaf",
                PrimitiveKind::SignedInteger,
                Some("Middle"),
                bounds(20, 80),
            ),
        ])
        .validate()
        .expect("transitively tighter immediate restrictions should be valid");

        assert_named_restriction_weakens_base(
            vec![
                base,
                middle,
                restricted_primitive(
                    "Leaf",
                    PrimitiveKind::SignedInteger,
                    Some("Middle"),
                    bounds(0, 95),
                ),
            ],
            "Leaf",
            "Middle",
        );
    }

    #[test]
    fn rejects_self_inheritance_cycle() {
        let mut declaration = declaration("A", TypeKind::Record { fields: Vec::new() });
        declaration.base_type = Some(named("A"));
        let error = schema(vec![declaration]).validate().unwrap_err();
        assert_eq!(
            error,
            ValidationError::InheritanceCycle {
                names: vec![QualifiedName::new(NS, "A"), QualifiedName::new(NS, "A")]
            }
        );
    }

    #[test]
    fn rejects_multi_level_inheritance_cycle_deterministically() {
        let mut a = declaration("A", TypeKind::Record { fields: Vec::new() });
        let mut b = declaration("B", TypeKind::Record { fields: Vec::new() });
        let mut c = declaration("C", TypeKind::Record { fields: Vec::new() });
        a.base_type = Some(named("B"));
        b.base_type = Some(named("C"));
        c.base_type = Some(named("A"));
        let error = schema(vec![a, b, c]).validate().unwrap_err();
        assert_eq!(
            error.to_string(),
            "complex-type inheritance cycle: {urn:test}A -> {urn:test}B -> {urn:test}C -> {urn:test}A"
        );
    }
}
