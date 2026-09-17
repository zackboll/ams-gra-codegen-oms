//! Shared backend contract for OMS/UCI source generation.

use ams_gra_oms_ir::SchemaIr;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    pub relative_path: PathBuf,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenError {
    pub message: String,
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CodegenError {}

pub trait Backend {
    fn name(&self) -> &'static str;

    /// Generate a deterministic set of files from normalized schema IR.
    ///
    /// # Errors
    ///
    /// Returns an error if the IR contains a semantic feature that this backend
    /// cannot represent without losing required OMS/UCI meaning.
    fn generate(&self, schema: &SchemaIr) -> Result<Vec<GeneratedFile>, CodegenError>;
}
