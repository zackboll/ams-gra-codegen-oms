//! Print a standalone Python model from the existing real Track XSD fixture.
use ams_gra_oms_backend_python::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../xsd-frontend/tests/fixtures/track.xsd"),
    )?;
    print!("{}", generate(&schema, GenerationWorld::ClosedSchemaSet)?);
    Ok(())
}
