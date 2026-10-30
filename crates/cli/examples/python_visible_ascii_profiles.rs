//! Profile inventory from the authoritative classifier over the actual pinned IR.
use ams_gra_oms_codegen_core::{StringProfile, string_profile};
use ams_gra_oms_ir::{PrimitiveKind, TypeKind};
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::path::Path;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let schema = load_schema_set(Path::new(&args[1])).expect("verified pinned schema");
    let profiles: Vec<_> = schema
        .types
        .iter()
        .filter_map(|declaration| {
            if declaration.kind != TypeKind::Primitive(PrimitiveKind::String) {
                return None;
            }
            match string_profile(PrimitiveKind::String, &declaration.constraints) {
                Ok(Some(StringProfile::VisibleAscii {
                    min_length,
                    max_length,
                })) => Some(serde_json::json!({
                    "name": declaration.name.local_name, "min": min_length, "max": max_length,
                })),
                _ => None,
            }
        })
        .collect();
    println!("{}", serde_json::to_string(&profiles).unwrap());
}
