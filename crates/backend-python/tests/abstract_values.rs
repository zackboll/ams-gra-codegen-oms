//! PY-019 independently checks the shared world guard and emitted-name boundary.
use ams_gra_oms_backend_python::generate;
use ams_gra_oms_codegen_core::{GenerationWorld, TypeEmission, plan_type_emissions};
use ams_gra_oms_ir::{NamespaceDecl, QualifiedName};
use ams_gra_oms_xsd_frontend::{load_schema_document, load_schema_set_with_overlays};
use std::path::Path;

#[test]
fn zero_one_multiple_and_private_descendants_never_close_open_world() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/open-world/task068-private");
    for overlay in [false, true] {
        let schema = load_schema_set_with_overlays(
            &fixtures.join("public.xsd"),
            &if overlay {
                vec![fixtures.join("private.xsd")]
            } else {
                vec![]
            },
        )
        .unwrap();
        let emissions = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        let projection = emissions
            .iter()
            .find_map(|emission| match emission {
                TypeEmission::AbstractValue(projection) => Some(projection),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            projection.concrete_descendants.len(),
            if overlay { 2 } else { 1 }
        );
        assert!(generate(&schema, GenerationWorld::ClosedSchemaSet).is_ok());
        let failure = generate(&schema, GenerationWorld::OpenExtensions).unwrap_err();
        assert!(
            failure
                .message
                .contains("abstract value Base is not closed under open-extensions generation"),
            "{failure}"
        );
    }
    let zero = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-uninhabited-abstract-optional.xsd"),
    )
    .unwrap();
    for world in [
        GenerationWorld::ClosedSchemaSet,
        GenerationWorld::OpenExtensions,
    ] {
        let failure = generate(&zero, world).unwrap_err();
        assert!(
            failure
                .message
                .contains(if world == GenerationWorld::ClosedSchemaSet {
                    "SidecarPoint has no concrete structural descendants"
                } else {
                    "SidecarPoint is not closed under open-extensions generation"
                }),
            "{failure}"
        );
    }
}

#[test]
fn ambiguous_identity_and_factory_annotation_collisions_fail_closed() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../xsd-frontend/tests/fixtures/backend-abstract-value-reference.xsd");
    let original = load_schema_document(&fixture).unwrap();
    let mut ambiguous = original.clone();
    let mut other = ambiguous
        .types
        .iter()
        .find(|ty| ty.name.local_name == "Derived")
        .unwrap()
        .clone();
    other.name = QualifiedName::new("urn:private", "Derived");
    ambiguous.types.push(other);
    ambiguous.namespaces.push(NamespaceDecl {
        uri: "urn:private".to_owned(),
        preferred_prefix: None,
    });
    assert!(generate(&ambiguous, GenerationWorld::ClosedSchemaSet).is_err());

    let mut collision = original;
    let mut other = collision
        .types
        .iter()
        .find(|ty| ty.name.local_name == "Derived")
        .unwrap()
        .clone();
    // The wrapper's deterministic factory would hide a class-body annotation.
    other.name.local_name = "from_Derived".to_owned();
    collision.types.push(other);
    let failure = generate(&collision, GenerationWorld::ClosedSchemaSet).unwrap_err();
    assert!(
        failure
            .message
            .contains("duplicate abstract factory or ambiguous subtype identity"),
        "{failure}"
    );
}
