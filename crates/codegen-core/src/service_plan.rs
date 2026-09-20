//! Language-neutral join of a portable Service Contract and normalized UCI IR.
//!
//! This module owns exactly one thing: the **join**. The Service Contract owns
//! service interface semantics (which functions exist, their grouping,
//! ownership, direction, mandate, topic, timing); the XSD frontend and
//! `SchemaIr` own message and type identity (which UCI messages exist, their
//! qualified names, payload types, and transitive type graphs). Neither
//! authority is moved: nothing here reinterprets contract fields, and nothing
//! here re-reads XSD syntax.
//!
//! It is deliberately placed in `codegen-core` rather than in a backend or in
//! the CLI, because the resolved plan is language-neutral: Ada, Rust, and C++
//! service generation (Task 031 and later) must all consume the same plan.
//!
//! It is also deliberately independent of [`crate::GenerationWorld`]. Whether
//! a contract is valid, and which UCI messages and types it selects, are facts
//! about the contract and the schema; whether a given backend can *render*
//! that selection under a closed or open world is a separate question.

use crate::MismatchRole;
use ams_gra_oms_ir::{MessageDecl, QualifiedName, SchemaIr, TypeDecl, TypeRef, TypeRefTarget};
use ams_gra_oms_service_contract::{
    Applicability, Capability, Contract, DataTransferExchange, Direction, Exchange,
    FunctionCategory, Mandate, NonOmsMessageExchange, OmsMessageExchange, RequiredGroup,
    SecurityExchange, ServiceKind, SpecialSignalExchange, StandardRole, Timing, TraceRef,
};
use std::collections::BTreeSet;
use std::fmt;

/// A failure while joining a contract to a schema set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServicePlanError {
    /// The contract named an OMS message that the supplied schema set does not
    /// declare. Resolution is exact local-name equality, so this is never a
    /// near-miss that could have been "helpfully" matched.
    UnknownOmsMessage {
        function: String,
        exchange: String,
        message: String,
    },
    /// The supplied schema set declares the same local message name in more
    /// than one namespace. Picking one would be arbitrary, so the candidates
    /// are reported and the plan fails.
    AmbiguousOmsMessage {
        function: String,
        exchange: String,
        message: String,
        candidates: Vec<QualifiedName>,
    },
    /// A resolved message's payload, or a type reached from it, names a type
    /// the schema set does not declare. This is a schema-set defect rather
    /// than a contract defect, but it is reported with contract context.
    UnresolvedPayloadType {
        message: QualifiedName,
        missing: QualifiedName,
    },
}

impl fmt::Display for ServicePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOmsMessage {
                function,
                exchange,
                message,
            } => write!(
                formatter,
                "function '{function}' exchange '{exchange}': OMS message '{message}' was not \
                 found in the supplied schema set"
            ),
            Self::AmbiguousOmsMessage {
                function,
                exchange,
                message,
                candidates,
            } => {
                write!(
                    formatter,
                    "function '{function}' exchange '{exchange}': OMS message '{message}' is \
                     ambiguous in the supplied schema set; candidates: "
                )?;
                for (index, candidate) in candidates.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    write!(
                        formatter,
                        "{{{}}}{}",
                        candidate.namespace_uri, candidate.local_name
                    )?;
                }
                Ok(())
            }
            Self::UnresolvedPayloadType { message, missing } => write!(
                formatter,
                "message {{{}}}{}: type {{{}}}{} is not declared in the supplied schema set",
                message.namespace_uri,
                message.local_name,
                missing.namespace_uri,
                missing.local_name
            ),
        }
    }
}

impl std::error::Error for ServicePlanError {}

/// Service identity, copied verbatim from the contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceIdentity {
    pub name: String,
    pub version: String,
    pub kind: ServiceKind,
    pub description: Option<String>,
}

/// Declared standards, including the logical UCI extension identifiers.
///
/// `uci_schema_version` is the contract's logical version (`"2.5"`), and
/// `schema_root_version` is whatever the supplied XSD root actually declared
/// in `xs:schema @version` (for UCI 2.5 that is the literal `"002.5.0"`).
/// Both are retained unmodified and are deliberately **not** compared: no
/// documented mapping between the two spellings exists, so inventing one by
/// string surgery would be a guess encoded as a rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceStandards {
    pub oms_version: String,
    pub uci_schema_version: String,
    pub uci_extension_schemas: Vec<String>,
    pub ams_gra_version: Option<String>,
    pub schema_root_version: Option<String>,
}

/// One Capability, preserved exactly as the contract declared it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityPlan {
    pub id: String,
    pub name: String,
    pub requires_position_information: bool,
}

/// One contract function, with its exchanges resolved where applicable.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionPlan {
    pub id: String,
    pub name: String,
    pub category: FunctionCategory,
    pub required_group: Option<RequiredGroup>,
    /// Capability ownership exactly as authored; never inferred.
    pub capability: Option<String>,
    pub standard_role: Option<StandardRole>,
    pub applicability: Applicability,
    pub not_applicable_reason: Option<String>,
    pub description: Option<String>,
    pub traceability: Vec<TraceRef>,
    /// Exchanges in contract order, including non-UCI kinds.
    pub exchanges: Vec<ResolvedExchange>,
}

/// One planned exchange.
///
/// Only the OMS Message variant carries resolved schema identity. The four
/// non-UCI kinds are preserved verbatim and are deliberately never resolved
/// against `SchemaIr`: a Data Transfer or Special Signal is a real part of the
/// service interface that simply is not a UCI message, so its absence from the
/// schema set is not an error.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedExchange {
    OmsMessage(ResolvedOmsMessageExchange),
    DataTransfer(DataTransferExchange),
    SpecialSignal(SpecialSignalExchange),
    SecurityExchange(SecurityExchange),
    NonOmsMessage(NonOmsMessageExchange),
}

impl ResolvedExchange {
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::OmsMessage(exchange) => &exchange.id,
            Self::DataTransfer(exchange) => &exchange.id,
            Self::SpecialSignal(exchange) => &exchange.id,
            Self::SecurityExchange(exchange) => &exchange.id,
            Self::NonOmsMessage(exchange) => &exchange.id,
        }
    }

    #[must_use]
    pub const fn kind_str(&self) -> &'static str {
        match self {
            Self::OmsMessage(_) => "oms_message",
            Self::DataTransfer(_) => "data_transfer",
            Self::SpecialSignal(_) => "special_signal",
            Self::SecurityExchange(_) => "security_exchange",
            Self::NonOmsMessage(_) => "non_oms_message",
        }
    }

    #[must_use]
    pub const fn direction(&self) -> Direction {
        match self {
            Self::OmsMessage(exchange) => exchange.direction,
            Self::DataTransfer(exchange) => exchange.direction,
            Self::SpecialSignal(exchange) => exchange.direction,
            Self::SecurityExchange(exchange) => exchange.direction,
            Self::NonOmsMessage(exchange) => exchange.direction,
        }
    }

    /// The OMS Message view, or `None` for the four non-UCI kinds.
    #[must_use]
    pub const fn as_oms_message(&self) -> Option<&ResolvedOmsMessageExchange> {
        match self {
            Self::OmsMessage(exchange) => Some(exchange),
            _ => None,
        }
    }
}

/// A contract OMS Message exchange joined to one normalized UCI message.
///
/// Every Service Contract field is retained alongside the resolved schema
/// identity, so nothing the author stated about direction, mandate, topic,
/// timing, or mapping is lost by resolution. Only normalized IR identities are
/// stored; no raw XSD syntax is copied into the plan.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedOmsMessageExchange {
    pub id: String,
    pub direction: Direction,
    pub mandate: Mandate,
    pub topic: String,
    pub timing: Timing,
    pub operational_attribute: Option<String>,
    pub subscription_group: Option<String>,
    pub appendix_c_mapping: Option<String>,
    pub traceability: Vec<TraceRef>,
    /// The local message name exactly as the contract wrote it.
    pub contract_message: String,
    /// The resolved, fully qualified normalized UCI message name.
    pub message_name: QualifiedName,
    /// The resolved message's normalized payload type reference.
    pub payload_type: TypeRef,
}

/// A language-neutral Resolved Service Plan.
///
/// This is the join of "what the contract uses" with "what the schema set
/// defines". It intentionally adds no policy of its own: no function is
/// reordered, no Capability is inferred, no Section 3.3 topology is
/// regenerated, and no generation world is consulted.
#[derive(Debug, Clone, PartialEq)]
pub struct ServicePlan {
    pub contract_version: String,
    pub service: ServiceIdentity,
    pub standards: ServiceStandards,
    /// The Capability inventory, preserving the contract's three distinct
    /// states: omitted (`None`), explicitly empty (`Some(vec![])`), and
    /// explicitly populated.
    pub capabilities: Option<Vec<CapabilityPlan>>,
    /// Functions in contract order. Never alphabetized.
    pub functions: Vec<FunctionPlan>,
    /// Unique selected UCI messages, in first-occurrence order.
    selected_messages: Vec<SelectedMessage>,
    /// Immutable semantic binding to the schema this plan was resolved
    /// against. Private: it is an implementation detail of mismatch
    /// detection, not part of the plan's published shape.
    binding: SchemaBinding,
}

/// A semantic snapshot of exactly the declarations a plan depends on.
///
/// # Why identities are not enough
///
/// The public library API accepts a [`ServicePlan`] and a [`SchemaIr`]
/// separately and explicitly attempts to diagnose wrong-schema reuse. A check
/// that compares only qualified names cannot do that: schema A and schema B
/// can declare the same message and type *names* while the payload a message
/// carries, a type's field list, or a field's constraints and cardinality all
/// differ. The plan would be silently applied to the wrong model.
///
/// This captures the declarations themselves, so verification is exact
/// semantic equality of the plan-relevant subset.
///
/// # Why this representation
///
/// Deliberately *not* pointer identity (the caller may legitimately rebuild an
/// equal schema), not `std::hash` or an ad-hoc integer digest (collisions
/// would silently accept a mismatch), and not a serialization (unstable, and
/// this crate has no serializer). Storing the declarations costs the closure's
/// size once, at resolution, and buys an exact answer with no false accept.
///
/// Scope is the selected service, so a change to an unrelated unselected
/// declaration does not invalidate the plan.
#[derive(Debug, Clone, PartialEq)]
struct SchemaBinding {
    /// Selected message declarations, in contract first-occurrence order.
    messages: Vec<MessageDecl>,
    /// The selected transitive named type closure, in schema declaration
    /// order.
    types: Vec<TypeDecl>,
}

/// Which selected identity failed semantic verification, and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanBindingMismatch {
    /// The supplied schema does not declare a selected identity at all.
    Missing {
        name: QualifiedName,
        role: MismatchRole,
    },
    /// The supplied schema declares the identity, but its declaration differs
    /// from the one the plan was resolved against.
    Changed {
        name: QualifiedName,
        role: MismatchRole,
    },
    /// The selected type closure itself differs: the supplied schema reaches a
    /// different set of declarations from the same selected messages.
    ClosureChanged,
}

impl fmt::Display for PlanBindingMismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { name, role } => write!(
                formatter,
                "{} {{{}}}{} is absent from the supplied schema set; the service plan was \
                 resolved against a different schema set",
                role.label(),
                name.namespace_uri,
                name.local_name
            ),
            Self::Changed { name, role } => write!(
                formatter,
                "{} {{{}}}{} is declared differently in the supplied schema set than in the one \
                 the service plan was resolved against",
                role.label(),
                name.namespace_uri,
                name.local_name
            ),
            Self::ClosureChanged => formatter.write_str(
                "the selected type closure differs from the one the service plan was resolved \
                 against; the plan was resolved against a different schema set",
            ),
        }
    }
}

impl std::error::Error for PlanBindingMismatch {}

/// One unique UCI message selected by the contract.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectedMessage {
    pub name: QualifiedName,
    pub payload_type: TypeRef,
    /// The local name(s) the contract used to select this message. Exactly one
    /// under exact local-name resolution, but kept as the authored spelling.
    pub contract_message: String,
}

impl ServicePlan {
    /// The unique UCI messages this contract selects, in first-occurrence order.
    ///
    /// Two exchanges may legitimately reference the same message with
    /// different topics or directions; both exchange occurrences are retained
    /// in [`ServicePlan::functions`], while this collection deduplicates them.
    /// Ordering is first occurrence in contract function/exchange order rather
    /// than a sorted-set order, so the author's presentation order survives
    /// into anything generated from the selection.
    #[must_use]
    pub fn selected_messages(&self) -> &[SelectedMessage] {
        &self.selected_messages
    }

    /// Verify that `schema` is semantically compatible with the schema this
    /// plan was resolved against.
    ///
    /// This is the **single** mismatch mechanism. Readiness analysis and
    /// selected-service generation projection both call it, so the two cannot
    /// drift into independently disagreeing notions of "wrong schema", and a
    /// mismatch is a typed error rather than a debug assertion or a panic on a
    /// failed lookup.
    ///
    /// Compatibility is exact semantic equality over the plan-relevant subset
    /// only: every selected message declaration, and every declaration in the
    /// selected transitive type closure. Two schema objects built separately
    /// but equal in that subset are compatible -- object identity is never
    /// required. Conversely, changing a selected message's payload type, a
    /// selected type's body, or a transitive dependency's constraints or
    /// cardinality is detected even though every qualified name is unchanged.
    ///
    /// Changes to declarations outside the selected closure do not invalidate
    /// the plan: a `ServicePlan` is selected-service scoped by construction.
    ///
    /// # Errors
    ///
    /// Returns the first [`PlanBindingMismatch`] in binding order: messages in
    /// contract first-occurrence order, then types in schema declaration
    /// order.
    pub fn verify_schema_binding(&self, schema: &SchemaIr) -> Result<(), PlanBindingMismatch> {
        for expected in &self.binding.messages {
            let actual = schema
                .messages
                .iter()
                .find(|message| message.name == expected.name)
                .ok_or_else(|| PlanBindingMismatch::Missing {
                    name: expected.name.clone(),
                    role: MismatchRole::Message,
                })?;
            if actual != expected {
                return Err(PlanBindingMismatch::Changed {
                    name: expected.name.clone(),
                    role: MismatchRole::Message,
                });
            }
        }
        for expected in &self.binding.types {
            let actual = schema
                .types
                .iter()
                .find(|declaration| declaration.name == expected.name)
                .ok_or_else(|| PlanBindingMismatch::Missing {
                    name: expected.name.clone(),
                    role: MismatchRole::TypeDeclaration,
                })?;
            if actual != expected {
                return Err(PlanBindingMismatch::Changed {
                    name: expected.name.clone(),
                    role: MismatchRole::TypeDeclaration,
                });
            }
        }
        // Every captured declaration matched, so re-walking the closure in the
        // supplied schema must reach exactly the captured set. A different
        // size means the supplied schema reaches declarations the plan never
        // saw -- possible when a message payload is a primitive in one schema
        // and named in another, which the per-declaration loop above cannot
        // observe.
        let closure = self
            .selected_type_closure(schema)
            .map_err(|_| PlanBindingMismatch::ClosureChanged)?;
        if closure.len() != self.binding.types.len() {
            return Err(PlanBindingMismatch::ClosureChanged);
        }
        Ok(())
    }

    /// Total number of exchange occurrences across all functions.
    #[must_use]
    pub fn exchange_occurrence_count(&self) -> usize {
        self.functions
            .iter()
            .map(|function| function.exchanges.len())
            .sum()
    }

    /// Number of OMS Message exchange occurrences (not unique messages).
    #[must_use]
    pub fn oms_message_exchange_count(&self) -> usize {
        self.functions
            .iter()
            .flat_map(|function| &function.exchanges)
            .filter(|exchange| exchange.as_oms_message().is_some())
            .count()
    }

    /// The transitive named type closure required by the selected messages.
    ///
    /// Starting from each selected message's `payload_type`, this follows the
    /// single shared semantic dependency model -- named base types, aliases,
    /// record fields, choice alternatives, and list item types -- and returns
    /// the reached declarations in **schema declaration order**, which is
    /// deterministic and independent of contract ordering.
    ///
    /// This answers "which normalized UCI type declarations does this Service
    /// Contract require?". It deliberately does **not** answer "can Ada, Rust,
    /// or C++ render all of them?": renderability is world- and
    /// backend-dependent and is Task 031's concern.
    ///
    /// # Errors
    ///
    /// Returns [`ServicePlanError::UnresolvedPayloadType`] if a reachable
    /// named type is not declared in the schema set.
    pub fn selected_type_closure<'schema>(
        &self,
        schema: &'schema SchemaIr,
    ) -> Result<Vec<&'schema TypeDecl>, ServicePlanError> {
        let mut selected = BTreeSet::new();
        for message in &self.selected_messages {
            let TypeRefTarget::Named(name) = &message.payload_type.target else {
                // A primitive payload pulls in no named declaration at all.
                continue;
            };
            visit_closure(schema, &message.name, name, &mut selected)?;
        }
        Ok(schema
            .types
            .iter()
            .filter(|declaration| selected.contains(&declaration.name))
            .collect())
    }
}

/// Depth-first accumulation of the named closure reachable from one type.
///
/// Cycles terminate naturally: a name already in `selected` is not revisited.
/// Whether cyclic declarations can be *emitted* is a separate backend question
/// handled by `plan_type_declarations`; closure membership is well defined
/// regardless.
fn visit_closure(
    schema: &SchemaIr,
    message: &QualifiedName,
    name: &QualifiedName,
    selected: &mut BTreeSet<QualifiedName>,
) -> Result<(), ServicePlanError> {
    if !selected.insert(name.clone()) {
        return Ok(());
    }
    let declaration = schema
        .types
        .iter()
        .find(|declaration| &declaration.name == name)
        .ok_or_else(|| ServicePlanError::UnresolvedPayloadType {
            message: message.clone(),
            missing: name.clone(),
        })?;
    for dependency in crate::direct_named_dependencies(declaration) {
        visit_closure(schema, message, dependency, selected)?;
    }
    Ok(())
}

/// Join a portable Service Contract to a normalized UCI schema set.
///
/// Contract order is authoritative throughout: functions, exchanges, and the
/// first-occurrence unique message selection all follow the contract. Schema
/// order is used only where the contract has nothing to say, namely the type
/// closure.
///
/// # Errors
///
/// Returns [`ServicePlanError`] if a contract OMS message is absent from, or
/// ambiguous within, the supplied schema set.
pub fn resolve_service_plan(
    contract: &Contract,
    schema: &SchemaIr,
) -> Result<ServicePlan, ServicePlanError> {
    let mut functions = Vec::with_capacity(contract.functions.len());
    let mut selected_messages: Vec<SelectedMessage> = Vec::new();

    for function in &contract.functions {
        let mut exchanges = Vec::with_capacity(function.exchanges.len());
        for exchange in &function.exchanges {
            let resolved = match exchange {
                Exchange::OmsMessage(oms) => {
                    let declaration = resolve_message(schema, &function.id, oms)?;
                    // Deduplicate on resolved identity, not on the authored
                    // spelling, and only append on first occurrence so the
                    // author's order is what a consumer sees.
                    if !selected_messages
                        .iter()
                        .any(|selected| selected.name == declaration.name)
                    {
                        selected_messages.push(SelectedMessage {
                            name: declaration.name.clone(),
                            payload_type: declaration.payload_type.clone(),
                            contract_message: oms.message.clone(),
                        });
                    }
                    ResolvedExchange::OmsMessage(ResolvedOmsMessageExchange {
                        id: oms.id.clone(),
                        direction: oms.direction,
                        mandate: oms.mandate,
                        topic: oms.topic.clone(),
                        timing: oms.timing.clone(),
                        operational_attribute: oms.operational_attribute.clone(),
                        subscription_group: oms.subscription_group.clone(),
                        appendix_c_mapping: oms.appendix_c_mapping.clone(),
                        traceability: oms.traceability.clone(),
                        contract_message: oms.message.clone(),
                        message_name: declaration.name.clone(),
                        payload_type: declaration.payload_type.clone(),
                    })
                }
                // The four non-UCI kinds pass through untouched. They are part
                // of the service interface and must survive planning, but they
                // are not UCI messages, so they are never looked up and their
                // absence from the schema set is never an error.
                Exchange::DataTransfer(transfer) => {
                    ResolvedExchange::DataTransfer(transfer.clone())
                }
                Exchange::SpecialSignal(signal) => ResolvedExchange::SpecialSignal(signal.clone()),
                Exchange::SecurityExchange(security) => {
                    ResolvedExchange::SecurityExchange(security.clone())
                }
                Exchange::NonOmsMessage(message) => {
                    ResolvedExchange::NonOmsMessage(message.clone())
                }
            };
            exchanges.push(resolved);
        }

        functions.push(FunctionPlan {
            id: function.id.clone(),
            name: function.name.clone(),
            category: function.category,
            required_group: function.required_group,
            capability: function.capability.clone(),
            standard_role: function.standard_role,
            applicability: function.applicability,
            not_applicable_reason: function.not_applicable_reason.clone(),
            description: function.description.clone(),
            traceability: function.traceability.clone(),
            exchanges,
        });
    }

    // Capture the semantic binding from the schema resolution actually used,
    // so later readiness/generation calls can verify they were handed a
    // compatible schema rather than merely one with the same names.
    let mut plan = ServicePlan {
        contract_version: contract.contract_version.clone(),
        service: ServiceIdentity {
            name: contract.service.name.clone(),
            version: contract.service.version.clone(),
            kind: contract.service.kind,
            description: contract.service.description.clone(),
        },
        standards: ServiceStandards {
            oms_version: contract.standards.oms_version.clone(),
            uci_schema_version: contract.standards.uci_schema_version.clone(),
            uci_extension_schemas: contract.standards.uci_extension_schemas.clone(),
            ams_gra_version: contract.standards.ams_gra_version.clone(),
            // Retained beside the contract's logical version, never compared.
            schema_root_version: schema.schema_version.clone(),
        },
        // `Option` in, `Option` out: omitted stays omitted, explicitly empty
        // stays explicitly empty.
        capabilities: contract.capabilities.as_ref().map(|capabilities| {
            capabilities
                .iter()
                .map(|capability: &Capability| CapabilityPlan {
                    id: capability.id.clone(),
                    name: capability.name.clone(),
                    requires_position_information: capability.requires_position_information,
                })
                .collect()
        }),
        functions,
        selected_messages,
        // Filled in immediately below; `selected_type_closure` needs the
        // assembled selection, so the binding cannot be built inline.
        binding: SchemaBinding {
            messages: Vec::new(),
            types: Vec::new(),
        },
    };

    plan.binding = SchemaBinding {
        messages: plan
            .selected_messages
            .iter()
            .map(|selected| {
                schema
                    .messages
                    .iter()
                    .find(|message| message.name == selected.name)
                    .cloned()
                    // Every selected identity came from this schema moments
                    // ago, so absence here is impossible rather than a user
                    // error; `UnresolvedPayloadType` names the identity if the
                    // schema is somehow inconsistent.
                    .ok_or_else(|| ServicePlanError::UnresolvedPayloadType {
                        message: selected.name.clone(),
                        missing: selected.name.clone(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?,
        types: plan
            .selected_type_closure(schema)?
            .into_iter()
            .cloned()
            .collect(),
    };
    Ok(plan)
}

/// Resolve one contract message name by exact local-name equality.
///
/// The matching rule is intentionally unforgiving: exact local-name equality,
/// with no case folding, prefix/suffix/substring matching, fuzzy scoring, or
/// first-match-wins. Zero matches fail, one match resolves, and more than one
/// fails as ambiguous with the candidates named. Guessing here would silently
/// bind a service to the wrong wire message.
fn resolve_message<'schema>(
    schema: &'schema SchemaIr,
    function: &str,
    exchange: &OmsMessageExchange,
) -> Result<&'schema MessageDecl, ServicePlanError> {
    let mut matches = schema
        .messages
        .iter()
        .filter(|message| message.name.local_name == exchange.message);
    let Some(first) = matches.next() else {
        return Err(ServicePlanError::UnknownOmsMessage {
            function: function.to_owned(),
            exchange: exchange.id.clone(),
            message: exchange.message.clone(),
        });
    };
    if matches.next().is_some() {
        let candidates = schema
            .messages
            .iter()
            .filter(|message| message.name.local_name == exchange.message)
            .map(|message| message.name.clone())
            .collect();
        return Err(ServicePlanError::AmbiguousOmsMessage {
            function: function.to_owned(),
            exchange: exchange.id.clone(),
            message: exchange.message.clone(),
            candidates,
        });
    }
    Ok(first)
}
