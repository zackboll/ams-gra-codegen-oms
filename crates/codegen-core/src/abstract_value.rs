use crate::structure::{
    StructuralProjectionError, effective_choice_alternatives, effective_record_fields,
    project_with_index,
};
use crate::world::GenerationWorld;
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
    /// Under [`GenerationWorld::OpenExtensions`] the known concrete
    /// descendants of an abstract structural value are not assumed
    /// exhaustive, so no closed representation of the value exists.
    ///
    /// This is deliberately distinct from `NoConcreteDescendants`, which is a
    /// closed-world statement about the supplied schema set. It applies to
    /// *every* abstract structural value target regardless of how many
    /// descendants happen to be known, and it never claims the schema is
    /// invalid.
    NotClosedUnderOpenExtensions(QualifiedName),
    /// A concrete transitive descendant of the abstract target exists in the
    /// schema set, but its own structural projection cannot be represented
    /// (for example an inherited member-name collision).
    ///
    /// Corrective cleanup after Task 033: this case previously caused the
    /// candidate to be *skipped silently*, which could either emit a closed
    /// sum missing a legal alternative, or — when every descendant failed —
    /// masquerade as [`Self::NoConcreteDescendants`] and wrongly authorize
    /// Task 026 absent-only elision. "A legal payload exists but we cannot
    /// represent it" and "no legal payload exists" are different facts and
    /// must never be conflated, so this now fails closed with the failing
    /// descendant and its underlying structural error retained.
    UnrepresentableConcreteDescendant {
        target: Box<QualifiedName>,
        descendant: Box<QualifiedName>,
        source: Box<StructuralProjectionError>,
    },
}

/// Closed-value topology of a demanded abstract structural value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbstractValueTopology<'a> {
    Acyclic(AbstractValueProjection<'a>),
    NoConcreteDescendants(QualifiedName),
    RecursiveValueGraph { cycle: Vec<QualifiedName> },
}

/// Objective descendant topology of an abstract structural value **within the
/// supplied schema set**.
///
/// This is a policy-independent statement of fact about the schema that was
/// loaded, and deliberately carries no world interpretation. In particular
/// `NoKnownConcreteDescendants` does *not* claim the target is globally
/// uninhabited: whether zero *known* descendants means zero *legal*
/// descendants is a [`GenerationWorld`] question answered by
/// [`classify_abstract_value_semantics`], not by this enum.
///
/// This is also distinct from schema validity: `NoKnownConcreteDescendants`
/// is a legitimate classification for a well-formed abstract target that
/// simply has zero concrete structural descendants in this schema set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbstractValueInhabitance<'a> {
    /// The schema set contains at least one concrete structural descendant.
    KnownConcreteDescendants(AbstractValueProjection<'a>),
    /// The schema set contains no concrete structural descendant of this
    /// target. Whether that means "uninhabited" depends on the world policy.
    NoKnownConcreteDescendants {
        declaration: &'a TypeDecl,
    },
    Recursive {
        cycle: Vec<QualifiedName>,
    },
}

/// World-interpreted semantics of an abstract structural value: schema
/// topology combined with the caller's [`GenerationWorld`] assertion.
///
/// This is the single shared place where "what the schema says" becomes "what
/// the generator may assume". Backends must consult it rather than
/// re-deriving the combination, so Ada, Rust, and C++ cannot disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbstractValueSemantics<'a> {
    /// `ClosedSchemaSet` + known descendants: the known descendant set is
    /// exhaustive, so Task 024 closed-sum lowering may be used.
    ClosedKnownSum(AbstractValueProjection<'a>),
    /// `ClosedSchemaSet` + zero known descendants: no legal payload exists in
    /// the declared universe, so Task 026 absent-only elision may apply to a
    /// supported optional occurrence.
    NoLegalPayload { declaration: &'a TypeDecl },
    /// `OpenExtensions` + *any* abstract structural value target: the known
    /// descendants are not assumed exhaustive, so no closed representation
    /// exists and the value position must fail closed.
    OpenUnrepresentable { declaration: &'a TypeDecl },
    /// The generated by-value graph is recursive; unchanged fail-closed
    /// boundary in both worlds.
    Recursive { cycle: Vec<QualifiedName> },
}

/// Classify one abstract structural target under an explicit world policy.
///
/// Under [`GenerationWorld::OpenExtensions`] this conservatively returns
/// `OpenUnrepresentable` for **every** abstract structural value target —
/// with zero, one, or many known descendants alike. The schema carries no
/// machine-readable extension-point discriminator, so the generator must not
/// guess that some particular descendant set happens to be exhaustive.
///
/// # Errors
///
/// Returns an error if `target` is missing, non-structural, or not abstract.
pub fn classify_abstract_value_semantics<'a>(
    schema: &'a SchemaIr,
    target: &QualifiedName,
    world: GenerationWorld,
) -> Result<AbstractValueSemantics<'a>, AbstractValueProjectionError> {
    let inhabitance = classify_abstract_value_inhabitance(schema, target)?;
    Ok(match (world, inhabitance) {
        (GenerationWorld::OpenExtensions, AbstractValueInhabitance::Recursive { cycle })
        | (GenerationWorld::ClosedSchemaSet, AbstractValueInhabitance::Recursive { cycle }) => {
            AbstractValueSemantics::Recursive { cycle }
        }
        (
            GenerationWorld::OpenExtensions,
            AbstractValueInhabitance::KnownConcreteDescendants(projection),
        ) => AbstractValueSemantics::OpenUnrepresentable {
            declaration: projection.declaration,
        },
        (
            GenerationWorld::OpenExtensions,
            AbstractValueInhabitance::NoKnownConcreteDescendants { declaration },
        ) => AbstractValueSemantics::OpenUnrepresentable { declaration },
        (
            GenerationWorld::ClosedSchemaSet,
            AbstractValueInhabitance::KnownConcreteDescendants(projection),
        ) => AbstractValueSemantics::ClosedKnownSum(projection),
        (
            GenerationWorld::ClosedSchemaSet,
            AbstractValueInhabitance::NoKnownConcreteDescendants { declaration },
        ) => AbstractValueSemantics::NoLegalPayload { declaration },
    })
}

/// Classify the objective descendant topology of an abstract structural
/// target in the supplied schema set, without conflating "no known
/// descendants" with "invalid schema".
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
        Ok(projection) => Ok(AbstractValueInhabitance::KnownConcreteDescendants(
            projection,
        )),
        Err(AbstractValueProjectionError::NoConcreteDescendants(name)) => {
            let declaration = schema
                .types
                .iter()
                .find(|declaration| declaration.name == name)
                .ok_or_else(|| AbstractValueProjectionError::MissingDeclaration(name.clone()))?;
            Ok(AbstractValueInhabitance::NoKnownConcreteDescendants { declaration })
        }
        Err(error) => Err(error),
    }
}

/// One field/alternative occurrence's storage semantics for an abstract
/// structural value target.
///
/// This is intentionally schema-neutral and syntax-free: no backend naming or
/// rendering policy appears here. Only a Task 026-supported occurrence shape
/// of a target with no legal payload returns `AbsentOnly`; every other
/// occurrence of such a target (positive minimum, repeated minimum, nillable,
/// or with non-default local constraints) is `Unsupported`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbstractValueOccurrenceRenderability {
    /// The target is not an abstract value needing special treatment in this
    /// world; ordinary closed-sum/by-value lowering applies unchanged.
    NotApplicable,
    /// The target has no legal payload in the asserted world and this
    /// occurrence's only legal state is absence: `minOccurs == 0`, not
    /// nillable, and no local constraints.
    AbsentOnly,
    /// The target has no representation for this occurrence: it has no legal
    /// payload yet the occurrence demands one, or the world does not close
    /// the target at all.
    Unsupported,
}

/// Classify whether a single occurrence (Record field, Choice alternative, or
/// message payload reference) of an abstract value can be represented without
/// ever constructing an impossible payload.
///
/// Under `ClosedSchemaSet` + `NoLegalPayload`, Task 026 supports only the
/// narrowest shape with authoritative evidence: an optional (`minOccurs == 0`,
/// `maxOccurs <= 1`), non-nillable occurrence with default local constraints.
/// Repeated zero-minimum occurrences, positive-minimum occurrences, nillable
/// occurrences, and occurrences with non-default local constraints remain
/// `Unsupported`, because no evidence justifies inventing a representation for
/// them yet. Task 028 deliberately does **not** extend this to repeated
/// always-empty collections.
///
/// Under `OpenExtensions` (`OpenUnrepresentable`) absent-only elision is
/// never granted: an external or private derived type may legally make the
/// occurrence present, so eliding its storage would silently lose data.
#[must_use]
pub fn abstract_value_occurrence_renderable(
    semantics: &AbstractValueSemantics<'_>,
    cardinality: Cardinality,
    nillable: bool,
    constraints_are_default: bool,
) -> AbstractValueOccurrenceRenderability {
    match semantics {
        // Task 024 closed sum, or a recursive boundary handled elsewhere.
        AbstractValueSemantics::ClosedKnownSum(_) | AbstractValueSemantics::Recursive { .. } => {
            AbstractValueOccurrenceRenderability::NotApplicable
        }
        // Open world: zero known descendants does NOT mean uninhabited, so
        // Task 026 elision must not occur. There is no representation for the
        // unknown future subtype either, so the occurrence fails closed.
        AbstractValueSemantics::OpenUnrepresentable { .. } => {
            AbstractValueOccurrenceRenderability::Unsupported
        }
        AbstractValueSemantics::NoLegalPayload { .. } => {
            let is_optional_single =
                cardinality.min_occurs == 0 && cardinality.max_occurs == Some(1);
            if is_optional_single && !nillable && constraints_are_default {
                AbstractValueOccurrenceRenderability::AbsentOnly
            } else {
                AbstractValueOccurrenceRenderability::Unsupported
            }
        }
    }
}

/// Classify occurrence renderability for one named-value reference in a
/// single call, resolving whether the reference even targets an abstract
/// structural declaration.
///
/// Non-abstract and non-structural targets, and primitive references, always
/// classify as `NotApplicable` in both worlds: the world policy only concerns
/// abstract structural *value* positions. Ordinary concrete values are
/// unaffected, and an abstract declaration used only as inheritance ancestry
/// is not a value reference at all.
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
    world: GenerationWorld,
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
    let semantics = classify_abstract_value_semantics(schema, target, world)?;
    Ok(abstract_value_occurrence_renderable(
        &semantics,
        cardinality,
        nillable,
        constraints_are_default,
    ))
}

/// Effective storage semantics for one Record field, Choice alternative, or
/// message payload reference, once world-interpreted abstract-value semantics
/// are taken into account.
///
/// This is the single shared decision point backends should consult instead
/// of independently re-deriving absence-only elision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectiveValueMember<'a> {
    /// Ordinary storage: render the field/alternative exactly as before.
    Stored(&'a FieldDecl),
    /// The field's target has no legal payload in the asserted world and this
    /// occurrence's only legal state is absence. No storage is generated.
    AbsentOnly(&'a FieldDecl),
}

/// Classify one field's storage semantics under an explicit world policy.
///
/// Under [`GenerationWorld::ClosedSchemaSet`] this applies Task 026's
/// absence-only elision rule when — and only when — the field is a supported
/// optional occurrence of a zero-known-descendant abstract structural target.
///
/// Under [`GenerationWorld::OpenExtensions`] `AbsentOnly` is never returned
/// merely because a target has zero known descendants: an external derived
/// type may legally make the field present, so the field keeps its storage
/// and the backend's abstract-reference validation fails it closed.
///
/// # Errors
///
/// Returns an error if the field's named target is missing from `schema`.
pub fn field_storage_semantics<'a>(
    schema: &SchemaIr,
    field: &'a FieldDecl,
    world: GenerationWorld,
) -> Result<EffectiveValueMember<'a>, AbstractValueProjectionError> {
    let renderability = abstract_value_reference_renderability(
        schema,
        &field.type_ref,
        field.cardinality,
        field.nillable,
        field.constraints == ConstraintSet::default(),
        world,
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
            // Deterministic across all three backends. It names the target,
            // names the policy, does not claim the schema is invalid, and
            // suggests no placeholder value.
            Self::NotClosedUnderOpenExtensions(name) => write!(
                f,
                "abstract value {} is not closed under open-extensions generation; \
                 external derived types cannot be represented",
                name.local_name
            ),
            Self::UnrepresentableConcreteDescendant {
                target,
                descendant,
                source,
            } => write!(
                f,
                "abstract value target {} has concrete descendant {} whose structural \
                 projection cannot be represented: {source:?}",
                target.local_name, descendant.local_name
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

    // Built once rather than per candidate: `project_structural_type` would
    // otherwise rebuild the whole-schema declaration index for every concrete
    // candidate, which is quadratic on full UCI roots.
    let declarations = schema
        .types
        .iter()
        .map(|declaration| (&declaration.name, declaration))
        .collect::<BTreeMap<_, _>>();

    let mut descendants = Vec::new();
    for candidate in &schema.types {
        if candidate.is_abstract || !is_structural(candidate) || candidate.name == *target {
            continue;
        }
        // Descendancy is decided from the declared base chain, which is
        // available even when the candidate is not *representable*. Deciding
        // it from a successful projection instead would make an unrenderable
        // descendant look like an unrelated declaration.
        if !inherits_from(&declarations, candidate, target) {
            continue;
        }
        match project_with_index(&declarations, &candidate.name) {
            Ok(_) => descendants.push(candidate),
            // Fail closed. The descendant is a legal concrete payload of the
            // abstract target, so neither omitting it from the closed sum nor
            // reporting the target as having no concrete descendants is true.
            Err(source) => {
                return Err(
                    AbstractValueProjectionError::UnrepresentableConcreteDescendant {
                        target: Box::new(target.clone()),
                        descendant: Box::new(candidate.name.clone()),
                        source: Box::new(source),
                    },
                );
            }
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

/// Project an abstract structural named *value* reference under an explicit
/// world policy, if it needs a generated closed-sum wrapper. Other named
/// references retain their exact existing named-value representation.
///
/// Under [`GenerationWorld::ClosedSchemaSet`] this is exactly the Task 024
/// behaviour. Under [`GenerationWorld::OpenExtensions`] every abstract
/// structural value reference fails with
/// [`AbstractValueProjectionError::NotClosedUnderOpenExtensions`], regardless
/// of how many concrete descendants happen to be known: a known descendant
/// set is never assumed exhaustive, and no name, namespace, or documentation
/// heuristic is consulted to decide otherwise.
///
/// A base-type relationship is not a value reference and never reaches here,
/// so concrete types extending an abstract base keep their Task 018 flattened
/// record layout in both worlds.
///
/// # Errors
///
/// Returns an error if the reference names a missing declaration, if the
/// target is an unrepresentable abstract value in the asserted world, or if
/// closed-world projection itself fails.
pub fn abstract_value_projection_for_ref<'a>(
    schema: &'a SchemaIr,
    type_ref: &TypeRef,
    world: GenerationWorld,
) -> Result<Option<AbstractValueProjection<'a>>, AbstractValueProjectionError> {
    let TypeRefTarget::Named(target) = &type_ref.target else {
        return Ok(None);
    };
    let declaration = schema
        .types
        .iter()
        .find(|declaration| declaration.name == *target)
        .ok_or_else(|| AbstractValueProjectionError::MissingDeclaration(target.clone()))?;
    if !(declaration.is_abstract && is_structural(declaration)) {
        return Ok(None);
    }
    match world {
        GenerationWorld::ClosedSchemaSet => project_abstract_value(schema, target).map(Some),
        GenerationWorld::OpenExtensions => Err(
            AbstractValueProjectionError::NotClosedUnderOpenExtensions(target.clone()),
        ),
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

/// Whether `candidate` reaches `ancestor` through declared base types.
///
/// This walks only `base_type` links and is deliberately independent of
/// structural representability, so an unrenderable descendant is still
/// recognized as a descendant. The visited guard bounds a malformed cyclic
/// base chain; cycles are diagnosed by `project_with_index`, not here.
fn inherits_from(
    declarations: &BTreeMap<&QualifiedName, &TypeDecl>,
    candidate: &TypeDecl,
    ancestor: &QualifiedName,
) -> bool {
    let mut visited = BTreeSet::new();
    let mut current = candidate;
    loop {
        let Some(base_ref) = &current.base_type else {
            return false;
        };
        let TypeRefTarget::Named(base_name) = &base_ref.target else {
            return false;
        };
        if base_name == ancestor {
            return true;
        }
        if !visited.insert(base_name.clone()) {
            return false;
        }
        let Some(base) = declarations.get(base_name) else {
            return false;
        };
        current = base;
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::{
        Cardinality, ConstraintSet, NamespaceDecl, PrimitiveKind, SourceRef, TypeRef,
    };

    const NS: &str = "urn:test";

    fn source() -> SourceRef {
        SourceRef {
            document: "test.ir".to_owned(),
            line: Some(1),
        }
    }

    fn field(name: &str) -> FieldDecl {
        FieldDecl {
            name: name.to_owned(),
            type_ref: TypeRef::primitive(PrimitiveKind::String),
            cardinality: Cardinality::REQUIRED_ONE,
            nillable: false,
            constraints: ConstraintSet::default(),
            documentation: None,
            source: source(),
        }
    }

    fn record(name: &str, members: &[&str]) -> TypeDecl {
        TypeDecl {
            name: QualifiedName::new(NS, name),
            is_abstract: false,
            base_type: None,
            kind: TypeKind::Record {
                fields: members.iter().map(|name| field(name)).collect(),
            },
            constraints: ConstraintSet::default(),
            documentation: None,
            source: source(),
        }
    }

    fn derived(mut declaration: TypeDecl, base: &str) -> TypeDecl {
        declaration.base_type = Some(TypeRef::named(QualifiedName::new(NS, base)));
        declaration
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

    /// Case A: a descendant that cannot be projected must never be silently
    /// dropped from the closed sum.
    #[test]
    fn unrepresentable_concrete_descendant_fails_closed_instead_of_partial_sum() {
        let mut base = record("AbstractBase", &["shared"]);
        base.is_abstract = true;
        let good = derived(record("GoodConcrete", &["good"]), "AbstractBase");
        // Re-declaring `shared` collides with the inherited member, so
        // `BadConcrete` cannot be structurally projected.
        let bad = derived(record("BadConcrete", &["shared"]), "AbstractBase");
        let schema = schema(vec![base, good, bad]);

        let error = project_abstract_value(&schema, &QualifiedName::new(NS, "AbstractBase"))
            .expect_err("a descendant that cannot be projected must fail closed");
        let AbstractValueProjectionError::UnrepresentableConcreteDescendant {
            target,
            descendant,
            source,
        } = error
        else {
            panic!("expected UnrepresentableConcreteDescendant, got {error:?}");
        };
        assert_eq!(target.local_name, "AbstractBase");
        assert_eq!(descendant.local_name, "BadConcrete");
        assert!(
            matches!(
                *source,
                StructuralProjectionError::InheritedMemberNameCollision { .. }
            ),
            "the underlying structural failure must be retained, got {source:?}"
        );
    }

    /// Case B: when *every* descendant fails to project, the target must not
    /// be reclassified as having no concrete descendants, because that would
    /// authorize Task 026 absent-only elision for a target that does have a
    /// legal payload.
    #[test]
    fn all_descendants_unrepresentable_is_not_no_concrete_descendants() {
        let mut base = record("AbstractBase", &["shared"]);
        base.is_abstract = true;
        let bad = derived(record("BadConcrete", &["shared"]), "AbstractBase");
        let schema = schema(vec![base, bad]);
        let target = QualifiedName::new(NS, "AbstractBase");

        let error = project_abstract_value(&schema, &target).expect_err("must fail closed");
        assert!(
            !matches!(
                error,
                AbstractValueProjectionError::NoConcreteDescendants(_)
            ),
            "'cannot represent a legal payload' must never become 'no legal payload', got {error:?}"
        );

        // Inhabitance classification must propagate the failure rather than
        // report `NoKnownConcreteDescendants`.
        let inhabitance = classify_abstract_value_inhabitance(&schema, &target);
        assert!(
            inhabitance.is_err(),
            "inhabitance must not claim zero known descendants"
        );

        // And therefore no world interpretation may reach `NoLegalPayload`.
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            assert!(
                classify_abstract_value_semantics(&schema, &target, world).is_err(),
                "{world:?} must not classify an unrepresentable descendant as a legal payload state"
            );
        }
    }

    /// Case B continued: an optional field of such a target stays unsupported,
    /// so no backend may elide it as absent-only.
    #[test]
    fn optional_field_of_unrepresentable_target_is_not_absent_only_elided() {
        let mut base = record("AbstractBase", &["shared"]);
        base.is_abstract = true;
        let bad = derived(record("BadConcrete", &["shared"]), "AbstractBase");
        let mut optional = field("payload");
        optional.type_ref = TypeRef::named(QualifiedName::new(NS, "AbstractBase"));
        optional.cardinality = Cardinality::OPTIONAL_ONE;
        let mut holder = record("Holder", &[]);
        holder.kind = TypeKind::Record {
            fields: vec![optional.clone()],
        };
        let schema = schema(vec![base, bad, holder]);

        let renderability = abstract_value_reference_renderability(
            &schema,
            &optional.type_ref,
            optional.cardinality,
            false,
            true,
            GenerationWorld::ClosedSchemaSet,
        );
        assert!(
            renderability.is_err(),
            "an optional occurrence must not be classified AbsentOnly, got {renderability:?}"
        );
    }

    /// Descendancy is decided from the declared base chain, so a transitive
    /// descendant behind an abstract intermediate is still detected.
    #[test]
    fn transitive_descendancy_uses_declared_base_chain() {
        let mut base = record("AbstractBase", &["shared"]);
        base.is_abstract = true;
        let mut middle = record("AbstractMiddle", &[]);
        middle.is_abstract = true;
        middle.base_type = Some(TypeRef::named(QualifiedName::new(NS, "AbstractBase")));
        let deep = derived(record("DeepConcrete", &["shared"]), "AbstractMiddle");
        let schema = schema(vec![base, middle, deep]);
        assert!(matches!(
            project_abstract_value(&schema, &QualifiedName::new(NS, "AbstractBase")),
            Err(AbstractValueProjectionError::UnrepresentableConcreteDescendant { .. })
        ));
    }

    /// A genuinely descendant-free abstract target still reports
    /// `NoConcreteDescendants`; the correction narrows that classification
    /// rather than removing it.
    #[test]
    fn genuine_zero_descendant_target_still_reports_no_concrete_descendants() {
        let mut base = record("AbstractBase", &[]);
        base.is_abstract = true;
        let unrelated = record("Unrelated", &["value"]);
        let schema = schema(vec![base, unrelated]);
        assert!(matches!(
            project_abstract_value(&schema, &QualifiedName::new(NS, "AbstractBase")),
            Err(AbstractValueProjectionError::NoConcreteDescendants(_))
        ));
    }

    /// Case C: an ordinary valid closed sum is unaffected by the fail-closed
    /// correction, including the concrete non-leaf and abstract-intermediate
    /// topology Task 024 established.
    #[test]
    fn valid_closed_sums_are_unchanged() {
        let mut base = record("AbstractBase", &[]);
        base.is_abstract = true;
        let mut middle = record("AbstractMiddle", &["middle"]);
        middle.is_abstract = true;
        middle.base_type = Some(TypeRef::named(QualifiedName::new(NS, "AbstractBase")));
        let first = derived(record("FirstConcrete", &["first"]), "AbstractBase");
        let second = derived(record("SecondConcrete", &["second"]), "AbstractMiddle");
        let schema = schema(vec![base, middle, first, second]);

        let projection =
            project_abstract_value(&schema, &QualifiedName::new(NS, "AbstractBase")).unwrap();
        assert_eq!(
            projection
                .concrete_descendants
                .iter()
                .map(|declaration| declaration.name.local_name.as_str())
                .collect::<Vec<_>>(),
            ["FirstConcrete", "SecondConcrete"],
            "schema declaration order is preserved and abstract intermediates are not variants"
        );
    }
}
