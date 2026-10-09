//! Research evidence through unchanged production projection/backend interfaces.
use ams_gra_oms_codegen_core::{
    AbstractValueProjectionError, AbstractValueTopology, GenerationWorld, ServiceGenerationError,
    ServiceGenerationProjection, classify_abstract_value_topology, project_abstract_value,
    project_service_generation_schema, resolve_service_plan,
};
use ams_gra_oms_ir::{NamespaceDecl, QualifiedName, SchemaIr};
use ams_gra_oms_service_contract::parse_yaml;
use ams_gra_oms_xsd_frontend::{load_schema_set, load_schema_set_with_overlays};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/open-world/task068-private")
        .join(name)
}

fn projection(
    schema: &SchemaIr,
    message: &str,
    world: GenerationWorld,
) -> Result<ServiceGenerationProjection, ServiceGenerationError> {
    let yaml = format!(
        "contract_version: '0.1'\nservice:\n  name: Research\n  version: '0.1'\n  kind: service\nstandards:\n  oms_version: '2.5'\n  uci_schema_version: '2.5'\nfunctions:\n  - id: f\n    name: Research\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: {message}\n        topic: research\n        timing:\n          kind: asynchronous\n"
    );
    let plan = resolve_service_plan(&parse_yaml(&yaml).unwrap(), schema).unwrap();
    project_service_generation_schema(&plan, schema, world)
}

fn assert_open_guard(schema: &SchemaIr, message: &str, target: QualifiedName) {
    assert_eq!(
        projection(schema, message, GenerationWorld::OpenExtensions).unwrap_err(),
        ServiceGenerationError::AbstractValue(
            AbstractValueProjectionError::NotClosedUnderOpenExtensions(target)
        )
    );
}

#[test]
fn known_private_and_zero_descendant_worlds_remain_distinct() {
    for (overlay, expected) in [
        (false, vec!["PublicA"]),
        (true, vec!["PublicA", "PrivateB"]),
    ] {
        let schema = load_schema_set_with_overlays(
            &fixture("public.xsd"),
            &if overlay {
                vec![fixture("private.xsd")]
            } else {
                vec![]
            },
        )
        .unwrap();
        let target = QualifiedName::new("urn:audit", "Base");
        let sum = project_abstract_value(&schema, &target).unwrap();
        assert_eq!(
            sum.concrete_descendants
                .iter()
                .map(|t| t.name.local_name.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        projection(&schema, "HolderReport", GenerationWorld::ClosedSchemaSet).unwrap();
        assert_open_guard(&schema, "HolderReport", target);
    }
    let zero = load_schema_set(&fixture("zero.xsd")).unwrap();
    let target = QualifiedName::new("urn:audit", "ExtensionBase");
    assert_eq!(
        project_abstract_value(&zero, &target).unwrap_err(),
        AbstractValueProjectionError::NoConcreteDescendants(target.clone())
    );
    assert_open_guard(&zero, "HolderReport", target);
}

#[test]
fn same_local_name_does_not_collapse_qnames_or_enable_backends() {
    let mut schema = load_schema_set(&fixture("public.xsd")).unwrap();
    let mut other = schema
        .types
        .iter()
        .find(|t| t.name.local_name == "PublicA")
        .unwrap()
        .clone();
    other.name = QualifiedName::new("urn:other", "PublicA");
    schema.namespaces.push(NamespaceDecl {
        uri: "urn:other".to_owned(),
        preferred_prefix: Some("other".to_owned()),
    });
    schema.types.push(other);
    schema.validate().unwrap();
    let target = QualifiedName::new("urn:audit", "Base");
    let names = project_abstract_value(&schema, &target)
        .unwrap()
        .concrete_descendants
        .iter()
        .map(|t| t.name.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            QualifiedName::new("urn:audit", "PublicA"),
            QualifiedName::new("urn:other", "PublicA")
        ]
    );
    assert_open_guard(&schema, "HolderReport", target);
    let world = GenerationWorld::ClosedSchemaSet;
    // Full-QName IR support is not multi-namespace backend support.
    assert!(ams_gra_oms_backend_ada::generate(&schema, world).is_err());
    assert!(ams_gra_oms_backend_rust::generate(&schema, world).is_err());
    assert!(ams_gra_oms_backend_cpp::generate(&schema, world).is_err());
}

#[test]
fn recursive_abstract_values_remain_a_separate_topology_blocker() {
    let mut schema = load_schema_set(&fixture("public.xsd")).unwrap();
    let holder_fields = match &schema
        .types
        .iter()
        .find(|t| t.name.local_name == "Holder")
        .unwrap()
        .kind
    {
        ams_gra_oms_ir::TypeKind::Record { fields } => fields.clone(),
        _ => panic!("record"),
    };
    let descendant = schema
        .types
        .iter_mut()
        .find(|t| t.name.local_name == "PublicA")
        .unwrap();
    let ams_gra_oms_ir::TypeKind::Record { fields } = &mut descendant.kind else {
        panic!("record")
    };
    fields.extend(holder_fields);
    schema.validate().unwrap();
    let target = QualifiedName::new("urn:audit", "Base");
    let AbstractValueTopology::RecursiveValueGraph { cycle } =
        classify_abstract_value_topology(&schema, &target).unwrap()
    else {
        panic!("recursive")
    };
    assert_eq!(
        cycle,
        vec![
            target.clone(),
            QualifiedName::new("urn:audit", "PublicA"),
            target.clone()
        ]
    );
    assert!(projection(&schema, "HolderReport", GenerationWorld::ClosedSchemaSet).is_err());
    assert_open_guard(&schema, "HolderReport", target);
}

/// Explicit opt-in: only the 47-declaration vertical, not a schema-wide campaign.
#[test]
#[ignore = "requires hash-verified AMS_GRA_TASK072_UCI_ROOT"]
fn pinned_data_record_list_management_request() {
    let root = std::env::var_os("AMS_GRA_TASK072_UCI_ROOT").expect("set verified root");
    let digest = std::process::Command::new("sha256sum")
        .arg(&root)
        .output()
        .expect("sha256sum is required for pinned evidence");
    assert!(digest.status.success());
    let digest = String::from_utf8(digest.stdout).unwrap();
    let hash = digest.split_whitespace().next().unwrap();
    assert!(matches!(
        hash,
        "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27"
            | "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b"
    ));
    let schema = load_schema_set(Path::new(&root)).unwrap();
    let message = "DataRecordListManagementRequest";
    let closed = projection(&schema, message, GenerationWorld::ClosedSchemaSet).unwrap();
    assert_eq!(closed.selected_type_names().len(), 43);
    assert_eq!(closed.generated_support_type_names().len(), 4);
    let target = QualifiedName::new(
        "https://www.vdl.afrl.af.mil/programs/oam",
        "ManagedListBaseType",
    );
    let sum = project_abstract_value(&schema, &target).unwrap();
    assert_eq!(sum.concrete_descendants.len(), 1);
    assert_eq!(
        sum.concrete_descendants[0].name.local_name,
        "ForeignKeyMapML"
    );
    let world = GenerationWorld::ClosedSchemaSet;
    ams_gra_oms_backend_ada::generate(closed.schema(), world).unwrap();
    ams_gra_oms_backend_rust::generate(closed.schema(), world).unwrap();
    ams_gra_oms_backend_cpp::generate(closed.schema(), world).unwrap();
    assert_open_guard(
        &schema,
        message,
        QualifiedName::new(
            "https://www.vdl.afrl.af.mil/programs/oam",
            "ManagedListBaseType",
        ),
    );
}
