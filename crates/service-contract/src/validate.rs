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
    non_empty("standards", "oms_version", &contract.standards.oms_version)?;
    non_empty(
        "standards",
        "uci_schema_version",
        &contract.standards.uci_schema_version,
    )?;
    for extension in &contract.standards.uci_extension_schemas {
        non_empty("standards", "uci_extension_schemas entry", extension)?;
    }

    let mut source_ids = BTreeSet::new();
    for source in &contract.sources {
        identifier("source", &source.id)?;
        non_empty(&format!("source '{}'", source.id), "title", &source.title)?;
        non_empty(&format!("source '{}'", source.id), "uri", &source.uri)?;
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

    let mut function_ids = BTreeSet::new();
    for function in &contract.functions {
        identifier("function", &function.id)?;
        let context = format!("function '{}'", function.id);
        non_empty(&context, "name", &function.name)?;
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
            }
            Exchange::DataTransfer(transfer) => {
                non_empty(&context, "name", &transfer.name)?;
                non_empty(&context, "protocol", &transfer.protocol)?;
                non_empty(&context, "data_type", &transfer.data_type)?;
                non_empty(&context, "data_format", &transfer.data_format)?;
                non_empty(&context, "sharing_pattern", &transfer.sharing_pattern)?;
            }
            Exchange::SpecialSignal(signal) => non_empty(&context, "name", &signal.name)?,
            Exchange::SecurityExchange(security) => non_empty(&context, "name", &security.name)?,
            Exchange::NonOmsMessage(message) => non_empty(&context, "name", &message.name)?,
        }

        validate_timing(function, id, exchange.timing())?;
        validate_traceability(&context, exchange.traceability(), source_ids)?;
    }
    Ok(())
}

/// Reject non-positive declared timing values.
///
/// The portable schema declares `exclusiveMinimum: 0` on every numeric timing
/// value, so zero, negative, and NaN rates or response times are rejected
/// rather than passed through to code generation as meaningless quantities.
/// Absent values stay absent: "unspecified" is legal and is not defaulted.
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
        // Explicitly stated as "not strictly greater than zero", spelled via
        // `partial_cmp` so NaN -- which compares as incomparable rather than
        // as less-than -- is rejected along with zero and negatives.
        if let Some(value) = value
            && value.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
        {
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
