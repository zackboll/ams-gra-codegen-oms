//! Language-neutral service API model and its shared naming preflight.
//!
//! Task 030 produced a [`ServicePlan`], the join of a portable Service Contract
//! and normalized UCI IR. Task 032 generated the UCI **type model** a plan
//! selects. This module adds the next, strictly separate artifact: the typed
//! **endpoint surface** of the service itself.
//!
//! ```text
//! portable Contract
//!       |
//!       v
//!   ServicePlan                (Task 030: contract x schema join)
//!       |
//!       v
//!   ServiceApiModel            (Task 047: this module, lowered ONCE)
//!       |
//!       +----------+----------+
//!      Ada        Rust       C++   (backends render syntax only)
//! ```
//!
//! # What the model is
//!
//! One entry per contract function and one per exchange **occurrence**, in
//! contract order, carrying only the facts the generated endpoint descriptors
//! expose: identifiers, human function names, exchange kind, direction,
//! mandate, and -- for OMS Message exchanges only -- the topic plus the
//! resolved message and payload identities.
//!
//! It is deliberately **not** [`ServicePlan::selected_messages`]. That list is
//! deduplicated because it drives the type closure; here two exchanges that
//! select the same UCI message remain two endpoints, because they are two
//! different parts of the service interface.
//!
//! # What the model is not
//!
//! It is not a serialized mirror of the portable contract. Traceability,
//! operational attributes, subscription groups, Appendix C mappings, timing
//! parameters, Capability ownership, standard roles, descriptions, and the
//! kind-specific details of the four non-OMS exchange kinds all remain in the
//! [`ServicePlan`], unmodified, for a later façade/runtime task to consume.
//!
//! # Authority
//!
//! Nothing here re-resolves a message name, infers a topic or direction, or
//! regroups exchanges: every fact is copied from the plan, whose resolution is
//! authoritative. The only new check is that each OMS payload really is a
//! generated type in the **projected** model the backend will render.
//!
//! # Naming
//!
//! Generated scope names come from portable contract **IDs** only -- never
//! from human-readable names, topics, or message names -- behind a fixed
//! `function_` / `exchange_` (Ada: `Function_` / `Exchange_`) prefix, so a
//! valid portable ID can never become a bare host-language reserved word.
//! Distinct IDs that normalize to one host identifier (`foo-bar` and
//! `foo_bar`) fail closed; nothing is suffixed or merged.
//!
//! This naming analysis is intentionally separate from the Schema IR
//! [`crate::validate_backend_names`] preflight: schema names come from XSD,
//! wrapper names from contract IDs, and they occupy different scopes.

use crate::backend_layout::{BackendModelLayout, ModelUnit};
use crate::{BackendLanguage, GenerationWorld, ResolvedExchange, ServicePlan, plan_type_emissions};
use ams_gra_oms_ir::{PrimitiveKind, QualifiedName, SchemaIr, TypeRef, TypeRefTarget};
use ams_gra_oms_service_contract::{Direction, Mandate, ServiceKind};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// The typed endpoint surface of one service, lowered once from a plan.
///
/// Fields are private: the only constructor is [`build_service_api_model`], so
/// a backend receiving one may rely on its payload bindings having been
/// verified against the projected model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceApiModel {
    service_name: String,
    service_version: String,
    service_kind: ServiceKind,
    functions: Vec<ServiceApiFunction>,
    emits_type_model: bool,
    world: GenerationWorld,
}

impl ServiceApiModel {
    /// The contract's `service.name`, verbatim (display text, not a symbol).
    #[must_use]
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// The contract's `service.version`, verbatim.
    #[must_use]
    pub fn service_version(&self) -> &str {
        &self.service_version
    }

    /// The contract's `service.kind`.
    #[must_use]
    pub const fn service_kind(&self) -> ServiceKind {
        self.service_kind
    }

    /// Every contract function, in contract order.
    #[must_use]
    pub fn functions(&self) -> &[ServiceApiFunction] {
        &self.functions
    }

    /// Whether the projected schema carries any type declaration, i.e.
    /// whether ordinary type generation emits a model file the wrapper must
    /// reference. The CLI (invoke type generation?) and every wrapper
    /// renderer (import the model?) consult this one decision.
    #[must_use]
    pub const fn emits_type_model(&self) -> bool {
        self.emits_type_model
    }

    /// The generation world the payload bindings were verified under. The
    /// artifact preflight uses it to name the model's generated top-level
    /// declarations exactly as type generation would.
    #[must_use]
    pub const fn world(&self) -> GenerationWorld {
        self.world
    }
}

/// One contract function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceApiFunction {
    id: String,
    name: String,
    exchanges: Vec<ServiceApiExchange>,
}

impl ServiceApiFunction {
    /// The portable function ID. This, not [`Self::name`], drives the
    /// generated scope name.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The human-readable function name. Display metadata only.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Every exchange occurrence, in contract order, including all four
    /// non-OMS kinds. Never deduplicated.
    #[must_use]
    pub fn exchanges(&self) -> &[ServiceApiExchange] {
        &self.exchanges
    }
}

/// One exchange occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceApiExchange {
    id: String,
    direction: Direction,
    mandate: Mandate,
    kind: ServiceApiExchangeKind,
}

impl ServiceApiExchange {
    /// The portable exchange ID, unique within its function only.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.direction
    }

    #[must_use]
    pub const fn mandate(&self) -> Mandate {
        self.mandate
    }

    #[must_use]
    pub const fn kind(&self) -> &ServiceApiExchangeKind {
        &self.kind
    }

    /// The OMS payload binding, or `None` for the four non-UCI kinds.
    #[must_use]
    pub const fn oms_binding(&self) -> Option<&ServiceApiOmsBinding> {
        match &self.kind {
            ServiceApiExchangeKind::OmsMessage(binding) => Some(binding),
            _ => None,
        }
    }
}

/// The five portable exchange kinds.
///
/// Only [`Self::OmsMessage`] carries a payload binding. The other four are
/// real parts of the service interface that are simply not UCI messages, so no
/// payload is fabricated for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceApiExchangeKind {
    OmsMessage(ServiceApiOmsBinding),
    DataTransfer,
    SpecialSignal,
    SecurityExchange,
    NonOmsMessage,
}

impl ServiceApiExchangeKind {
    /// The portable `kind` spelling, emitted verbatim as the `KIND` constant.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::OmsMessage(_) => "oms_message",
            Self::DataTransfer => "data_transfer",
            Self::SpecialSignal => "special_signal",
            Self::SecurityExchange => "security_exchange",
            Self::NonOmsMessage => "non_oms_message",
        }
    }
}

/// The resolved UCI identity behind one OMS Message exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceApiOmsBinding {
    topic: String,
    message_name: QualifiedName,
    payload_type: TypeRef,
    payload_name: QualifiedName,
}

impl ServiceApiOmsBinding {
    /// The authored topic, verbatim. Never inferred.
    #[must_use]
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// The resolved, fully qualified UCI message identity.
    #[must_use]
    pub const fn message_name(&self) -> &QualifiedName {
        &self.message_name
    }

    /// The resolved payload type, exactly as the plan resolved it. Always
    /// [`TypeRefTarget::Named`] and always a generated top-level type of the
    /// projected model: [`build_service_api_model`] rejects anything else.
    #[must_use]
    pub const fn payload_type(&self) -> &TypeRef {
        &self.payload_type
    }

    /// The payload's qualified type name: the `Named` target of
    /// [`Self::payload_type`], extracted once at lowering time so renderers
    /// never have to handle a primitive case that cannot occur.
    #[must_use]
    pub const fn payload_name(&self) -> &QualifiedName {
        &self.payload_name
    }
}

/// Why an OMS exchange's payload cannot be bound to a generated model type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnboundPayloadReason {
    /// The payload is a primitive value. Selected type generation emits no
    /// declaration for it, so no generated model type exists for a `Payload`
    /// alias to name, and none is invented.
    Primitive(PrimitiveKind),
    /// The projected schema does not declare the resolved message, or
    /// declares it with a different payload than the plan resolved.
    MessageNotProjected,
    /// The payload names a declaration the projected model does not emit as
    /// its own generated top-level type.
    NotGenerated(QualifiedName),
}

/// One OMS exchange whose payload has no generated model type to bind to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnboundPayload {
    pub function: String,
    pub exchange: String,
    pub message: QualifiedName,
    pub reason: UnboundPayloadReason,
}

/// A failure to produce, or to name, the service API for one service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceApiError {
    /// An OMS exchange's payload has no generated model type to bind to.
    ///
    /// Boxed so every `Result` carrying this error stays small.
    UnboundPayload(Box<UnboundPayload>),
    /// Type emission planning over the projected schema failed. Projection
    /// already ran the same planner, so this is an integrity defect.
    EmissionPlan(String),
    /// A generated wrapper name cannot be emitted safely in one language.
    Name(ServiceApiNameError),
    /// The wrapper entrypoint and a model artifact would be written to the
    /// same output path.
    ///
    /// Boxed so every `Result` carrying this error stays small.
    ArtifactPathCollision(Box<ServiceApiArtifactCollision>),
    /// A model-generated entity and a wrapper-generated name conflict in a
    /// host-language declarative region the two artifacts genuinely share.
    ///
    /// Boxed so every `Result` carrying this error stays small.
    ModelWrapperNameCollision(Box<ServiceApiModelNameCollision>),
    /// The model artifact layout could not be derived from the projected
    /// schema. Backend preflight already validated the same namespace with
    /// the same rules, so this is an integrity defect, never an ordinary
    /// NOT READY.
    ModelLayout(String),
}

impl fmt::Display for ServiceApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnboundPayload(unbound) => {
                let UnboundPayload {
                    function,
                    exchange,
                    message,
                    reason,
                } = unbound.as_ref();
                write!(
                    formatter,
                    "function '{function}' exchange '{exchange}': OMS message {{{}}}{} ",
                    message.namespace_uri, message.local_name
                )?;
                match reason {
                    UnboundPayloadReason::Primitive(kind) => write!(
                        formatter,
                        "has primitive payload {kind:?}, which has no generated model type to \
                         bind as Payload"
                    ),
                    UnboundPayloadReason::MessageNotProjected => formatter
                        .write_str("is not carried unchanged by the projected selected schema"),
                    UnboundPayloadReason::NotGenerated(payload) => write!(
                        formatter,
                        "has payload {{{}}}{}, which the projected model does not generate as \
                         a type",
                        payload.namespace_uri, payload.local_name
                    ),
                }
            }
            Self::EmissionPlan(message) => write!(
                formatter,
                "service API payload binding could not plan the projected model: {message}"
            ),
            Self::Name(error) => error.fmt(formatter),
            Self::ArtifactPathCollision(collision) => collision.fmt(formatter),
            Self::ModelWrapperNameCollision(collision) => collision.fmt(formatter),
            Self::ModelLayout(message) => write!(
                formatter,
                "service API artifact preflight could not derive the model layout: {message}"
            ),
        }
    }
}

impl std::error::Error for ServiceApiError {}

impl From<ServiceApiNameError> for ServiceApiError {
    fn from(error: ServiceApiNameError) -> Self {
        Self::Name(error)
    }
}

/// Lower a resolved plan to its language-neutral service API model.
///
/// `projected_schema` must be the schema selected-service generation hands to
/// the backend (the [`crate::ServiceGenerationProjection`] schema), and `world`
/// the world it is generated under. The plan's resolved message and payload
/// identities are used verbatim and are never looked up by name again; each
/// OMS payload is only *verified* to be a named declaration the projected
/// model emits as its own generated type.
///
/// Contract order is preserved exactly, every exchange occurrence is kept
/// (none is deduplicated), and all four non-OMS kinds are represented without
/// a fabricated payload.
///
/// # Errors
///
/// Returns [`ServiceApiError::UnboundPayload`] when an OMS payload is a
/// primitive, when the projected schema does not carry the resolved message
/// unchanged, or when the payload is not a generated type there; and
/// [`ServiceApiError::EmissionPlan`] if the projected model cannot be planned.
pub fn build_service_api_model(
    plan: &ServicePlan,
    projected_schema: &SchemaIr,
    world: GenerationWorld,
) -> Result<ServiceApiModel, ServiceApiError> {
    let emits_type_model = !projected_schema.types.is_empty();
    // The planner is consulted only when a model exists: an empty projection
    // (a service with zero OMS exchanges) needs no planner.
    let generated = if emits_type_model {
        plan_type_emissions(projected_schema, world)
            .map_err(|error| ServiceApiError::EmissionPlan(error.message))?
            .iter()
            .filter(|emission| emission.emits_own_top_level_name())
            .map(|emission| emission.name().clone())
            .collect::<BTreeSet<_>>()
    } else {
        BTreeSet::new()
    };

    let mut functions = Vec::with_capacity(plan.functions.len());
    for function in &plan.functions {
        let mut exchanges = Vec::with_capacity(function.exchanges.len());
        for exchange in &function.exchanges {
            let kind = match exchange {
                ResolvedExchange::OmsMessage(oms) => {
                    let unbound = |reason| {
                        ServiceApiError::UnboundPayload(Box::new(UnboundPayload {
                            function: function.id.clone(),
                            exchange: oms.id.clone(),
                            message: oms.message_name.clone(),
                            reason,
                        }))
                    };
                    // Identity AND payload must match: the plan's resolution
                    // is authoritative, and this only confirms the projected
                    // schema still carries exactly what was resolved.
                    let carried = projected_schema.messages.iter().any(|message| {
                        message.name == oms.message_name && message.payload_type == oms.payload_type
                    });
                    if !carried {
                        return Err(unbound(UnboundPayloadReason::MessageNotProjected));
                    }
                    let payload_name = match &oms.payload_type.target {
                        TypeRefTarget::Primitive(kind) => {
                            return Err(unbound(UnboundPayloadReason::Primitive(*kind)));
                        }
                        TypeRefTarget::Named(name) if !generated.contains(name) => {
                            return Err(unbound(UnboundPayloadReason::NotGenerated(name.clone())));
                        }
                        TypeRefTarget::Named(name) => name.clone(),
                    };
                    ServiceApiExchangeKind::OmsMessage(ServiceApiOmsBinding {
                        topic: oms.topic.clone(),
                        message_name: oms.message_name.clone(),
                        payload_type: oms.payload_type.clone(),
                        payload_name,
                    })
                }
                ResolvedExchange::DataTransfer(_) => ServiceApiExchangeKind::DataTransfer,
                ResolvedExchange::SpecialSignal(_) => ServiceApiExchangeKind::SpecialSignal,
                ResolvedExchange::SecurityExchange(_) => ServiceApiExchangeKind::SecurityExchange,
                ResolvedExchange::NonOmsMessage(_) => ServiceApiExchangeKind::NonOmsMessage,
            };
            exchanges.push(ServiceApiExchange {
                id: exchange.id().to_owned(),
                direction: exchange.direction(),
                mandate: exchange.mandate(),
                kind,
            });
        }
        functions.push(ServiceApiFunction {
            id: function.id.clone(),
            name: function.name.clone(),
            exchanges,
        });
    }

    Ok(ServiceApiModel {
        service_name: plan.service.name.clone(),
        service_version: plan.service.version.clone(),
        service_kind: plan.service.kind,
        functions,
        emits_type_model,
        world,
    })
}

// ---------------------------------------------------------------------------
// Naming
// ---------------------------------------------------------------------------

/// The fixed, contract-independent identifiers one language's wrapper emits.
///
/// Renderers read their spellings from here instead of hard-coding them, so
/// the names the preflight checks are exactly the names that are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceApiFixedNames {
    /// The wrapper entrypoint file, written into the same output directory
    /// as the model files. The artifact preflight checks it against the
    /// shared [`crate::BackendModelLayout`].
    pub file: &'static str,
    /// The wrapper's root module / namespace / package.
    pub root: &'static str,
    /// The Rust module the selected model file is mounted as. `None` for C++
    /// and Ada, which reference the model by its own namespace/package.
    pub model_module: Option<&'static str>,
    pub service_name: &'static str,
    pub service_version: &'static str,
    pub service_kind: &'static str,
    pub id: &'static str,
    pub name: &'static str,
    pub kind: &'static str,
    pub direction: &'static str,
    pub mandate: &'static str,
    pub topic: &'static str,
    pub payload: &'static str,
}

/// The fixed wrapper identifiers for `language`.
#[must_use]
pub const fn service_api_fixed_names(language: BackendLanguage) -> ServiceApiFixedNames {
    match language {
        BackendLanguage::Rust => ServiceApiFixedNames {
            file: "service_api.rs",
            root: "service_api",
            model_module: Some("model"),
            service_name: "SERVICE_NAME",
            service_version: "SERVICE_VERSION",
            service_kind: "SERVICE_KIND",
            id: "ID",
            name: "NAME",
            kind: "KIND",
            direction: "DIRECTION",
            mandate: "MANDATE",
            topic: "TOPIC",
            payload: "Payload",
        },
        BackendLanguage::Cpp => ServiceApiFixedNames {
            file: "service_api.hpp",
            root: "service_api",
            model_module: None,
            service_name: "service_name",
            service_version: "service_version",
            service_kind: "service_kind",
            id: "id",
            name: "name",
            kind: "kind",
            direction: "direction",
            mandate: "mandate",
            topic: "topic",
            payload: "Payload",
        },
        BackendLanguage::Ada => ServiceApiFixedNames {
            file: "service_api.ads",
            root: "Service_API",
            model_module: None,
            service_name: "Service_Name",
            service_version: "Service_Version",
            service_kind: "Service_Kind",
            id: "Id",
            name: "Name",
            kind: "Kind",
            direction: "Direction",
            mandate: "Mandate",
            topic: "Topic",
            payload: "Payload",
        },
    }
}

/// Which generated declarative region a wrapper name occupies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceApiRegion {
    /// The generated file's top level.
    File,
    /// Directly inside the wrapper root: service constants and function
    /// scopes.
    Service,
    /// Inside one function scope: its constants and exchange scopes.
    Function(String),
    /// Inside one exchange scope: its constants and `Payload`.
    Exchange { function: String, exchange: String },
}

impl fmt::Display for ServiceApiRegion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File => formatter.write_str("service API file scope"),
            Self::Service => formatter.write_str("service API root scope"),
            Self::Function(function) => write!(formatter, "scope of function '{function}'"),
            Self::Exchange { function, exchange } => write!(
                formatter,
                "scope of function '{function}' exchange '{exchange}'"
            ),
        }
    }
}

/// What produced one generated wrapper name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceApiNameOwner {
    /// A fixed wrapper identifier such as `SERVICE_NAME` or `Payload`.
    Fixed(&'static str),
    /// A function scope, derived from this portable function ID.
    Function(String),
    /// An exchange scope, derived from this portable exchange ID.
    Exchange { function: String, exchange: String },
}

impl fmt::Display for ServiceApiNameOwner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixed(name) => write!(formatter, "fixed name '{name}'"),
            Self::Function(id) => write!(formatter, "function id '{id}'"),
            Self::Exchange { function, exchange } => {
                write!(formatter, "function '{function}' exchange id '{exchange}'")
            }
        }
    }
}

/// A generated service API name that cannot be emitted safely.
///
/// A contract that triggers this is still a **valid portable contract**: the
/// failure is a statement about one target language, never about the
/// contract, which is why no naming rule lives in the contract crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceApiNameError {
    /// The ID is outside the portable identifier grammar, so no scope name is
    /// derived from it. Unreachable from a portable-valid contract; kept
    /// because `ServicePlan` is a public library type.
    InvalidIdentifier {
        language: BackendLanguage,
        owner: ServiceApiNameOwner,
    },
    /// The generated identifier is a reserved word of the target language.
    ReservedWord {
        language: BackendLanguage,
        owner: ServiceApiNameOwner,
        generated: String,
    },
    /// Two distinct names normalize to one identifier in one region.
    ///
    /// Boxed so the whole error stays small: it is returned by value from
    /// every preflight call, including ones that succeed.
    Collision(Box<ServiceApiCollision>),
}

/// Two distinct names that normalize to one identifier in one region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceApiCollision {
    pub language: BackendLanguage,
    pub region: ServiceApiRegion,
    pub generated: String,
    /// The name registered first, in contract order.
    pub first: ServiceApiNameOwner,
    /// The later name that collided with it.
    pub second: ServiceApiNameOwner,
}

impl fmt::Display for ServiceApiNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier { language, owner } => write!(
                formatter,
                "{} service API cannot form a scope name from {owner}: it is not a portable \
                 identifier",
                language.name()
            ),
            Self::ReservedWord {
                language,
                owner,
                generated,
            } => write!(
                formatter,
                "{} service API name for {owner} generates reserved word '{generated}'",
                language.name()
            ),
            Self::Collision(collision) => {
                let ServiceApiCollision {
                    language,
                    region,
                    generated,
                    first,
                    second,
                } = collision.as_ref();
                write!(
                    formatter,
                    "{} service API names for {first} and {second} both generate '{generated}' \
                     in the {region}; normalization collisions fail closed",
                    language.name()
                )
            }
        }
    }
}

impl std::error::Error for ServiceApiNameError {}

/// The words of one portable ID, or `None` outside the portable grammar
/// `^[a-z][a-z0-9]*(?:[-_][a-z0-9]+)*$`.
///
/// Rechecked here rather than trusted: a scope name must never be derived from
/// an unchecked string, and [`ServicePlan`] is a public library type.
fn portable_id_words(id: &str) -> Option<Vec<&str>> {
    if !id.as_bytes().first()?.is_ascii_lowercase() {
        return None;
    }
    let words = id.split(['-', '_']).collect::<Vec<_>>();
    words
        .iter()
        .all(|word| {
            !word.is_empty()
                && word
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
        .then_some(words)
}

/// The single shared, deterministic scope-name rule.
///
/// Rust and C++: `{prefix}` then each ID word, joined by `_`. Ada: the same
/// shape with every word's first character upper-cased. Both `-` and `_` map
/// to `_`, which is exactly why distinct portable IDs can collide and must be
/// checked. The fixed prefix guarantees the result is never a bare reserved
/// word, so no Task 044-style escape is needed or used.
fn scope_name(language: BackendLanguage, prefix: &str, id: &str) -> Option<String> {
    let words = portable_id_words(id)?;
    let mut name = match language {
        BackendLanguage::Rust | BackendLanguage::Cpp => prefix.to_owned(),
        BackendLanguage::Ada => capitalize(prefix),
    };
    for word in words {
        name.push('_');
        match language {
            BackendLanguage::Rust | BackendLanguage::Cpp => name.push_str(word),
            BackendLanguage::Ada => name.push_str(&capitalize(word)),
        }
    }
    Some(name)
}

fn capitalize(word: &str) -> String {
    let mut characters = word.chars();
    characters.next().map_or_else(String::new, |first| {
        let mut capitalized = first.to_ascii_uppercase().to_string();
        capitalized.push_str(characters.as_str());
        capitalized
    })
}

/// The generated scope name for one contract function.
///
/// # Errors
///
/// Returns [`ServiceApiNameError::InvalidIdentifier`] for an ID outside the
/// portable grammar.
pub fn service_api_function_scope_name(
    language: BackendLanguage,
    function_id: &str,
) -> Result<String, ServiceApiNameError> {
    scope_name(language, "function", function_id).ok_or_else(|| {
        ServiceApiNameError::InvalidIdentifier {
            language,
            owner: ServiceApiNameOwner::Function(function_id.to_owned()),
        }
    })
}

/// The generated scope name for one exchange, nested in its function scope.
///
/// # Errors
///
/// Returns [`ServiceApiNameError::InvalidIdentifier`] for an ID outside the
/// portable grammar.
pub fn service_api_exchange_scope_name(
    language: BackendLanguage,
    function_id: &str,
    exchange_id: &str,
) -> Result<String, ServiceApiNameError> {
    scope_name(language, "exchange", exchange_id).ok_or_else(|| {
        ServiceApiNameError::InvalidIdentifier {
            language,
            owner: ServiceApiNameOwner::Exchange {
                function: function_id.to_owned(),
                exchange: exchange_id.to_owned(),
            },
        }
    })
}

/// One declarative region's claimed identifiers, keyed by host identity.
struct ApiRegion {
    language: BackendLanguage,
    region: ServiceApiRegion,
    taken: BTreeMap<String, ServiceApiNameOwner>,
}

impl ApiRegion {
    const fn new(language: BackendLanguage, region: ServiceApiRegion) -> Self {
        Self {
            language,
            region,
            taken: BTreeMap::new(),
        }
    }

    fn claim(
        &mut self,
        owner: ServiceApiNameOwner,
        generated: String,
    ) -> Result<(), ServiceApiNameError> {
        if crate::backend_names::is_reserved(self.language, &generated) {
            return Err(ServiceApiNameError::ReservedWord {
                language: self.language,
                owner,
                generated,
            });
        }
        // Ada identifiers are case-insensitive; Rust and C++ are not.
        let key = match self.language {
            BackendLanguage::Ada => generated.to_ascii_lowercase(),
            BackendLanguage::Rust | BackendLanguage::Cpp => generated.clone(),
        };
        if let Some(first) = self.taken.get(&key) {
            return Err(ServiceApiNameError::Collision(Box::new(
                ServiceApiCollision {
                    language: self.language,
                    region: self.region.clone(),
                    generated,
                    first: first.clone(),
                    second: owner,
                },
            )));
        }
        self.taken.insert(key, owner);
        Ok(())
    }

    fn claim_fixed(&mut self, names: &[&'static str]) -> Result<(), ServiceApiNameError> {
        for name in names {
            self.claim(ServiceApiNameOwner::Fixed(name), (*name).to_owned())?;
        }
        Ok(())
    }
}

/// One function's naming inputs: its ID and each exchange's `(id, is_oms)`.
type NamingFunction<'a> = (&'a str, Vec<(&'a str, bool)>);

/// The single naming analysis every public entry point delegates to.
///
/// Regions mirror the generated nesting exactly: file scope, the wrapper
/// root, one region per function scope, one per exchange scope. Checks run in
/// contract order, so the first reported error is deterministic.
fn validate_names<'a>(
    language: BackendLanguage,
    functions: impl IntoIterator<Item = NamingFunction<'a>>,
) -> Result<(), ServiceApiNameError> {
    let fixed = service_api_fixed_names(language);

    let mut file = ApiRegion::new(language, ServiceApiRegion::File);
    file.claim_fixed(&[fixed.root])?;
    if let Some(model) = fixed.model_module {
        file.claim_fixed(&[model])?;
    }

    let mut service = ApiRegion::new(language, ServiceApiRegion::Service);
    service.claim_fixed(&[
        fixed.service_name,
        fixed.service_version,
        fixed.service_kind,
    ])?;

    for (function_id, exchanges) in functions {
        service.claim(
            ServiceApiNameOwner::Function(function_id.to_owned()),
            service_api_function_scope_name(language, function_id)?,
        )?;

        let mut function =
            ApiRegion::new(language, ServiceApiRegion::Function(function_id.to_owned()));
        function.claim_fixed(&[fixed.id, fixed.name])?;
        for (exchange_id, is_oms) in exchanges {
            function.claim(
                ServiceApiNameOwner::Exchange {
                    function: function_id.to_owned(),
                    exchange: exchange_id.to_owned(),
                },
                service_api_exchange_scope_name(language, function_id, exchange_id)?,
            )?;

            let mut exchange = ApiRegion::new(
                language,
                ServiceApiRegion::Exchange {
                    function: function_id.to_owned(),
                    exchange: exchange_id.to_owned(),
                },
            );
            exchange.claim_fixed(&[fixed.id, fixed.kind, fixed.direction, fixed.mandate])?;
            if is_oms {
                exchange.claim_fixed(&[fixed.topic, fixed.payload])?;
            }
        }
    }
    Ok(())
}

/// Check every name one language's wrapper would emit for `model`.
///
/// Every backend calls this again immediately before rendering, so a caller
/// that bypassed readiness still fails closed rather than emitting source that
/// cannot compile.
///
/// # Errors
///
/// Returns the first [`ServiceApiNameError`] in contract order.
pub fn validate_service_api_names(
    model: &ServiceApiModel,
    language: BackendLanguage,
) -> Result<(), ServiceApiNameError> {
    validate_names(
        language,
        model.functions.iter().map(|function| {
            (
                function.id.as_str(),
                function
                    .exchanges
                    .iter()
                    .map(|exchange| (exchange.id.as_str(), exchange.oms_binding().is_some()))
                    .collect(),
            )
        }),
    )
}

/// Check every wrapper name directly from a plan.
///
/// Names depend only on contract IDs and exchange kinds, so they can be
/// checked even when no projected schema exists (for example when an abstract
/// value already made the selection NOT READY). Uses exactly the same
/// analysis as [`validate_service_api_names`].
///
/// # Errors
///
/// Returns the first [`ServiceApiNameError`] in contract order.
pub fn validate_service_plan_api_names(
    plan: &ServicePlan,
    language: BackendLanguage,
) -> Result<(), ServiceApiNameError> {
    validate_names(
        language,
        plan.functions.iter().map(|function| {
            (
                function.id.as_str(),
                function
                    .exchanges
                    .iter()
                    .map(|exchange| (exchange.id(), exchange.as_oms_message().is_some()))
                    .collect(),
            )
        }),
    )
}

/// The complete service API preflight for one language: lower the model
/// (verifying every payload binding), check every generated wrapper name,
/// then check that the wrapper and the type model can be emitted together
/// ([`validate_service_api_artifacts`]).
///
/// Readiness consults this, so READY means `service-generate` can write the
/// complete artifact set -- model files and wrapper -- without a path
/// collision or a model/wrapper name conflict.
///
/// # Errors
///
/// Returns the first [`ServiceApiError`] found.
pub fn service_api_preflight(
    plan: &ServicePlan,
    projected_schema: &SchemaIr,
    language: BackendLanguage,
    world: GenerationWorld,
) -> Result<ServiceApiModel, ServiceApiError> {
    let model = build_service_api_model(plan, projected_schema, world)?;
    validate_service_api_names(&model, language)?;
    validate_service_api_artifacts(&model, projected_schema, language)?;
    Ok(model)
}
/// Check that `model`'s wrapper can be emitted beside the type model that
/// type generation writes for `projected_schema`, in `language`.
///
/// Every model-side fact comes from the shared [`BackendModelLayout`] and the
/// shared generated-name registration, i.e. from the same functions the
/// backends render with; nothing here re-derives a file or unit name.
///
/// Checked, in order:
///
/// 1. **Paths.** The wrapper file must not be any model artifact path,
///    including the optional Ada body.
/// 2. **C++ shared namespaces.** Only when the model namespace path and the
///    wrapper namespace path actually coincide are their contents compared.
///    Two namespaces with one name are a legal reopening and are descended
///    into; any other pairing of kinds is a redeclaration. A model entity
///    named `std` in a shared region would hide `::std` from the wrapper's
///    `std::string_view` constants.
/// 3. **Ada visibility at each `Payload`.** The wrapper names the model as
///    `Outer.Inner.Type` from inside `Service_API.Function_*.Exchange_*`,
///    where direct visibility finds the innermost homograph first. Any
///    wrapper declaration already visible there and spelled `Outer`
///    (case-insensitively) hides the model package.
///
/// Rust needs only the path check: the model is mounted as the fixed module
/// `model` and referenced by a `super`-relative path, so no URI-derived
/// identifier ever enters a wrapper scope.
///
/// A service with no type model has no model artifacts and passes
/// trivially.
///
/// # Errors
///
/// [`ServiceApiError::ArtifactPathCollision`],
/// [`ServiceApiError::ModelWrapperNameCollision`], or
/// [`ServiceApiError::ModelLayout`] when the model layout itself cannot be
/// derived.
pub fn validate_service_api_artifacts(
    model: &ServiceApiModel,
    projected_schema: &SchemaIr,
    language: BackendLanguage,
) -> Result<(), ServiceApiError> {
    if !model.emits_type_model {
        return Ok(());
    }
    let layout = BackendModelLayout::for_schema(projected_schema, language)
        .map_err(|error| ServiceApiError::ModelLayout(error.message))?;
    check_artifact_paths(&layout)?;
    match &layout.unit {
        ModelUnit::RustFile => Ok(()),
        ModelUnit::CppNamespace(namespace) => check_cpp_scopes(model, namespace, || {
            crate::backend_names::top_level_generated_names(
                projected_schema,
                BackendLanguage::Cpp,
                model.world,
            )
            .map_err(|error| ServiceApiError::ModelLayout(error.to_string()))
        }),
        ModelUnit::AdaPackage(package) => check_ada_payload_visibility(model, package),
    }
}

/// The wrapper entrypoint must not be any model artifact path, including an
/// optional one.
///
/// Every generated path is lowercase by construction; the comparison is
/// case-insensitive anyway so a case-insensitive filesystem can never be the
/// one to discover a collision.
fn check_artifact_paths(layout: &BackendModelLayout) -> Result<(), ServiceApiError> {
    let file = service_api_fixed_names(layout.language).file;
    match layout
        .artifacts
        .iter()
        .find(|artifact| artifact.relative_path.eq_ignore_ascii_case(file))
    {
        Some(artifact) => Err(ServiceApiError::ArtifactPathCollision(Box::new(
            ServiceApiArtifactCollision {
                language: layout.language,
                path: artifact.relative_path.clone(),
                model_artifact_optional: artifact.optional,
            },
        ))),
        None => Ok(()),
    }
}

/// The C++ combined-structure walk described on
/// [`validate_service_api_artifacts`].
///
/// The model occupies exactly `::outer`, `::outer::inner`, and the top level
/// of `::outer::inner`. The wrapper occupies `::service_api`, its constants
/// and function namespaces, each function's constants and exchange
/// namespaces, and each exchange's constants and `Payload`. Regions are
/// compared only where both paths reach them; `model_top_level` is computed
/// only if the walk gets that deep.
fn check_cpp_scopes(
    model: &ServiceApiModel,
    namespace: &[String; 2],
    model_top_level: impl FnOnce() -> Result<Vec<String>, ServiceApiError>,
) -> Result<(), ServiceApiError> {
    const LANGUAGE: BackendLanguage = BackendLanguage::Cpp;
    let fixed = service_api_fixed_names(LANGUAGE);
    let [outer, inner] = namespace;
    // Global scope: `namespace outer` vs `namespace service_api`. Distinct
    // names share nothing below the global scope, which is the normal case.
    if outer != fixed.root {
        return Ok(());
    }
    let root_scope = format!("::{}", fixed.root);
    let collision = |scope: &str, generated: &str, model_entity, wrapper, region, conflict| {
        ServiceApiError::ModelWrapperNameCollision(Box::new(ServiceApiModelNameCollision {
            language: LANGUAGE,
            scope: scope.to_owned(),
            generated: generated.to_owned(),
            model: model_entity,
            wrapper,
            region,
            conflict,
        }))
    };

    // `::service_api` is shared. The model declares only `namespace inner`
    // here; the wrapper declares three constants and one namespace per
    // function.
    //
    // Unreachable from any schema today: `outer` is one URI component and
    // never contains `_`, while the root does (asserted by
    // `model_paths_cannot_contain_the_wrapper_file_by_construction`). The
    // walk exists so the verdict never silently depends on that spelling.
    let model_namespace = ServiceApiModelEntity::CppNamespace(format!("::{outer}::{inner}"));
    // A model `namespace std` here would capture the wrapper's unqualified
    // `std::string_view`, first written by the `service_name` constant.
    if inner == "std" {
        return Err(collision(
            &root_scope,
            inner,
            model_namespace,
            ServiceApiNameOwner::Fixed(fixed.service_name),
            ServiceApiRegion::Service,
            ServiceApiModelConflict::Hides {
                referenced: "::std".to_owned(),
            },
        ));
    }
    for constant in [
        fixed.service_name,
        fixed.service_version,
        fixed.service_kind,
    ] {
        if inner == constant {
            return Err(collision(
                &root_scope,
                inner,
                model_namespace,
                ServiceApiNameOwner::Fixed(constant),
                ServiceApiRegion::Service,
                ServiceApiModelConflict::Redeclaration,
            ));
        }
    }
    let mut shared_function = None;
    for function in &model.functions {
        let scope = service_api_function_scope_name(LANGUAGE, &function.id)?;
        if *inner == scope {
            // namespace + namespace: a legal reopening. Descend.
            shared_function = Some(function);
            break;
        }
    }
    let Some(function) = shared_function else {
        return Ok(());
    };

    // `::service_api::function_x` is shared. The model declares its
    // top-level types here; the wrapper declares the `id` and `name`
    // constants and one namespace per exchange.
    //
    // Classified against real compiler behaviour (g++ 14, -std=c++17):
    //
    // * a model type vs an exchange namespace is a redeclaration error;
    // * a model class/enum vs a constant is legal C++, but the constant then
    //   hides the model type (and an alias or template of that name is a
    //   hard error), so it is rejected as hiding;
    // * a model entity named `std` breaks the wrapper's `std::string_view`.
    //
    // Every C++ model top-level name is UpperCamel and every wrapper name
    // here starts lowercase, so none of this is reachable from a real
    // schema; it is kept so the verdict never rests on that spelling rule.
    let function_scope = format!("::{outer}::{inner}");
    let function_region = ServiceApiRegion::Function(function.id.clone());
    let mut wrapper_names: Vec<(String, ServiceApiNameOwner, bool)> = vec![
        (
            fixed.id.to_owned(),
            ServiceApiNameOwner::Fixed(fixed.id),
            false,
        ),
        (
            fixed.name.to_owned(),
            ServiceApiNameOwner::Fixed(fixed.name),
            false,
        ),
    ];
    for exchange in &function.exchanges {
        wrapper_names.push((
            service_api_exchange_scope_name(LANGUAGE, &function.id, &exchange.id)?,
            ServiceApiNameOwner::Exchange {
                function: function.id.clone(),
                exchange: exchange.id.clone(),
            },
            true,
        ));
    }
    for declared in model_top_level()? {
        let model_entity =
            ServiceApiModelEntity::CppDeclaration(format!("{function_scope}::{declared}"));
        if declared == "std" {
            return Err(collision(
                &function_scope,
                &declared,
                model_entity,
                ServiceApiNameOwner::Fixed(fixed.id),
                function_region,
                ServiceApiModelConflict::Hides {
                    referenced: "::std".to_owned(),
                },
            ));
        }
        if let Some((_, owner, is_namespace)) =
            wrapper_names.iter().find(|(name, ..)| *name == declared)
        {
            let conflict = if *is_namespace {
                ServiceApiModelConflict::Redeclaration
            } else {
                ServiceApiModelConflict::Hides {
                    referenced: format!("{function_scope}::{declared}"),
                }
            };
            return Err(collision(
                &function_scope,
                &declared,
                model_entity,
                owner.clone(),
                function_region,
                conflict,
            ));
        }
    }
    Ok(())
}

/// The Ada visibility check described on [`validate_service_api_artifacts`].
///
/// At `subtype Payload is Outer.Inner.Type;` inside
/// `Service_API.Function_F.Exchange_E`, direct visibility of `Outer` is
/// searched innermost first through everything declared *before* that
/// point. Declarations after it are not yet visible and are not checked.
fn check_ada_payload_visibility(
    model: &ServiceApiModel,
    package: &[String; 2],
) -> Result<(), ServiceApiError> {
    const LANGUAGE: BackendLanguage = BackendLanguage::Ada;
    let fixed = service_api_fixed_names(LANGUAGE);
    let outer = &package[0];
    let hides = |name: &str| name.eq_ignore_ascii_case(outer);
    let model_entity = || ServiceApiModelEntity::AdaPackage(package.join("."));
    let mut service_names = vec![(
        fixed.root.to_owned(),
        ServiceApiNameOwner::Fixed(fixed.root),
        ServiceApiRegion::File,
    )];
    for constant in [
        fixed.service_name,
        fixed.service_version,
        fixed.service_kind,
    ] {
        service_names.push((
            constant.to_owned(),
            ServiceApiNameOwner::Fixed(constant),
            ServiceApiRegion::Service,
        ));
    }

    for function in &model.functions {
        let function_scope = service_api_function_scope_name(LANGUAGE, &function.id)?;
        service_names.push((
            function_scope.clone(),
            ServiceApiNameOwner::Function(function.id.clone()),
            ServiceApiRegion::Service,
        ));
        let function_region = ServiceApiRegion::Function(function.id.clone());
        let mut function_names = vec![
            (
                fixed.id.to_owned(),
                ServiceApiNameOwner::Fixed(fixed.id),
                function_region.clone(),
            ),
            (
                fixed.name.to_owned(),
                ServiceApiNameOwner::Fixed(fixed.name),
                function_region.clone(),
            ),
        ];
        for exchange in &function.exchanges {
            let exchange_scope =
                service_api_exchange_scope_name(LANGUAGE, &function.id, &exchange.id)?;
            function_names.push((
                exchange_scope.clone(),
                ServiceApiNameOwner::Exchange {
                    function: function.id.clone(),
                    exchange: exchange.id.clone(),
                },
                function_region.clone(),
            ));
            if exchange.oms_binding().is_none() {
                continue;
            }
            let exchange_region = ServiceApiRegion::Exchange {
                function: function.id.clone(),
                exchange: exchange.id.clone(),
            };
            // Declared before the Payload subtype, plus the subtype itself,
            // which may not be named inside its own declaration.
            let exchange_names = [
                fixed.id,
                fixed.kind,
                fixed.direction,
                fixed.mandate,
                fixed.topic,
                fixed.payload,
            ]
            .map(|name| {
                (
                    name.to_owned(),
                    ServiceApiNameOwner::Fixed(name),
                    exchange_region.clone(),
                )
            });
            // Innermost first: that is the homograph lookup actually finds.
            let mut visible = exchange_names
                .iter()
                .chain(function_names.iter().rev())
                .chain(service_names.iter().rev());
            if let Some((generated, owner, region)) = visible.find(|(name, ..)| hides(name)) {
                let scope = format!("{}.{function_scope}.{exchange_scope}", fixed.root);
                return Err(ServiceApiError::ModelWrapperNameCollision(Box::new(
                    ServiceApiModelNameCollision {
                        language: LANGUAGE,
                        scope,
                        generated: generated.clone(),
                        model: model_entity(),
                        wrapper: owner.clone(),
                        region: region.clone(),
                        conflict: ServiceApiModelConflict::Hides {
                            referenced: outer.clone(),
                        },
                    },
                )));
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Artifact preflight: the model and the wrapper, emitted together
// ---------------------------------------------------------------------------

/// The wrapper entrypoint would be written to the same path as a model file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceApiArtifactCollision {
    pub language: BackendLanguage,
    /// The one relative output path both artifacts claim.
    pub path: String,
    /// True when the model side is written only for some schemas (the Ada
    /// package body). It is reserved regardless.
    pub model_artifact_optional: bool,
}

impl fmt::Display for ServiceApiArtifactCollision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} service API file '{}' is also a{} model artifact path derived from the \
             schema namespace; the wrapper and the model would overwrite each other",
            self.language.name(),
            self.path,
            if self.model_artifact_optional {
                "n optional"
            } else {
                ""
            }
        )
    }
}

/// The model-generated entity on one side of a model/wrapper name conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceApiModelEntity {
    /// A C++ namespace the model header opens, fully qualified.
    CppNamespace(String),
    /// A C++ declaration directly inside the model namespace, fully
    /// qualified.
    CppDeclaration(String),
    /// An Ada library package of the model, fully expanded.
    AdaPackage(String),
}

impl fmt::Display for ServiceApiModelEntity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CppNamespace(name) => write!(formatter, "model namespace '{name}'"),
            Self::CppDeclaration(name) => write!(formatter, "model declaration '{name}'"),
            Self::AdaPackage(name) => write!(formatter, "model package '{name}'"),
        }
    }
}

/// Why a model entity and a wrapper name cannot coexist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceApiModelConflict {
    /// One name declared twice in one declarative region as two entities
    /// that cannot share it (anything other than a namespace reopening a
    /// namespace).
    Redeclaration,
    /// A wrapper declaration hides the name `referenced` at a point where the
    /// wrapper must resolve it to the model (Ada: the model package prefix of
    /// a `Payload` subtype; C++: `std` in a constant's type).
    Hides { referenced: String },
}

/// A model-generated entity and a wrapper-generated name conflict in a
/// declarative region the two artifacts genuinely share.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceApiModelNameCollision {
    pub language: BackendLanguage,
    /// The host-language scope where the conflict takes effect: the shared
    /// region for a redeclaration (e.g. `::service_api`), or the reference
    /// site for hiding (e.g. `Service_API.Function_F.Exchange_E`).
    pub scope: String,
    /// The conflicting identifier.
    pub generated: String,
    pub model: ServiceApiModelEntity,
    pub wrapper: ServiceApiNameOwner,
    /// Where, in contract terms, the wrapper side is declared: the file, the
    /// wrapper root, or one contract function/exchange scope.
    pub region: ServiceApiRegion,
    pub conflict: ServiceApiModelConflict,
}

impl fmt::Display for ServiceApiModelNameCollision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let language = self.language.name();
        match &self.conflict {
            ServiceApiModelConflict::Redeclaration => write!(
                formatter,
                "{language} {} and service API {} (in the {}) both declare '{}' in {}; the \
                 model and the wrapper cannot be compiled together",
                self.model, self.wrapper, self.region, self.generated, self.scope
            ),
            ServiceApiModelConflict::Hides { referenced } => write!(
                formatter,
                "{language} service API {} declares '{}' (in the {}), which hides '{referenced}' \
                 of the {} where the wrapper names it in {}",
                self.wrapper, self.generated, self.region, self.model, self.scope
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [BackendLanguage; 3] = BackendLanguage::ALL;

    fn names<'a>(functions: &[(&'a str, &[(&'a str, bool)])]) -> Vec<NamingFunction<'a>> {
        functions
            .iter()
            .map(|(id, exchanges)| (*id, exchanges.to_vec()))
            .collect()
    }

    #[test]
    fn scope_names_follow_the_shared_rule() {
        use BackendLanguage::{Ada, Cpp, Rust};
        let function = |language, id| service_api_function_scope_name(language, id).unwrap();
        let exchange = |language, id| service_api_exchange_scope_name(language, "f", id).unwrap();
        assert_eq!(
            function(Rust, "position-input-example"),
            "function_position_input_example"
        );
        assert_eq!(
            function(Cpp, "position-input-example"),
            "function_position_input_example"
        );
        assert_eq!(
            function(Ada, "position-input-example"),
            "Function_Position_Input_Example"
        );
        assert_eq!(
            exchange(Rust, "position-report-input"),
            "exchange_position_report_input"
        );
        assert_eq!(
            exchange(Ada, "position-report-input"),
            "Exchange_Position_Report_Input"
        );
        assert_eq!(exchange(Cpp, "a1_2b-c3"), "exchange_a1_2b_c3");
        assert_eq!(exchange(Ada, "a1_2b-c3"), "Exchange_A1_2b_C3");
    }

    #[test]
    fn reserved_looking_ids_are_safe_behind_the_prefix() {
        for language in ALL {
            for id in [
                "type", "match", "range", "operator", "function", "package", "end", "self",
                "delete", "record",
            ] {
                assert_eq!(
                    validate_names(language, names(&[(id, &[(id, true)])])),
                    Ok(()),
                    "{language:?} {id}"
                );
            }
        }
    }

    #[test]
    fn ids_outside_the_portable_grammar_are_rejected() {
        for language in ALL {
            for id in ["", "Foo", "1abc", "a--b", "a-", "_a", "a b", "a.b", "é"] {
                assert!(
                    matches!(
                        service_api_function_scope_name(language, id),
                        Err(ServiceApiNameError::InvalidIdentifier { .. })
                    ),
                    "{language:?} {id:?}"
                );
            }
        }
    }

    #[test]
    fn function_normalization_collision_fails_closed() {
        for language in ALL {
            let error = validate_names(language, names(&[("foo-bar", &[]), ("foo_bar", &[])]))
                .expect_err("foo-bar and foo_bar must collide");
            let ServiceApiNameError::Collision(collision) = &error else {
                panic!("{language:?}: expected a collision, got {error:?}");
            };
            assert_eq!(collision.language, language);
            assert_eq!(collision.region, ServiceApiRegion::Service, "{language:?}");
            assert_eq!(
                collision.first,
                ServiceApiNameOwner::Function("foo-bar".into())
            );
            assert_eq!(
                collision.second,
                ServiceApiNameOwner::Function("foo_bar".into())
            );
        }
    }

    #[test]
    fn exchange_normalization_collision_in_one_function_fails_closed() {
        for language in ALL {
            let error = validate_names(
                language,
                names(&[("f", &[("foo-bar", true), ("foo_bar", false)])]),
            )
            .expect_err("two exchanges in one function must collide");
            assert!(
                matches!(
                    &error,
                    ServiceApiNameError::Collision(collision)
                        if collision.region == ServiceApiRegion::Function("f".into())
                ),
                "{language:?}: {error:?}"
            );
            // The rendered diagnostic names the language and both raw IDs.
            let text = error.to_string();
            assert!(text.contains(language.name()), "{text}");
            assert!(
                text.contains("'foo-bar'") && text.contains("'foo_bar'"),
                "{text}"
            );
        }
    }

    #[test]
    fn the_same_exchange_id_in_different_functions_is_legal() {
        for language in ALL {
            assert_eq!(
                validate_names(
                    language,
                    names(&[
                        ("function-a", &[("status-output", true)]),
                        ("function-b", &[("status-output", true)]),
                    ])
                ),
                Ok(()),
                "{language:?}"
            );
        }
    }

    /// The fixed identifiers are legal, unreserved, and cannot be reached by
    /// any prefixed scope name, so no contract can collide with them.
    #[test]
    fn fixed_names_are_safe_and_unreachable_from_contract_ids() {
        for language in ALL {
            let fixed = service_api_fixed_names(language);
            for name in [
                fixed.root,
                fixed.service_name,
                fixed.service_version,
                fixed.service_kind,
                fixed.id,
                fixed.name,
                fixed.kind,
                fixed.direction,
                fixed.mandate,
                fixed.topic,
                fixed.payload,
            ]
            .into_iter()
            .chain(fixed.model_module)
            {
                assert!(
                    !crate::backend_names::is_reserved(language, name),
                    "{language:?} {name}"
                );
                let folded = name.to_ascii_lowercase();
                assert!(
                    !folded.starts_with("function_") && !folded.starts_with("exchange_"),
                    "{language:?} {name}"
                );
            }
        }
    }

    // -----------------------------------------------------------------
    // Artifact preflight (Task 047 corrective)
    // -----------------------------------------------------------------

    use crate::backend_layout::{ModelArtifact, namespace_uri_components};
    use ams_gra_oms_ir::NamespaceDecl;

    fn namespace_only(uri: &str) -> SchemaIr {
        SchemaIr {
            schema_version: None,
            namespaces: vec![NamespaceDecl {
                uri: uri.to_owned(),
                preferred_prefix: None,
            }],
            types: Vec::new(),
            messages: Vec::new(),
        }
    }

    /// A model with the given `(function, [(exchange, is_oms)])` shape. The
    /// payload binding carries a placeholder: the artifact preflight never
    /// reads it, only whether a binding exists.
    fn api_model(functions: &[(&str, &[(&str, bool)])]) -> ServiceApiModel {
        let placeholder = QualifiedName::new("urn:x:y", "P");
        ServiceApiModel {
            service_name: "s".into(),
            service_version: "1".into(),
            service_kind: ServiceKind::Service,
            emits_type_model: true,
            world: GenerationWorld::ClosedSchemaSet,
            functions: functions
                .iter()
                .map(|(id, exchanges)| ServiceApiFunction {
                    id: (*id).to_owned(),
                    name: "F".into(),
                    exchanges: exchanges
                        .iter()
                        .map(|(id, is_oms)| ServiceApiExchange {
                            id: (*id).to_owned(),
                            direction: Direction::Input,
                            mandate: Mandate::Mandatory,
                            kind: if *is_oms {
                                ServiceApiExchangeKind::OmsMessage(ServiceApiOmsBinding {
                                    topic: "t".into(),
                                    message_name: placeholder.clone(),
                                    payload_type: TypeRef::named(placeholder.clone()),
                                    payload_name: placeholder.clone(),
                                })
                            } else {
                                ServiceApiExchangeKind::SpecialSignal
                            },
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    /// URIs chosen to push every naming rule toward the wrapper's names:
    /// spelled-out wrapper names in every case/punctuation variant, the
    /// wrapper's own stems, Ada parents, and the real UCI namespace.
    const HOSTILE_URIS: &[&str] = &[
        "urn:test:serviceApi",
        "urn:test:service_api",
        "urn:test:service-api",
        "urn:test:service.api",
        "urn:test:SERVICE_API",
        "urn:serviceApi:serviceName",
        "urn:service_api:service_name",
        "urn:service:api",
        "urn:service_api:rs",
        "urn:x:service_api.rs",
        "urn:x:service_api.hpp",
        "urn:service_api:ads",
        "urn:service:api:ads",
        "https://www.vdl.afrl.af.mil/programs/oam",
    ];

    /// Inventory, by construction: every model artifact path in every
    /// language is built only from URI components, which contain no `_`,
    /// while every wrapper file contains `_`. The two sets are therefore
    /// disjoint for EVERY namespace URI, not just the ones tried here.
    #[test]
    fn model_paths_cannot_contain_the_wrapper_file_by_construction() {
        for uri in HOSTILE_URIS {
            assert!(
                namespace_uri_components(uri)
                    .iter()
                    .all(|part| part.bytes().all(|byte| byte.is_ascii_alphanumeric())),
                "{uri}"
            );
            for language in ALL {
                let file = service_api_fixed_names(language).file;
                assert!(file.contains('_'), "{language:?}: premise of the proof");
                let Ok(layout) = BackendModelLayout::for_schema(&namespace_only(uri), language)
                else {
                    continue;
                };
                for artifact in &layout.artifacts {
                    let stem = artifact
                        .relative_path
                        .rsplit_once('.')
                        .map_or(artifact.relative_path.as_str(), |(stem, _)| stem);
                    assert!(
                        !stem.contains('_'),
                        "{language:?} {uri}: {}",
                        artifact.relative_path
                    );
                    assert!(
                        !artifact.relative_path.eq_ignore_ascii_case(file),
                        "{language:?} {uri}"
                    );
                }
                // Units, too: a model unit component can never equal a
                // wrapper root, every one of which contains `_`.
                match &layout.unit {
                    ModelUnit::CppNamespace([outer, _]) | ModelUnit::AdaPackage([outer, _]) => {
                        let root = service_api_fixed_names(language).root;
                        assert!(root.contains('_') && !outer.contains('_'), "{uri}");
                    }
                    ModelUnit::RustFile => {}
                }
                let model = api_model(&[("f", &[("e", true)])]);
                assert_eq!(check_artifact_paths(&layout), Ok(()), "{language:?} {uri}");
                // The whole preflight agrees for Rust (paths only) and C++.
                if language != BackendLanguage::Ada {
                    assert!(
                        validate_service_api_artifacts(&model, &namespace_only(uri), language)
                            .is_ok(),
                        "{language:?} {uri}"
                    );
                }
            }
        }
    }

    /// The path check itself is live: fed a layout that DOES claim the
    /// wrapper path -- including only the optional Ada body -- it fails with
    /// the typed error naming the language and path.
    #[test]
    fn a_layout_claiming_the_wrapper_path_is_a_typed_collision() {
        for language in ALL {
            let file = service_api_fixed_names(language).file;
            let mut layout = BackendModelLayout::for_schema(
                &namespace_only("https://www.vdl.afrl.af.mil/programs/oam"),
                language,
            )
            .unwrap();
            for optional in [false, true] {
                layout.artifacts.push(ModelArtifact {
                    relative_path: file.to_ascii_uppercase(),
                    optional,
                });
                let error = check_artifact_paths(&layout).unwrap_err();
                let ServiceApiError::ArtifactPathCollision(collision) = &error else {
                    panic!("{language:?}: {error:?}");
                };
                assert_eq!(collision.language, language);
                assert_eq!(collision.path, file.to_ascii_uppercase());
                assert_eq!(collision.model_artifact_optional, optional);
                let text = error.to_string();
                assert!(
                    text.contains(language.name()) && text.contains("overwrite"),
                    "{text}"
                );
                layout.artifacts.pop();
            }
        }
    }

    fn cpp_scopes(
        model: &ServiceApiModel,
        namespace: [&str; 2],
        top_level: &[&str],
    ) -> Result<(), ServiceApiError> {
        let top_level = top_level.iter().map(|name| (*name).to_owned()).collect();
        check_cpp_scopes(model, &namespace.map(str::to_owned), || Ok(top_level))
    }

    fn cpp_collision(result: Result<(), ServiceApiError>) -> ServiceApiModelNameCollision {
        match result {
            Err(ServiceApiError::ModelWrapperNameCollision(collision)) => *collision,
            other => panic!("expected a model/wrapper collision, got {other:?}"),
        }
    }

    /// Positive controls. The walk stops at the global scope unless the model
    /// namespace IS `service_api`, and it only descends through legal
    /// namespace reopenings, so none of these is a false positive.
    #[test]
    fn cpp_scope_positive_controls() {
        let model = api_model(&[("f", &[("e", true)])]);
        let names = &["PayloadA", "BoundedVector", "id", "name", "std"];
        // Ordinary UCI.
        assert_eq!(cpp_scopes(&model, ["programs", "oam"], names), Ok(()));
        // A harmless prefix / look-alike spelling shares no scope.
        for outer in ["serviceapi", "service", "api", "service_api_x"] {
            assert_eq!(
                cpp_scopes(&model, [outer, "service_name"], names),
                Ok(()),
                "{outer}"
            );
        }
        // `::service_api` is reopened by the model, but its inner namespace
        // matches nothing the wrapper declares there.
        assert_eq!(cpp_scopes(&model, ["service_api", "model"], names), Ok(()));
        // `::service_api::function_f` is reopened too (namespace + namespace
        // is legal), and its contents do not overlap the wrapper's.
        assert_eq!(
            cpp_scopes(
                &model,
                ["service_api", "function_f"],
                &["PayloadA", "BoundedVector"]
            ),
            Ok(())
        );
        // Wrapper `id`/`name` live in `::service_api::function_f`; a model in
        // a DIFFERENT function namespace may declare the same names freely.
        assert_eq!(
            cpp_scopes(&model, ["service_api", "function_g"], &["Id", "Name"]),
            Ok(())
        );
    }

    /// Negative controls: every genuine same-region conflict. Each was
    /// reproduced with g++ 14 `-std=c++17 -pedantic-errors` against a real
    /// generated wrapper before being encoded here; see the corrective
    /// section of `docs/task-047-service-api-wrappers.md`.
    #[test]
    fn cpp_scope_negative_controls() {
        let model = api_model(&[("f", &[("e", true), ("s", false)])]);
        // namespace ::service_api::service_* vs a `string_view` constant.
        for constant in ["service_name", "service_version", "service_kind"] {
            let collision = cpp_collision(cpp_scopes(&model, ["service_api", constant], &[]));
            assert_eq!(collision.scope, "::service_api");
            assert_eq!(collision.generated, constant);
            assert_eq!(collision.wrapper, ServiceApiNameOwner::Fixed(constant));
            assert_eq!(collision.region, ServiceApiRegion::Service);
            assert_eq!(collision.conflict, ServiceApiModelConflict::Redeclaration);
            assert_eq!(
                collision.model,
                ServiceApiModelEntity::CppNamespace(format!("::service_api::{constant}"))
            );
        }
        // A model namespace named `std` inside `::service_api` would make the
        // wrapper's unqualified `std::string_view` resolve to it.
        let collision = cpp_collision(cpp_scopes(&model, ["service_api", "std"], &[]));
        assert!(matches!(
            collision.conflict,
            ServiceApiModelConflict::Hides { ref referenced } if referenced == "::std"
        ));
        // Reopened function namespace: model types vs wrapper `id`, `name`,
        // and exchange namespaces (OMS or not) in the same region.
        for (declared, owner) in [
            ("id", ServiceApiNameOwner::Fixed("id")),
            ("name", ServiceApiNameOwner::Fixed("name")),
            (
                "exchange_e",
                ServiceApiNameOwner::Exchange {
                    function: "f".into(),
                    exchange: "e".into(),
                },
            ),
            (
                "exchange_s",
                ServiceApiNameOwner::Exchange {
                    function: "f".into(),
                    exchange: "s".into(),
                },
            ),
        ] {
            let collision = cpp_collision(cpp_scopes(
                &model,
                ["service_api", "function_f"],
                &["PayloadA", declared],
            ));
            assert_eq!(collision.scope, "::service_api::function_f", "{declared}");
            // A namespace redeclared as a type is an error; a constant beside
            // a same-named type is legal C++ but hides the model type.
            let expected = if declared.starts_with("exchange_") {
                ServiceApiModelConflict::Redeclaration
            } else {
                ServiceApiModelConflict::Hides {
                    referenced: format!("::service_api::function_f::{declared}"),
                }
            };
            assert_eq!(collision.conflict, expected, "{declared}");
            assert_eq!(collision.wrapper, owner, "{declared}");
            assert_eq!(collision.region, ServiceApiRegion::Function("f".into()));
            assert_eq!(
                collision.model,
                ServiceApiModelEntity::CppDeclaration(format!(
                    "::service_api::function_f::{declared}"
                ))
            );
            let text = ServiceApiError::ModelWrapperNameCollision(Box::new(collision)).to_string();
            assert!(text.starts_with("C++ "), "{text}");
            assert!(
                text.contains("model declaration '::service_api::function_f::"),
                "{text}"
            );
        }
    }

    /// Ada: the model parent package is hidden at a `Payload` reference by
    /// any same-spelled (case-insensitive) wrapper declaration visible there,
    /// innermost first. Names that are not yet declared, and scopes the
    /// reference is not nested in, do not hide it.
    #[test]
    fn ada_payload_visibility() {
        let package = |outer: &str| [outer.to_owned(), "Model".to_owned()];
        let model = api_model(&[("f", &[("e", true)])]);
        for (outer, generated, region) in [
            ("Name", "Name", ServiceApiRegion::Function("f".into())),
            ("NAME", "Name", ServiceApiRegion::Function("f".into())),
            (
                "Payload",
                "Payload",
                ServiceApiRegion::Exchange {
                    function: "f".into(),
                    exchange: "e".into(),
                },
            ),
            (
                "Topic",
                "Topic",
                ServiceApiRegion::Exchange {
                    function: "f".into(),
                    exchange: "e".into(),
                },
            ),
            ("Service_Kind", "Service_Kind", ServiceApiRegion::Service),
            ("Function_F", "Function_F", ServiceApiRegion::Service),
            (
                "Exchange_E",
                "Exchange_E",
                ServiceApiRegion::Function("f".into()),
            ),
            ("Service_API", "Service_API", ServiceApiRegion::File),
        ] {
            let error = check_ada_payload_visibility(&model, &package(outer)).unwrap_err();
            let ServiceApiError::ModelWrapperNameCollision(collision) = &error else {
                panic!("{outer}: {error:?}");
            };
            assert_eq!(collision.generated, generated, "{outer}");
            assert_eq!(collision.region, region, "{outer}");
            assert_eq!(
                collision.scope, "Service_API.Function_F.Exchange_E",
                "{outer}"
            );
        }
        // Positive controls: real UCI; a harmless prefix; a later sibling
        // exchange scope that is not yet visible at the first Payload; and a
        // non-OMS exchange, which names no model at all.
        for outer in ["Programs", "Names", "Exchange_Z"] {
            let model = api_model(&[("f", &[("e", true), ("z", false)])]);
            assert_eq!(
                check_ada_payload_visibility(&model, &package(outer)),
                Ok(()),
                "{outer}"
            );
        }
        let signals_only = api_model(&[("f", &[("e", false)])]);
        assert_eq!(
            check_ada_payload_visibility(&signals_only, &package("Id")),
            Ok(())
        );
    }

    /// No type model, no model artifacts: the artifact preflight is vacuous
    /// even for a namespace-less projected schema.
    #[test]
    fn zero_model_services_have_no_artifact_boundary() {
        let mut model = api_model(&[("f", &[("e", false)])]);
        model.emits_type_model = false;
        let empty = SchemaIr {
            namespaces: Vec::new(),
            ..namespace_only("")
        };
        for language in ALL {
            assert_eq!(
                validate_service_api_artifacts(&model, &empty, language),
                Ok(())
            );
        }
    }

    /// Inventory, by construction: every C++ model top-level identifier (a
    /// support type or an emitted declaration) starts with an uppercase
    /// letter, while every wrapper name in a function namespace starts
    /// lowercase. So a reopened `::service_api::function_*` model namespace
    /// can never actually clash with its contents; the check stays as
    /// defence only.
    #[test]
    fn cpp_model_top_level_names_are_upper_camel() {
        let schema = SchemaIr {
            types: vec![ams_gra_oms_ir::TypeDecl {
                name: QualifiedName::new("urn:service_api:function_f", "id"),
                is_abstract: false,
                base_type: None,
                kind: ams_gra_oms_ir::TypeKind::Record { fields: Vec::new() },
                constraints: ams_gra_oms_ir::ConstraintSet::default(),
                documentation: None,
                source: ams_gra_oms_ir::SourceRef {
                    document: "x.xsd".into(),
                    line: None,
                },
            }],
            ..namespace_only("urn:service_api:function_f")
        };
        let names = crate::backend_names::top_level_generated_names(
            &schema,
            BackendLanguage::Cpp,
            GenerationWorld::ClosedSchemaSet,
        )
        .unwrap();
        assert!(names.contains(&"Id".to_owned()), "{names:?}");
        assert!(
            names
                .iter()
                .all(|name| name.as_bytes()[0].is_ascii_uppercase()),
            "{names:?}"
        );
        let fixed = service_api_fixed_names(BackendLanguage::Cpp);
        for wrapper in [fixed.id, fixed.name, "exchange_e", "std"] {
            assert!(wrapper.as_bytes()[0].is_ascii_lowercase(), "{wrapper}");
        }
    }
}
