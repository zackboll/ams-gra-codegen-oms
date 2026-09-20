//! Portable v0.1 Service Contract semantic validation.
//!
//! This is deliberately *portable* validation only: the rules encoded here are
//! the ones the published v0.1 JSON Schema states, plus the numeric-positivity
//! and structural rules needed for safe downstream codegen. It is **not** an
//! OMS profile engine. Nothing here knows which required functions an OMS 2.5
//! Service must declare, which Section 3.3 functions a Capability implies, or
//! how to complete a partial contract; that authority stays with the Service
//! Contract project. Validation therefore only ever rejects; it never fills
//! anything in.
//!
//! Failure is fail-closed and deterministic: the first violation in contract
//! order is reported, with logical (function/exchange/capability) identity
//! rather than byte offsets.
//!
//! # JSON Schema `format` policy
//!
//! The authoritative v0.1 schema annotates `source.date` with `format: date`
//! and `source.uri` with `format: uri`. It declares
//! `$schema: https://json-schema.org/draft/2020-12/schema` and does **not**
//! opt into the format-assertion vocabulary, so under draft 2020-12 those
//! keywords are *annotations*, not assertions: a conforming validator does not
//! reject a contract because a `uri` is unparseable or a `date` is not a real
//! calendar date.
//!
//! This validator therefore treats `format` as an annotation too, deliberately
//! and not by omission. Both fields are still checked for the constraints the
//! schema *does* assert (`type: string`, and `minLength: 1` on `source.note`
//! and friends). Adding ad-hoc URI or date parsing here would make this
//! validator stricter than the published schema, rejecting contracts the
//! authority accepts -- the mirror image of the parity defect this module
//! exists to prevent. If upstream later adopts the format-assertion
//! vocabulary, enforcement belongs here at that point and not before.

use crate::{Applicability, Contract, Exchange, FunctionCategory, Timing};
use std::collections::BTreeSet;
use std::fmt;

/// A violated portable v0.1 invariant.
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticError {
    /// An identifier did not match the portable identifier grammar.
    InvalidIdentifier {
        role: &'static str,
        value: String,
    },
    DuplicateSourceId {
        id: String,
    },
    DuplicateCapabilityId {
        id: String,
    },
    DuplicateFunctionId {
        id: String,
    },
    DuplicateExchangeId {
        function: String,
        id: String,
    },
    /// A function claims a Capability the inventory does not declare.
    UnknownCapabilityReference {
        function: String,
        capability: String,
    },
    /// A traceability entry cites a `sources` entry that does not exist.
    UnknownSourceReference {
        context: String,
        source: String,
    },
    /// `category: specific` with a `required_group`.
    SpecificFunctionWithRequiredGroup {
        function: String,
    },
    NotApplicableWithoutReason {
        function: String,
    },
    NotApplicableWithExchanges {
        function: String,
    },
    /// A timing value that must be strictly positive was not.
    NonPositiveTimingValue {
        function: String,
        exchange: String,
        field: &'static str,
        value: f64,
    },
    /// A required non-empty string was empty or whitespace-only.
    EmptyText {
        context: String,
        field: &'static str,
    },
    /// The contract declared no functions.
    ///
    /// The portable schema states `functions.minItems = 1`. Serde happily
    /// deserializes `functions: []`, so this must be rejected semantically or
    /// a function-free contract would be accepted here while the published
    /// schema rejects it.
    NoFunctions,
    /// `standards.uci_extension_schemas` repeated an identifier.
    ///
    /// The portable schema states `uniqueItems: true`. The contract itself is
    /// invalid, so this is rejected here rather than left for the CLI's
    /// `--extension` mapping layer to notice later.
    DuplicateExtensionSchema {
        id: String,
    },
    /// A declared timing value was not a finite number.
    ///
    /// Distinct from [`Self::NonPositiveTimingValue`]: `+Infinity` *is*
    /// strictly greater than zero, so a comparison-only check admitted it.
    /// The portable schema's `type: number` admits only finite JSON numbers.
    NonFiniteTimingValue {
        function: String,
        exchange: String,
        field: &'static str,
        value: f64,
    },
}

impl fmt::Display for SemanticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier { role, value } => write!(
                formatter,
                "{role} identifier '{value}' does not match the portable identifier grammar \
                 ^[a-z][a-z0-9]*(?:[-_][a-z0-9]+)*$"
            ),
            Self::DuplicateSourceId { id } => write!(formatter, "duplicate source id '{id}'"),
            Self::DuplicateCapabilityId { id } => {
                write!(formatter, "duplicate capability id '{id}'")
            }
            Self::DuplicateFunctionId { id } => write!(formatter, "duplicate function id '{id}'"),
            Self::DuplicateExchangeId { function, id } => write!(
                formatter,
                "function '{function}': duplicate exchange id '{id}'"
            ),
            Self::UnknownCapabilityReference {
                function,
                capability,
            } => write!(
                formatter,
                "function '{function}': unknown capability '{capability}'; it is not declared in \
                 the contract capability inventory"
            ),
            Self::UnknownSourceReference { context, source } => write!(
                formatter,
                "{context}: traceability cites unknown source '{source}'"
            ),
            Self::SpecificFunctionWithRequiredGroup { function } => write!(
                formatter,
                "function '{function}': a specific function must not declare required_group"
            ),
            Self::NotApplicableWithoutReason { function } => write!(
                formatter,
                "function '{function}': applicability 'not_applicable' requires \
                 not_applicable_reason"
            ),
            Self::NotApplicableWithExchanges { function } => write!(
                formatter,
                "function '{function}': applicability 'not_applicable' requires an empty \
                 exchange list"
            ),
            Self::NonPositiveTimingValue {
                function,
                exchange,
                field,
                value,
            } => write!(
                formatter,
                "function '{function}' exchange '{exchange}': timing {field} must be greater \
                 than zero, found {value}"
            ),
            Self::EmptyText { context, field } => {
                write!(formatter, "{context}: {field} must not be empty")
            }
            Self::NoFunctions => {
                write!(formatter, "contract must declare at least one function")
            }
            Self::DuplicateExtensionSchema { id } => write!(
                formatter,
                "standards: duplicate uci_extension_schemas entry '{id}'"
            ),
            Self::NonFiniteTimingValue {
                function,
                exchange,
                field,
                value,
            } => write!(
                formatter,
                "function '{function}' exchange '{exchange}': timing {field} must be a finite \
                 number, found {value}"
            ),
        }
    }
}

impl std::error::Error for SemanticError {}

/// The portable identifier grammar: `^[a-z][a-z0-9]*(?:[-_][a-z0-9]+)*$`.
///
/// Implemented directly rather than with a regex dependency; the grammar is
/// small, and a hand-written scanner keeps the accepted language obvious.
/// Accepts a lowercase alphanumeric run, then zero or more separator-prefixed
/// non-empty lowercase alphanumeric runs. Trailing separators, doubled
/// separators, uppercase, and non-ASCII are all rejected.
fn is_portable_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    match characters.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => return false,
    }
    // `true` while the previous character was a separator, which therefore
    // still owes at least one alphanumeric character.
    let mut awaiting_segment = false;
    for character in characters {
        match character {
            'a'..='z' | '0'..='9' => awaiting_segment = false,
            '-' | '_' => {
                if awaiting_segment {
                    return false;
                }
                awaiting_segment = true;
            }
            _ => return false,
        }
    }
    !awaiting_segment
}

fn identifier(role: &'static str, value: &str) -> Result<(), SemanticError> {
    if is_portable_identifier(value) {
        Ok(())
    } else {
        Err(SemanticError::InvalidIdentifier {
            role,
            value: value.to_owned(),
        })
    }
}

/// Enforce `minLength: 1` on an **optional** string.
///
/// The portable schema puts `minLength: 1` on optional strings just as it does
/// on required ones, so `description: ""` is as invalid as a missing required
/// name. Absence stays legal and is not defaulted: only a *present* value is
/// checked. Emptiness means the same thing here as everywhere else in this
/// validator -- no meaningful text -- so whitespace-only fails too, keeping
/// optional and required fields consistent.
fn optional_non_empty(
    context: &str,
    field: &'static str,
    value: Option<&String>,
) -> Result<(), SemanticError> {
    match value {
        Some(value) => non_empty(context, field, value),
        None => Ok(()),
    }
}

fn non_empty(context: &str, field: &'static str, value: &str) -> Result<(), SemanticError> {
    if value.trim().is_empty() {
        return Err(SemanticError::EmptyText {
            context: context.to_owned(),
            field,
        });
    }
    Ok(())
}

/// Validate every portable v0.1 invariant this build relies on.
///
/// Rules are checked in contract order, and the first violation wins, so the
/// diagnostic for a given input is stable across runs and platforms.
///
/// # Errors
///
/// Returns the first [`SemanticError`] in contract order.
pub(crate) fn validate(contract: &Contract) -> Result<(), SemanticError> {
    non_empty("service", "name", &contract.service.name)?;
    non_empty("service", "version", &contract.service.version)?;
    optional_non_empty(
        "service",
        "description",
        contract.service.description.as_ref(),
    )?;
    non_empty("standards", "oms_version", &contract.standards.oms_version)?;
    non_empty(
        "standards",
        "uci_schema_version",
        &contract.standards.uci_schema_version,
    )?;
    optional_non_empty(
        "standards",
        "ams_gra_version",
        contract.standards.ams_gra_version.as_ref(),
    )?;
    // `uniqueItems: true` on the extension list. Checked here, on the
    // contract, rather than deferred to the CLI's `--extension` mapping: the
    // contract is invalid on its own terms.
    let mut extension_ids = BTreeSet::new();
    for extension in &contract.standards.uci_extension_schemas {
        non_empty("standards", "uci_extension_schemas entry", extension)?;
        if !extension_ids.insert(extension.as_str()) {
            return Err(SemanticError::DuplicateExtensionSchema {
                id: extension.clone(),
            });
        }
    }

    let mut source_ids = BTreeSet::new();
    for source in &contract.sources {
        identifier("source", &source.id)?;
        let context = format!("source '{}'", source.id);
        non_empty(&context, "title", &source.title)?;
        optional_non_empty(&context, "document_number", source.document_number.as_ref())?;
        optional_non_empty(&context, "revision", source.revision.as_ref())?;
        non_empty(&context, "uri", &source.uri)?;
        optional_non_empty(&context, "note", source.note.as_ref())?;
        if !source_ids.insert(source.id.as_str()) {
            return Err(SemanticError::DuplicateSourceId {
                id: source.id.clone(),
            });
        }
    }

    // `None` (omitted) and `Some(vec![])` (explicitly empty) are both legal
    // and are validated identically; the distinction is preserved in the IR,
    // not collapsed here.
    let mut capability_ids = BTreeSet::new();
    for capability in contract.capabilities.iter().flatten() {
        identifier("capability", &capability.id)?;
        non_empty(
            &format!("capability '{}'", capability.id),
            "name",
            &capability.name,
        )?;
        if !capability_ids.insert(capability.id.as_str()) {
            return Err(SemanticError::DuplicateCapabilityId {
                id: capability.id.clone(),
            });
        }
    }

    // `functions.minItems = 1`. Serde deserializes `functions: []` without
    // complaint, so without this an empty contract would be accepted here
    // while the published schema rejects it.
    if contract.functions.is_empty() {
        return Err(SemanticError::NoFunctions);
    }

    let mut function_ids = BTreeSet::new();
    for function in &contract.functions {
        identifier("function", &function.id)?;
        let context = format!("function '{}'", function.id);
        non_empty(&context, "name", &function.name)?;
        optional_non_empty(&context, "description", function.description.as_ref())?;
        if !function_ids.insert(function.id.as_str()) {
            return Err(SemanticError::DuplicateFunctionId {
                id: function.id.clone(),
            });
        }

        if function.category == FunctionCategory::Specific && function.required_group.is_some() {
            return Err(SemanticError::SpecificFunctionWithRequiredGroup {
                function: function.id.clone(),
            });
        }

        if function.applicability == Applicability::NotApplicable {
            // A not-applicable function must justify itself and must not
            // simultaneously declare interface content.
            match &function.not_applicable_reason {
                Some(reason) => non_empty(&context, "not_applicable_reason", reason)?,
                None => {
                    return Err(SemanticError::NotApplicableWithoutReason {
                        function: function.id.clone(),
                    });
                }
            }
            if !function.exchanges.is_empty() {
                return Err(SemanticError::NotApplicableWithExchanges {
                    function: function.id.clone(),
                });
            }
        }

        if let Some(capability) = &function.capability {
            identifier("capability reference", capability)?;
            // Ownership must point at a declared Capability. An omitted
            // inventory therefore cannot satisfy a reference: absence of an
            // inventory is not permission to invent one.
            if !capability_ids.contains(capability.as_str()) {
                return Err(SemanticError::UnknownCapabilityReference {
                    function: function.id.clone(),
                    capability: capability.clone(),
                });
            }
        }

        validate_traceability(&context, &function.traceability, &source_ids)?;
        validate_exchanges(function, &source_ids)?;
    }

    Ok(())
}

fn validate_traceability(
    context: &str,
    traceability: &[crate::TraceRef],
    source_ids: &BTreeSet<&str>,
) -> Result<(), SemanticError> {
    for trace in traceability {
        identifier("traceability source", &trace.source)?;
        // Both optional trace fields carry `minLength: 1`.
        optional_non_empty(context, "traceability locator", trace.locator.as_ref())?;
        optional_non_empty(context, "traceability note", trace.note.as_ref())?;
        if !source_ids.contains(trace.source.as_str()) {
            return Err(SemanticError::UnknownSourceReference {
                context: context.to_owned(),
                source: trace.source.clone(),
            });
        }
    }
    Ok(())
}

fn validate_exchanges(
    function: &crate::Function,
    source_ids: &BTreeSet<&str>,
) -> Result<(), SemanticError> {
    let mut exchange_ids = BTreeSet::new();
    for exchange in &function.exchanges {
        let id = exchange.id();
        identifier("exchange", id)?;
        // Exchange ids are unique per function, not globally: two functions
        // may each legitimately name an exchange 'status-output'.
        if !exchange_ids.insert(id) {
            return Err(SemanticError::DuplicateExchangeId {
                function: function.id.clone(),
                id: id.to_owned(),
            });
        }
        let context = format!("function '{}' exchange '{id}'", function.id);

        // Kind-specific required text. The IR already makes these fields
        // non-optional per kind; this rejects present-but-empty values.
        match exchange {
            Exchange::OmsMessage(oms) => {
                non_empty(&context, "message", &oms.message)?;
                non_empty(&context, "topic", &oms.topic)?;
                // Optional OMS metadata also carries `minLength: 1`.
                optional_non_empty(
                    &context,
                    "operational_attribute",
                    oms.operational_attribute.as_ref(),
                )?;
                optional_non_empty(
                    &context,
                    "subscription_group",
                    oms.subscription_group.as_ref(),
                )?;
                optional_non_empty(
                    &context,
                    "appendix_c_mapping",
                    oms.appendix_c_mapping.as_ref(),
                )?;
            }
            Exchange::DataTransfer(transfer) => {
                non_empty(&context, "name", &transfer.name)?;
                non_empty(&context, "protocol", &transfer.protocol)?;
                non_empty(&context, "data_type", &transfer.data_type)?;
                non_empty(&context, "data_format", &transfer.data_format)?;
                non_empty(&context, "sharing_pattern", &transfer.sharing_pattern)?;
            }
            // The three non-OMS kinds share an optional `details`/`reference`
            // pair, each with `minLength: 1`.
            Exchange::SpecialSignal(signal) => {
                non_empty(&context, "name", &signal.name)?;
                optional_non_empty(&context, "details", signal.details.as_ref())?;
                optional_non_empty(&context, "reference", signal.reference.as_ref())?;
            }
            Exchange::SecurityExchange(security) => {
                non_empty(&context, "name", &security.name)?;
                optional_non_empty(&context, "details", security.details.as_ref())?;
                optional_non_empty(&context, "reference", security.reference.as_ref())?;
            }
            Exchange::NonOmsMessage(message) => {
                non_empty(&context, "name", &message.name)?;
                optional_non_empty(&context, "details", message.details.as_ref())?;
                optional_non_empty(&context, "reference", message.reference.as_ref())?;
            }
        }

        validate_timing(function, id, exchange.timing())?;
        validate_traceability(&context, exchange.traceability(), source_ids)?;
    }
    Ok(())
}

/// Reject non-finite and non-positive declared timing values.
///
/// The portable schema declares `type: number` with `exclusiveMinimum: 0` on
/// every numeric timing value, so infinities, NaN, zero, and negative rates or
/// response times are all rejected rather than passed through to code
/// generation as meaningless quantities. Absent values stay absent:
/// "unspecified" is legal and is not defaulted.
///
/// Rejecting a value here is a *validity* statement only. It does not turn the
/// surviving values into deadlines: upstream identifies the nominal/max timing
/// columns as informative, and this crate preserves that.
fn validate_timing(
    function: &crate::Function,
    exchange: &str,
    timing: &Timing,
) -> Result<(), SemanticError> {
    let values: [(&'static str, Option<f64>); 2] = match timing {
        Timing::Asynchronous {} => return Ok(()),
        Timing::OnDemand {
            nominal_response_seconds,
            max_response_seconds,
        } => [
            ("nominal_response_seconds", *nominal_response_seconds),
            ("max_response_seconds", *max_response_seconds),
        ],
        Timing::Periodic {
            nominal_rate_hz,
            max_rate_hz,
        } => [
            ("nominal_rate_hz", *nominal_rate_hz),
            ("max_rate_hz", *max_rate_hz),
        ],
    };
    for (field, value) in values {
        let Some(value) = value else { continue };
        // Finiteness first. A comparison-only check admitted `+Infinity`,
        // because infinity *is* strictly greater than zero -- but the portable
        // schema's `type: number` covers finite JSON numbers only, and an
        // infinite rate or response time is not a quantity downstream code
        // generation could mean anything by. NaN is non-finite too, so it is
        // caught here rather than relying on comparison incomparability.
        if !value.is_finite() {
            return Err(SemanticError::NonFiniteTimingValue {
                function: function.id.clone(),
                exchange: exchange.to_owned(),
                field,
                value,
            });
        }
        // Then `exclusiveMinimum: 0`. The value is known finite, so a plain
        // comparison is now exact.
        if value <= 0.0 {
            return Err(SemanticError::NonPositiveTimingValue {
                function: function.id.clone(),
                exchange: exchange.to_owned(),
                field,
                value,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_portable_identifier;

    #[test]
    fn accepts_portable_identifiers() {
        for value in [
            "a",
            "esm",
            "mission-data",
            "position_input",
            "a1-b2_c3",
            "uci25",
        ] {
            assert!(is_portable_identifier(value), "{value} should be accepted");
        }
    }

    #[test]
    fn rejects_non_portable_identifiers() {
        for value in [
            "",
            "1abc",
            "-abc",
            "_abc",
            "abc-",
            "abc_",
            "abc--d",
            "abc__d",
            "abc-_d",
            "ABC",
            "Esm",
            "esm.radar",
            "esm radar",
            "esm/radar",
            "esmé",
        ] {
            assert!(!is_portable_identifier(value), "{value} should be rejected");
        }
    }
}
