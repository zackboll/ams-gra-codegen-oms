//! Backend/world readiness over a contract-selected UCI type model.
//!
//! Task 030 produced a [`ServicePlan`]: a language-neutral, world-independent
//! answer to *what does this Service Contract select?*. This module answers the
//! strictly separate question Task 030 deferred: *can backend X render that
//! selection under world Y, today?*
//!
//! The two are kept apart on purpose. A plan is a fact about a contract and a
//! schema set; readiness is a fact about a backend's current capability under
//! an asserted type universe. Folding [`BackendLanguage`] or [`GenerationWorld`]
//! into the plan would make the same contract resolve differently depending on
//! who was going to compile it, which is exactly the coupling ADR-0005 rejects.
//!
//! Renderability itself is **not** redefined here. Every capability question is
//! delegated to [`CoverageAnalysis`], which owns the primitive, cardinality,
//! Choice, inheritance, abstract-value, and world rules established by Tasks
//! 017--029. This module only *selects* which declarations to ask about, and
//! orders the answers.
//!
//! Only baseline capability is consulted: no [`crate::FeatureFamily`]
//! hypothetical is enabled, so a READY result never means "ready if a feature
//! were implemented later".

use crate::coverage::DeclarationRenderability;
use crate::{
    BackendLanguage, BackendPreflightError, CoverageAnalysis, CoverageError, GenerationWorld,
    PlanBindingMismatch, ServiceApiError, ServiceGenerationError, ServicePlan, ServicePlanError,
    backend_preflight, project_service_generation_schema, service_api_preflight,
    validate_service_plan_api_names,
};
use ams_gra_oms_ir::{PrimitiveKind, QualifiedName, SchemaIr, TypeRefTarget};
use std::collections::BTreeSet;
use std::fmt;

/// A failure while measuring backend readiness for a selected service model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceReadinessError {
    /// The selected closure could not be computed from the supplied schema.
    Plan(ServicePlanError),
    /// Backend capability analysis of the supplied schema failed.
    Coverage(CoverageError),
    /// The plan selects a message the supplied schema set does not declare.
    ///
    /// The library API takes a plan and a schema separately, so a caller can
    /// pass a plan that was resolved against a *different* schema set. That is
    /// a programming error, but it must fail deterministically rather than
    /// panic on an index lookup.
    PlanSchemaMismatch {
        /// The selected identity the supplied schema does not declare.
        missing: QualifiedName,
        /// What kind of selection referenced it.
        role: MismatchRole,
    },
    /// The supplied schema declares every selected identity, but not with the
    /// same semantics the plan was resolved against.
    ///
    /// Identity-only checks cannot see this: the message and type names all
    /// match while a payload type, declaration body, or a dependency's
    /// constraints differ. Detected by [`ServicePlan::verify_schema_binding`].
    PlanBinding(PlanBindingMismatch),
    /// Projecting the selected model onto a generable schema failed for a
    /// reason that is an **integrity defect**, not an ordinary backend
    /// capability limit.
    ///
    /// Readiness previously ended its projection match with `Err(_) => None`,
    /// reasoning that a projection failure is already visible as a
    /// per-declaration blocker. That holds for abstract-value capability
    /// failures, but `ProjectedSchemaInvalid` means the projection dropped a
    /// required dependency and `ProjectedEmissionPlan` can report a planning
    /// failure with no per-declaration counterpart. Silently mapping either
    /// to "no backend blocker" could leave a false READY, so they are
    /// propagated as typed errors instead.
    Projection(Box<ServiceGenerationError>),
    /// Lowering the service API model failed for an integrity reason rather
    /// than an ordinary wrapper boundary. Only
    /// [`ServiceApiError::EmissionPlan`] reaches here: projection already
    /// planned the same schema, so that failure is a defect, not a NOT READY.
    ///
    /// Boxed, like [`Self::Projection`], so every readiness `Result` stays
    /// small.
    ServiceApi(Box<ServiceApiError>),
}

/// Which part of the selection referred to an absent schema identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MismatchRole {
    /// A selected OMS message identity.
    Message,
    /// A type declaration in a selected message's dependency closure.
    TypeDeclaration,
}

impl MismatchRole {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Message => "selected OMS message",
            Self::TypeDeclaration => "selected type declaration",
        }
    }
}

impl fmt::Display for ServiceReadinessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plan(error) => error.fmt(formatter),
            Self::Coverage(error) => error.fmt(formatter),
            Self::PlanSchemaMismatch { missing, role } => write!(
                formatter,
                "{} {{{}}}{} is absent from the supplied schema set; the service plan was \
                 resolved against a different schema set",
                role.label(),
                missing.namespace_uri,
                missing.local_name
            ),
            Self::PlanBinding(mismatch) => mismatch.fmt(formatter),
            Self::Projection(error) => write!(
                formatter,
                "selected-service projection failed while measuring readiness: {error}"
            ),
            Self::ServiceApi(error) => write!(
                formatter,
                "service API lowering failed while measuring readiness: {error}"
            ),
        }
    }
}

impl std::error::Error for ServiceReadinessError {}

impl From<PlanBindingMismatch> for ServiceReadinessError {
    fn from(mismatch: PlanBindingMismatch) -> Self {
        Self::PlanBinding(mismatch)
    }
}

impl From<ServicePlanError> for ServiceReadinessError {
    fn from(error: ServicePlanError) -> Self {
        Self::Plan(error)
    }
}

impl From<CoverageError> for ServiceReadinessError {
    fn from(error: CoverageError) -> Self {
        Self::Coverage(error)
    }
}

/// The first deterministic reason a selected message cannot be rendered.
///
/// Typed rather than prose: a caller that wants to act on a blocker (a CI gate,
/// a future selected-generation planner) must not have to parse a sentence.
/// The CLI formats this into text; the core result stays structured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceMessageBlocker {
    /// The earliest non-renderable declaration in this message's dependency
    /// closure, in schema declaration order.
    Declaration(QualifiedName),
    /// The message's payload is an unsupported primitive value.
    Primitive(PrimitiveKind),
    /// The message's payload *reference* is unsupported even though every
    /// declaration it reaches is renderable. In practice this is an abstract
    /// structural value payload under `OpenExtensions`, where no representation
    /// for the value position exists regardless of the declarations behind it.
    PayloadReference(QualifiedName),
}

impl fmt::Display for ServiceMessageBlocker {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Declaration(name) | Self::PayloadReference(name) => {
                write!(formatter, "{{{}}}{}", name.namespace_uri, name.local_name)
            }
            Self::Primitive(kind) => write!(formatter, "primitive {kind:?}"),
        }
    }
}

/// One selected OMS message that the requested backend/world cannot render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockedMessage {
    /// The local name exactly as the contract spelled it.
    pub contract_message: String,
    /// The resolved, fully qualified UCI message identity.
    pub message_name: QualifiedName,
    /// The single deterministic first blocker.
    pub blocker: ServiceMessageBlocker,
}

/// Whether one backend can render a contract's selected UCI type model under
/// one asserted generation world.
///
/// Counts and lists cover **only** what the contract selects. Unselected
/// declarations are invisible here even when they are unrenderable: that is the
/// point of contract-selected analysis, and it is why a service can be READY
/// against a schema set whose full-schema generation fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceBackendReadiness {
    pub language: BackendLanguage,
    pub world: GenerationWorld,
    /// Size of the contract-selected type closure.
    pub selected_types_total: usize,
    /// How many of those the backend can render today.
    pub selected_types_renderable: usize,
    /// Unique selected OMS messages (repeated selections counted once).
    pub selected_messages_total: usize,
    /// How many of those have a fully renderable closure.
    pub selected_messages_renderable: usize,
    /// Selected declarations the backend cannot render, in **schema
    /// declaration order**, matching [`ServicePlan::selected_type_closure`].
    pub unsupported_types: Vec<QualifiedName>,
    /// Blocked selected messages, in **contract first-occurrence order**,
    /// matching [`ServicePlan::selected_messages`].
    pub blocked_messages: Vec<BlockedMessage>,
    /// A global backend precondition the **projected** selected schema
    /// violates, if any.
    ///
    /// Per-declaration capability is not the whole story: two individually
    /// renderable selected declarations can still be ungenerable together,
    /// because their generated names collide or because the selected closure
    /// spans more than one namespace. Those are properties of the projected
    /// schema as a whole, so they are measured on exactly the input that
    /// selected-service generation would hand to the backend.
    ///
    /// This is a backend **capability** blocker, not a claim that the schema
    /// is malformed; multi-namespace IR is valid input the backends simply do
    /// not generate yet.
    pub backend_blocker: Option<BackendPreflightError>,
    /// A Task 047 service API wrapper boundary, if any: a wrapper name that
    /// cannot be emitted safely in this language (for example two portable
    /// IDs that normalize to one identifier), or an OMS payload with no
    /// generated model type to bind.
    ///
    /// Kept apart from `unsupported_types` and `blocked_messages` on purpose:
    /// it is not a statement about UCI type capability, and it never changes
    /// a selected-type or selected-message count.
    pub service_api_blocker: Option<ServiceApiError>,
}

impl ServiceBackendReadiness {
    /// True when every selected OMS message closure is renderable.
    ///
    /// A contract with zero OMS Message exchanges is vacuously ready: Data
    /// Transfer, Special Signal, Security Exchange, and non-OMS Message
    /// exchanges are genuine parts of a service interface that simply require
    /// no UCI type model, so they can neither satisfy nor fail UCI type
    /// readiness.
    /// A global backend precondition violation makes the service NOT READY
    /// even when every selected declaration is individually renderable,
    /// because generation of that same projected schema would fail.
    ///
    /// Since Task 047 a service API boundary does too: READY means
    /// `service-generate` can produce the type model AND the wrapper.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.blocked_messages.is_empty()
            && self.backend_blocker.is_none()
            && self.service_api_blocker.is_none()
    }
}

/// Measure whether `language` can render `plan`'s selected UCI type model
/// under `world`.
///
/// `schema` must be the schema set the plan was resolved against; a mismatch is
/// detected and reported rather than panicking.
///
/// Exactly one [`CoverageAnalysis`] and exactly one baseline renderability
/// snapshot are built per call -- over the projected selected schema when
/// projection succeeds, over `schema` only when it does not -- and both are
/// then reused for every selected type and message query. The projection
/// itself is likewise computed once, not per declaration or per message.
/// No hypothetical feature combinations are evaluated,
/// so this is not a disguised full coverage report: nothing here reaches
/// `CoverageAnalysis::report` or `CoverageAnalysis::impact`.
///
/// The cost of one call is almost entirely [`CoverageAnalysis::new`], spent
/// building the abstract-value topology and elision indexes. Because that
/// analysis is now built over the **projected** schema rather than the whole
/// schema, a selected service pays for its own selection rather than for all
/// of UCI: full UCI 2.5 service-check measures about 5.9 s per language,
/// down from ~15 s when the analysis was whole-schema. Plan resolution and the
/// renderability snapshot remain negligible beside it.
///
/// # Errors
///
/// Returns [`ServiceReadinessError`] if the selected closure cannot be
/// computed, if capability analysis of the schema fails, or if the plan
/// references identities the supplied schema set does not declare.
pub fn analyze_service_readiness(
    plan: &ServicePlan,
    schema: &SchemaIr,
    language: BackendLanguage,
    world: GenerationWorld,
) -> Result<ServiceBackendReadiness, ServiceReadinessError> {
    // Wrong-schema reuse is diagnosed FIRST, once, by the single shared
    // binding mechanism. Doing it up front means every lookup below is known
    // to be against the schema the plan was resolved against, so the
    // identity-only fallbacks that remain are unreachable defence in depth
    // rather than the primary check.
    plan.verify_schema_binding(schema)?;

    // Task 030's single dependency model, reused rather than re-implemented.
    // Counts and ordering are a fact about the *contract*, so they come from
    // the original schema even when capability is measured on the projection.
    let closure = plan.selected_type_closure(schema)?;

    // Projection runs FIRST, because it decides which schema the capability
    // questions below are legitimately asked about.
    //
    // # Why capability cannot be measured on the full schema
    //
    // Generated-name safety and the global backend preconditions are
    // *scope-dependent*: they are relationships between the declarations that
    // are actually emitted together, not properties of a declaration alone.
    // Measuring them over the whole schema imported failures from
    // declarations selected generation removes. A selected `foo_bar` beside
    // an unselected `fooBar` both generate `FooBar`, so full-schema analysis
    // marks both unsafe -- yet the projection contains only one of them and
    // generates cleanly. Readiness would report NOT READY for a service
    // `service-generate` then produced successfully, which is precisely the
    // readiness/generation disagreement this module exists to prevent.
    //
    // The same applies to conditional generated support: a schema whose
    // *unselected* part has an unbounded member makes Rust emit
    // `UnboundedVec`, but if the projection drops that member the support
    // type is not emitted and the spelling is free again.
    //
    // So whenever projection succeeds, the projected schema is authoritative:
    // it is byte-for-byte the schema `service-generate` hands to the backend.
    let projection = match project_service_generation_schema(plan, schema, world) {
        Ok(projection) => Ok(projection),
        Err(ServiceGenerationError::Plan(error)) => return Err(error.into()),
        Err(ServiceGenerationError::PlanSchemaMismatch { missing, role }) => {
            return Err(ServiceReadinessError::PlanSchemaMismatch { missing, role });
        }
        Err(ServiceGenerationError::PlanBinding(mismatch)) => {
            return Err(ServiceReadinessError::PlanBinding(mismatch));
        }
        // An abstract structural value that cannot be represented under the
        // asserted world is an ordinary *capability* limit, not a naming or
        // global-precondition failure. No projected schema exists to analyze,
        // so attribution falls back to the original schema, which still
        // explains the blocker deterministically. Retained below.
        Err(error @ ServiceGenerationError::AbstractValue(_)) => Err(error),
        // The projected subset failed `SchemaIr::validate`, meaning projection
        // dropped a required named dependency. That is an internal defect, not
        // a statement about backend capability, so it must never be softened
        // into an ordinary NOT READY.
        error @ Err(ServiceGenerationError::ProjectedSchemaInvalid(_))
        // Emission planning over the projected schema failed. Unlike the
        // abstract-value case this has no guaranteed per-declaration
        // counterpart, so it is propagated rather than assumed duplicated.
        | error @ Err(ServiceGenerationError::ProjectedEmissionPlan(_)) => {
            return Err(ServiceReadinessError::Projection(Box::new(
                error.expect_err("matched on an Err arm"),
            )));
        }
    };

    // Exactly one schema is analyzed per call: the projection when it exists,
    // the original only when projection produced none. Building both would
    // double the dominant cost (`CoverageAnalysis::new`) for no added signal.
    let analyzed_schema = match &projection {
        Ok(projection) => projection.schema(),
        Err(_) => schema,
    };

    // One analysis, one snapshot, reused below. Task 026 showed what repeated
    // whole-schema scans cost; nothing in this function may rebuild either.
    let analysis = CoverageAnalysis::new(analyzed_schema, world)?;
    let renderability = analysis.baseline_renderability(language);
    let baseline = BTreeSet::new();

    let mut unsupported_types = Vec::new();
    for declaration in &closure {
        let index = declaration_index(&analysis, &declaration.name, MismatchRole::TypeDeclaration)?;
        if !renderability.is_renderable(index) {
            // `closure` is already in schema declaration order, so this
            // preserves it without a sort.
            unsupported_types.push(declaration.name.clone());
        }
    }

    let mut blocked_messages = Vec::new();
    // `selected_messages` is unique and in contract first-occurrence order, so
    // repeated exchange selections of one message are analyzed once and appear
    // at most once here.
    for selected in plan.selected_messages() {
        // Looked up in the analyzed schema so the message declaration and the
        // renderability snapshot describe the same type universe. Projection
        // retains every selected message verbatim, so this resolves to the
        // same declaration the original schema carries.
        let declaration = analyzed_schema
            .messages
            .iter()
            .find(|message| message.name == selected.name)
            .ok_or_else(|| ServiceReadinessError::PlanSchemaMismatch {
                missing: selected.name.clone(),
                role: MismatchRole::Message,
            })?;
        if analysis.message_closure_renderable(declaration, language, &baseline, &renderability)? {
            continue;
        }
        let blocker = match &selected.payload_type.target {
            TypeRefTarget::Primitive(kind) => ServiceMessageBlocker::Primitive(*kind),
            TypeRefTarget::Named(payload) => {
                match first_declaration_blocker(&analysis, payload, &renderability)? {
                    Some(blocker) => blocker,
                    // Every declaration behind the payload renders, so the
                    // payload *position* itself is what fails: an abstract
                    // structural value under `OpenExtensions`.
                    None => {
                        debug_assert!(!analysis.message_payload_reference_renderable(
                            declaration,
                            language,
                            &baseline
                        ));
                        ServiceMessageBlocker::PayloadReference(payload.clone())
                    }
                }
            }
        };
        blocked_messages.push(BlockedMessage {
            contract_message: selected.contract_message.clone(),
            message_name: selected.name.clone(),
            blocker,
        });
    }

    // Global backend preconditions, measured on exactly the schema that
    // selected-service generation would hand to the backend. Using the
    // projection is what makes readiness and generation agree: checking the
    // *full* schema instead would wrongly block a service whose selected
    // closure narrows to one namespace or drops a colliding declaration, and
    // checking nothing at all is the historical defect that let a
    // multi-namespace selection report READY.
    let backend_blocker = match &projection {
        // Measured in the same world the readiness verdict is stated for, so
        // a world-sensitive generated name is never reserved here that the
        // requested world could not emit.
        Ok(projection) => backend_preflight(projection.schema(), language, world).err(),
        // An abstract structural value that cannot be represented under the
        // asserted world is an ordinary *capability* limit, and it is already
        // reported above as a per-declaration or per-message blocker: the
        // same `CoverageAnalysis` rules that reject the declaration here also
        // reject the projection there. Relabelling it as a global precondition
        // would double-report one cause.
        //
        // That invariant is asserted rather than assumed: if this arm is ever
        // reached while the readiness result would still be READY, the two
        // analyses have drifted and the debug build fails loudly instead of
        // emitting a false READY.
        Err(error) => {
            debug_assert!(
                !unsupported_types.is_empty() || !blocked_messages.is_empty(),
                "an abstract-value projection failure must already appear as a \
                 per-declaration or per-message blocker, but readiness found none: {error}"
            );
            // Fail closed even in release: if the invariant does not hold,
            // reporting no blocker would be a false READY.
            if unsupported_types.is_empty() && blocked_messages.is_empty() {
                return Err(ServiceReadinessError::Projection(Box::new(error.clone())));
            }
            None
        }
    };

    // Task 047: READY must mean `service-generate` can emit BOTH the selected
    // type model AND its service API wrapper, so the shared wrapper preflight
    // joins the same authoritative verdict. It is reported in its own field,
    // never as a fake unsupported UCI declaration, and it changes no count.
    //
    // Wrapper names derive only from contract IDs and exchange kinds, so they
    // are always checked -- even when the type selection is already blocked.
    // Payload binding is checked only when the type selection is otherwise
    // ready: a message already blocked above would otherwise be reported twice
    // for one cause.
    let service_api_blocker = match validate_service_plan_api_names(plan, language) {
        Err(name) => Some(ServiceApiError::Name(name)),
        Ok(()) => match &projection {
            Ok(projection)
                if unsupported_types.is_empty()
                    && blocked_messages.is_empty()
                    && backend_blocker.is_none() =>
            {
                match service_api_preflight(plan, projection.schema(), language, world) {
                    Ok(_) => None,
                    // Projection already planned this exact schema, so a
                    // planning failure here is an integrity defect, never an
                    // ordinary NOT READY.
                    Err(error @ ServiceApiError::EmissionPlan(_)) => {
                        return Err(ServiceReadinessError::ServiceApi(Box::new(error)));
                    }
                    Err(error) => Some(error),
                }
            }
            _ => None,
        },
    };

    let selected_types_total = closure.len();
    let selected_messages_total = plan.selected_messages().len();
    Ok(ServiceBackendReadiness {
        language,
        world,
        selected_types_total,
        selected_types_renderable: selected_types_total - unsupported_types.len(),
        selected_messages_total,
        selected_messages_renderable: selected_messages_total - blocked_messages.len(),
        unsupported_types,
        blocked_messages,
        backend_blocker,
        service_api_blocker,
    })
}

/// The earliest non-renderable declaration reachable from `payload`, in schema
/// declaration order.
///
/// `CoverageAnalysis::dependency_closure` returns the closure in original
/// declaration order, so "first non-renderable member" is already the
/// deterministic schema-order answer. Choosing a `BTreeMap`/hash member instead
/// would make the reported blocker depend on name spelling rather than on the
/// schema.
fn first_declaration_blocker(
    analysis: &CoverageAnalysis<'_>,
    payload: &QualifiedName,
    renderability: &DeclarationRenderability,
) -> Result<Option<ServiceMessageBlocker>, ServiceReadinessError> {
    for declaration in analysis.dependency_closure(payload)? {
        let index = declaration_index(analysis, &declaration.name, MismatchRole::TypeDeclaration)?;
        if !renderability.is_renderable(index) {
            return Ok(Some(ServiceMessageBlocker::Declaration(
                declaration.name.clone(),
            )));
        }
    }
    Ok(None)
}

/// Pre-indexed O(log n) lookup, with a typed mismatch instead of a panic.
fn declaration_index(
    analysis: &CoverageAnalysis<'_>,
    name: &QualifiedName,
    role: MismatchRole,
) -> Result<usize, ServiceReadinessError> {
    analysis
        .declaration_index(name)
        .ok_or_else(|| ServiceReadinessError::PlanSchemaMismatch {
            missing: name.clone(),
            role,
        })
}
