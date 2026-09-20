//! Language-neutral projection of a contract-selected UCI type model into an
//! ordinary [`SchemaIr`] that the existing backends can generate from.
//!
//! Task 030 answered *what does this contract select?* ([`ServicePlan`]), and
//! Task 031 answered *can backend X render that selection under world Y?*
//! ([`crate::ServiceBackendReadiness`]). This module answers the strictly
//! mechanical question those two leave open: *what schema do we hand to a
//! backend so that it generates the selected model and nothing else?*
//!
//! The answer is deliberately a **projected `SchemaIr`**, not a new backend
//! entry point. Ada, Rust, and C++ already know how to turn a `SchemaIr` into
//! source; teaching each of them what a Service Contract is would triple the
//! places a contract could be misinterpreted. Instead, exactly one
//! language-neutral function narrows the schema, and from the backend's point
//! of view selected generation is indistinguishable from ordinary generation.
//!
//! # Semantic closure versus generated support closure
//!
//! [`ServicePlan::selected_type_closure`] is a *semantic* closure: base types,
//! aliases, Record fields, Choice alternatives, and List item types. It is the
//! honest answer to "which declarations does this contract require?" and it is
//! **not** changed here.
//!
//! Generated code needs slightly more than semantics. Task 024 lowers an
//! abstract structural value into a closed sum over every concrete transitive
//! descendant, and those descendants are not ordinary named dependencies of
//! anything in the selected closure. So:
//!
//! ```text
//! abstract Base ; ConcreteA : Base ; ConcreteB : Base
//! Holder { Value : Base }
//! message SelectedReport -> Holder
//!
//! contract-selected types : Holder, Base
//! generated support types : ConcreteA, ConcreteB
//! ```
//!
//! Folding `ConcreteA`/`ConcreteB` into `selected_type_closure` would corrupt
//! Task 030's meaning (the contract selects neither), so they are tracked
//! separately and reported separately.
//!
//! # What this module is not
//!
//! It contains no backend capability rules. A projection can succeed for a
//! declaration Ada cannot render; that remains Task 031's readiness verdict,
//! which callers are expected to consult *first*. Support expansion is
//! therefore never a way to smuggle an unsupported selected type past
//! readiness.

use crate::abstract_value::is_structural;
use crate::{
    AbstractValueProjectionError, CodegenError, GenerationWorld, MismatchRole, ServicePlan,
    ServicePlanError, direct_named_dependencies, plan_type_emissions, project_abstract_value,
};
use ams_gra_oms_ir::{QualifiedName, SchemaIr, TypeDecl, TypeKind, TypeRef, TypeRefTarget};
use std::collections::BTreeSet;
use std::fmt;

/// A failure while projecting a selected service model onto a generable schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceGenerationError {
    /// The contract-selected semantic closure could not be computed.
    Plan(ServicePlanError),
    /// The supplied plan was resolved against a different schema set.
    ///
    /// The library API takes a plan and a schema separately, so this is a
    /// programming error rather than a user error, but it must be reported
    /// deterministically instead of panicking on a failed lookup.
    PlanSchemaMismatch {
        missing: QualifiedName,
        role: MismatchRole,
    },
    /// A selected value position targets an abstract structural declaration
    /// that has no generated representation under the asserted world.
    ///
    /// Under [`GenerationWorld::OpenExtensions`] this is every abstract
    /// structural value: an external derived type could exist, so no closed
    /// sum over the currently known descendants is honest. The projection
    /// fails closed rather than emitting a partial sum or quietly behaving as
    /// if the world were closed.
    AbstractValue(AbstractValueProjectionError),
    /// The projected subset schema failed [`SchemaIr::validate`].
    ///
    /// This always indicates a projection defect -- a required named
    /// dependency was dropped -- and is never suppressed to make a subset
    /// "work".
    ProjectedSchemaInvalid(String),
    /// The projected schema could not be planned into generated entities.
    ProjectedEmissionPlan(CodegenError),
}

impl fmt::Display for ServiceGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plan(error) => error.fmt(formatter),
            Self::PlanSchemaMismatch { missing, role } => write!(
                formatter,
                "{} {{{}}}{} is absent from the supplied schema set; the service plan was \
                 resolved against a different schema set",
                role.label(),
                missing.namespace_uri,
                missing.local_name
            ),
            Self::AbstractValue(error) => write!(
                formatter,
                "selected model cannot be projected for generation: {error}"
            ),
            Self::ProjectedSchemaInvalid(message) => write!(
                formatter,
                "projected service schema is invalid: {message}; this is a projection defect"
            ),
            Self::ProjectedEmissionPlan(error) => write!(
                formatter,
                "projected service schema cannot be planned for generation: {error}"
            ),
        }
    }
}

impl std::error::Error for ServiceGenerationError {}

/// A contract-selected UCI type model, projected onto an ordinary schema.
///
/// The projected schema is **owned**, so it can be handed straight to
/// `Backend::generate` without any parallel backend path, and the caller's
/// original [`SchemaIr`] is never mutated. Declarations and messages are
/// cloned verbatim, including their `SourceRef`: a projection is a narrowing
/// of an existing schema, not a new synthetic schema authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceGenerationProjection {
    schema: SchemaIr,
    selected_type_names: Vec<QualifiedName>,
    generated_support_type_names: Vec<QualifiedName>,
    selected_message_names: Vec<QualifiedName>,
}

impl ServiceGenerationProjection {
    /// The projected schema, suitable for ordinary backend generation.
    #[must_use]
    pub const fn schema(&self) -> &SchemaIr {
        &self.schema
    }

    /// Declarations the **contract** selects, in original schema order.
    ///
    /// This is exactly [`ServicePlan::selected_type_closure`]'s membership.
    #[must_use]
    pub fn selected_type_names(&self) -> &[QualifiedName] {
        &self.selected_type_names
    }

    /// Declarations retained only because generated representation needs
    /// them, in original schema order.
    ///
    /// Classification rule: a declaration is "selected" if and only if it is
    /// in the raw semantic closure; everything else in the projected schema is
    /// "support", including concrete closed-sum descendants, their own
    /// dependencies, and abstract intermediates retained for effective
    /// structural inheritance. The two lists are disjoint by construction and
    /// their union is exactly `schema().types`.
    #[must_use]
    pub fn generated_support_type_names(&self) -> &[QualifiedName] {
        &self.generated_support_type_names
    }

    /// The selected OMS message identities present in the projected schema,
    /// in original schema order.
    #[must_use]
    pub fn selected_message_names(&self) -> &[QualifiedName] {
        &self.selected_message_names
    }
}

/// Project a resolved [`ServicePlan`] onto a schema a backend can generate.
///
/// Performs no filesystem access and no language rendering. Callers are
/// expected to have consulted [`crate::analyze_service_readiness`] first:
/// projection answers "what must be generated", not "can this backend render
/// it".
///
/// # Errors
///
/// Returns [`ServiceGenerationError`] if the selected closure cannot be
/// computed, if the plan does not match the supplied schema, if a selected
/// abstract structural value has no representation in `world`, or if the
/// projected subset fails schema validation or emission planning.
pub fn project_service_generation_schema(
    plan: &ServicePlan,
    schema: &SchemaIr,
    world: GenerationWorld,
) -> Result<ServiceGenerationProjection, ServiceGenerationError> {
    // 1. The raw contract-selected semantic closure, computed once by Task
    //    030's walker. It is deliberately not recomputed here: a second
    //    dependency walker would be free to drift.
    let selected = plan
        .selected_type_closure(schema)
        .map_err(ServiceGenerationError::Plan)?;
    let selected_names = selected
        .iter()
        .map(|declaration| declaration.name.clone())
        .collect::<BTreeSet<_>>();

    // 2. Fixed-point generated-support expansion. Every newly admitted
    //    declaration contributes its ordinary named dependencies and, for any
    //    abstract structural VALUE position it contains, every concrete
    //    transitive descendant Task 024 stores as a wrapper variant. Those
    //    descendants can themselves introduce new dependencies and new nested
    //    abstract values, so this iterates rather than expanding once.
    let mut required = selected_names.clone();
    let mut pending = selected_names.iter().cloned().collect::<Vec<_>>();

    // Selected message payload references are value positions too, so a
    // message whose payload is directly an abstract structural type demands
    // the same wrapper treatment as a Record field would.
    for message in plan.selected_messages() {
        if let Some(target) = abstract_value_target(schema, &message.payload_type)? {
            admit_abstract_value(schema, &target, world, &mut required, &mut pending)?;
        }
    }

    while let Some(name) = pending.pop() {
        let declaration = find_declaration(schema, &name)?;
        for dependency in direct_named_dependencies(declaration) {
            if required.insert(dependency.clone()) {
                pending.push(dependency.clone());
            }
        }
        for target in declared_abstract_value_targets(schema, declaration)? {
            admit_abstract_value(schema, &target, world, &mut required, &mut pending)?;
        }
    }

    // 3. Filter the ORIGINAL declaration sequence. Schema order, not
    //    traversal-discovery order and not contract presentation order, is
    //    what the projected schema preserves, so reordering the contract's
    //    exchanges cannot change generated output.
    let types = schema
        .types
        .iter()
        .filter(|declaration| required.contains(&declaration.name))
        .cloned()
        .collect::<Vec<_>>();
    let selected_type_names = types
        .iter()
        .map(|declaration| declaration.name.clone())
        .filter(|name| selected_names.contains(name))
        .collect::<Vec<_>>();
    let generated_support_type_names = types
        .iter()
        .map(|declaration| declaration.name.clone())
        .filter(|name| !selected_names.contains(name))
        .collect::<Vec<_>>();

    // 4. Only the selected messages survive. A message is not retained just
    //    because its payload type happens to be required for another reason.
    let mut selected_message_identities = BTreeSet::new();
    for message in plan.selected_messages() {
        if !schema
            .messages
            .iter()
            .any(|declaration| declaration.name == message.name)
        {
            return Err(ServiceGenerationError::PlanSchemaMismatch {
                missing: message.name.clone(),
                role: MismatchRole::Message,
            });
        }
        selected_message_identities.insert(message.name.clone());
    }
    let messages = schema
        .messages
        .iter()
        .filter(|message| selected_message_identities.contains(&message.name))
        .cloned()
        .collect::<Vec<_>>();
    let selected_message_names = messages
        .iter()
        .map(|message| message.name.clone())
        .collect::<Vec<_>>();

    // 5. Namespaces are narrowed, never flattened or renamed. If the selected
    //    model genuinely spans namespaces the existing backend boundary
    //    reports it; this module does not invent multi-namespace generation.
    let used = types
        .iter()
        .map(|declaration| declaration.name.namespace_uri.clone())
        .chain(
            messages
                .iter()
                .map(|message| message.name.namespace_uri.clone()),
        )
        .collect::<BTreeSet<_>>();
    let namespaces = schema
        .namespaces
        .iter()
        .filter(|namespace| used.contains(&namespace.uri))
        .cloned()
        .collect::<Vec<_>>();

    let projected = SchemaIr {
        // Exactly the original schema root's version. Nothing is derived from
        // the contract's logical UCI version, and the two are never compared.
        schema_version: schema.schema_version.clone(),
        namespaces,
        types,
        messages,
    };

    // 6. The subset must stand on its own. A missing named dependency is a
    //    Task 032 bug, so validation is required rather than skipped.
    projected
        .validate()
        .map_err(|error| ServiceGenerationError::ProjectedSchemaInvalid(error.to_string()))?;
    // Reusing the shared emission planner (rather than writing a second
    // selected topological sort) proves the projection carries everything the
    // generated by-value representation needs, including every closed-sum
    // variant.
    plan_type_emissions(&projected, world)
        .map_err(ServiceGenerationError::ProjectedEmissionPlan)?;

    Ok(ServiceGenerationProjection {
        schema: projected,
        selected_type_names,
        generated_support_type_names,
        selected_message_names,
    })
}

/// Admit one abstract structural value target's generated representation.
///
/// Under `ClosedSchemaSet` every concrete transitive descendant becomes a
/// wrapper variant and is admitted, including concrete non-leaf descendants;
/// abstract intermediates are not variants but are pulled in anyway when a
/// descendant's `base_type` ancestry needs them, through the ordinary
/// dependency walk. A zero-known-descendant target admits nothing extra: Task
/// 026 elision handles it, and inventing a concrete support type would be
/// fabrication.
fn admit_abstract_value(
    schema: &SchemaIr,
    target: &QualifiedName,
    world: GenerationWorld,
    required: &mut BTreeSet<QualifiedName>,
    pending: &mut Vec<QualifiedName>,
) -> Result<(), ServiceGenerationError> {
    if world == GenerationWorld::OpenExtensions {
        return Err(ServiceGenerationError::AbstractValue(
            AbstractValueProjectionError::NotClosedUnderOpenExtensions(target.clone()),
        ));
    }
    let descendants = match project_abstract_value(schema, target) {
        Ok(projection) => projection.concrete_descendants,
        // Task 026: retained as a declaration, no invented payload.
        Err(AbstractValueProjectionError::NoConcreteDescendants(_)) => return Ok(()),
        Err(error) => return Err(ServiceGenerationError::AbstractValue(error)),
    };
    for descendant in descendants {
        if required.insert(descendant.name.clone()) {
            pending.push(descendant.name.clone());
        }
    }
    Ok(())
}

/// Abstract structural value targets referenced by one declaration's own
/// value positions.
///
/// Declared members are used rather than effective members: an inherited
/// field arrives with its declaring base, which is itself admitted through the
/// `base_type` dependency edge and visited in turn. Base-type ancestry is
/// never treated as a value position, matching Task 024.
fn declared_abstract_value_targets(
    schema: &SchemaIr,
    declaration: &TypeDecl,
) -> Result<Vec<QualifiedName>, ServiceGenerationError> {
    let references: Vec<&TypeRef> = match &declaration.kind {
        TypeKind::Record { fields } => fields.iter().map(|field| &field.type_ref).collect(),
        TypeKind::Choice { alternatives } => alternatives
            .iter()
            .map(|alternative| &alternative.type_ref)
            .collect(),
        TypeKind::Alias(target) => vec![target],
        TypeKind::List { item_type, .. } => vec![item_type],
        TypeKind::Primitive(_) | TypeKind::Enumeration { .. } => Vec::new(),
    };
    let mut targets = Vec::new();
    for reference in references {
        targets.extend(abstract_value_target(schema, reference)?);
    }
    Ok(targets)
}

/// The abstract structural target of one value reference, if any.
fn abstract_value_target(
    schema: &SchemaIr,
    reference: &TypeRef,
) -> Result<Option<QualifiedName>, ServiceGenerationError> {
    let TypeRefTarget::Named(name) = &reference.target else {
        return Ok(None);
    };
    let declaration = find_declaration(schema, name)?;
    Ok((declaration.is_abstract && is_structural(declaration)).then(|| name.clone()))
}

fn find_declaration<'schema>(
    schema: &'schema SchemaIr,
    name: &QualifiedName,
) -> Result<&'schema TypeDecl, ServiceGenerationError> {
    schema
        .types
        .iter()
        .find(|declaration| &declaration.name == name)
        .ok_or_else(|| ServiceGenerationError::PlanSchemaMismatch {
            missing: name.clone(),
            role: MismatchRole::TypeDeclaration,
        })
}
