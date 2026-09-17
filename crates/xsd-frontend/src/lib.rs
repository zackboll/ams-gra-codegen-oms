//! XSD frontend for OMS/UCI schemas.
//!
//! Bootstrap status: the API boundary is established, but full XSD parsing is
//! intentionally not implemented until it is tested against the authoritative
//! UCI schema corpus.

use ams_gra_oms_ir::SchemaIr;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontendError {
    NotImplemented,
    InvalidInput(String),
}

impl fmt::Display for FrontendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotImplemented => write!(f, "full UCI XSD parsing is not implemented yet"),
            Self::InvalidInput(message) => write!(f, "invalid schema input: {message}"),
        }
    }
}

impl std::error::Error for FrontendError {}

/// Parse and normalize an authoritative OMS/UCI schema tree into semantic IR.
///
/// # Errors
///
/// Returns an explicit error until the first tested XSD vertical slice is
/// implemented. Unsupported schema constructs must remain hard errors rather
/// than being silently dropped.
pub fn load_schema_set(_root: &Path) -> Result<SchemaIr, FrontendError> {
    Err(FrontendError::NotImplemented)
}
