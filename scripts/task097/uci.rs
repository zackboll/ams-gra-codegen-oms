use ams_gra_oms_codegen_core::*;
use ams_gra_oms_ir::*;
use serde_json::{Value, json};
use std::{collections::{BTreeMap, BTreeSet}, fs, path::Path};

fn q(name: &QualifiedName) -> String {
    format!("{{{}}}{}", name.namespace_uri, name.local_name)
}

fn contract(name: &str, release: &str) -> ams_gra_oms_service_contract::Contract {
    ams_gra_oms_service_contract::parse_yaml(&contract_text(name, release)).unwrap()
}

fn contract_text(name: &str, release: &str) -> String {
    format!("contract_version: \"0.1\"\nservice:\n  name: task097-analysis-only\n  version: \"0.1.0\"\n  kind: service\nstandards:\n  oms_version: \"2.{release}\"\n  uci_schema_version: \"2.{release}\"\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: {name}\n        topic: task097-analysis\n        timing:\n          kind: asynchronous\n")
}

fn measure(schema: &SchemaIr, message: &str, release: &str, lossless: bool) -> Value {
    let world = GenerationWorld::ClosedSchemaSet;
    let plan = resolve_service_plan(&contract(message, release), schema).unwrap();
    let selected = plan.selected_type_closure(schema).unwrap();
    let projection = match project_service_generation_schema(&plan, schema, world) {
        Ok(p) => p,
        Err(error) => return json!({"classification":"PROJECTION_ERROR", "error":error.to_string(), "selected_count":selected.len()}),
    };
    let projected = projection.schema();
    let model = match service_api_preflight(&plan, projected, BackendLanguage::Ada, world) {
        Ok(model) => model,
        Err(error) => return json!({"classification":"API_ERROR", "error":error.to_string(), "selected_count":selected.len()}),
    };
    let model_error = ams_gra_oms_backend_ada::generate(projected, world).err().map(|e| e.to_string());
    let mut profiles = vec![("portable", AdaCodecRuntime::Portable), ("float32", AdaCodecRuntime::LinuxX86_64Glibc241)];
    // BASELINE_LOSSLESS_PROFILE
    if lossless { profiles.push(("lossless-v1", AdaCodecRuntime::LinuxX86_64Glibc241LosslessV1)); }
    let mut rows = BTreeMap::new();
    for (name, runtime) in profiles {
        let codec = analyze_service_codec_with_runtime(&model, projected, BackendLanguage::Ada, world, runtime).unwrap();
        rows.insert(name, json!({"renderable":codec.renderable_emitted_declarations,
            "emitted":codec.emitted_declarations,"first_blocker":codec.blocker.as_ref().map(ToString::to_string),
            "ready":codec.is_ready() && model_error.is_none()}));
    }
    json!({"classification":"MEASURED", "selected_count":selected.len(),
        "model_error":model_error, "emitted_names":projected.types.iter().map(|d| q(&d.name)).collect::<Vec<_>>(), "profiles":rows})
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let out = Path::new(&args[1]);
    fs::create_dir_all(out).unwrap();
    let lossless = args[4] == "current";
    for (release, path) in [("5", &args[2]), ("6", &args[3])] {
        // Exactly one normalized production parse per root per source variant.
        let schema = ams_gra_oms_xsd_frontend::load_schema_set(Path::new(path)).unwrap();
        let named: BTreeMap<_, _> = schema.types.iter().filter(|d| matches!(d.kind, TypeKind::Primitive(PrimitiveKind::Float64)))
            .map(|d| (d.name.clone(), d)).collect();
        let mut seeds = BTreeSet::new();
        let declarations: Vec<_> = named.values().map(|d| {
            let domain = floating_domain(PrimitiveKind::Float64, &d.constraints);
            if domain.is_ok() { seeds.insert(d.name.clone()); }
            json!({"name":q(&d.name), "facets":format!("{:?}",d.constraints), "domain":format!("{domain:?}"),"admitted_named_domain":domain.is_ok()})
        }).collect();
        let mut fields = Vec::new();
        for owner in &schema.types {
            let members = match &owner.kind {
                TypeKind::Record { .. } => effective_record_fields(&schema, &owner.name).unwrap(),
                TypeKind::Choice { .. } => effective_choice_alternatives(&schema, &owner.name).unwrap(),
                _ => continue,
            };
            for field in members {
                let direct = field.type_ref.target == TypeRefTarget::Primitive(PrimitiveKind::Float64);
                let is_named = matches!(&field.type_ref.target, TypeRefTarget::Named(n) if named.contains_key(n));
                if !direct && !is_named { continue; }
                let choice = matches!(owner.kind, TypeKind::Choice { .. });
                if direct && !choice && field.constraints == Default::default() { seeds.insert(owner.name.clone()); }
                fields.push(json!({"owner":q(&owner.name),"field":field.name,"direct":direct,"named":is_named,
                    "choice_scalar_rejected":choice,"cardinality":format!("{:?}",field.cardinality),
                    "facets":format!("{:?}",field.constraints), "source":format!("{:?}",field.source)}));
            }
        }
        let exposures = static_dependency_exposures(&schema, &seeds).unwrap();
        let messages: BTreeSet<_> = exposures.iter().map(|e|e.message.clone()).collect();
        let inventory = json!({"root":path,"parse_count":1,"declarations":declarations,"effective_fields":fields,
            "static_messages":messages.iter().map(q).collect::<Vec<_>>(),
            "witnesses":exposures.iter().map(|e|json!({"message":q(&e.message),"seed":q(&e.seed),"witness":e.witness.iter().map(q).collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "scope":"Static schema reachability; NOT evidence of deployed Service Contract selection"});
        fs::write(out.join(format!("2{release}-inventory.json")), serde_json::to_string_pretty(&inventory).unwrap()+"\n").unwrap();
        let mut ranked = Vec::new();
        for message in messages {
            let plan = resolve_service_plan(&contract(&message.local_name, release), &schema).unwrap();
            ranked.push((plan.selected_type_closure(&schema).unwrap().len(),message));
        }
        ranked.sort();
        let mut selected: Vec<_> = ranked.iter().take(6).map(|(_, q)|q.local_name.clone()).collect();
        for control in ["DLZ", "Log", "RDMA_InitializeSetup", "SurvivabilityRiskLevel"] {
            if schema.messages.iter().any(|m|m.name.local_name==control) && !selected.iter().any(|n|n==control) {
                selected.push(control.to_owned());
            }
        }
        let mut rows = Vec::new();
        for name in selected {
            let result = measure(&schema, &name, release, lossless);
            fs::write(out.join(format!("2{release}-{name}.yaml")),contract_text(&name,release)).unwrap();
            rows.push(json!({"message":name,"scope":"unchanged real message + recorded analysis-only contract", "outcome":result}));
            println!("TASK097_UCI_SELECTION release=2.{release} message={name}");
        }
        if let Some(owner) = schema.types.iter().find(|d|d.name.local_name=="ScanDataBaseType") {
            let owner_seeds = BTreeSet::from([owner.name.clone()]);
            let reaching = static_dependency_exposures(&schema, &owner_seeds).unwrap();
            let mut owner_schema = schema.clone();
            owner_schema.messages.push(MessageDecl { name: QualifiedName::new(&owner.name.namespace_uri,"Task097Owner"),
                payload_type:TypeRef::named(owner.name.clone()),documentation:None,source:owner.source.clone() });
            let result = measure(&owner_schema,"Task097Owner",release,lossless);
            rows.push(json!({"owner":"ScanDataBaseType","scope":"SYNTHETIC OWNER-ONLY PROJECTION, NOT message/service selection", "outcome":result,
                "actual_static_reaching_messages":reaching.iter().map(|e|q(&e.message)).collect::<BTreeSet<_>>()}));
        }
        fs::write(out.join(format!("2{release}-selections.json")),serde_json::to_string_pretty(&rows).unwrap()+"\n").unwrap();
    }
    fs::write(out.join("result.json"),"{\"completed\":true,\"root_parses\":2}\n").unwrap();
    println!("TASK097_UCI_COMPLETE variant={}",args[4]);
}