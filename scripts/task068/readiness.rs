use ams_gra_oms_codegen_core::*;
use ams_gra_oms_ir::*;
use ams_gra_oms_service_contract::parse_yaml;
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::path::Path;
fn main() {
    for (rel, file) in [
        ("2.5", "/tmp/task068/manifests/uci25-root.txt"),
        ("2.6", "/tmp/task068/manifests/uci26-root.txt"),
    ] {
        let root = std::fs::read_to_string(file).unwrap();
        let s = load_schema_set(Path::new(root.trim())).unwrap();
        let initial =
            std::fs::read_to_string("/tmp/task068/evidence/inventory-initial.txt").unwrap();
        let names = initial
            .lines()
            .filter(|l| l.starts_with(&format!("FAIL\t{rel}\t")))
            .map(|l| l.split('\t').nth(2).unwrap())
            .collect::<Vec<_>>();
        let mut yaml = format!(
            "contract_version: \"0.1\"\nservice:\n  name: Audit\n  version: \"0.1\"\n  kind: service\nstandards:\n  oms_version: \"2.5\"\n  uci_schema_version: \"2.5\"\nfunctions:\n  - id: f1\n    name: Audit\n    category: specific\n    applicability: applicable\n    exchanges:\n"
        );
        for (i, m) in names.iter().enumerate() {
            yaml += &format!(
                "      - id: e{i}\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: {m}\n        topic: audit\n        timing:\n          kind: asynchronous\n"
            );
        }
        let c = parse_yaml(&yaml).unwrap();
        let p = resolve_service_plan(&c, &s).unwrap();
        for l in [
            BackendLanguage::Ada,
            BackendLanguage::Rust,
            BackendLanguage::Cpp,
        ] {
            let r = analyze_service_readiness(&p, &s, l, GenerationWorld::OpenExtensions).unwrap();
            println!(
                "{rel}\t{l:?}\tready={}\tblocked={}\tselected={}\trenderable={}\tsupport={}\tapi={:?}\tbackend={:?}",
                r.is_ready(),
                r.blocked_messages.len(),
                r.selected_messages_total,
                r.selected_messages_renderable,
                r.generated_support_types_total,
                r.service_api_blocker,
                r.backend_blocker
            );
            for b in r.blocked_messages {
                println!(
                    "BLOCK\t{rel}\t{l:?}\t{}\t{:?}",
                    b.message_name.local_name, b.blocker
                );
            }
        }
    }
}
