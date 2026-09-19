use crate::structure::{
    effective_choice_alternatives, effective_record_fields, project_structural_type,
};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, FieldDecl, QualifiedName, SchemaIr, TypeDecl, TypeKind, TypeRef,
    TypeRefTarget,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// A closed, schema-known by-value representation of an abstract structural declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbstractValueProjection<'a> {
    pub declaration: &'a TypeDecl,
    /// Concrete transitive descendants in original Schema IR order.
    pub concrete_descendants: Vec<&'a TypeDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbstractValueProjectionError {
    MissingDeclaration(QualifiedName),
    NonStructuralTarget(QualifiedName),
    NonAbstractTarget(QualifiedName),
    NoConcreteDescendants(QualifiedName),
}

/// Closed-value topology of a demanded abstract structural value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbstractValueTopology<'a> {
    Acyclic(AbstractValueProjection<'a>),
    NoConcreteDescendants(QualifiedName),
    RecursiveValueGraph { cycle: Vec<QualifiedName> },
}

/// Whether an abstract structural value's payload set has any inhabitant in
/// the current, closed normalized schema set.
///
/// This is distinct from schema validity: `Uninhabited` is a legitimate
/// classification for a well-formed abstract target that simply has zero
/// concrete structural descendants right now. A different schema set that
/// adds a concrete descendant reclassifies the same target as `Inhabited`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbstractValueInhabitance<'a> {
    Inhabited(AbstractValueProjection<'a>),
    Uninhabited { declaration: &'a TypeDecl },
    Recursive { cycle: Vec<QualifiedName> },
}

/// Classify whether an abstract structural target has any legal by-value
/// payload in the current schema set, without conflating "no inhabitants"
/// with "invalid schema".
///
/// This intentionally uses the cheap `project_abstract_value` projection
/// rather than the recursive-cycle-detecting `classify_abstract_value_topology`:
/// callers that only need to distinguish "uninhabited" from "has candidate
/// concrete descendants" do not need whole-graph cycle detection, and
/// avoiding it keeps per-field occurrence checks linear in schema size.
/// Callers that also need cycle detection should use
/// `classify_abstract_value_topology` directly.
pub fn classify_abstract_value_inhabitance<'a>(
    schema: &'a SchemaIr,
    target: &QualifiedName,
) -> Result<AbstractValueInhabitance<'a>, AbstractValueProjectionError> {
    match project_abstract_value(schema, target) {
        Ok(projection) => Ok(AbstractValueInhabitance::Inhabited(projection)),
        Err(AbstractValueProjectionError::NoConcreteDescendants(name)) => {
            let declaration = schema
                .types
                .iter()
                .find(|declaration| declaration.name == name)
                .ok_or_else(|| AbstractValueProjectionError::MissingDeclaration(name.clone()))?;
            Ok(AbstractValueInhabitance::Uninhabited { declaration })
        }
        Err(error) => Err(error),
    }
}

/// One field/alternative occurrence's storage semantics for a value target
/// whose abstract structural payload set may be uninhabited.
///
/// This is intentionally schema-neutral and syntax-free: no backend naming or
/// rendering policy appears here. Only a Task 026-supported occurrence shape
/// of an uninhabited target returns `AbsentOnly`; every other occurrence of an
/// uninhabited target (positive minimum, repeated minimum, nillable, or with
/// non-default local constraints) is `Unsupported`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbstractValueOccurrenceRenderability {
    /// The target is not a zero-descendant abstract value; ordinary
    /// closed-sum/by-value lowering applies unchanged.
    NotApplicable,
    /// The target is uninhabited and this occurrence's only legal state is
    /// absence: `minOccurs == 0`, not nillable, and no local constraints.
    AbsentOnly,
    /// The target is uninhabited and this occurrence requires at least one
    /// payload (positive minimum), is nillable, is a repeated occurrence, or
    /// carries local constraints; no representation exists.
    Unsupported,
}

/// Classify whether a single occurrence (Record field, Choice alternative, or
/// message payload reference) of a possibly-uninhabited abstract value can be
/// represented without ever constructing an impossible payload.
///
/// Task 026 supports only the narrowest shape with authoritative evidence:
/// an optional (`minOccurs == 0`, `maxOccurs <= 1`), non-nillable occurrence
/// with default local constraints. Repeated zero-minimum occurrences,
/// positive-minimum occurrences, nillable occurrences, and occurrences with
/// non-default local constraints remain `Unsupported` even though the target
/// is uninhabited, because no evidence justifies inventing a representation
/// for them yet.
#[must_use]
pub fn abstract_value_occurrence_renderable(
    inhabitance: &AbstractValueInhabitance<'_>,
    cardinality: Cardinality,
    nillable: bool,
    constraints_are_default: bool,
) -> AbstractValueOccurrenceRenderability {
    if !matches!(inhabitance, AbstractValueInhabitance::Uninhabited { .. }) {
        return AbstractValueOccurrenceRenderability::NotApplicable;
    }
    let is_optional_single = cardinality.min_occurs == 0 && cardinality.max_occurs == Some(1);
    if is_optional_single && !nillable && constraints_are_default {
        AbstractValueOccurrenceRenderability::AbsentOnly
    } else {
        AbstractValueOccurrenceRenderability::Unsupported
    }
}

/// Classify occurrence renderability for one named-value reference in a
/// single call, resolving whether the reference even targets an abstract
/// structural declaration.
///
/// Non-abstract and non-structural targets, and primitive references, always
/// classify as `NotApplicable`: Task 026 only concerns abstract structural
/// values that may be uninhabited.
///
/// # Errors
///
/// Returns an error if `type_ref` names a declaration missing from `schema`.
pub fn abstract_value_reference_renderability(
    schema: &SchemaIr,
    type_ref: &TypeRef,
    cardinality: Cardinality,
    nillable: bool,
    constraints_are_default: bool,
) -> Result<AbstractValueOccurrenceRenderability, AbstractValueProjectionError> {
    let TypeRefTarget::Named(target) = &type_ref.target else {
        return Ok(AbstractValueOccurrenceRenderability::NotApplicable);
    };
    let declaration = schema
        .types
        .iter()
        .find(|declaration| declaration.name == *target)
        .ok_or_else(|| AbstractValueProjectionError::MissingDeclaration(target.clone()))?;
    if !(declaration.is_abstract && is_structural(declaration)) {
        return Ok(AbstractValueOccurrenceRenderability::NotApplicable);
    }
    let inhabitance = classify_abstract_value_inhabitance(schema, target)?;
    Ok(abstract_value_occurrence_renderable(
        &inhabitance,
        cardinality,
        nillable,
        constraints_are_default,
    ))
}

/// Effective storage semantics for one Record field, Choice alternative, or
/// message payload reference, once uninhabited-abstract-value semantics are
/// taken into account.
///
/// This is the single shared decision point backends should consult instead
/// of independently re-deriving zero-descendant absence-only elision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectiveValueMember<'a> {
    /// Ordinary storage: render the field/alternative exactly as before.
    Stored(&'a FieldDecl),
    /// The field's payload set is uninhabited and this occurrence's only
    /// legal state is absence. No storage should be generated for it.
    AbsentOnly(&'a FieldDecl),
}

/// Classify one field's storage semantics against a schema's abstract-value
/// inhabitance, applying Task 026's absence-only elision rule when — and only
/// when — the field is a supported optional occurrence of an uninhabited
/// abstract structural target.
///
/// # Errors
///
/// Returns an error if the field's named target is missing from `schema`.
pub fn field_storage_semantics<'a>(
    schema: &SchemaIr,
    field: &'a FieldDecl,
) -> Result<EffectiveValueMember<'a>, AbstractValueProjectionError> {
    let renderability = abstract_value_reference_renderability(
        schema,
        &field.type_ref,
        field.cardinality,
        field.nillable,
        field.constraints == ConstraintSet::default(),
    )?;
    Ok(match renderability {
        AbstractValueOccurrenceRenderability::AbsentOnly => EffectiveValueMember::AbsentOnly(field),
        AbstractValueOccurrenceRenderability::NotApplicable
        | AbstractValueOccurrenceRenderability::Unsupported => EffectiveValueMember::Stored(field),
    })
}

impl fmt::Display for AbstractValueProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDeclaration(name) => write!(f, "missing declaration {}", name.local_name),
            Self::NonStructuralTarget(name) => {
                write!(
                    f,
                    "abstract value target {} is non-structural",
                    name.local_name
                )
            }
            Self::NonAbstractTarget(name) => {
                write!(
                    f,
                    "abstract value target {} is not abstract",
                    name.local_name
                )
            }
            Self::NoConcreteDescendants(name) => write!(
                f,
                "abstract value target {} has no concrete structural descendants",
                name.local_name
            ),
        }
    }
}

impl std::error::Error for AbstractValueProjectionError {}

/// Project every concrete transitive structural descendant of an abstract target.
///
/// Abstract intermediates are ancestry metadata and are intentionally not variants.
pub fn project_abstract_value<'a>(
    schema: &'a SchemaIr,
    target: &QualifiedName,
) -> Result<AbstractValueProjection<'a>, AbstractValueProjectionError> {
    let declaration = schema
        .types
        .iter()
        .find(|declaration| declaration.name == *target)
        .ok_or_else(|| AbstractValueProjectionError::MissingDeclaration(target.clone()))?;
    if !is_structural(declaration) {
        return Err(AbstractValueProjectionError::NonStructuralTarget(
            target.clone(),
        ));
    }
    if !declaration.is_abstract {
        return Err(AbstractValueProjectionError::NonAbstractTarget(
            target.clone(),
        ));
    }

    let mut descendants = Vec::new();
    for candidate in &schema.types {
        if candidate.is_abstract || !is_structural(candidate) || candidate.name == *target {
            continue;
        }
        let Ok(projection) = project_structural_type(schema, &candidate.name) else {
            continue;
        };
        if projection
            .ancestry
            .iter()
            .any(|level| level.declaration.name == *target)
        {
            descendants.push(candidate);
        }
    }
    if descendants.is_empty() {
        return Err(AbstractValueProjectionError::NoConcreteDescendants(
            target.clone(),
        ));
    }
    Ok(AbstractValueProjection {
        declaration,
        concrete_descendants: descendants,
    })
}

/// Project an abstract structural named value reference, if it needs a
/// generated closed-sum wrapper. Other named references retain their exact
/// existing named-value representation.
pub fn abstract_value_projection_for_ref<'a>(
    schema: &'a SchemaIr,
    type_ref: &TypeRef,
) -> Result<Option<AbstractValueProjection<'a>>, AbstractValueProjectionError> {
    let TypeRefTarget::Named(target) = &type_ref.target else {
        return Ok(None);
    };
    let declaration = schema
        .types
        .iter()
        .find(|declaration| declaration.name == *target)
        .ok_or_else(|| AbstractValueProjectionError::MissingDeclaration(target.clone()))?;
    if declaration.is_abstract && is_structural(declaration) {
        project_abstract_value(schema, target).map(Some)
    } else {
        Ok(None)
    }
}

/// Classify generated by-value topology for an abstract wrapper without
/// conflating structural ancestry with containment.
pub fn classify_abstract_value_topology<'a>(
    schema: &'a SchemaIr,
    target: &QualifiedName,
) -> Result<AbstractValueTopology<'a>, AbstractValueProjectionError> {
    let projection = match project_abstract_value(schema, target) {
        Ok(projection) => projection,
        Err(AbstractValueProjectionError::NoConcreteDescendants(name)) => {
            return Ok(AbstractValueTopology::NoConcreteDescendants(name));
        }
        Err(error) => return Err(error),
    };
    let targets = abstract_value_targets(schema)
        .into_iter()
        .map(|declaration| declaration.name.clone())
        .collect::<BTreeSet<_>>();
    let mut visiting = Vec::new();
    let mut visited = BTreeSet::new();
    if let Some(cycle) =
        visit_generated_value(schema, target, &targets, &mut visiting, &mut visited)?
    {
        return Ok(AbstractValueTopology::RecursiveValueGraph { cycle });
    }
    Ok(AbstractValueTopology::Acyclic(projection))
}

fn visit_generated_value(
    schema: &SchemaIr,
    name: &QualifiedName,
    wrappers: &BTreeSet<QualifiedName>,
    visiting: &mut Vec<QualifiedName>,
    visited: &mut BTreeSet<QualifiedName>,
) -> Result<Option<Vec<QualifiedName>>, AbstractValueProjectionError> {
    if let Some(position) = visiting.iter().position(|candidate| candidate == name) {
        let mut cycle = visiting[position..].to_vec();
        cycle.push(name.clone());
        return Ok(Some(cycle));
    }
    if !visited.insert(name.clone()) {
        return Ok(None);
    }
    visiting.push(name.clone());
    let declaration = schema
        .types
        .iter()
        .find(|candidate| candidate.name == *name)
        .ok_or_else(|| AbstractValueProjectionError::MissingDeclaration(name.clone()))?;
    let dependencies = if wrappers.contains(name) {
        match project_abstract_value(schema, name) {
            Ok(projection) => projection
                .concrete_descendants
                .into_iter()
                .map(|candidate| candidate.name.clone())
                .collect(),
            Err(AbstractValueProjectionError::NoConcreteDescendants(_)) => Vec::new(),
            Err(error) => return Err(error),
        }
    } else {
        match declaration.kind {
            TypeKind::Record { .. } => effective_record_fields(schema, name)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(|field| named(&field.type_ref))
                .collect(),
            TypeKind::Choice { .. } => effective_choice_alternatives(schema, name)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(|field| named(&field.type_ref))
                .collect(),
            TypeKind::Alias(ref reference) => named(reference).into_iter().collect(),
            TypeKind::List { ref item_type, .. } => named(item_type).into_iter().collect(),
            TypeKind::Primitive(_) | TypeKind::Enumeration { .. } => Vec::new(),
        }
    };
    for dependency in dependencies {
        if let Some(cycle) =
            visit_generated_value(schema, &dependency, wrappers, visiting, visited)?
        {
            return Ok(Some(cycle));
        }
    }
    visiting.pop();
    Ok(None)
}

fn named(reference: &TypeRef) -> Option<QualifiedName> {
    match &reference.target {
        TypeRefTarget::Named(name) => Some(name.clone()),
        TypeRefTarget::Primitive(_) => None,
    }
}

/// Return abstract structural declarations occurring in named value positions.
///
/// A base-type relationship is intentionally excluded. Results preserve Schema IR order.
pub fn abstract_value_targets(schema: &SchemaIr) -> Vec<&TypeDecl> {
    let declarations = schema
        .types
        .iter()
        .map(|declaration| (declaration.name.clone(), declaration))
        .collect::<BTreeMap<_, _>>();
    let mut targets = BTreeSet::new();
    for declaration in &schema.types {
        match &declaration.kind {
            TypeKind::Record { fields } => fields.iter().for_each(|field| {
                collect_abstract_target(&declarations, &field.type_ref, &mut targets);
            }),
            TypeKind::Choice { alternatives } => alternatives.iter().for_each(|alternative| {
                collect_abstract_target(&declarations, &alternative.type_ref, &mut targets);
            }),
            TypeKind::Alias(target) => collect_abstract_target(&declarations, target, &mut targets),
            TypeKind::List { item_type, .. } => {
                collect_abstract_target(&declarations, item_type, &mut targets);
            }
            TypeKind::Primitive(_) | TypeKind::Enumeration { .. } => {}
        }
    }
    for message in &schema.messages {
        collect_abstract_target(&declarations, &message.payload_type, &mut targets);
    }
    schema
        .types
        .iter()
        .filter(|declaration| targets.contains(&declaration.name))
        .collect()
}

fn collect_abstract_target(
    declarations: &BTreeMap<QualifiedName, &TypeDecl>,
    type_ref: &TypeRef,
    targets: &mut BTreeSet<QualifiedName>,
) {
    let TypeRefTarget::Named(name) = &type_ref.target else {
        return;
    };
    if declarations
        .get(name)
        .is_some_and(|declaration| declaration.is_abstract && is_structural(declaration))
    {
        targets.insert(name.clone());
    }
}

pub(crate) fn is_structural(declaration: &TypeDecl) -> bool {
    matches!(
        declaration.kind,
        TypeKind::Record { .. } | TypeKind::Choice { .. }
    )
}
