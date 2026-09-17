//! C++ backend bootstrap.

use ams_gra_oms_codegen_core::{Backend, CodegenError, GeneratedFile};
use ams_gra_oms_ir::SchemaIr;

#[derive(Debug, Default, Clone, Copy)]
pub struct CppBackend;

impl Backend for CppBackend {
    fn name(&self) -> &'static str {
        "cpp"
    }

    fn generate(&self, _schema: &SchemaIr) -> Result<Vec<GeneratedFile>, CodegenError> {
        Ok(Vec::new())
    }
}
