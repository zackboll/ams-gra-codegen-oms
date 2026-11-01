//! PY-014: normalized profiles, ancestry, aliases, semantic demand and names.
use ams_gra_oms_backend_python::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_ir::{ConstraintSet, PrimitiveKind, TypeKind, TypeRef};
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::path::Path;

#[test]
fn duration_ancestry_and_aliases_fail_closed() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/python-duration/oam.xsd"),
    )
    .unwrap();
    for world in [
        GenerationWorld::ClosedSchemaSet,
        GenerationWorld::OpenExtensions,
    ] {
        assert!(generate(&schema, world).is_ok());
        let mut unused = schema.clone();
        unused.messages.clear();
        unused.types.retain(|d| d.name.local_name == "Ancestor");
        assert!(
            !generate(&unused, world)
                .unwrap()
                .contains("_check_duration")
        );
        for variant in 0..9 {
            let mut bad = schema.clone();
            let parent = bad
                .types
                .iter()
                .position(|d| d.name.local_name == "Span")
                .unwrap();
            let child = bad
                .types
                .iter()
                .position(|d| d.name.local_name == "Restricted")
                .unwrap();
            match variant {
                0 => bad.types[parent].constraints.max_length = Some(30),
                1 => bad.types[parent].kind = TypeKind::Primitive(PrimitiveKind::Time),
                2 => {
                    bad.types[parent].base_type =
                        Some(TypeRef::named(bad.types[child].name.clone()))
                }
                3 => bad.types[child].constraints.length = Some(8),
                4 => bad.types[child].base_type = Some(TypeRef::primitive(PrimitiveKind::String)),
                5 => {
                    bad.types[child].kind =
                        TypeKind::Alias(TypeRef::primitive(PrimitiveKind::Duration))
                }
                6 => {
                    bad.types[child].kind =
                        TypeKind::Alias(TypeRef::named(bad.types[parent].name.clone()))
                }
                7 => {
                    bad.types[child].base_type = Some(TypeRef::named(
                        ams_gra_oms_ir::QualifiedName::new("urn:missing", "Missing"),
                    ))
                }
                8 => bad.types[parent].name.local_name = "str".into(),
                _ => unreachable!(),
            }
            assert!(generate(&bad, world).is_err(), "variant {variant}");
        }
        let mut local = schema.clone();
        let ancestor = local
            .types
            .iter_mut()
            .find(|d| d.name.local_name == "Ancestor")
            .unwrap();
        if let TypeKind::Record { fields } = &mut ancestor.kind {
            fields[0].constraints = ConstraintSet {
                max_length: Some(30),
                ..ConstraintSet::default()
            };
        }
        assert!(generate(&local, world).is_err());
    }
}
