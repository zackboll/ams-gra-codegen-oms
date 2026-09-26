//! The generic global-element OMS JSON envelope (OMSC-SPC-013 Rev B §6.1.1).
//!
//! The runtime -- not the payload codec -- owns this layer, because it
//! depends only on the message QName, never on the payload type:
//!
//! ```text
//! { "<global element key>": <payload JSON> }
//! ```
//!
//! One formatter, [`oms_global_element_name`], produces the key for all
//! three uses: the outgoing top-level member, the expected incoming member,
//! and the OWP `SUB` message name.

use serde_json::{Map, Value};
use std::fmt;

/// The OAM namespace whose global elements are named by local name alone.
pub const OAM_NAMESPACE: &str = "https://www.vdl.afrl.af.mil/programs/oam";

/// The LA-CAL name of a global element: the JSON member key of its OMS JSON
/// text and its OWP `SUB` message name.
///
/// OMSC-SPC-013 Rev B §6.1.1 (and the `SUB` syntax in §5.1.1.1): `{name}`
/// when `{target namespace}` is exactly [`OAM_NAMESPACE`], otherwise
/// `{{target namespace}}{name}`. The rule is applied literally; an empty
/// namespace yields `{}name`.
#[must_use]
pub fn oms_global_element_name(namespace: &str, local_name: &str) -> String {
    if namespace == OAM_NAMESPACE {
        local_name.to_owned()
    } else {
        format!("{{{namespace}}}{local_name}")
    }
}

/// Wrap an encoded payload as the one-member OMS JSON object keyed by
/// `element_name` (an [`oms_global_element_name`]).
#[must_use]
pub fn wrap_global_element(element_name: String, payload: Value) -> Value {
    let mut object = Map::with_capacity(1);
    object.insert(element_name, payload);
    Value::Object(object)
}

/// Parse raw OMS JSON text and return the payload of the one expected
/// global element.
///
/// The text must be a JSON object with **exactly one** member whose key
/// equals `expected_element_name`. Anything else is rejected; the message
/// identity is never inferred from the JSON shape.
///
/// # Errors
///
/// Returns the [`MessageDecodeError`] envelope variant describing the
/// violation.
pub fn unwrap_global_element(
    expected_element_name: &str,
    raw: &str,
) -> Result<Value, MessageDecodeError> {
    let value: Value = serde_json::from_str(raw).map_err(MessageDecodeError::InvalidJson)?;
    let Value::Object(object) = value else {
        return Err(MessageDecodeError::NotAnObject);
    };
    if object.len() != 1 {
        return Err(MessageDecodeError::MemberCount {
            found: object.len(),
        });
    }
    let (key, payload) = object
        .into_iter()
        .next()
        .ok_or(MessageDecodeError::MemberCount { found: 0 })?;
    if key != expected_element_name {
        return Err(MessageDecodeError::UnexpectedMember {
            expected: expected_element_name.to_owned(),
            found: key,
        });
    }
    Ok(payload)
}

/// Why an incoming `MSG` was not handed to its typed handler.
#[derive(Debug)]
#[non_exhaustive]
pub enum MessageDecodeError {
    /// The `MSG` text is not JSON.
    InvalidJson(serde_json::Error),
    /// The JSON text is not an object.
    NotAnObject,
    /// The object does not have exactly one member.
    MemberCount { found: usize },
    /// The one member is not the subscribed global element.
    UnexpectedMember { expected: String, found: String },
    /// The payload codec rejected the member's value.
    Codec(crate::CodecError),
}

impl fmt::Display for MessageDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(error) => write!(f, "MSG payload is not JSON: {error}"),
            Self::NotAnObject => f.write_str("OMS JSON text is not an object"),
            Self::MemberCount { found } => write!(
                f,
                "OMS JSON object must have exactly one member, found {found}"
            ),
            Self::UnexpectedMember { expected, found } => write!(
                f,
                "OMS JSON member {found:?} is not the subscribed global element {expected:?}"
            ),
            Self::Codec(error) => write!(f, "payload codec rejected the message: {error}"),
        }
    }
}

impl std::error::Error for MessageDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidJson(error) => Some(error),
            Self::Codec(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn oam_namespace_uses_the_local_name() {
        assert_eq!(
            oms_global_element_name(OAM_NAMESPACE, "PositionReport"),
            "PositionReport"
        );
    }

    #[test]
    fn other_namespaces_use_clark_notation() {
        assert_eq!(
            oms_global_element_name("urn:test", "MessageA"),
            "{urn:test}MessageA"
        );
        // Near-misses of the OAM URI are ordinary namespaces.
        assert_eq!(
            oms_global_element_name("https://www.vdl.afrl.af.mil/programs/oam/", "M"),
            "{https://www.vdl.afrl.af.mil/programs/oam/}M"
        );
        assert_eq!(
            oms_global_element_name("http://www.vdl.afrl.af.mil/programs/oam", "M"),
            "{http://www.vdl.afrl.af.mil/programs/oam}M"
        );
        assert_eq!(oms_global_element_name("", "M"), "{}M");
    }

    #[test]
    fn wrap_produces_exactly_one_member() {
        let wrapped = wrap_global_element("{urn:test}MessageB".into(), json!({"Count": 7}));
        assert_eq!(
            serde_json::to_string(&wrapped).expect("serialize"),
            r#"{"{urn:test}MessageB":{"Count":7}}"#
        );
    }

    #[test]
    fn unwrap_accepts_only_the_expected_single_member() {
        let expected = "{urn:test}MessageA";
        assert_eq!(
            unwrap_global_element(expected, r#"{"{urn:test}MessageA":{"Count":42}}"#)
                .expect("expected member"),
            json!({"Count": 42})
        );
        for (raw, check) in [
            ("{}", "MemberCount { found: 0 }"),
            (
                r#"{"{urn:test}MessageA":{},"{urn:test}MessageB":{}}"#,
                "MemberCount { found: 2 }",
            ),
            (r#"{"MessageA":{}}"#, "UnexpectedMember"),
            (r#"{"{urn:test}MessageB":{}}"#, "UnexpectedMember"),
            (r#"{"{urn:other}MessageA":{}}"#, "UnexpectedMember"),
            ("[1]", "NotAnObject"),
            ("not json", "InvalidJson"),
        ] {
            let error = unwrap_global_element(expected, raw).expect_err(raw);
            assert!(format!("{error:?}").contains(check), "{raw}: {error:?}");
        }
    }
}
