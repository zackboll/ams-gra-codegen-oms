//! Shared backend contract for OMS/UCI source generation.

mod abstract_value;
mod coverage;
mod integral;
mod structure;
mod world;

pub use abstract_value::{
    AbstractValueInhabitance, AbstractValueOccurrenceRenderability, AbstractValueProjection,
    AbstractValueProjectionError, AbstractValueSemantics, AbstractValueTopology,
    EffectiveValueMember, abstract_value_occurrence_renderable, abstract_value_projection_for_ref,
    abstract_value_reference_renderability, abstract_value_targets,
    classify_abstract_value_inhabitance, classify_abstract_value_semantics,
    classify_abstract_value_topology, field_storage_semantics, project_abstract_value,
};
pub use coverage::{
    BackendCoverage, BackendLanguage, CoverageAnalysis, CoverageError, FeatureFamily,
    SchemaInventory,
};
pub use integral::{InclusiveIntegralDomain, inclusive_integral_domain};
pub use structure::{
    EffectiveStructuralType, StructuralKind, StructuralLevel, StructuralProjectionError,
    StructuralSegment, StructuralSegmentContent, effective_choice_alternatives,
    effective_record_fields, project_structural_type,
};
pub use world::GenerationWorld;

/// Largest `Positive` array index guaranteed by the Ada language.
///
/// A conforming Ada `Standard.Integer` includes `-32_767 .. 32_767`, so this
/// is the largest schema cardinality bound that can portably appear in the
/// generated `Positive range 1 .. bound` arrays.
pub const ADA_PORTABLE_POSITIVE_INDEX_MAX: u64 = 32_767;

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

/// One generated type entity. Abstract structural declarations are emitted only
/// when referenced as values, in which case they become a closed sum wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeEmission<'a> {
    Declaration(&'a TypeDecl),
    AbstractValue(AbstractValueProjection<'a>),
}

impl TypeEmission<'_> {
    fn name(&self) -> &QualifiedName {
        match self {
            Self::Declaration(declaration) => &declaration.name,
            Self::AbstractValue(projection) => &projection.declaration.name,
        }
    }
}

/// Plan generated by-value entities rather than schema declarations alone,
/// under an explicit [`GenerationWorld`] policy.
///
/// Structural inheritance is not a generated containment edge: concrete
/// Records store their effective fields, while an abstract value wrapper stores
/// each concrete descendant. With no abstract value target this delegates to
/// the legacy planner exactly, in both worlds — a schema with no abstract
/// value reference generates byte-identical output regardless of policy.
///
/// Under [`GenerationWorld::OpenExtensions`] the planner is defensive: if any
/// abstract structural value target is actually demanded, it fails closed
/// rather than constructing a closed sum wrapper that would silently exclude
/// external derived types. Backend semantic validation normally rejects such
/// a schema first, but the planner must not depend on that ordering.
///
/// # Errors
///
/// Returns an error if the schema IR is invalid, if generated value
/// dependencies are cyclic, or if an abstract value target cannot be
/// represented under the asserted world.
pub fn plan_type_emissions(
    schema: &SchemaIr,
    world: GenerationWorld,
) -> Result<Vec<TypeEmission<'_>>, CodegenError> {
    let targets = abstract_value_targets(schema);
    if targets.is_empty() {
        return plan_type_declarations(schema).map(|declarations| {
            declarations
                .into_iter()
                .map(TypeEmission::Declaration)
                .collect()
        });
    }
    schema.validate().map_err(|error| CodegenError {
        message: format!("invalid schema IR: {error}"),
    })?;

    if world == GenerationWorld::OpenExtensions {
        // Task 028 section 22: a demanded abstract value target has no
        // representation in the open world, and the planner must never
        // silently emit a closed sum for it. Report the first demanded target
        // in stable Schema IR order.
        let target = targets[0];
        return Err(CodegenError {
            message: format!(
                "unsupported abstract structural value: {}",
                AbstractValueProjectionError::NotClosedUnderOpenExtensions(target.name.clone())
            ),
        });
    }

    // A zero-descendant abstract value target has no wrapper to emit: Task 026
    // requires callers to have already validated that every occurrence of
    // such a target is a supported absent-only occurrence (or the schema
    // failed validation earlier). Skipping wrapper construction here is safe
    // because `emission_dependencies` below never creates an edge to an
    // absent-only field's target.
    let mut projections = Vec::new();
    for target in &targets {
        match project_abstract_value(schema, &target.name) {
            Ok(projection) => projections.push(projection),
            Err(error @ AbstractValueProjectionError::NoConcreteDescendants(_)) => {
                // Task 026: a zero-descendant target has no wrapper to emit,
                // but only when every occurrence of it is a supported
                // absent-only occurrence. Any occurrence outside that shape
                // (positive minimum, repeated minimum, nillable, or
                // constrained) fails closed exactly as it did before Task
                // 026, because no evidence justifies inventing a payload for
                // it.
                ensure_zero_descendant_target_only_used_as_absent_only(schema, &target.name)
                    .map_err(|_| CodegenError {
                        message: format!("unsupported abstract structural value: {error}"),
                    })?;
            }
            Err(error) => {
                return Err(CodegenError {
                    message: format!("unsupported abstract structural value: {error}"),
                });
            }
        }
    }
    let mut emissions = Vec::new();
    for declaration in &schema.types {
        if !declaration.is_abstract
            || !matches!(
                declaration.kind,
                TypeKind::Record { .. } | TypeKind::Choice { .. }
            )
        {
            emissions.push(TypeEmission::Declaration(declaration));
        } else if let Some(projection) = projections
            .iter()
            .find(|projection| projection.declaration.name == declaration.name)
        {
            emissions.push(TypeEmission::AbstractValue(projection.clone()));
        }
    }
    let entity_indices = emissions
        .iter()
        .enumerate()
        .map(|(index, entity)| (entity.name().clone(), index))
        .collect::<BTreeMap<_, _>>();
    let dependencies = emissions
        .iter()
        .map(|entity| emission_dependencies(schema, entity, &entity_indices))
        .collect::<Result<Vec<_>, _>>()?;
    topological_emissions(&emissions, &dependencies)
}

fn emission_dependencies(
    schema: &SchemaIr,
    entity: &TypeEmission<'_>,
    indices: &BTreeMap<QualifiedName, usize>,
) -> Result<BTreeSet<usize>, CodegenError> {
    let names: Vec<QualifiedName> = match entity {
        TypeEmission::AbstractValue(projection) => projection
            .concrete_descendants
            .iter()
            .map(|declaration| declaration.name.clone())
            .collect(),
        TypeEmission::Declaration(declaration) => match &declaration.kind {
            TypeKind::Record { .. } => effective_record_fields(schema, &declaration.name)
                .map_err(|error| CodegenError {
                    message: format!("unsupported structural value: {error:?}"),
                })?
                .into_iter()
                .filter(|field| !is_absent_only_field(schema, field))
                .filter_map(|field| named_target(&field.type_ref))
                .collect(),
            TypeKind::Choice { .. } => effective_choice_alternatives(schema, &declaration.name)
                .map_err(|error| CodegenError {
                    message: format!("unsupported structural value: {error:?}"),
                })?
                .into_iter()
                .filter_map(|field| named_target(&field.type_ref))
                .collect(),
            _ => dependencies(declaration).into_iter().cloned().collect(),
        },
    };
    names
        .into_iter()
        .map(|name| {
            indices.get(&name).copied().ok_or_else(|| CodegenError {
                message: format!(
                    "generated value dependency {} has no emitted entity",
                    format_name(&name)
                ),
            })
        })
        .collect()
}

fn named_target(type_ref: &TypeRef) -> Option<QualifiedName> {
    match &type_ref.target {
        TypeRefTarget::Named(name) => Some(name.clone()),
        TypeRefTarget::Primitive(_) => None,
    }
}

/// True when `field` is a Task 026 supported absent-only occurrence of a
/// zero-descendant abstract structural value, and therefore contributes no
/// generated storage or dependency edge.
fn is_absent_only_field(schema: &SchemaIr, field: &ams_gra_oms_ir::FieldDecl) -> bool {
    // Only reachable from the `ClosedSchemaSet` path: the open world returns
    // early above, so absent-only elision cannot be applied there.
    matches!(
        field_storage_semantics(schema, field, GenerationWorld::ClosedSchemaSet),
        Ok(EffectiveValueMember::AbsentOnly(_))
    )
}

/// Verify that every direct reference to a zero-descendant abstract value
/// `target` in `schema` is a Task 026 supported absent-only Record field
/// occurrence.
///
/// Choice alternatives, message payloads, and any occurrence outside the
/// supported optional-single shape (positive minimum, repeated minimum,
/// nillable, or non-default local constraints) fail this check, preserving
/// the pre-Task-026 fail-closed boundary for every use this task does not
/// support.
fn ensure_zero_descendant_target_only_used_as_absent_only(
    schema: &SchemaIr,
    target: &QualifiedName,
) -> Result<(), ()> {
    for declaration in &schema.types {
        match &declaration.kind {
            TypeKind::Record { fields } => {
                for field in fields {
                    if named_target(&field.type_ref).as_ref() != Some(target) {
                        continue;
                    }
                    if !matches!(
                        field_storage_semantics(schema, field, GenerationWorld::ClosedSchemaSet),
                        Ok(EffectiveValueMember::AbsentOnly(_))
                    ) {
                        return Err(());
                    }
                }
            }
            TypeKind::Choice { alternatives } => {
                if alternatives
                    .iter()
                    .any(|alternative| named_target(&alternative.type_ref).as_ref() == Some(target))
                {
                    return Err(());
                }
            }
            TypeKind::Alias(_)
            | TypeKind::List { .. }
            | TypeKind::Primitive(_)
            | TypeKind::Enumeration { .. } => {}
        }
    }
    if schema
        .messages
        .iter()
        .any(|message| named_target(&message.payload_type).as_ref() == Some(target))
    {
        return Err(());
    }
    Ok(())
}

fn topological_emissions<'a>(
    emissions: &[TypeEmission<'a>],
    dependencies: &[BTreeSet<usize>],
) -> Result<Vec<TypeEmission<'a>>, CodegenError> {
    let mut dependents = vec![Vec::new(); emissions.len()];
    let mut counts = Vec::with_capacity(emissions.len());
    for (dependent, dependencies) in dependencies.iter().enumerate() {
        counts.push(dependencies.len());
        for &dependency in dependencies {
            dependents[dependency].push(dependent);
        }
    }
    let mut ready = counts
        .iter()
        .enumerate()
        .filter_map(|(index, &count)| (count == 0).then_some(index))
        .collect::<BTreeSet<_>>();
    let mut result = Vec::with_capacity(emissions.len());
    while let Some(index) = ready.pop_first() {
        result.push(emissions[index].clone());
        for &dependent in &dependents[index] {
            counts[dependent] -= 1;
            if counts[dependent] == 0 {
                ready.insert(dependent);
            }
        }
    }
    if result.len() != emissions.len() {
        let unresolved = counts.iter().map(|&count| count != 0).collect::<Vec<_>>();
        let cycle = recover_cycle(dependencies, &unresolved).expect("unresolved graph has a cycle");
        let labels = cycle
            .into_iter()
            .map(|index| format_name(emissions[index].name()))
            .collect::<Vec<_>>();
        return Err(CodegenError {
            message: format!(
                "cyclic generated value dependencies are unsupported: {}",
                labels.join(" -> ")
            ),
        });
    }
    Ok(result)
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

    /// Generate a deterministic set of files from normalized schema IR under
    /// an explicit [`GenerationWorld`] policy.
    ///
    /// The world argument is mandatory by design: there is no production entry
    /// point that silently assumes `ClosedSchemaSet`. Task 027 showed that
    /// schema-set closure does not imply type-universe closure, so the caller
    /// must state the assumption at the API boundary.
    ///
    /// # Errors
    ///
    /// Returns an error if the IR contains a semantic feature that this backend
    /// cannot represent without losing required OMS/UCI meaning, including an
    /// abstract structural value reference under
    /// [`GenerationWorld::OpenExtensions`].
    fn generate(
        &self,
        schema: &SchemaIr,
        world: GenerationWorld,
    ) -> Result<Vec<GeneratedFile>, CodegenError>;
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
    fn abstract_value_projection_and_emission_order_are_closed_and_deterministic() {
        let mut base = record("Base", &[]);
        base.is_abstract = true;
        let mut middle = record("Middle", &[]);
        middle.is_abstract = true;
        middle.base_type = Some(named("Base"));
        let mut concrete_b = record("ConcreteB", &[]);
        concrete_b.base_type = Some(named("Middle"));
        let mut concrete_a = record("ConcreteA", &[]);
        concrete_a.base_type = Some(named("Base"));
        let holder = record("Holder", &["Base"]);
        let schema = schema(vec![holder, base, concrete_b, middle, concrete_a]);
        let projection = project_abstract_value(&schema, &QualifiedName::new(NS, "Base")).unwrap();
        assert_eq!(
            projection
                .concrete_descendants
                .iter()
                .map(|value| value.name.local_name.as_str())
                .collect::<Vec<_>>(),
            ["ConcreteB", "ConcreteA"]
        );
        let emissions = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        assert_eq!(
            emissions
                .iter()
                .map(|emission| emission.name().local_name.as_str())
                .collect::<Vec<_>>(),
            ["ConcreteB", "ConcreteA", "Base", "Holder"]
        );
    }

    #[test]
    fn no_abstract_value_target_keeps_legacy_plan() {
        let schema = schema(vec![record("Holder", &["Value"]), scalar("Value")]);
        assert_eq!(
            plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet)
                .unwrap()
                .iter()
                .map(|entity| entity.name().clone())
                .collect::<Vec<_>>(),
            plan_type_declarations(&schema)
                .unwrap()
                .iter()
                .map(|declaration| declaration.name.clone())
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn base_only_abstract_declaration_needs_no_wrapper_or_projection() {
        let mut base = record("AbstractBaseOnly", &[]);
        base.is_abstract = true;
        let mut derived = record("Derived", &[]);
        derived.base_type = Some(named("AbstractBaseOnly"));
        let schema = schema(vec![derived, base]);
        assert!(abstract_value_targets(&schema).is_empty());
        assert_eq!(
            plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet)
                .unwrap()
                .iter()
                .map(|entity| entity.name().local_name.as_str())
                .collect::<Vec<_>>(),
            ["AbstractBaseOnly", "Derived"]
        );
    }

    #[test]
    fn value_target_without_concrete_descendants_fails_closed() {
        let mut base = record("EmptyBase", &[]);
        base.is_abstract = true;
        let schema = schema(vec![record("Holder", &["EmptyBase"]), base]);
        let error = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).unwrap_err();
        assert!(
            error
                .message
                .contains("EmptyBase has no concrete structural descendants")
        );
    }

    /// Task 028 section 22: even called directly, and even though backend
    /// semantic validation normally rejects first, the shared planner must
    /// never silently emit a closed abstract sum in the open world.
    #[test]
    fn open_world_planner_refuses_to_emit_a_closed_sum() {
        let mut base = record("Base", &[]);
        base.is_abstract = true;
        let mut derived = record("Derived", &[]);
        derived.base_type = Some(named("Base"));
        let schema = schema(vec![record("Holder", &["Base"]), base, derived]);

        // Closed world: the Task 024 wrapper is emitted exactly as before.
        let closed = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        assert!(
            closed
                .iter()
                .any(|emission| matches!(emission, TypeEmission::AbstractValue(_))),
            "closed world must still emit the Task 024 wrapper"
        );

        // Open world: no wrapper, and a diagnostic naming the policy.
        let error = plan_type_emissions(&schema, GenerationWorld::OpenExtensions).unwrap_err();
        assert!(
            error.message.contains("Base")
                && error.message.contains("open-extensions")
                && error.message.contains("external derived types"),
            "open planner diagnostic must name target and policy: {error}"
        );
    }

    /// Section 18: the open-world rejection must not be keyed on UCI `EXT`
    /// naming. A synthetic `Base` with several descendants, including a
    /// concrete non-leaf, fails identically.
    #[test]
    fn open_world_rejection_is_not_keyed_on_ext_names() {
        let mut base = record("Base", &[]);
        base.is_abstract = true;
        let mut first = record("First", &[]);
        first.base_type = Some(named("Base"));
        let mut second = record("Second", &[]);
        second.base_type = Some(named("Base"));
        // Concrete non-leaf: `Third` extends the concrete `First`.
        let mut third = record("Third", &[]);
        third.base_type = Some(named("First"));
        let schema = schema(vec![
            record("Holder", &["Base"]),
            base,
            first,
            second,
            third,
        ]);

        assert!(
            plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).is_ok(),
            "many-descendant closed sum must keep working in the closed world"
        );
        let error = plan_type_emissions(&schema, GenerationWorld::OpenExtensions).unwrap_err();
        assert!(
            error.message.contains("open-extensions"),
            "a non-EXT-named target with many descendants must still fail open: {error}"
        );
    }

    /// Section 12/43: an abstract declaration used only as ancestry is not a
    /// value reference, so both worlds plan it identically.
    #[test]
    fn base_only_ancestry_plans_identically_in_both_worlds() {
        let mut base = record("AbstractBaseOnly", &[]);
        base.is_abstract = true;
        let mut derived = record("Derived", &[]);
        derived.base_type = Some(named("AbstractBaseOnly"));
        let schema = schema(vec![derived, base]);
        let names = |world| {
            plan_type_emissions(&schema, world)
                .unwrap()
                .iter()
                .map(|entity| entity.name().clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names(GenerationWorld::ClosedSchemaSet),
            names(GenerationWorld::OpenExtensions)
        );
    }

    /// Section 15: a zero-known-descendant optional field is absent-only only
    /// in the closed world. Open-world, an external descendant may make it
    /// present, so it must not be elided and must fail closed instead.
    #[test]
    fn zero_descendant_optional_is_only_elided_in_the_closed_world() {
        let mut base = record("EmptyBase", &[]);
        base.is_abstract = true;
        let mut holder = record("Holder", &[]);
        let mut optional_field = field(named("EmptyBase"));
        optional_field.cardinality = Cardinality::OPTIONAL_ONE;
        let TypeKind::Record { fields } = &mut holder.kind else {
            unreachable!();
        };
        fields.push(optional_field.clone());
        let schema = schema(vec![holder, base]);

        assert_eq!(
            field_storage_semantics(&schema, &optional_field, GenerationWorld::ClosedSchemaSet)
                .unwrap(),
            EffectiveValueMember::AbsentOnly(&optional_field)
        );
        assert_eq!(
            field_storage_semantics(&schema, &optional_field, GenerationWorld::OpenExtensions)
                .unwrap(),
            EffectiveValueMember::Stored(&optional_field),
            "open world must not elide storage for a possibly-external payload"
        );
        assert!(plan_type_emissions(&schema, GenerationWorld::OpenExtensions).is_err());
    }

    #[test]
    fn optional_uninhabited_abstract_value_is_elided_from_emissions_and_dependencies() {
        let mut base = record("EmptyBase", &[]);
        base.is_abstract = true;
        let mut holder = record("Holder", &[]);
        let mut optional_field = field(named("EmptyBase"));
        optional_field.cardinality = Cardinality::OPTIONAL_ONE;
        let TypeKind::Record { fields } = &mut holder.kind else {
            unreachable!();
        };
        fields.push(optional_field);
        let schema = schema(vec![holder, base]);
        let emissions = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        // Neither a wrapper for the uninhabited target nor a dangling
        // dependency edge is produced; only Holder is emitted.
        assert_eq!(
            emissions
                .iter()
                .map(|entity| entity.name().local_name.as_str())
                .collect::<Vec<_>>(),
            ["Holder"]
        );
    }

    #[test]
    fn repeated_uninhabited_abstract_value_remains_unsupported() {
        let mut base = record("EmptyBase", &[]);
        base.is_abstract = true;
        let mut holder = record("Holder", &[]);
        let mut repeated_field = field(named("EmptyBase"));
        repeated_field.cardinality = Cardinality {
            min_occurs: 0,
            max_occurs: None,
        };
        let TypeKind::Record { fields } = &mut holder.kind else {
            unreachable!();
        };
        fields.push(repeated_field);
        let schema = schema(vec![holder, base]);
        let error = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).unwrap_err();
        assert!(
            error
                .message
                .contains("EmptyBase has no concrete structural descendants")
        );
    }

    #[test]
    fn uninhabited_abstract_value_as_choice_alternative_remains_unsupported() {
        let mut base = record("EmptyBase", &[]);
        base.is_abstract = true;
        let choice = declaration(
            "Selector",
            TypeKind::Choice {
                alternatives: vec![field(named("EmptyBase"))],
            },
        );
        let schema = schema(vec![choice, base]);
        let error = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).unwrap_err();
        assert!(
            error
                .message
                .contains("EmptyBase has no concrete structural descendants")
        );
    }

    #[test]
    fn uninhabited_abstract_value_as_message_payload_remains_unsupported() {
        let mut base = record("EmptyBase", &[]);
        base.is_abstract = true;
        let mut schema = schema(vec![base]);
        schema.messages.push(ams_gra_oms_ir::MessageDecl {
            name: QualifiedName::new(NS, "Notify"),
            payload_type: named("EmptyBase"),
            documentation: None,
            source: source(),
        });
        let error = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).unwrap_err();
        assert!(
            error
                .message
                .contains("EmptyBase has no concrete structural descendants")
        );
    }

    #[test]
    fn nillable_uninhabited_abstract_optional_field_remains_unsupported() {
        let mut base = record("EmptyBase", &[]);
        base.is_abstract = true;
        let mut holder = record("Holder", &[]);
        let mut optional_field = field(named("EmptyBase"));
        optional_field.cardinality = Cardinality::OPTIONAL_ONE;
        optional_field.nillable = true;
        let TypeKind::Record { fields } = &mut holder.kind else {
            unreachable!();
        };
        fields.push(optional_field);
        let schema = schema(vec![holder, base]);
        let error = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).unwrap_err();
        assert!(
            error
                .message
                .contains("EmptyBase has no concrete structural descendants")
        );
    }

    #[test]
    fn constrained_uninhabited_abstract_optional_field_remains_unsupported() {
        let mut base = record("EmptyBase", &[]);
        base.is_abstract = true;
        let mut holder = record("Holder", &[]);
        let mut optional_field = field(named("EmptyBase"));
        optional_field.cardinality = Cardinality::OPTIONAL_ONE;
        optional_field.constraints.length = Some(4);
        let TypeKind::Record { fields } = &mut holder.kind else {
            unreachable!();
        };
        fields.push(optional_field);
        let schema = schema(vec![holder, base]);
        let error = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).unwrap_err();
        assert!(
            error
                .message
                .contains("EmptyBase has no concrete structural descendants")
        );
    }

    #[test]
    fn recursive_abstract_value_topologies_are_classified_deterministically() {
        let mut base = record("Base", &[]);
        base.is_abstract = true;
        let mut concrete = record("Concrete", &["Base"]);
        concrete.base_type = Some(named("Base"));
        let schema = schema(vec![base, concrete, record("Holder", &["Base"])]);
        let topology =
            classify_abstract_value_topology(&schema, &QualifiedName::new(NS, "Base")).unwrap();
        assert_eq!(
            topology,
            AbstractValueTopology::RecursiveValueGraph {
                cycle: vec![
                    QualifiedName::new(NS, "Base"),
                    QualifiedName::new(NS, "Concrete"),
                    QualifiedName::new(NS, "Base"),
                ],
            }
        );
    }

    #[test]
    fn mutual_abstract_value_topology_is_recursive() {
        let mut a = record("A", &[]);
        a.is_abstract = true;
        let mut a1 = record("A1", &["B"]);
        a1.base_type = Some(named("A"));
        let mut b = record("B", &[]);
        b.is_abstract = true;
        let mut b1 = record("B1", &["A"]);
        b1.base_type = Some(named("B"));
        let schema = schema(vec![a, a1, b, b1, record("Holder", &["A"])]);
        assert!(matches!(
            classify_abstract_value_topology(&schema, &QualifiedName::new(NS, "A")).unwrap(),
            AbstractValueTopology::RecursiveValueGraph { .. }
        ));
    }

    #[test]
    fn command_pet_choice_topology_is_recursive_without_inheritance_containment() {
        let mut pet = record("CommandPET", &[]);
        pet.is_abstract = true;
        let choice = record("SubtestChoice", &["CommandPET"]);
        let mut command = record("Command", &["SubtestChoice"]);
        command.base_type = Some(named("CommandPET"));
        let schema = schema(vec![
            pet,
            choice,
            command,
            record("Holder", &["CommandPET"]),
        ]);
        assert_eq!(
            classify_abstract_value_topology(&schema, &QualifiedName::new(NS, "CommandPET"))
                .unwrap(),
            AbstractValueTopology::RecursiveValueGraph {
                cycle: vec![
                    QualifiedName::new(NS, "CommandPET"),
                    QualifiedName::new(NS, "Command"),
                    QualifiedName::new(NS, "SubtestChoice"),
                    QualifiedName::new(NS, "CommandPET"),
                ],
            }
        );
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
