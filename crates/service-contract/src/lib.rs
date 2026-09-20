//! Portable AMS GRA Service Contract v0.1 parsing and typed Contract IR.
//!
//! This crate is the first independent (non-Python) consumer of the portable
//! Service Contract format published by `zackboll/ams-gra-service-contract`.
//! It deliberately owns **only** the portable contract: parsing, the typed
//! Contract IR, and portable semantic validation.
//!
//! Boundaries, stated as code-level intent:
//!
//! * it does **not** depend on `xsd-frontend`, `codegen-core`, any language
//!   backend, the CLI, or any Sleet/runtime code, so the portable contract
//!   format can never acquire a hidden XSD dependency;
//! * it does **not** reimplement the OMS profile engine. Profile conformance
//!   (which required functions a Service must have, which Capabilities imply
//!   which Section 3.3 functions) stays owned by the Service Contract project.
//!   Nothing here infers, completes, or regenerates contract content;
//! * it does **not** resolve UCI message names. An OMS Message exchange keeps
//!   the author's local message name as an opaque string; joining that to a
//!   normalized UCI `SchemaIr` is the service-plan layer's job.
//!
//! See `docs/service-contract-integration.md` for the compatibility baseline.

mod validate;

use serde::Deserialize;
use std::fmt;
use std::fs;
use std::path::Path;

pub use validate::SemanticError;

/// The single portable contract version this crate understands.
pub const SUPPORTED_CONTRACT_VERSION: &str = "0.1";

/// A parsed, portable-valid v0.1 Service Contract.
///
/// Field order mirrors the portable schema. Author order is preserved for
/// every sequence: `functions`, each function's `exchanges`, `capabilities`,
/// and `sources` are never sorted or deduplicated by this crate.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub contract_version: String,
    pub service: Service,
    pub standards: Standards,
    #[serde(default)]
    pub sources: Vec<Source>,
    /// Capability inventory.
    ///
    /// `None`, `Some(vec![])`, and a non-empty list are three **distinct**
    /// author statements and are kept distinct forever:
    ///
    /// * `None` -- the contract says nothing about Capabilities;
    /// * `Some(vec![])` -- the author explicitly asserted "no Capabilities";
    /// * `Some(non-empty)` -- an explicit Capability inventory.
    ///
    /// Normalizing omitted to empty would silently convert "unstated" into an
    /// assertion the author never made, so it is never done.
    pub capabilities: Option<Vec<Capability>>,
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub name: String,
    pub version: String,
    pub kind: ServiceKind,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    Service,
    Subsystem,
    Isolator,
}

impl ServiceKind {
    /// The portable spelling, for diagnostics and deterministic reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Service => "service",
            Self::Subsystem => "subsystem",
            Self::Isolator => "isolator",
        }
    }
}

impl fmt::Display for ServiceKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Standards {
    pub oms_version: String,
    /// The contract's *logical* UCI schema version, such as `"2.5"`.
    ///
    /// This is not the XSD root's `xs:schema @version` release string. See
    /// `docs/service-contract-integration.md` for the recorded evidence and
    /// why no string-surgery normalization is performed.
    pub uci_schema_version: String,
    /// Logical extension-schema identifiers, **not** filesystem paths.
    #[serde(default)]
    pub uci_extension_schemas: Vec<String>,
    pub ams_gra_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub title: String,
    pub document_number: Option<String>,
    pub revision: Option<String>,
    pub date: Option<String>,
    pub uri: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceRef {
    pub source: String,
    pub locator: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub id: String,
    pub name: String,
    pub requires_position_information: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Function {
    pub id: String,
    pub name: String,
    pub category: FunctionCategory,
    pub required_group: Option<RequiredGroup>,
    /// Capability ownership: which Capability this function belongs to.
    ///
    /// Absence means the contract did not assign one. It is never inferred
    /// from the function name, standard role, or exchange content.
    pub capability: Option<String>,
    pub standard_role: Option<StandardRole>,
    pub applicability: Applicability,
    pub not_applicable_reason: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub traceability: Vec<TraceRef>,
    pub exchanges: Vec<Exchange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionCategory {
    Required,
    Specific,
}

impl FunctionCategory {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Specific => "specific",
        }
    }
}

impl fmt::Display for FunctionCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredGroup {
    Service,
    Subsystem,
    Capability,
}

impl RequiredGroup {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Service => "service",
            Self::Subsystem => "subsystem",
            Self::Capability => "capability",
        }
    }
}

impl fmt::Display for RequiredGroup {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandardRole {
    CapabilityStatus,
    CapabilityEnableDisable,
    CapabilityOperations,
    PositionInformationProcessing,
}

impl StandardRole {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityStatus => "capability_status",
            Self::CapabilityEnableDisable => "capability_enable_disable",
            Self::CapabilityOperations => "capability_operations",
            Self::PositionInformationProcessing => "position_information_processing",
        }
    }
}

impl fmt::Display for StandardRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Applicability {
    Applicable,
    NotApplicable,
}

impl Applicability {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Applicable => "applicable",
            Self::NotApplicable => "not_applicable",
        }
    }
}

impl fmt::Display for Applicability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Input,
    Output,
}

impl Direction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mandate {
    Mandatory,
    Optional,
}

impl Mandate {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mandatory => "mandatory",
            Self::Optional => "optional",
        }
    }
}

impl fmt::Display for Mandate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One contract exchange.
///
/// The five portable exchange kinds are modelled as enum variants rather than
/// one struct with a `kind` tag plus a bag of optional fields, so each kind's
/// required fields are non-optional in the IR and unrepresentable states
/// (a Data Transfer carrying a `message`, an OMS Message carrying a
/// `protocol`) simply cannot be constructed.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Exchange {
    OmsMessage(OmsMessageExchange),
    DataTransfer(DataTransferExchange),
    SpecialSignal(SpecialSignalExchange),
    SecurityExchange(SecurityExchange),
    NonOmsMessage(NonOmsMessageExchange),
}

impl Exchange {
    /// The author-assigned exchange identifier, for every kind.
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

    /// The portable `kind` spelling, for diagnostics and reports.
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

    #[must_use]
    pub const fn mandate(&self) -> Mandate {
        match self {
            Self::OmsMessage(exchange) => exchange.mandate,
            Self::DataTransfer(exchange) => exchange.mandate,
            Self::SpecialSignal(exchange) => exchange.mandate,
            Self::SecurityExchange(exchange) => exchange.mandate,
            Self::NonOmsMessage(exchange) => exchange.mandate,
        }
    }

    #[must_use]
    pub const fn timing(&self) -> &Timing {
        match self {
            Self::OmsMessage(exchange) => &exchange.timing,
            Self::DataTransfer(exchange) => &exchange.timing,
            Self::SpecialSignal(exchange) => &exchange.timing,
            Self::SecurityExchange(exchange) => &exchange.timing,
            Self::NonOmsMessage(exchange) => &exchange.timing,
        }
    }

    #[must_use]
    pub fn traceability(&self) -> &[TraceRef] {
        match self {
            Self::OmsMessage(exchange) => &exchange.traceability,
            Self::DataTransfer(exchange) => &exchange.traceability,
            Self::SpecialSignal(exchange) => &exchange.traceability,
            Self::SecurityExchange(exchange) => &exchange.traceability,
            Self::NonOmsMessage(exchange) => &exchange.traceability,
        }
    }
}

/// A UCI/OMS message exchange.
///
/// `message` is the author's *local* UCI message name, kept verbatim. This
/// crate performs no schema lookup, no case folding, and no qualification.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OmsMessageExchange {
    pub id: String,
    pub direction: Direction,
    pub mandate: Mandate,
    pub message: String,
    pub topic: String,
    pub operational_attribute: Option<String>,
    pub subscription_group: Option<String>,
    pub appendix_c_mapping: Option<String>,
    pub timing: Timing,
    #[serde(default)]
    pub traceability: Vec<TraceRef>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataTransferExchange {
    pub id: String,
    pub direction: Direction,
    pub mandate: Mandate,
    pub name: String,
    pub protocol: String,
    pub data_type: String,
    pub data_format: String,
    pub sharing_pattern: String,
    pub timing: Timing,
    #[serde(default)]
    pub traceability: Vec<TraceRef>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecialSignalExchange {
    pub id: String,
    pub direction: Direction,
    pub mandate: Mandate,
    pub name: String,
    pub details: Option<String>,
    pub reference: Option<String>,
    pub timing: Timing,
    #[serde(default)]
    pub traceability: Vec<TraceRef>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityExchange {
    pub id: String,
    pub direction: Direction,
    pub mandate: Mandate,
    pub name: String,
    pub details: Option<String>,
    pub reference: Option<String>,
    pub timing: Timing,
    #[serde(default)]
    pub traceability: Vec<TraceRef>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NonOmsMessageExchange {
    pub id: String,
    pub direction: Direction,
    pub mandate: Mandate,
    pub name: String,
    pub details: Option<String>,
    pub reference: Option<String>,
    pub timing: Timing,
    #[serde(default)]
    pub traceability: Vec<TraceRef>,
}

/// Exchange timing, as three structurally distinct portable kinds.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Timing {
    /// Spelled as an empty struct variant rather than a unit variant on
    /// purpose: serde's internally tagged *unit* variants silently accept and
    /// discard sibling keys, so `kind: asynchronous` plus a stray
    /// `nominal_rate_hz` would parse. An empty struct variant honors
    /// `deny_unknown_fields` and rejects it.
    Asynchronous {},
    OnDemand {
        nominal_response_seconds: Option<f64>,
        max_response_seconds: Option<f64>,
    },
    Periodic {
        nominal_rate_hz: Option<f64>,
        max_rate_hz: Option<f64>,
    },
}

impl Timing {
    #[must_use]
    pub const fn kind_str(&self) -> &'static str {
        match self {
            Self::Asynchronous {} => "asynchronous",
            Self::OnDemand { .. } => "on_demand",
            Self::Periodic { .. } => "periodic",
        }
    }
}

/// A portable Service Contract failure.
///
/// The variants are kept apart because they mean different things to an
/// operator: the bytes were not the format, the format was a version this
/// build does not implement, or the contract parsed but says something the
/// portable v0.1 rules forbid.
#[derive(Debug, Clone, PartialEq)]
pub enum ContractError {
    /// Unreadable input, or an unrecognized file extension.
    Io(String),
    /// Malformed YAML/JSON, an unknown property, or a type mismatch.
    Parse(String),
    /// Well-formed input declaring a `contract_version` this build cannot
    /// interpret. Never downgraded to a best-effort parse.
    UnsupportedVersion { found: String },
    /// A portable v0.1 semantic invariant was violated.
    Semantic(SemanticError),
}

impl fmt::Display for ContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "unable to read service contract: {message}"),
            Self::Parse(message) => write!(formatter, "invalid service contract: {message}"),
            Self::UnsupportedVersion { found } => write!(
                formatter,
                "unsupported service contract version '{found}'; this build implements \
                 contract_version '{SUPPORTED_CONTRACT_VERSION}'"
            ),
            Self::Semantic(error) => write!(formatter, "invalid service contract: {error}"),
        }
    }
}

impl std::error::Error for ContractError {}

impl From<SemanticError> for ContractError {
    fn from(error: SemanticError) -> Self {
        Self::Semantic(error)
    }
}

/// Parse a portable Service Contract from YAML text.
///
/// # Errors
///
/// Returns [`ContractError`] for malformed YAML, unknown properties, an
/// unsupported `contract_version`, or any portable v0.1 semantic violation.
pub fn parse_yaml(text: &str) -> Result<Contract, ContractError> {
    let contract: Contract =
        serde_yaml::from_str(text).map_err(|error| ContractError::Parse(error.to_string()))?;
    finish(contract)
}

/// Parse a portable Service Contract from JSON text.
///
/// # Errors
///
/// Returns [`ContractError`] for malformed JSON, unknown properties, an
/// unsupported `contract_version`, or any portable v0.1 semantic violation.
pub fn parse_json(text: &str) -> Result<Contract, ContractError> {
    let contract: Contract =
        serde_json::from_str(text).map_err(|error| ContractError::Parse(error.to_string()))?;
    finish(contract)
}

/// Load a portable Service Contract from a `.yaml`, `.yml`, or `.json` file.
///
/// The syntax is selected by file extension rather than sniffed, so an input
/// is never silently reinterpreted as the other syntax.
///
/// # Errors
///
/// Returns [`ContractError`] for an unreadable file, an unrecognized
/// extension, malformed input, an unsupported version, or a semantic
/// violation.
pub fn load_contract(path: &Path) -> Result<Contract, ContractError> {
    let text = fs::read_to_string(path)
        .map_err(|error| ContractError::Io(format!("{}: {error}", path.display())))?;
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("yaml" | "yml") => parse_yaml(&text),
        Some("json") => parse_json(&text),
        _ => Err(ContractError::Io(format!(
            "{}: unsupported service contract extension; expected .yaml, .yml, or .json",
            path.display()
        ))),
    }
}

/// Version-gate first, then validate portable semantics.
///
/// The version check runs before the semantic rules so a future contract is
/// reported as "unsupported version" rather than as a pile of confusing
/// invariant failures derived from v0.1 assumptions.
fn finish(contract: Contract) -> Result<Contract, ContractError> {
    if contract.contract_version != SUPPORTED_CONTRACT_VERSION {
        return Err(ContractError::UnsupportedVersion {
            found: contract.contract_version,
        });
    }
    validate::validate(&contract)?;
    Ok(contract)
}
