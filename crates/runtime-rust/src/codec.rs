//! The typed payload ⇄ JSON seam (Task 049 boundary; Task 050 populates it).
//!
//! A codec provider converts ONE payload value to and from the JSON value of
//! its global element -- the payload body only. It never sees the global
//! element key, the topic, or any OWP framing; the runtime owns those.
//! Generated model types are not required to implement Serde.

use serde_json::Value;
use std::error::Error;
use std::fmt;

/// Encode and decode the JSON body of payload type `P`.
///
/// One provider type may implement this for many payload types
/// (`OmsJsonCodec<A>`, `OmsJsonCodec<B>`, ...). The provider is shared with
/// the runtime's worker thread, hence `Send + Sync + 'static`.
pub trait OmsJsonCodec<P>: Send + Sync + 'static {
    /// The JSON value of `value`, without the global-element wrapper.
    ///
    /// # Errors
    ///
    /// A [`CodecError`] when `value` cannot be represented.
    fn encode_payload(&self, value: &P) -> Result<Value, CodecError>;

    /// The typed payload of a global element's JSON value (the wrapper
    /// already removed and checked by the runtime).
    ///
    /// # Errors
    ///
    /// A [`CodecError`] when `value` is not a valid `P`.
    fn decode_payload(&self, value: &Value) -> Result<P, CodecError>;
}

/// A payload codec failure, with an optional underlying cause.
#[derive(Debug)]
pub struct CodecError {
    message: String,
    source: Option<Box<dyn Error + Send + Sync + 'static>>,
}

impl CodecError {
    /// A failure described by `message`.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    /// A failure described by `message`, caused by `source`.
    #[must_use]
    pub fn with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// The failure description.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for CodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec_error_preserves_its_source() {
        let cause = serde_json::from_str::<Value>("{").expect_err("invalid");
        let error = CodecError::with_source("bad payload", cause);
        assert_eq!(error.to_string(), "bad payload");
        assert!(error.source().is_some());
        assert!(CodecError::new("x").source().is_none());
    }
}
