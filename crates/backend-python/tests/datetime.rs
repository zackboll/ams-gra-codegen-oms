//! External IR must not bypass the shared effective temporal profile proof.
use ams_gra_oms_backend_python::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_ir::{ConstraintSet, PrimitiveKind, TypeKind, TypeRef};
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::path::Path;

#[test]
fn datetime_ancestry_is_proven_not_flattened() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/python-datetime/oam.xsd"),
    )
    .unwrap();
    for world in [
        GenerationWorld::ClosedSchemaSet,
        GenerationWorld::OpenExtensions,
    ] {
        assert!(generate(&schema, world).is_ok());
        let mut unused_ancestor = schema.clone();
        unused_ancestor.messages.clear();
        unused_ancestor
            .types
            .retain(|d| d.name.local_name == "Ancestor");
        let source = generate(&unused_ancestor, world).unwrap();
        assert!(
            !source.contains("_check_datetime"),
            "unused abstract ancestry must not demand helpers"
        );
        for variant in 0..5 {
            let mut bad = schema.clone();
            let parent = bad
                .types
                .iter()
                .position(|d| d.name.local_name == "Instant")
                .unwrap();
            let child = bad
                .types
                .iter()
                .position(|d| d.name.local_name == "Restricted")
                .unwrap();
            match variant {
                0 => bad.types[parent].constraints = ConstraintSet::default(),
                1 => bad.types[parent].kind = TypeKind::Primitive(PrimitiveKind::Time),
                2 => {
                    bad.types[parent].base_type =
                        Some(TypeRef::named(bad.types[child].name.clone()))
                }
                3 => bad.types[child].constraints = ConstraintSet::default(),
                4 => bad.types[child].base_type = Some(TypeRef::primitive(PrimitiveKind::String)),
                _ => unreachable!(),
            }
            assert!(generate(&bad, world).is_err(), "variant {variant}");
        }
    }
}
