use ams_gra_oms_codegen_core::*;
use ams_gra_oms_ir::*;
use ams_gra_oms_service_contract::parse_yaml;
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::{collections::BTreeMap, path::Path, time::Instant};
fn plan(s: &SchemaIr, m: &str) -> ServicePlan {
    let c=parse_yaml(&format!("contract_version: \"0.1\"\nservice:\n  name: Audit\n  version: \"0.1\"\n  kind: service\nstandards:\n  oms_version: \"2.5\"\n  uci_schema_version: \"2.5\"\nfunctions:\n  - id: f1\n    name: Audit\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e1\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: {m}\n        topic: audit\n        timing:\n          kind: asynchronous\n")).unwrap();
    resolve_service_plan(&c, s).unwrap()
}
fn main() {
    let start = Instant::now();
    for (rel, file) in [
        ("2.5", "/tmp/task068/manifests/uci25-root.txt"),
        ("2.6", "/tmp/task068/manifests/uci26-root.txt"),
    ] {
        let root = std::fs::read_to_string(file).unwrap();
        let s = load_schema_set(Path::new(root.trim())).unwrap();
        let a = CoverageAnalysis::new(&s, GenerationWorld::OpenExtensions).unwrap();
        for l in [
            BackendLanguage::Ada,
            BackendLanguage::Rust,
            BackendLanguage::Cpp,
        ] {
            println!("COVERAGE\t{rel}\t{:?}", a.backend_coverage(l).unwrap());
            let renderable = a.renderable_message_closure_names(l).unwrap();
            let mut blocked = s
                .messages
                .iter()
                .filter(|m| !renderable.contains(&m.name))
                .map(|m| m.name.local_name.as_str())
                .collect::<Vec<_>>();
            blocked.sort();
            println!("CLOSURE_FAILED\t{rel}\t{l:?}\t{}", blocked.join(","));
        }
        let mut messages = s.messages.iter().collect::<Vec<_>>();
        messages.sort_by_key(|m| &m.name);
        let mut targets = BTreeMap::new();
        let mut count = 0;
        for m in messages {
            let p = plan(&s, &m.name.local_name);
            match project_service_generation_schema(&p, &s, GenerationWorld::OpenExtensions) {
                Err(ServiceGenerationError::AbstractValue(
                    AbstractValueProjectionError::NotClosedUnderOpenExtensions(t),
                )) => {
                    count += 1;
                    let selected = p.selected_type_closure(&s).unwrap().len();
                    let closed =
                        project_service_generation_schema(&p, &s, GenerationWorld::ClosedSchemaSet);
                    let support = closed
                        .as_ref()
                        .map(|c| c.generated_support_type_names().len());
                    println!(
                        "FAIL\t{rel}\t{}\t{}\t{selected}\t{support:?}\t{:?}",
                        m.name.local_name,
                        t.local_name,
                        closed.as_ref().err()
                    );
                    targets.insert(t.clone(), ());
                }
                Err(e) => println!("OTHER\t{rel}\t{}\t{e:?}", m.name.local_name),
                Ok(_) => {}
            }
        }
        println!("COUNT\t{rel}\t{count}");
        for t in targets.keys() {
            let top = classify_abstract_value_topology(&s, t);
            println!("TARGET\t{rel}\t{}\t{top:?}", t.local_name);
        }
    }
    eprintln!("elapsed {:?}", start.elapsed());
}
