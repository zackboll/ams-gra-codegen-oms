//! Deliberately narrow XSD frontend for OMS/UCI schemas.
//!
//! The supported subset is documented by [`load_schema_document`] and
//! [`load_schema_set`]. Anything outside that subset is rejected rather than
//! approximated.

use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, EnumVariant, FieldDecl, MessageDecl, NamespaceDecl, PrimitiveKind,
    QualifiedName, SchemaIr, SourceRef, TypeDecl, TypeKind, TypeRef, TypeRefTarget,
};
use roxmltree::{Document, Node};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema";
const UCI_VERSION_NS: &str = "https://www.vdl.afrl.af.mil/programs/oam";

#[derive(Debug)]
struct ParsedSchemaDocument {
    source: PathBuf,
    target_namespace: String,
    preferred_prefix: Option<String>,
    schema_version: Option<String>,
    dependencies: Vec<SchemaDependency>,
    declarations: Vec<TypeDecl>,
    messages: Vec<MessageDecl>,
}

#[derive(Debug, Clone)]
enum SchemaDependency {
    Include {
        schema_location: String,
        position: TextPosition,
    },
    Import {
        namespace: String,
        schema_location: String,
        position: TextPosition,
    },
}

#[derive(Debug, Clone, Copy)]
struct TextPosition {
    line: u32,
    column: u32,
}

#[derive(Debug)]
enum NamespaceExpectation<'a> {
    Include(&'a str),
    Import(&'a str),
}

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
/// This API supports a target namespace, named simple types that are
/// either integer restrictions with range bounds or string enumerations, and
/// named complex types containing a sequence or choice of explicitly typed
/// elements, and schema-level named and typed message
/// elements. Imports and includes are explicit errors;
/// use [`load_schema_set`] when dependencies should be traversed. Anonymous
/// types and all other XSD constructs are also explicit errors. Schema-level
/// `elementFormDefault` and `attributeFormDefault` are validated and discarded
/// because XML instance namespace qualification is outside the normalized type
/// model.
/// Leading annotations containing plain-text `xs:documentation` are accepted.
/// Documentation for types, global messages, sequence fields, and enumeration
/// variants is normalized into the corresponding IR field; schema and restriction
/// documentation has no semantic IR owner and is discarded. Other annotation
/// content remains unsupported.
/// UCI/OAM declaration-version attributes on supported messages and named types
/// are validated as nonempty opaque metadata and discarded. The marker remains
/// required for global-message classification but optional on generic named
/// type declarations.
///
/// # Errors
///
/// Returns an I/O or XML/schema validation error for malformed input, or
/// [`FrontendError::UnsupportedConstruct`] for syntax outside the subset.
pub fn load_schema_document(path: &Path) -> Result<SchemaIr, FrontendError> {
    let document = parse_schema_document(path)?;
    if let Some(dependency) = document.dependencies.first() {
        let (construct, position) = match dependency {
            SchemaDependency::Include { position, .. } => ("xs:include", position),
            SchemaDependency::Import { position, .. } => ("xs:import", position),
        };
        return Err(FrontendError::UnsupportedConstruct(format!(
            "{construct} in standalone schema document at {}:{}:{}",
            document.source.display(),
            position.line,
            position.column
        )));
    }
    documents_into_ir(vec![document])
}

/// Load a root XSD document and recursively normalize its local imports and
/// includes into one language-neutral schema IR.
///
/// Dependency paths are resolved relative to the referring document. Only
/// local filesystem locations are supported. Documents are discovered in
/// deterministic pre-order depth-first traversal, following dependency source
/// order; declarations retain source order within each document.
///
/// # Errors
///
/// Returns an error for unreadable or malformed documents, unsupported XSD,
/// invalid dependency namespaces or locations, duplicate declarations, and
/// unresolved named type references.
pub fn load_schema_set(path: &Path) -> Result<SchemaIr, FrontendError> {
    let mut loader = SchemaSetLoader::default();
    loader.load(path, None, None)?;
    documents_into_ir(loader.documents)
}

#[derive(Default)]
struct SchemaSetLoader {
    identities: BTreeMap<PathBuf, usize>,
    documents: Vec<ParsedSchemaDocument>,
}

impl SchemaSetLoader {
    fn load(
        &mut self,
        path: &Path,
        expectation: Option<NamespaceExpectation<'_>>,
        referring_document: Option<&Path>,
    ) -> Result<(), FrontendError> {
        let identity = fs::canonicalize(path).map_err(|error| {
            let context = referring_document.map_or_else(String::new, |source| {
                format!(" referenced from {}", source.display())
            });
            FrontendError::Io(format!("{}{}: {error}", path.display(), context))
        })?;

        if let Some(index) = self.identities.get(&identity).copied() {
            validate_dependency_namespace(&self.documents[index], expectation)?;
            return Ok(());
        }

        let document = parse_schema_document(path)?;
        validate_dependency_namespace(&document, expectation)?;

        let index = self.documents.len();
        self.identities.insert(identity, index);
        self.documents.push(document);

        let dependencies = self.documents[index].dependencies.clone();
        let including_namespace = self.documents[index].target_namespace.clone();
        let source = self.documents[index].source.clone();

        for dependency in dependencies {
            let location = match &dependency {
                SchemaDependency::Include {
                    schema_location, ..
                }
                | SchemaDependency::Import {
                    schema_location, ..
                } => schema_location,
            };
            let expectation = match &dependency {
                SchemaDependency::Include { .. } => {
                    NamespaceExpectation::Include(&including_namespace)
                }
                SchemaDependency::Import { namespace, .. } => {
                    NamespaceExpectation::Import(namespace)
                }
            };
            reject_remote_location(location, &source)?;
            let dependency_path = source
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(location);
            self.load(&dependency_path, Some(expectation), Some(&source))?;
        }
        Ok(())
    }
}

fn parse_schema_document(path: &Path) -> Result<ParsedSchemaDocument, FrontendError> {
    if !path.is_file() {
        return Err(FrontendError::InvalidInput(format!(
            "expected one XSD file, got {}",
            path.display()
        )));
    }

    let xml = fs::read_to_string(path)
        .map_err(|error| contextualize(FrontendError::Io(error.to_string()), path))?;
    let document = Document::parse(&xml)
        .map_err(|error| contextualize(FrontendError::InvalidInput(error.to_string()), path))?;
    parse_schema_document_xml(path, &document).map_err(|error| contextualize(error, path))
}

fn parse_schema_document_xml(
    path: &Path,
    document: &Document<'_>,
) -> Result<ParsedSchemaDocument, FrontendError> {
    let schema = document.root_element();
    require_xsd_element(schema, "schema")?;
    reject_unexpected_attributes(
        schema,
        &[
            "targetNamespace",
            "version",
            "elementFormDefault",
            "attributeFormDefault",
        ],
    )?;
    parse_form_default(schema, "elementFormDefault")?;
    parse_form_default(schema, "attributeFormDefault")?;

    let target_namespace = required_attribute(schema, "targetNamespace")?.to_owned();
    let source_document = path.display().to_string();
    let mut dependencies = Vec::new();
    let mut declarations = Vec::new();
    let mut messages = Vec::new();
    let (_, children) = children_after_optional_annotation(schema)?;
    for child in children {
        require_xsd_namespace(child)?;
        match child.tag_name().name() {
            "include" => {
                reject_unexpected_attributes(child, &["schemaLocation"])?;
                dependencies.push(SchemaDependency::Include {
                    schema_location: required_attribute(child, "schemaLocation")?.to_owned(),
                    position: text_position(child),
                });
            }
            "import" => {
                reject_unexpected_attributes(child, &["namespace", "schemaLocation"])?;
                dependencies.push(SchemaDependency::Import {
                    namespace: required_attribute(child, "namespace")?.to_owned(),
                    schema_location: required_attribute(child, "schemaLocation")?.to_owned(),
                    position: text_position(child),
                });
            }
            "simpleType" => declarations.push(parse_simple_type(
                child,
                document,
                &source_document,
                &target_namespace,
            )?),
            "complexType" => declarations.push(parse_complex_type(
                child,
                document,
                &source_document,
                &target_namespace,
            )?),
            "element" => messages.push(parse_global_element(
                child,
                document,
                &source_document,
                &target_namespace,
            )?),
            other => return Err(unsupported(child, other)),
        }
    }

    Ok(ParsedSchemaDocument {
        source: path.to_owned(),
        preferred_prefix: preferred_prefix(schema, &target_namespace),
        target_namespace,
        schema_version: schema.attribute("version").map(str::to_owned),
        dependencies,
        declarations,
        messages,
    })
}

fn documents_into_ir(documents: Vec<ParsedSchemaDocument>) -> Result<SchemaIr, FrontendError> {
    let schema_version = documents
        .first()
        .and_then(|document| document.schema_version.clone());
    let mut namespace_uris = BTreeSet::new();
    let mut namespaces = Vec::new();
    let mut types = Vec::new();
    let mut messages = Vec::new();

    for document in documents {
        if namespace_uris.insert(document.target_namespace.clone()) {
            namespaces.push(NamespaceDecl {
                uri: document.target_namespace,
                preferred_prefix: document.preferred_prefix,
            });
        }
        for declaration in document.declarations {
            types.push(declaration);
        }
        messages.extend(document.messages);
    }

    let schema = SchemaIr {
        schema_version,
        namespaces,
        types,
        messages,
    };
    schema
        .validate()
        .map_err(|error| FrontendError::InvalidInput(error.to_string()))?;
    Ok(schema)
}

fn parse_simple_type(
    node: Node<'_, '_>,
    document: &Document<'_>,
    source_document: &str,
    target_namespace: &str,
) -> Result<TypeDecl, FrontendError> {
    validate_uci_declaration_version(node, &["name"], false)?;
    let name = qualified_declaration_name(node, target_namespace)?;
    let (documentation, restriction) = exactly_one_content_child(node)?;
    require_xsd_element(restriction, "restriction")?;
    reject_unexpected_attributes(restriction, &["base"])?;
    let lexical_base = required_attribute(restriction, "base")?;
    let base = resolve_type_ref(restriction, lexical_base)?;
    let (_, restriction_children) = children_after_optional_annotation(restriction)?;

    let (kind, constraints) = match &base.target {
        TypeRefTarget::Primitive(
            primitive @ (PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger),
        ) => {
            let intrinsic = builtin_primitive_semantics(restriction, lexical_base)?.constraints;
            let mut explicit = ConstraintSet::default();
            for facet in restriction_children {
                require_xsd_namespace(facet)?;
                reject_unexpected_attributes(facet, &["value"])?;
                let name = facet.tag_name().name();
                match name {
                    "minInclusive" | "maxInclusive" | "minExclusive" | "maxExclusive" => {
                        validate_constraint_facet_children(facet)?;
                    }
                    other => return Err(unsupported(facet, other)),
                }
                match name {
                    "minInclusive" => set_once(
                        &mut explicit.min_inclusive,
                        parse_integer(required_attribute(facet, "value")?)?,
                        facet,
                    )?,
                    "maxInclusive" => set_once(
                        &mut explicit.max_inclusive,
                        parse_integer(required_attribute(facet, "value")?)?,
                        facet,
                    )?,
                    "minExclusive" => set_once(
                        &mut explicit.min_exclusive,
                        parse_integer(required_attribute(facet, "value")?)?,
                        facet,
                    )?,
                    "maxExclusive" => set_once(
                        &mut explicit.max_exclusive,
                        parse_integer(required_attribute(facet, "value")?)?,
                        facet,
                    )?,
                    _ => unreachable!("supported integer facet was checked above"),
                }
            }
            (
                TypeKind::Primitive(*primitive),
                intersect_integer_constraints(&intrinsic, &explicit),
            )
        }
        TypeRefTarget::Primitive(PrimitiveKind::String)
            if !restriction_children.is_empty()
                && restriction_children
                    .iter()
                    .all(|facet| facet.tag_name().name() == "enumeration") =>
        {
            let mut variants = Vec::new();
            for facet in restriction_children {
                require_xsd_element(facet, "enumeration")?;
                reject_unexpected_attributes(facet, &["value"])?;
                let (documentation, children) = children_after_optional_annotation(facet)?;
                if !children.is_empty() {
                    return Err(unsupported(facet, "enumeration child"));
                }
                variants.push(EnumVariant {
                    wire_value: required_attribute(facet, "value")?.to_owned(),
                    documentation,
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
        TypeRefTarget::Primitive(primitive) => (
            TypeKind::Primitive(*primitive),
            parse_scalar_restriction_facets(*primitive, &restriction_children)?,
        ),
        _ => return Err(unsupported(restriction, "restriction base type")),
    };

    Ok(TypeDecl {
        name,
        is_abstract: false,
        base_type: Some(base),
        kind,
        constraints,
        documentation,
        source: source_ref(node, document, source_document),
    })
}

fn parse_scalar_restriction_facets(
    primitive: PrimitiveKind,
    facets: &[Node<'_, '_>],
) -> Result<ConstraintSet, FrontendError> {
    let mut constraints = ConstraintSet::default();
    for &facet in facets {
        require_xsd_namespace(facet)?;
        reject_unexpected_attributes(facet, &["value"])?;
        let name = facet.tag_name().name();
        match (primitive, name) {
            (
                PrimitiveKind::String | PrimitiveKind::Binary,
                "length" | "minLength" | "maxLength",
            )
            | (PrimitiveKind::String, "pattern") => {
                validate_constraint_facet_children(facet)?;
            }
            _ => return Err(unsupported(facet, name)),
        }
        match (primitive, name) {
            (PrimitiveKind::String | PrimitiveKind::Binary, "length") => {
                let value = parse_u64(required_attribute(facet, "value")?, "length facet")?;
                set_once(&mut constraints.length, value, facet)?;
            }
            (PrimitiveKind::String | PrimitiveKind::Binary, "minLength") => {
                let value = parse_u64(required_attribute(facet, "value")?, "minLength facet")?;
                set_once(&mut constraints.min_length, value, facet)?;
            }
            (PrimitiveKind::String | PrimitiveKind::Binary, "maxLength") => {
                let value = parse_u64(required_attribute(facet, "value")?, "maxLength facet")?;
                set_once(&mut constraints.max_length, value, facet)?;
            }
            (PrimitiveKind::String, "pattern") => constraints
                .patterns
                .push(required_attribute(facet, "value")?.to_owned()),
            _ => unreachable!("supported scalar facet was checked above"),
        }
    }
    Ok(constraints)
}

fn validate_constraint_facet_children(facet: Node<'_, '_>) -> Result<(), FrontendError> {
    let (_documentation, children) = children_after_optional_annotation(facet)?;
    if let Some(child) = children.first() {
        return Err(unsupported(*child, child.tag_name().name()));
    }
    Ok(())
}

fn parse_complex_type(
    node: Node<'_, '_>,
    document: &Document<'_>,
    source_document: &str,
    target_namespace: &str,
) -> Result<TypeDecl, FrontendError> {
    validate_uci_declaration_version(node, &["name", "abstract"], false)?;
    let name = qualified_declaration_name(node, target_namespace)?;
    let is_abstract = parse_boolean_attribute(node, "abstract", false)?;
    let (documentation, children) = children_after_optional_annotation(node)?;
    let (base_type, kind) = match children.as_slice() {
        [] if is_abstract => (None, TypeKind::Record { fields: Vec::new() }),
        [content] if content.tag_name().namespace() == Some(XSD_NS) => {
            match content.tag_name().name() {
                "sequence" | "choice" => {
                    (None, parse_compositor(*content, document, source_document)?)
                }
                "complexContent" => parse_complex_content(*content, document, source_document)?,
                other => return Err(unsupported(*content, other)),
            }
        }
        [] => {
            return Err(FrontendError::InvalidInput(
                "xs:complexType requires a child".to_owned(),
            ));
        }
        _ => return Err(unsupported(node, "multiple content-model children")),
    };

    Ok(TypeDecl {
        name,
        is_abstract,
        base_type,
        kind,
        constraints: ConstraintSet::default(),
        documentation,
        source: source_ref(node, document, source_document),
    })
}

fn parse_complex_content(
    node: Node<'_, '_>,
    document: &Document<'_>,
    source_document: &str,
) -> Result<(Option<TypeRef>, TypeKind), FrontendError> {
    reject_unexpected_attributes(node, &[])?;
    let mut children = element_children(node);
    let extension = children.next().ok_or_else(|| {
        FrontendError::InvalidInput("xs:complexContent requires a child".to_owned())
    })?;
    if children.next().is_some() {
        return Err(unsupported(node, "multiple content-model children"));
    }
    require_xsd_element(extension, "extension")?;
    reject_unexpected_attributes(extension, &["base"])?;
    let base_type = resolve_type_ref(extension, required_attribute(extension, "base")?)?;
    if !matches!(base_type.target, TypeRefTarget::Named(_)) {
        return Err(unsupported(extension, "extension primitive base"));
    }

    let children = element_children(extension).collect::<Vec<_>>();
    let kind = match children.as_slice() {
        [] => TypeKind::Record { fields: Vec::new() },
        [compositor] => parse_compositor(*compositor, document, source_document)?,
        _ => {
            return Err(unsupported(
                extension,
                "multiple extension content children",
            ));
        }
    };
    Ok((Some(base_type), kind))
}

fn parse_compositor(
    compositor: Node<'_, '_>,
    document: &Document<'_>,
    source_document: &str,
) -> Result<TypeKind, FrontendError> {
    require_xsd_namespace(compositor)?;
    let is_choice = match compositor.tag_name().name() {
        "sequence" => false,
        "choice" => true,
        other => return Err(unsupported(compositor, other)),
    };
    reject_unexpected_attributes(compositor, &[])?;
    let mut fields = Vec::new();
    for element in element_children(compositor) {
        require_xsd_element(element, "element")?;
        fields.push(parse_local_element(element, document, source_document)?);
    }
    Ok(if is_choice {
        TypeKind::Choice {
            alternatives: fields,
        }
    } else {
        TypeKind::Record { fields }
    })
}

fn parse_local_element(
    element: Node<'_, '_>,
    document: &Document<'_>,
    source_document: &str,
) -> Result<FieldDecl, FrontendError> {
    reject_unexpected_attributes(
        element,
        &["name", "type", "minOccurs", "maxOccurs", "nillable"],
    )?;
    let (documentation, children) = children_after_optional_annotation(element)?;
    if !children.is_empty() {
        return Err(unsupported(element, "anonymous element type"));
    }
    let semantics = resolve_type_semantics(element, required_attribute(element, "type")?)?;
    Ok(FieldDecl {
        name: required_attribute(element, "name")?.to_owned(),
        type_ref: semantics.type_ref,
        cardinality: parse_cardinality(element)?,
        nillable: parse_boolean_attribute(element, "nillable", false)?,
        constraints: semantics.constraints,
        documentation,
        source: source_ref(element, document, source_document),
    })
}

fn parse_global_element(
    node: Node<'_, '_>,
    document: &Document<'_>,
    source_document: &str,
    target_namespace: &str,
) -> Result<MessageDecl, FrontendError> {
    validate_uci_declaration_version(node, &["name", "type"], true)?;

    let (documentation, children) = children_after_optional_annotation(node)?;
    if !children.is_empty() {
        return Err(unsupported(node, "anonymous global element type"));
    }

    let payload_type = resolve_type_ref(node, required_attribute(node, "type")?)?;
    if !matches!(payload_type.target, TypeRefTarget::Named(_)) {
        let position = text_position(node);
        return Err(FrontendError::UnsupportedConstruct(format!(
            "UCI message payload must reference a named schema type at {}:{}",
            position.line, position.column
        )));
    }

    Ok(MessageDecl {
        name: qualified_declaration_name(node, target_namespace)?,
        payload_type,
        documentation,
        source: source_ref(node, document, source_document),
    })
}

fn validate_uci_declaration_version(
    node: Node<'_, '_>,
    allowed_unqualified: &[&str],
    required: bool,
) -> Result<(), FrontendError> {
    let mut version = None;
    for attribute in node.attributes() {
        match attribute.namespace() {
            None if allowed_unqualified.contains(&attribute.name()) => {}
            Some(UCI_VERSION_NS) if attribute.name() == "version" => {
                version = Some(attribute.value());
            }
            _ => {
                return Err(unsupported(
                    node,
                    &format!("{} @{}", node.tag_name().name(), attribute.name()),
                ));
            }
        }
    }

    let position = text_position(node);
    let Some(value) = version else {
        if required {
            return Err(FrontendError::InvalidInput(format!(
                "xs:{} is missing required UCI version attribute at {}:{}",
                node.tag_name().name(),
                position.line,
                position.column
            )));
        }
        return Ok(());
    };
    if value.trim().is_empty() {
        return Err(FrontendError::InvalidInput(format!(
            "xs:{} UCI version attribute must not be empty at {}:{}",
            node.tag_name().name(),
            position.line,
            position.column
        )));
    }
    Ok(())
}

fn validate_dependency_namespace(
    document: &ParsedSchemaDocument,
    expectation: Option<NamespaceExpectation<'_>>,
) -> Result<(), FrontendError> {
    let Some(expectation) = expectation else {
        return Ok(());
    };
    let (kind, expected) = match expectation {
        NamespaceExpectation::Include(namespace) => ("xs:include", namespace),
        NamespaceExpectation::Import(namespace) => ("xs:import", namespace),
    };
    if document.target_namespace != expected {
        return Err(FrontendError::InvalidInput(format!(
            "{kind} target namespace mismatch for {}: expected {expected}, found {}",
            document.source.display(),
            document.target_namespace
        )));
    }
    Ok(())
}

fn reject_remote_location(location: &str, source: &Path) -> Result<(), FrontendError> {
    if location.starts_with("http://") || location.starts_with("https://") {
        return Err(FrontendError::InvalidInput(format!(
            "remote schema location {location} in {} is unsupported; only local filesystem paths are allowed",
            source.display()
        )));
    }
    Ok(())
}

fn contextualize(error: FrontendError, path: &Path) -> FrontendError {
    match error {
        FrontendError::Io(message) => FrontendError::Io(format!("{}: {message}", path.display())),
        FrontendError::InvalidInput(message) => {
            FrontendError::InvalidInput(format!("{}: {message}", path.display()))
        }
        FrontendError::UnsupportedConstruct(message) => {
            let message = message.rsplit_once(" at ").map_or_else(
                || format!("{message} at {}", path.display()),
                |(construct, position)| format!("{construct} at {}:{position}", path.display()),
            );
            FrontendError::UnsupportedConstruct(message)
        }
    }
}

fn resolve_type_ref(node: Node<'_, '_>, lexical: &str) -> Result<TypeRef, FrontendError> {
    Ok(resolve_type_semantics(node, lexical)?.type_ref)
}

#[derive(Debug)]
struct ResolvedTypeSemantics {
    type_ref: TypeRef,
    constraints: ConstraintSet,
}

#[derive(Debug)]
struct BuiltinPrimitiveSemantics {
    kind: PrimitiveKind,
    constraints: ConstraintSet,
}

fn resolve_type_semantics(
    node: Node<'_, '_>,
    lexical: &str,
) -> Result<ResolvedTypeSemantics, FrontendError> {
    let (prefix, local_name) = lexical.split_once(':').ok_or_else(|| {
        FrontendError::InvalidInput(format!("type QName must be prefixed: {lexical}"))
    })?;
    let namespace_uri = node
        .lookup_namespace_uri(Some(prefix))
        .ok_or_else(|| FrontendError::InvalidInput(format!("unbound QName prefix {prefix}")))?;
    if namespace_uri == XSD_NS {
        let semantics = builtin_primitive_semantics(node, lexical)?;
        Ok(ResolvedTypeSemantics {
            type_ref: TypeRef::primitive(semantics.kind),
            constraints: semantics.constraints,
        })
    } else {
        Ok(ResolvedTypeSemantics {
            type_ref: TypeRef::named(QualifiedName::new(namespace_uri, local_name)),
            constraints: ConstraintSet::default(),
        })
    }
}

fn builtin_primitive_semantics(
    node: Node<'_, '_>,
    lexical: &str,
) -> Result<BuiltinPrimitiveSemantics, FrontendError> {
    let local_name = lexical.rsplit_once(':').map_or(lexical, |(_, name)| name);
    let (kind, min_inclusive, max_inclusive) = match local_name {
        "boolean" => (PrimitiveKind::Boolean, None, None),
        "byte" => (PrimitiveKind::SignedInteger, Some(-128), Some(127)),
        "short" => (PrimitiveKind::SignedInteger, Some(-32_768), Some(32_767)),
        "int" => (
            PrimitiveKind::SignedInteger,
            Some(-2_147_483_648),
            Some(2_147_483_647),
        ),
        "long" => (
            PrimitiveKind::SignedInteger,
            Some(-9_223_372_036_854_775_808),
            Some(9_223_372_036_854_775_807),
        ),
        "unsignedByte" => (PrimitiveKind::UnsignedInteger, Some(0), Some(255)),
        "unsignedShort" => (PrimitiveKind::UnsignedInteger, Some(0), Some(65_535)),
        "unsignedInt" => (PrimitiveKind::UnsignedInteger, Some(0), Some(4_294_967_295)),
        "float" => (PrimitiveKind::Float32, None, None),
        "double" => (PrimitiveKind::Float64, None, None),
        "dateTime" => (PrimitiveKind::DateTime, None, None),
        "time" => (PrimitiveKind::Time, None, None),
        "duration" => (PrimitiveKind::Duration, None, None),
        "hexBinary" => (PrimitiveKind::Binary, None, None),
        "string" => (PrimitiveKind::String, None, None),
        "integer" => (PrimitiveKind::SignedInteger, None, None),
        other => return Err(unsupported(node, other)),
    };
    Ok(BuiltinPrimitiveSemantics {
        kind,
        constraints: ConstraintSet {
            min_inclusive,
            max_inclusive,
            ..ConstraintSet::default()
        },
    })
}

fn parse_cardinality(node: Node<'_, '_>) -> Result<Cardinality, FrontendError> {
    let min_occurs = parse_u64_attribute(node, "minOccurs", 1)?;
    let max_occurs = match node.attribute("maxOccurs") {
        Some("unbounded") => None,
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

fn exactly_one_content_child<'a, 'input>(
    node: Node<'a, 'input>,
) -> Result<(Option<String>, Node<'a, 'input>), FrontendError> {
    let (documentation, children) = children_after_optional_annotation(node)?;
    let mut children = children.into_iter();
    let child = children.next().ok_or_else(|| {
        FrontendError::InvalidInput(format!("xs:{} requires a child", node.tag_name().name()))
    })?;
    if children.next().is_some() {
        return Err(unsupported(node, "multiple content-model children"));
    }
    Ok((documentation, child))
}

fn children_after_optional_annotation<'a, 'input>(
    node: Node<'a, 'input>,
) -> Result<(Option<String>, Vec<Node<'a, 'input>>), FrontendError> {
    let mut children = element_children(node).peekable();
    let documentation = children
        .next_if(|child| {
            child.tag_name().namespace() == Some(XSD_NS) && child.tag_name().name() == "annotation"
        })
        .map(parse_annotation)
        .transpose()?
        .flatten();
    let children = children.collect::<Vec<_>>();
    if let Some(annotation) = children.iter().find(|child| {
        child.tag_name().namespace() == Some(XSD_NS) && child.tag_name().name() == "annotation"
    }) {
        return Err(unsupported(
            *annotation,
            "annotation outside leading position",
        ));
    }
    Ok((documentation, children))
}

fn parse_annotation(node: Node<'_, '_>) -> Result<Option<String>, FrontendError> {
    require_xsd_element(node, "annotation")?;
    reject_unexpected_attributes(node, &[])?;
    let mut documentation = Vec::new();
    for child in element_children(node) {
        require_xsd_namespace(child)?;
        match child.tag_name().name() {
            "documentation" => {
                reject_unexpected_attributes(child, &[])?;
                if let Some(nested) = element_children(child).next() {
                    return Err(unsupported(nested, nested.tag_name().name()));
                }
                let normalized = normalize_documentation(child.text().unwrap_or_default());
                if !normalized.is_empty() {
                    documentation.push(normalized);
                }
            }
            other => return Err(unsupported(child, other)),
        }
    }
    if documentation.is_empty() {
        Ok(None)
    } else {
        Ok(Some(documentation.join("\n\n")))
    }
}

fn normalize_documentation(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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
                &format!("{} @{}", node.tag_name().name(), attribute.name()),
            ));
        }
    }
    Ok(())
}

fn parse_form_default(node: Node<'_, '_>, attribute_name: &str) -> Result<(), FrontendError> {
    match node.attribute(attribute_name) {
        None | Some("qualified" | "unqualified") => Ok(()),
        Some(value) => {
            let position = text_position(node);
            Err(FrontendError::InvalidInput(format!(
                "xs:schema @{attribute_name} must be qualified or unqualified, got {value} at {}:{}",
                position.line, position.column
            )))
        }
    }
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

fn intersect_integer_constraints(
    intrinsic: &ConstraintSet,
    explicit: &ConstraintSet,
) -> ConstraintSet {
    let lower = strongest_lower_bound(intrinsic)
        .into_iter()
        .chain(strongest_lower_bound(explicit))
        .max_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    let upper = strongest_upper_bound(intrinsic)
        .into_iter()
        .chain(strongest_upper_bound(explicit))
        .min_by(|left, right| left.0.cmp(&right.0).then(right.1.cmp(&left.1)));
    ConstraintSet {
        min_inclusive: lower.and_then(|(value, exclusive)| (!exclusive).then_some(value)),
        min_exclusive: lower.and_then(|(value, exclusive)| exclusive.then_some(value)),
        max_inclusive: upper.and_then(|(value, exclusive)| (!exclusive).then_some(value)),
        max_exclusive: upper.and_then(|(value, exclusive)| exclusive.then_some(value)),
        ..ConstraintSet::default()
    }
}

fn strongest_lower_bound(constraints: &ConstraintSet) -> Option<(i128, bool)> {
    constraints
        .min_inclusive
        .map(|value| (value, false))
        .into_iter()
        .chain(constraints.min_exclusive.map(|value| (value, true)))
        .max_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)))
}

fn strongest_upper_bound(constraints: &ConstraintSet) -> Option<(i128, bool)> {
    constraints
        .max_inclusive
        .map(|value| (value, false))
        .into_iter()
        .chain(constraints.max_exclusive.map(|value| (value, true)))
        .min_by(|left, right| left.0.cmp(&right.0).then(right.1.cmp(&left.1)))
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
    let position = text_position(node);
    FrontendError::UnsupportedConstruct(format!(
        "{prefix}{name} at {}:{}",
        position.line, position.column
    ))
}

fn text_position(node: Node<'_, '_>) -> TextPosition {
    let position = node.document().text_pos_at(node.range().start);
    TextPosition {
        line: position.row,
        column: position.col,
    }
}
