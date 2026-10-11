//! Independent opt-in Task 073 evidence; no Task 072 source dependency.
use ams_gra_oms_codegen_core::{
    AbstractValueProjectionError, GenerationWorld, ServiceGenerationError, project_abstract_value,
    project_service_generation_schema, resolve_service_plan,
};
use ams_gra_oms_ir::QualifiedName;
use ams_gra_oms_service_contract::parse_yaml;
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires hash-verified AMS_GRA_TASK073_UCI_ROOT"]
fn pinned_vertical_keeps_closed_admission_and_open_guard() {
    let root = std::env::var_os("AMS_GRA_TASK073_UCI_ROOT").expect("set pinned root");
    let digest = Command::new("sha256sum").arg(&root).output().unwrap();
    assert!(digest.status.success());
    let digest = String::from_utf8(digest.stdout).unwrap();
    let release = match digest.split_whitespace().next().unwrap() {
        "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27" => "2.5",
        "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b" => "2.6",
        other => panic!("unrecognized root hash {other}"),
    };
    let schema = load_schema_set(Path::new(&root)).unwrap();
    let contract = parse_yaml(&format!(
        "contract_version: \"0.1\"\nservice:\n  name: AuthorityAudit\n  version: \"0.1\"\n  kind: service\nstandards:\n  oms_version: \"2.5\"\n  uci_schema_version: \"{release}\"\nfunctions:\n  - id: f\n    name: Audit\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: DataRecordListManagementRequest\n        topic: audit\n        timing:\n          kind: asynchronous\n"
    )).unwrap();
    let plan = resolve_service_plan(&contract, &schema).unwrap();
    let closed =
        project_service_generation_schema(&plan, &schema, GenerationWorld::ClosedSchemaSet)
            .unwrap();
    assert_eq!(closed.selected_type_names().len(), 43);
    assert_eq!(closed.generated_support_type_names().len(), 4);
    let base = QualifiedName::new(
        "https://www.vdl.afrl.af.mil/programs/oam",
        "ManagedListBaseType",
    );
    let projection = project_abstract_value(&schema, &base).unwrap();
    assert_eq!(projection.concrete_descendants.len(), 1);
    assert_eq!(
        projection.concrete_descendants[0].name.local_name,
        "ForeignKeyMapML"
    );
    assert!(matches!(
        project_service_generation_schema(&plan, &schema, GenerationWorld::OpenExtensions),
        Err(ServiceGenerationError::AbstractValue(AbstractValueProjectionError::NotClosedUnderOpenExtensions(q))) if q == base
    ));
    ams_gra_oms_backend_rust::generate(closed.schema(), GenerationWorld::ClosedSchemaSet).unwrap();
    ams_gra_oms_backend_ada::generate(closed.schema(), GenerationWorld::ClosedSchemaSet).unwrap();
    ams_gra_oms_backend_cpp::generate(closed.schema(), GenerationWorld::ClosedSchemaSet).unwrap();
}
