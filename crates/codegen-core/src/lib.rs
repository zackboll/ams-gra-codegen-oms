//! Shared backend contract for OMS/UCI source generation.

use ams_gra_oms_ir::{QualifiedName, SchemaIr, TypeDecl, TypeKind, TypeRef, TypeRefTarget};
use std::collections::{BTreeMap, BTreeSet};
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
/// Among declarations whose dependencies have all been satisfied, the
/// declaration with the lowest original Schema IR index is emitted next.
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
    let declaration_count = schema.types.len();
    let dependency_indices = schema
        .types
        .iter()
        .map(|declaration| {
            dependencies(declaration)
                .into_iter()
                .map(|dependency| indices[dependency])
                .collect::<BTreeSet<_>>()
        })
        .collect::<Vec<_>>();
    let mut dependents = vec![Vec::new(); declaration_count];
    let mut dependency_counts = Vec::with_capacity(declaration_count);
    for (dependent, dependencies) in dependency_indices.iter().enumerate() {
        dependency_counts.push(dependencies.len());
        for &dependency in dependencies {
            dependents[dependency].push(dependent);
        }
    }

    let mut ready = dependency_counts
        .iter()
        .enumerate()
        .filter_map(|(index, &count)| (count == 0).then_some(index))
        .collect::<BTreeSet<_>>();
    let mut plan = Vec::with_capacity(schema.types.len());
    while let Some(index) = ready.pop_first() {
        plan.push(&schema.types[index]);
        for &dependent in &dependents[index] {
            dependency_counts[dependent] -= 1;
            if dependency_counts[dependent] == 0 {
                ready.insert(dependent);
            }
        }
    }

    if plan.len() != declaration_count {
        let unresolved = dependency_counts
            .iter()
            .map(|&count| count != 0)
            .collect::<Vec<_>>();
        let cycle = recover_cycle(&dependency_indices, &unresolved)
            .expect("an unresolved finite dependency graph must contain a cycle");
        let names = cycle
            .into_iter()
            .map(|index| format_name(&schema.types[index].name))
            .collect::<Vec<_>>();
        return Err(CodegenError {
            message: format!(
                "cyclic type declaration dependencies are unsupported: {}",
                names.join(" -> ")
            ),
        });
    }
    Ok(plan)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

fn recover_cycle(dependencies: &[BTreeSet<usize>], unresolved: &[bool]) -> Option<Vec<usize>> {
    let mut states = vec![VisitState::Unvisited; dependencies.len()];
    let mut stack = Vec::new();
    for index in 0..dependencies.len() {
        if unresolved[index] && states[index] == VisitState::Unvisited {
            if let Some(cycle) =
                find_cycle(index, dependencies, unresolved, &mut states, &mut stack)
            {
                return Some(cycle);
            }
        }
    }
    None
}

fn find_cycle(
    index: usize,
    dependencies: &[BTreeSet<usize>],
    unresolved: &[bool],
    states: &mut [VisitState],
    stack: &mut Vec<usize>,
) -> Option<Vec<usize>> {
    states[index] = VisitState::Visiting;
    stack.push(index);
    for &dependency in &dependencies[index] {
        if !unresolved[dependency] {
            continue;
        }
        match states[dependency] {
            VisitState::Unvisited => {
                if let Some(cycle) = find_cycle(dependency, dependencies, unresolved, states, stack)
                {
                    return Some(cycle);
                }
            }
            VisitState::Visiting => {
                let cycle_start = stack
                    .iter()
                    .position(|candidate| *candidate == dependency)
                    .expect("visiting declaration must be on the DFS stack");
                return Some(
                    stack[cycle_start..]
                        .iter()
                        .copied()
                        .chain(std::iter::once(dependency))
                        .collect(),
                );
            }
            VisitState::Visited => {}
        }
    }
    stack.pop();
    states[index] = VisitState::Visited;
    None
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
        Cardinality, ConstraintSet, FieldDecl, NamespaceDecl, NumericValue, PrimitiveKind,
        SourceRef,
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
    fn structural_base_moves_before_derived() {
        let mut derived = record("Derived", &[]);
        derived.base_type = Some(named("Base"));
        assert_eq!(
            names(&schema(vec![derived, record("Base", &[])])).unwrap(),
            ["Base", "Derived"]
        );
    }

    #[test]
    fn multi_level_structural_bases_are_planned_in_chain_order() {
        let mut leaf = record("Leaf", &[]);
        leaf.base_type = Some(named("Middle"));
        let mut middle = record("Middle", &[]);
        middle.base_type = Some(named("Base"));
        assert_eq!(
            names(&schema(vec![leaf, middle, record("Base", &[])])).unwrap(),
            ["Base", "Middle", "Leaf"]
        );
    }

    #[test]
    fn named_simple_restriction_bases_are_planned_base_first() {
        let mut leaf = scalar("Leaf");
        leaf.base_type = Some(named("Middle"));
        let mut middle = scalar("Middle");
        middle.base_type = Some(named("Base"));
        assert_eq!(
            names(&schema(vec![leaf, middle, scalar("Base")])).unwrap(),
            ["Base", "Middle", "Leaf"]
        );
    }

    #[test]
    fn malformed_named_restriction_is_rejected_before_planning() {
        let mut base = declaration("Base", TypeKind::Primitive(PrimitiveKind::SignedInteger));
        base.constraints.min_inclusive = Some(NumericValue::Integer(0));
        base.constraints.max_inclusive = Some(NumericValue::Integer(100));
        let mut derived = declaration("Derived", TypeKind::Primitive(PrimitiveKind::SignedInteger));
        derived.base_type = Some(named("Base"));
        derived.constraints.min_inclusive = Some(NumericValue::Integer(-50));
        derived.constraints.max_inclusive = Some(NumericValue::Integer(500));

        let error = plan_type_declarations(&schema(vec![base, derived])).unwrap_err();
        assert!(
            error.message.starts_with("invalid schema IR: ")
                && error.message.contains("effective constraints")
                && error.message.contains("{urn:test}Derived")
                && error.message.contains("{urn:test}Base"),
            "{error}"
        );
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
    fn unrelated_declaration_before_later_dependency_remains_before_it() {
        let schema = schema(vec![record("A", &["C"]), scalar("B"), scalar("C")]);
        assert_eq!(names(&schema).unwrap(), ["B", "C", "A"]);
    }

    #[test]
    fn repeated_reference_creates_one_dependency_edge() {
        let schema = schema(vec![record("A", &["B", "B"]), scalar("B")]);
        assert_eq!(names(&schema).unwrap(), ["B", "A"]);
    }

    #[test]
    fn newly_ready_declarations_use_original_index_priority() {
        let schema = schema(vec![
            record("A", &["C"]),
            record("B", &["C"]),
            scalar("C"),
            scalar("D"),
        ]);
        assert_eq!(names(&schema).unwrap(), ["C", "A", "B", "D"]);
    }

    #[test]
    fn repeated_planning_produces_identical_qualified_name_sequences() {
        let schema = schema(vec![record("A", &["C"]), scalar("B"), scalar("C")]);
        let expected = plan_type_declarations(&schema)
            .unwrap()
            .into_iter()
            .map(|declaration| declaration.name.clone())
            .collect::<Vec<_>>();
        for _ in 0..10 {
            assert_eq!(
                plan_type_declarations(&schema)
                    .unwrap()
                    .into_iter()
                    .map(|declaration| declaration.name.clone())
                    .collect::<Vec<_>>(),
                expected
            );
        }
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
