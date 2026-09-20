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
    BackendLanguage, CoverageAnalysis, CoverageError, GenerationWorld, ServicePlan,
    ServicePlanError,
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
        }
    }
}

impl std::error::Error for ServiceReadinessError {}

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
}

impl ServiceBackendReadiness {
    /// True when every selected OMS message closure is renderable.
    ///
    /// A contract with zero OMS Message exchanges is vacuously ready: Data
    /// Transfer, Special Signal, Security Exchange, and non-OMS Message
    /// exchanges are genuine parts of a service interface that simply require
    /// no UCI type model, so they can neither satisfy nor fail UCI type
    /// readiness.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.blocked_messages.is_empty()
    }
}

/// Measure whether `language` can render `plan`'s selected UCI type model
/// under `world`.
///
/// `schema` must be the schema set the plan was resolved against; a mismatch is
/// detected and reported rather than panicking.
///
/// Exactly one [`CoverageAnalysis`] and exactly one baseline renderability
/// snapshot are built per call, and both are then reused for every selected
/// type and message query. No hypothetical feature combinations are evaluated,
/// so this is not a disguised full coverage report: nothing here reaches
/// `CoverageAnalysis::report` or `CoverageAnalysis::impact`.
///
/// Measured against authoritative UCI 2.5 (5,557 types), the cost of one call
/// is almost entirely [`CoverageAnalysis::new`] -- roughly 244 s of a ~255 s
/// total, spent building the abstract-value topology and elision indexes over
/// the whole schema. The parts Task 031 added are negligible beside it: one
/// complete baseline renderability snapshot plus every message closure measures
/// about 46 ms, and plan resolution about 13 us. Readiness is therefore already
/// at the floor imposed by constructing the shared analysis once; the remaining
/// cost is pre-existing whole-schema indexing, not per-selected-type work.
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
    // One analysis, one snapshot, reused below. Task 026 showed what repeated
    // whole-schema scans cost; nothing in this function may rebuild either.
    let analysis = CoverageAnalysis::new(schema, world)?;
    let renderability = analysis.baseline_renderability(language);
    let baseline = BTreeSet::new();

    // Task 030's single dependency model, reused rather than re-implemented.
    let closure = plan.selected_type_closure(schema)?;
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
        let declaration = schema
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
