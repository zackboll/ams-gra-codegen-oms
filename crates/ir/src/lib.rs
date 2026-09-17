//! Language-neutral semantic schema IR for OMS/UCI code generation.
//!
//! This crate intentionally contains no XML/XSD parser logic and no
//! language-specific code-generation policy.

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
}
