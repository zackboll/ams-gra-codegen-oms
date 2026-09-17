//! Rust backend bootstrap.

use ams_gra_oms_codegen_core::{Backend, CodegenError, GeneratedFile};
use ams_gra_oms_ir::SchemaIr;

#[derive(Debug, Default, Clone, Copy)]
pub struct RustBackend;

impl Backend for RustBackend {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn generate(&self, _schema: &SchemaIr) -> Result<Vec<GeneratedFile>, CodegenError> {
        Ok(Vec::new())
    }
}
