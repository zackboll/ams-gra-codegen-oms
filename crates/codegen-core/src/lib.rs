//! Shared backend contract for OMS/UCI source generation.

use ams_gra_oms_ir::{QualifiedName, SchemaIr, TypeDecl, TypeKind, TypeRef, TypeRefTarget};
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    pub relative_path: PathBuf,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenError {
    pub message: String,
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CodegenError {}

/// Produce a dependency-first declaration order from normalized schema IR.
///
/// Declarations and each declaration's references are visited in IR order.
/// A declaration is appended after recursively visiting its dependencies, so
/// unrelated declarations retain IR order unless dependency constraints force
/// movement.
///
/// # Errors
///
/// Returns an error if the schema is semantically invalid or if named type
/// dependencies contain a direct or indirect cycle.
pub fn plan_type_declarations(schema: &SchemaIr) -> Result<Vec<&TypeDecl>, CodegenError> {
    schema.validate().map_err(|error| CodegenError {
        message: format!("invalid schema IR: {error}"),
    })?;

    let indices = schema
        .types
        .iter()
        .enumerate()
        .map(|(index, declaration)| (declaration.name.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let mut states = vec![VisitState::Unvisited; schema.types.len()];
    let mut stack = Vec::new();
    let mut plan = Vec::with_capacity(schema.types.len());
    for index in 0..schema.types.len() {
        visit(index, schema, &indices, &mut states, &mut stack, &mut plan)?;
    }
    Ok(plan)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

fn visit<'a>(
    index: usize,
    schema: &'a SchemaIr,
    indices: &BTreeMap<QualifiedName, usize>,
    states: &mut [VisitState],
    stack: &mut Vec<usize>,
    plan: &mut Vec<&'a TypeDecl>,
) -> Result<(), CodegenError> {
    match states[index] {
        VisitState::Visited => return Ok(()),
        VisitState::Visiting => {
            let cycle_start = stack
                .iter()
                .position(|candidate| *candidate == index)
                .expect("visiting declaration must be on the DFS stack");
            let names = stack[cycle_start..]
                .iter()
                .copied()
                .chain(std::iter::once(index))
                .map(|cycle_index| format_name(&schema.types[cycle_index].name))
                .collect::<Vec<_>>();
            return Err(CodegenError {
                message: format!(
                    "cyclic type declaration dependencies are unsupported: {}",
                    names.join(" -> ")
                ),
            });
        }
        VisitState::Unvisited => {}
    }

    states[index] = VisitState::Visiting;
    stack.push(index);
    for dependency in dependencies(&schema.types[index]) {
        let dependency_index = indices[dependency];
        visit(dependency_index, schema, indices, states, stack, plan)?;
    }
    stack.pop();
    states[index] = VisitState::Visited;
    plan.push(&schema.types[index]);
    Ok(())
}

fn dependencies(declaration: &TypeDecl) -> Vec<&QualifiedName> {
    let mut dependencies = Vec::new();
    if let Some(base_type) = &declaration.base_type {
        push_named(&mut dependencies, base_type);
    }
    match &declaration.kind {
        TypeKind::Primitive(_) | TypeKind::Enumeration { .. } => {}
        TypeKind::Alias(target) => push_named(&mut dependencies, target),
        TypeKind::Record { fields } => {
            for field in fields {
                push_named(&mut dependencies, &field.type_ref);
            }
        }
        TypeKind::Choice { alternatives } => {
            for alternative in alternatives {
                push_named(&mut dependencies, &alternative.type_ref);
            }
        }
        TypeKind::List { item_type, .. } => push_named(&mut dependencies, item_type),
    }
    dependencies
}

fn push_named<'a>(dependencies: &mut Vec<&'a QualifiedName>, type_ref: &'a TypeRef) {
    if let TypeRefTarget::Named(name) = &type_ref.target {
        dependencies.push(name);
    }
}

fn format_name(name: &QualifiedName) -> String {
    format!("{{{}}}{}", name.namespace_uri, name.local_name)
}

pub trait Backend {
    fn name(&self) -> &'static str;

    /// Generate a deterministic set of files from normalized schema IR.
    ///
    /// # Errors
    ///
    /// Returns an error if the IR contains a semantic feature that this backend
    /// cannot represent without losing required OMS/UCI meaning.
    fn generate(&self, schema: &SchemaIr) -> Result<Vec<GeneratedFile>, CodegenError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::{
        Cardinality, ConstraintSet, FieldDecl, NamespaceDecl, PrimitiveKind, SourceRef,
    };

    const NS: &str = "urn:test";

    fn source() -> SourceRef {
        SourceRef {
            document: "test.ir".to_owned(),
            line: None,
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

    fn scalar(name: &str) -> TypeDecl {
        declaration(name, TypeKind::Primitive(PrimitiveKind::String))
    }

    fn record(name: &str, dependencies: &[&str]) -> TypeDecl {
        declaration(
            name,
            TypeKind::Record {
                fields: dependencies
                    .iter()
                    .map(|dependency| field(named(dependency)))
                    .collect(),
            },
        )
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

    fn names(schema: &SchemaIr) -> Result<Vec<&str>, CodegenError> {
        Ok(plan_type_declarations(schema)?
            .into_iter()
            .map(|declaration| declaration.name.local_name.as_str())
            .collect())
    }

    #[test]
    fn no_dependencies_preserve_original_order() {
        let schema = schema(vec![scalar("A"), scalar("B"), scalar("C")]);
        assert_eq!(names(&schema).unwrap(), ["A", "B", "C"]);
    }

    #[test]
    fn later_dependency_moves_before_consumer() {
        let schema = schema(vec![record("A", &["B"]), scalar("B")]);
        assert_eq!(names(&schema).unwrap(), ["B", "A"]);
    }

    #[test]
    fn transitive_dependency_chain_orders_correctly() {
        let schema = schema(vec![record("A", &["B"]), record("B", &["C"]), scalar("C")]);
        assert_eq!(names(&schema).unwrap(), ["C", "B", "A"]);
    }

    #[test]
    fn unrelated_declarations_remain_stable() {
        let schema = schema(vec![
            scalar("First"),
            record("Consumer", &["Dependency"]),
            scalar("Dependency"),
            scalar("Last"),
        ]);
        assert_eq!(
            names(&schema).unwrap(),
            ["First", "Dependency", "Consumer", "Last"]
        );
    }

    #[test]
    fn multiple_consumers_are_deterministic() {
        let schema = schema(vec![
            record("First", &["Shared"]),
            record("Second", &["Shared"]),
            scalar("Shared"),
        ]);
        assert_eq!(names(&schema).unwrap(), ["Shared", "First", "Second"]);
        assert_eq!(names(&schema).unwrap(), ["Shared", "First", "Second"]);
    }

    #[test]
    fn self_cycle_fails() {
        let schema = schema(vec![record("A", &["A"])]);
        let error = plan_type_declarations(&schema).unwrap_err();
        assert!(error.message.contains("{urn:test}A -> {urn:test}A"));
    }

    #[test]
    fn multi_node_cycle_fails() {
        let schema = schema(vec![record("A", &["B"]), record("B", &["A"])]);
        let error = plan_type_declarations(&schema).unwrap_err();
        assert!(
            error
                .message
                .contains("{urn:test}A -> {urn:test}B -> {urn:test}A")
        );
    }

    #[test]
    fn primitive_fields_do_not_create_edges() {
        let schema = schema(vec![
            declaration(
                "A",
                TypeKind::Record {
                    fields: vec![field(TypeRef::primitive(PrimitiveKind::String))],
                },
            ),
            scalar("B"),
        ]);
        assert_eq!(names(&schema).unwrap(), ["A", "B"]);
    }

    #[test]
    fn alias_base_list_and_choice_dependencies_are_recognized() {
        let mut based = scalar("Based");
        based.base_type = Some(named("BaseDependency"));
        let schema = schema(vec![
            declaration("Alias", TypeKind::Alias(named("AliasDependency"))),
            based,
            declaration(
                "List",
                TypeKind::List {
                    item_type: named("ListDependency"),
                    cardinality: Cardinality::REQUIRED_ONE,
                },
            ),
            declaration(
                "Choice",
                TypeKind::Choice {
                    alternatives: vec![field(named("ChoiceDependency"))],
                },
            ),
            scalar("AliasDependency"),
            scalar("BaseDependency"),
            scalar("ListDependency"),
            scalar("ChoiceDependency"),
        ]);
        assert_eq!(
            names(&schema).unwrap(),
            [
                "AliasDependency",
                "Alias",
                "BaseDependency",
                "Based",
                "ListDependency",
                "List",
                "ChoiceDependency",
                "Choice"
            ]
        );
    }
}
