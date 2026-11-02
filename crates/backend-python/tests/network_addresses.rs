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

#[test]
fn addresses_revalidate_without_choice_and_other_alternating_profiles_stay_unsupported() {
    let mut schema = fixture();
    schema.messages.clear();
    schema.types.retain(|decl| {
        matches!(decl.kind, TypeKind::Primitive(_)) || decl.name.local_name == "IPv4_EndpointType"
    });
    let output = generate(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
    assert!(output.contains("_choice_carrier_validators = {"));
    let directory = std::env::temp_dir().join(format!("py017-no-choice-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(directory.join("model.py"), output).unwrap();
    let probe = "import model as m\na=m.Leaf('1.2.3.4')\nobject.__setattr__(a, 'value', '999.1.2.3')\ntry:\n m.IPv4_EndpointType(ipv4_address=a)\nexcept ValueError:\n pass\nelse:\n raise RuntimeError('corrupted address accepted without Choice')\n";
    for optimized in [false, true] {
        let mut command = std::process::Command::new(
            std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()),
        );
        if optimized {
            command.arg("-O");
        }
        let result = command
            .args(["-c", probe])
            .current_dir(&directory)
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .output()
            .unwrap();
        assert!(result.status.success(), "{result:?}");
    }
    std::fs::remove_dir_all(directory).unwrap();
    let address = schema
        .types
        .iter_mut()
        .find(|decl| decl.name.local_name == "IPv4_AddressType")
        .unwrap();
    address.constraints.min_length = None;
    address.constraints.max_length = None;
    address.constraints.length = Some(2);
    address.constraints.lexical.pattern_groups[0].alternatives[0].expression =
        "[A-Z]{2}|[ ]{2}".into();
    assert!(generate(&schema, GenerationWorld::ClosedSchemaSet).is_err());
}
