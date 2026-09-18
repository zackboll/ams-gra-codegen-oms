//! Language-neutral semantic schema IR for OMS/UCI code generation.
//!
//! This crate intentionally contains no XML/XSD parser logic and no
//! language-specific code-generation policy.

use std::collections::BTreeSet;
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
            validate_constraints(&declaration.constraints, &owner)?;
            if let Some(base_type) = &declaration.base_type {
                validate_reference(base_type, &declared_types, "base type", &declaration.source)?;
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
    ContradictoryLengthConstraints {
        owner: String,
    },
    EmptyEnumeration {
        name: QualifiedName,
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
            Self::ContradictoryLengthConstraints { owner } => {
                write!(f, "contradictory length constraints on {owner}")
            }
            Self::EmptyEnumeration { name } => {
                write!(f, "enumeration {} has no variants", format_name(name))
            }
        }
    }
}

impl std::error::Error for ValidationError {}

fn validate_fields(
    fields: &[FieldDecl],
    declared_types: &BTreeSet<QualifiedName>,
    location: &'static str,
) -> Result<(), ValidationError> {
    for field in fields {
        validate_reference(&field.type_ref, declared_types, location, &field.source)?;
        validate_cardinality(field.cardinality, &format!("field {}", field.name))?;
        validate_constraints(&field.constraints, &format!("field {}", field.name))?;
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

fn validate_constraints(constraints: &ConstraintSet, owner: &str) -> Result<(), ValidationError> {
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
            if lower > upper || (lower == upper && (lower_exclusive || upper_exclusive)) {
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
    pub min_inclusive: Option<i128>,
    pub max_inclusive: Option<i128>,
    pub min_exclusive: Option<i128>,
    pub max_exclusive: Option<i128>,
    pub length: Option<u64>,
    pub min_length: Option<u64>,
    pub max_length: Option<u64>,
    pub patterns: Vec<String>,
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
        value.constraints.min_inclusive = Some(10);
        value.constraints.max_exclusive = Some(10);
        assert!(matches!(
            schema(vec![value]).validate(),
            Err(ValidationError::ContradictoryNumericRange { .. })
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
}
