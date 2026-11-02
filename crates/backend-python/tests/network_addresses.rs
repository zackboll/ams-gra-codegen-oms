//! Admission and ancestry tests against normalized IR, independently of XSD projection.
use ams_gra_oms_backend_python::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_ir::{ConstraintSet, PrimitiveKind, TypeKind, TypeRef};
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::path::Path;

fn fixture() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/python-network-addresses/oam.xsd"),
    )
    .unwrap()
}

#[test]
fn profile_ancestry_is_proven_at_every_edge_and_aliases_remain_unsupported() {
    let original = fixture();
    let ipv4 = original
        .types
        .iter()
        .position(|d| d.name.local_name == "IPv4_AddressType")
        .unwrap();
    let ipv6 = original
        .types
        .iter()
        .position(|d| d.name.local_name == "IPv6_AddressType")
        .unwrap();
    let leaf = original
        .types
        .iter()
        .position(|d| d.name.local_name == "Leaf")
        .unwrap();
    for world in [
        GenerationWorld::ClosedSchemaSet,
        GenerationWorld::OpenExtensions,
    ] {
        let source = generate(&original, world).unwrap();
        assert!(source.contains("def _check_ipv4("));
        assert!(source.contains("def _check_ipv6("));
        assert!(
            source.len() < 25_000,
            "Cartesian branches must not be rendered"
        );
        for mutation in 0..7 {
            let mut schema = original.clone();
            match mutation {
                0 => {
                    schema.types[leaf].base_type =
                        Some(TypeRef::named(schema.types[ipv6].name.clone()))
                }
                1 => schema.types[ipv4].constraints = ConstraintSet::default(),
                2 => schema.types[ipv4].is_abstract = true,
                3 => {
                    schema.types[leaf].base_type =
                        Some(TypeRef::named(schema.types[leaf].name.clone()))
                }
                4 => {
                    schema.types[leaf].base_type = Some(TypeRef::primitive(PrimitiveKind::Boolean))
                }
                5 => {
                    schema.types[leaf].kind =
                        TypeKind::Alias(TypeRef::named(schema.types[ipv4].name.clone()));
                    schema.types[leaf].constraints = ConstraintSet::default();
                }
                6 => schema.types[ipv4].constraints.max_length = Some(16),
                _ => unreachable!(),
            }
            assert!(generate(&schema, world).is_err(), "mutation {mutation}");
        }
    }
}

#[test]
fn no_address_helpers_are_emitted_for_address_free_schemas() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../xsd-frontend/tests/fixtures/track.xsd"),
    )
    .unwrap();
    let output = generate(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
    for name in ["_check_ipv4", "_check_ipv6", "_choice_carrier_validators"] {
        assert!(!output.contains(name));
    }
}
