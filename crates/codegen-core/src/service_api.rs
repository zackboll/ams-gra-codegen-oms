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
/// (verifying every payload binding), then check every generated name.
///
/// Readiness consults this, so READY means `service-generate` can produce the
/// wrapper as well as the type model.
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
    Ok(model)
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
}
