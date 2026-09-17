//! Deliberately narrow XSD frontend for OMS/UCI schemas.
//!
//! The supported subset is documented by [`load_schema_document`]. Anything outside
//! that subset is rejected rather than approximated.

use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, EnumVariant, FieldDecl, NamespaceDecl, PrimitiveKind,
    QualifiedName, SchemaIr, SourceRef, TypeDecl, TypeKind, TypeRef, TypeRefTarget,
};
use roxmltree::{Document, Node};
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::Path;

const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontendError {
    Io(String),
    InvalidInput(String),
    UnsupportedConstruct(String),
}

impl fmt::Display for FrontendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(f, "unable to read schema input: {message}"),
            Self::InvalidInput(message) => write!(f, "invalid schema input: {message}"),
            Self::UnsupportedConstruct(name) => {
                write!(f, "unsupported XSD construct: {name}")
            }
        }
    }
}

impl std::error::Error for FrontendError {}

/// Parse and normalize one XSD document into semantic IR.
///
/// This first slice supports a target namespace, named simple types that are
/// either integer restrictions with inclusive bounds or string enumerations,
/// and named complex types containing a sequence of explicitly typed elements
/// with finite occurrence bounds. Imports, includes, anonymous types, and all
/// other XSD constructs are explicit errors.
///
/// # Errors
///
/// Returns an I/O or XML/schema validation error for malformed input, or
/// [`FrontendError::UnsupportedConstruct`] for syntax outside the subset.
pub fn load_schema_document(path: &Path) -> Result<SchemaIr, FrontendError> {
    if !path.is_file() {
        return Err(FrontendError::InvalidInput(format!(
            "expected one XSD file, got {}",
            path.display()
        )));
    }

    let xml = fs::read_to_string(path).map_err(|error| FrontendError::Io(error.to_string()))?;
    let document =
        Document::parse(&xml).map_err(|error| FrontendError::InvalidInput(error.to_string()))?;
    let schema = document.root_element();
    require_xsd_element(schema, "schema")?;
    reject_unexpected_attributes(schema, &["targetNamespace", "version"])?;

    let target_namespace = required_attribute(schema, "targetNamespace")?.to_owned();
    let source_document = path.display().to_string();
    let declared_names = collect_declared_names(schema, &target_namespace)?;

    let mut types = Vec::new();
    for child in element_children(schema) {
        require_xsd_namespace(child)?;
        let declaration = match child.tag_name().name() {
            "simpleType" => {
                parse_simple_type(child, &document, &source_document, &target_namespace)?
            }
            "complexType" => parse_complex_type(
                child,
                &document,
                &source_document,
                &target_namespace,
                &declared_names,
            )?,
            other => return Err(unsupported(child, other)),
        };
        types.push(declaration);
    }

    Ok(SchemaIr {
        schema_version: schema.attribute("version").map(str::to_owned),
        namespaces: vec![NamespaceDecl {
            uri: target_namespace.clone(),
            preferred_prefix: preferred_prefix(schema, &target_namespace),
        }],
        types,
        messages: Vec::new(),
    })
}

fn collect_declared_names(
    schema: Node<'_, '_>,
    target_namespace: &str,
) -> Result<BTreeSet<QualifiedName>, FrontendError> {
    let mut names = BTreeSet::new();
    for child in element_children(schema) {
        require_xsd_namespace(child)?;
        match child.tag_name().name() {
            "simpleType" | "complexType" => {
                let name = QualifiedName::new(target_namespace, required_attribute(child, "name")?);
                if !names.insert(name.clone()) {
                    return Err(FrontendError::InvalidInput(format!(
                        "duplicate type {}",
                        name.local_name
                    )));
                }
            }
            other => return Err(unsupported(child, other)),
        }
    }
    Ok(names)
}

fn parse_simple_type(
    node: Node<'_, '_>,
    document: &Document<'_>,
    source_document: &str,
    target_namespace: &str,
) -> Result<TypeDecl, FrontendError> {
    reject_unexpected_attributes(node, &["name"])?;
    let name = qualified_declaration_name(node, target_namespace)?;
    let restriction = exactly_one_child(node)?;
    require_xsd_element(restriction, "restriction")?;
    reject_unexpected_attributes(restriction, &["base"])?;
    let base = resolve_type_ref(restriction, required_attribute(restriction, "base")?)?;

    let (kind, constraints) = match &base.target {
        TypeRefTarget::Primitive(PrimitiveKind::SignedInteger) => {
            let mut constraints = ConstraintSet::default();
            for facet in element_children(restriction) {
                require_xsd_namespace(facet)?;
                reject_unexpected_attributes(facet, &["value"])?;
                let value = required_attribute(facet, "value")?;
                match facet.tag_name().name() {
                    "minInclusive" => {
                        set_once(&mut constraints.min_inclusive, parse_integer(value)?, facet)?;
                    }
                    "maxInclusive" => {
                        set_once(&mut constraints.max_inclusive, parse_integer(value)?, facet)?;
                    }
                    other => return Err(unsupported(facet, other)),
                }
            }
            if let (Some(min), Some(max)) = (constraints.min_inclusive, constraints.max_inclusive) {
                if min > max {
                    return Err(FrontendError::InvalidInput(format!(
                        "inverted integer range on {}",
                        name.local_name
                    )));
                }
            }
            (
                TypeKind::Primitive(PrimitiveKind::SignedInteger),
                constraints,
            )
        }
        TypeRefTarget::Primitive(PrimitiveKind::String) => {
            let mut variants = Vec::new();
            for facet in element_children(restriction) {
                require_xsd_element(facet, "enumeration")?;
                reject_unexpected_attributes(facet, &["value"])?;
                if element_children(facet).next().is_some() {
                    return Err(unsupported(facet, "xs:enumeration child"));
                }
                variants.push(EnumVariant {
                    wire_value: required_attribute(facet, "value")?.to_owned(),
                    documentation: None,
                });
            }
            if variants.is_empty() {
                return Err(FrontendError::InvalidInput(format!(
                    "string restriction {} must contain enumeration facets",
                    name.local_name
                )));
            }
            (TypeKind::Enumeration { variants }, ConstraintSet::default())
        }
        _ => return Err(unsupported(restriction, "restriction base type")),
    };

    Ok(TypeDecl {
        name,
        is_abstract: false,
        base_type: Some(base),
        kind,
        constraints,
        documentation: None,
        source: source_ref(node, document, source_document),
    })
}

fn parse_complex_type(
    node: Node<'_, '_>,
    document: &Document<'_>,
    source_document: &str,
    target_namespace: &str,
    declared_names: &BTreeSet<QualifiedName>,
) -> Result<TypeDecl, FrontendError> {
    reject_unexpected_attributes(node, &["name"])?;
    let name = qualified_declaration_name(node, target_namespace)?;
    let sequence = exactly_one_child(node)?;
    require_xsd_element(sequence, "sequence")?;
    reject_unexpected_attributes(sequence, &[])?;

    let mut fields = Vec::new();
    for element in element_children(sequence) {
        require_xsd_element(element, "element")?;
        reject_unexpected_attributes(
            element,
            &["name", "type", "minOccurs", "maxOccurs", "nillable"],
        )?;
        if element_children(element).next().is_some() {
            return Err(unsupported(element, "anonymous element type"));
        }
        let type_ref = resolve_type_ref(element, required_attribute(element, "type")?)?;
        if let TypeRefTarget::Named(named) = &type_ref.target {
            if !declared_names.contains(named) {
                return Err(FrontendError::InvalidInput(format!(
                    "unresolved type reference {{{}}}{}",
                    named.namespace_uri, named.local_name
                )));
            }
        }

        fields.push(FieldDecl {
            name: required_attribute(element, "name")?.to_owned(),
            type_ref,
            cardinality: parse_cardinality(element)?,
            nillable: parse_boolean_attribute(element, "nillable", false)?,
            constraints: ConstraintSet::default(),
            documentation: None,
            source: source_ref(element, document, source_document),
        });
    }

    Ok(TypeDecl {
        name,
        is_abstract: false,
        base_type: None,
        kind: TypeKind::Record { fields },
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source_ref(node, document, source_document),
    })
}

fn resolve_type_ref(node: Node<'_, '_>, lexical: &str) -> Result<TypeRef, FrontendError> {
    let (prefix, local_name) = lexical.split_once(':').ok_or_else(|| {
        FrontendError::InvalidInput(format!("type QName must be prefixed: {lexical}"))
    })?;
    let namespace_uri = node
        .lookup_namespace_uri(Some(prefix))
        .ok_or_else(|| FrontendError::InvalidInput(format!("unbound QName prefix {prefix}")))?;

    if namespace_uri == XSD_NS {
        let primitive = match local_name {
            "integer" => PrimitiveKind::SignedInteger,
            "string" => PrimitiveKind::String,
            other => return Err(FrontendError::UnsupportedConstruct(format!("xs:{other}"))),
        };
        Ok(TypeRef::primitive(primitive))
    } else {
        Ok(TypeRef::named(QualifiedName::new(
            namespace_uri,
            local_name,
        )))
    }
}

fn parse_cardinality(node: Node<'_, '_>) -> Result<Cardinality, FrontendError> {
    let min_occurs = parse_u64_attribute(node, "minOccurs", 1)?;
    let max_occurs = match node.attribute("maxOccurs") {
        Some("unbounded") => return Err(unsupported(node, "maxOccurs=unbounded")),
        Some(value) => Some(parse_u64(value, "maxOccurs")?),
        None => Some(1),
    };
    let cardinality = Cardinality {
        min_occurs,
        max_occurs,
    };
    if !cardinality.is_valid() {
        return Err(FrontendError::InvalidInput(format!(
            "minOccurs exceeds maxOccurs on element {}",
            required_attribute(node, "name")?
        )));
    }
    Ok(cardinality)
}

fn qualified_declaration_name(
    node: Node<'_, '_>,
    target_namespace: &str,
) -> Result<QualifiedName, FrontendError> {
    Ok(QualifiedName::new(
        target_namespace,
        required_attribute(node, "name")?,
    ))
}

fn exactly_one_child<'a, 'input>(
    node: Node<'a, 'input>,
) -> Result<Node<'a, 'input>, FrontendError> {
    let mut children = element_children(node);
    let child = children.next().ok_or_else(|| {
        FrontendError::InvalidInput(format!("xs:{} requires a child", node.tag_name().name()))
    })?;
    if children.next().is_some() {
        return Err(unsupported(node, "multiple content-model children"));
    }
    Ok(child)
}

fn element_children<'a, 'input>(node: Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(Node::is_element)
}

fn require_xsd_namespace(node: Node<'_, '_>) -> Result<(), FrontendError> {
    if node.tag_name().namespace() != Some(XSD_NS) {
        return Err(unsupported(node, node.tag_name().name()));
    }
    Ok(())
}

fn require_xsd_element(node: Node<'_, '_>, expected: &str) -> Result<(), FrontendError> {
    require_xsd_namespace(node)?;
    if node.tag_name().name() != expected {
        return Err(unsupported(node, node.tag_name().name()));
    }
    Ok(())
}

fn required_attribute<'a>(node: Node<'a, '_>, name: &str) -> Result<&'a str, FrontendError> {
    node.attribute(name).ok_or_else(|| {
        FrontendError::InvalidInput(format!(
            "xs:{} is missing required attribute {name}",
            node.tag_name().name()
        ))
    })
}

fn reject_unexpected_attributes(node: Node<'_, '_>, allowed: &[&str]) -> Result<(), FrontendError> {
    for attribute in node.attributes() {
        if attribute.namespace().is_some() || !allowed.contains(&attribute.name()) {
            return Err(unsupported(
                node,
                &format!("xs:{} @{}", node.tag_name().name(), attribute.name()),
            ));
        }
    }
    Ok(())
}

fn parse_u64_attribute(node: Node<'_, '_>, name: &str, default: u64) -> Result<u64, FrontendError> {
    node.attribute(name)
        .map_or(Ok(default), |value| parse_u64(value, name))
}

fn parse_u64(value: &str, name: &str) -> Result<u64, FrontendError> {
    value.parse().map_err(|_| {
        FrontendError::InvalidInput(format!("{name} is not a non-negative integer: {value}"))
    })
}

fn parse_integer(value: &str) -> Result<i128, FrontendError> {
    value.parse().map_err(|_| {
        FrontendError::InvalidInput(format!("integer facet is not an integer: {value}"))
    })
}

fn parse_boolean_attribute(
    node: Node<'_, '_>,
    name: &str,
    default: bool,
) -> Result<bool, FrontendError> {
    match node.attribute(name) {
        None => Ok(default),
        Some("true" | "1") => Ok(true),
        Some("false" | "0") => Ok(false),
        Some(value) => Err(FrontendError::InvalidInput(format!(
            "{name} is not an XSD boolean: {value}"
        ))),
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, node: Node<'_, '_>) -> Result<(), FrontendError> {
    if slot.replace(value).is_some() {
        return Err(FrontendError::InvalidInput(format!(
            "duplicate xs:{} facet",
            node.tag_name().name()
        )));
    }
    Ok(())
}

fn source_ref(node: Node<'_, '_>, document: &Document<'_>, path: &str) -> SourceRef {
    SourceRef {
        document: path.to_owned(),
        line: Some(document.text_pos_at(node.range().start).row),
    }
}

fn preferred_prefix(node: Node<'_, '_>, namespace_uri: &str) -> Option<String> {
    node.namespaces()
        .find(|namespace| namespace.uri() == namespace_uri)
        .and_then(|namespace| namespace.name())
        .map(str::to_owned)
}

fn unsupported(node: Node<'_, '_>, name: &str) -> FrontendError {
    let prefix = if node.tag_name().namespace() == Some(XSD_NS) {
        "xs:"
    } else {
        ""
    };
    FrontendError::UnsupportedConstruct(format!("{prefix}{name}"))
}
