//! PY-009 evidence helper: use the existing resolver, never reconstruct closure.
use ams_gra_oms_codegen_core::resolve_service_plan;
use ams_gra_oms_service_contract::load_contract;
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::path::Path;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let schema = load_schema_set(Path::new(&args[1])).expect("verified pinned schema");
    let contract = load_contract(Path::new(&args[2])).expect("pinned service contract");
    let plan = resolve_service_plan(&contract, &schema).expect("resolve exact selection");
    let closure = plan
        .selected_type_closure(&schema)
        .expect("selected closure");
    let names: Vec<_> = closure
        .iter()
        .map(|d| format!("{{{}}}{}", d.name.namespace_uri, d.name.local_name))
        .collect();
    println!("{}", serde_json::to_string(&names).unwrap());
}
