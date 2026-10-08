//! Read-only deployment projection of resolved OMS exchange occurrences.
//! No message resolution, backend readiness, or runtime policy is performed.

use ams_gra_oms_codegen_core::{
    OAM_NAMESPACE, ResolvedExchange, ServiceApiOmsOperation, ServicePlan,
};
use ams_gra_oms_service_contract::{Direction, Mandate};
use std::fmt::{self, Write};

/// One authored OMS exchange, in contract order; group presence is retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteOccurrence {
    pub function_id: String,
    pub exchange_id: String,
    pub direction: Direction,
    pub operation: ServiceApiOmsOperation,
    pub mandate: Mandate,
    pub topic: String,
    pub message_namespace: String,
    pub message_local_name: String,
    pub subscription_group: Option<String>,
}

/// Exact message identity. Namespaces are never discarded during aggregation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteMessage {
    pub namespace: String,
    pub local_name: String,
}

/// One distinct topic and its distinct messages, in first-occurrence order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicRoutes {
    pub topic: String,
    pub messages: Vec<RouteMessage>,
}

/// OMS-only inventory, not necessarily the complete service interface.
/// Occurrences are authoritative; aggregation is derived, never caller-supplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteManifest {
    occurrences: Vec<RouteOccurrence>,
    excluded_non_oms_occurrences: usize,
}

impl RouteManifest {
    pub fn occurrences(&self) -> &[RouteOccurrence] {
        &self.occurrences
    }

    pub fn excluded_non_oms_occurrences(&self) -> usize {
        self.excluded_non_oms_occurrences
    }

    pub fn topics(&self) -> Vec<TopicRoutes> {
        let mut topics: Vec<TopicRoutes> = Vec::new();
        for route in &self.occurrences {
            let index = topics.iter().position(|t| t.topic == route.topic);
            let index = index.unwrap_or_else(|| {
                topics.push(TopicRoutes {
                    topic: route.topic.clone(),
                    messages: Vec::new(),
                });
                topics.len() - 1
            });
            let message = RouteMessage {
                namespace: route.message_namespace.clone(),
                local_name: route.message_local_name.clone(),
            };
            if !topics[index].messages.contains(&message) {
                topics[index].messages.push(message);
            }
        }
        topics
    }
}

pub fn build_route_manifest(plan: &ServicePlan) -> RouteManifest {
    let mut manifest = RouteManifest {
        occurrences: Vec::new(),
        excluded_non_oms_occurrences: 0,
    };
    for function in &plan.functions {
        for exchange in &function.exchanges {
            let ResolvedExchange::OmsMessage(exchange) = exchange else {
                manifest.excluded_non_oms_occurrences += 1;
                continue;
            };
            manifest.occurrences.push(RouteOccurrence {
                function_id: function.id.clone(),
                exchange_id: exchange.id.clone(),
                direction: exchange.direction,
                operation: ServiceApiOmsOperation::for_direction(exchange.direction),
                mandate: exchange.mandate,
                topic: exchange.topic.clone(),
                message_namespace: exchange.message_name.namespace_uri.clone(),
                message_local_name: exchange.message_name.local_name.clone(),
                subscription_group: exchange.subscription_group.clone(),
            });
        }
    }
    manifest
}

pub const TSV_HEADER: &str = "function_id\texchange_id\tdirection\toperation\tmandate\ttopic\tmessage_namespace\tmessage_local_name\thas_subscription_group\tsubscription_group\n";

fn escape_tsv(value: &str) -> String {
    let mut result = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '\t' => result.push_str("\\t"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            c if c.is_control() => {
                write!(result, "\\u{{{:x}}}", c as u32).unwrap();
            }
            c => result.push(c),
        }
    }
    result
}

pub fn render_route_tsv(manifest: &RouteManifest) -> String {
    let mut output = TSV_HEADER.to_owned();
    for route in manifest.occurrences() {
        let values = [
            route.function_id.as_str(),
            route.exchange_id.as_str(),
            route.direction.as_str(),
            route.operation.as_str(),
            route.mandate.as_str(),
            route.topic.as_str(),
            route.message_namespace.as_str(),
            route.message_local_name.as_str(),
            if route.subscription_group.is_some() {
                "true"
            } else {
                "false"
            },
            route.subscription_group.as_deref().unwrap_or(""),
        ];
        output.push_str(&values.map(escape_tsv).join("\t"));
        output.push('\n');
    }
    output
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteError {
    InvalidServiceId(String),
    InvalidServiceUuid(String),
    InvalidTopic {
        function: String,
        exchange: String,
        topic: String,
    },
    InvalidMessage {
        function: String,
        exchange: String,
        message: String,
    },
    UnsupportedNamespace {
        function: String,
        exchange: String,
        namespace: String,
    },
}

impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidServiceId(id) => write!(
                f,
                "invalid Sleet service_id {id:?}; expected [A-Za-z0-9_.-]+"
            ),
            Self::InvalidServiceUuid(uuid) => write!(f, "invalid Sleet service_uuid {uuid:?}"),
            Self::InvalidTopic {
                function,
                exchange,
                topic,
            } => write!(
                f,
                "function {function:?} exchange {exchange:?}: invalid Sleet topic {topic:?}; expected [A-Za-z0-9_.-]+"
            ),
            Self::InvalidMessage {
                function,
                exchange,
                message,
            } => write!(
                f,
                "function {function:?} exchange {exchange:?}: invalid Sleet message identifier {message:?}; expected [A-Za-z0-9_.-]+"
            ),
            Self::UnsupportedNamespace {
                function,
                exchange,
                namespace,
            } => write!(
                f,
                "function {function:?} exchange {exchange:?}: unsupported Sleet message namespace {namespace:?}; only exact OAM namespace {OAM_NAMESPACE:?} is supported"
            ),
        }
    }
}

impl std::error::Error for RouteError {}

/// Exact pinned sleet-types/src/owp.rs identifier class (e38f61d).
pub fn is_sleet_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}

pub fn validate_service_identity(service_id: &str, service_uuid: &str) -> Result<(), RouteError> {
    if !is_sleet_identifier(service_id) {
        return Err(RouteError::InvalidServiceId(service_id.to_owned()));
    }
    uuid::Uuid::parse_str(service_uuid)
        .map_err(|_| RouteError::InvalidServiceUuid(service_uuid.to_owned()))?;
    Ok(())
}

// All emitted strings have passed the ASCII identifier or UUID parser gates.
// UUID input is canonicalized so alternate accepted UUID spellings remain safe.
fn quoted(value: &str) -> String {
    format!("\"{value}\"")
}

/// Sleet permissions authorize BOTH PUB and SUB, not direction-specific policy.
/// Validation completes before any output is returned. Every topic gets one
/// nonempty binding: Sleet's permissive missing-binding fallback is never used.
pub fn render_sleet_toml(
    manifest: &RouteManifest,
    service_id: &str,
    service_uuid: &str,
) -> Result<String, RouteError> {
    validate_service_identity(service_id, service_uuid)?;
    for route in manifest.occurrences() {
        if !is_sleet_identifier(&route.topic) {
            return Err(RouteError::InvalidTopic {
                function: route.function_id.clone(),
                exchange: route.exchange_id.clone(),
                topic: route.topic.clone(),
            });
        }
        if route.message_namespace != OAM_NAMESPACE {
            return Err(RouteError::UnsupportedNamespace {
                function: route.function_id.clone(),
                exchange: route.exchange_id.clone(),
                namespace: route.message_namespace.clone(),
            });
        }
        if !is_sleet_identifier(&route.message_local_name) {
            return Err(RouteError::InvalidMessage {
                function: route.function_id.clone(),
                exchange: route.exchange_id.clone(),
                message: route.message_local_name.clone(),
            });
        }
    }
    let topics = manifest.topics();
    let uuid = uuid::Uuid::parse_str(service_uuid)
        .expect("validated UUID")
        .to_string();
    let mut output = format!(
        "service_id = {}\nservice_uuid = {}\nallowed_topics = [{}]\n",
        quoted(service_id),
        quoted(&uuid),
        topics
            .iter()
            .map(|t| quoted(&t.topic))
            .collect::<Vec<_>>()
            .join(", ")
    );
    for topic in topics {
        write!(
            output,
            "\n[[topic_bindings]]\ntopic = {}\nallowed_messages = [{}]\n",
            quoted(&topic.topic),
            topic
                .messages
                .iter()
                .map(|m| quoted(&m.local_name))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
    }
    Ok(output)
}
