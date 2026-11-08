//! PY-018 selected/support identities from the production projection, not a walker.
use ams_gra_oms_codegen_core::{
    GenerationWorld, project_service_generation_schema, resolve_service_plan,
};
use ams_gra_oms_service_contract::load_contract;
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::path::Path;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let schema = load_schema_set(Path::new(&args[1])).expect("pinned schema");
    let contract = load_contract(Path::new(&args[2])).expect("exact selected contract");
    let plan = resolve_service_plan(&contract, &schema).expect("resolve selection");
    let qname = |name: &ams_gra_oms_ir::QualifiedName| {
        format!("{{{}}}{}", name.namespace_uri, name.local_name)
    };
    let selected: Vec<_> = plan
        .selected_type_closure(&schema)
        .expect("selected closure")
        .iter()
        .map(|decl| qname(&decl.name))
        .collect();
    let mut worlds = serde_json::Map::new();
    for (world, label) in [
        (GenerationWorld::ClosedSchemaSet, "closed-schema"),
        (GenerationWorld::OpenExtensions, "open-extensions"),
    ] {
        let result = match project_service_generation_schema(&plan, &schema, world) {
            Ok(projection) => {
                serde_json::json!({"selected": projection.selected_type_names().iter().map(qname).collect::<Vec<_>>(),
                "support": projection.generated_support_type_names().iter().map(qname).collect::<Vec<_>>()})
            }
            Err(error) => serde_json::json!({"projection_error": error.to_string()}),
        };
        worlds.insert(label.to_owned(), result);
    }
    println!(
        "{}",
        serde_json::json!({"selected": selected, "worlds": worlds})
    );
}
