//! Normalized facet/ancestry proof, independent of declaration spelling.
use ams_gra_oms_backend_python::generate;
use ams_gra_oms_codegen_core::{GenerationWorld, UnicodeStringProfile};
use ams_gra_oms_ir::{ConstraintSet, PrimitiveKind, QualifiedName, TypeKind, TypeRef};
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::path::Path;

#[test]
fn exact_facets_ancestry_and_names_fail_closed() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/python-unicode31/oam.xsd"),
    )
    .unwrap();
    let date = schema
        .types
        .iter()
        .position(|d| d.name.local_name == "NITF_DateType")
        .unwrap();
    let leaf = schema
        .types
        .iter()
        .position(|d| d.name.local_name == "Leaf")
        .unwrap();
    for world in [
        GenerationWorld::ClosedSchemaSet,
        GenerationWorld::OpenExtensions,
    ] {
        assert!(
            generate(&schema, world)
                .unwrap()
                .contains("def _check_unicode31(")
        );
        for mutation in 0..12 {
            let mut changed = schema.clone();
            match mutation {
                0 => changed.types[date].constraints.max_length = Some(8),
                1 => changed.types[date].constraints = ConstraintSet::default(),
                2 => changed.types[date].constraints.lexical.pattern_groups[0]
                    .alternatives
                    .reverse(),
                3 => changed.types[date].is_abstract = true,
                4 => {
                    changed.types[leaf].base_type =
                        Some(TypeRef::named(changed.types[leaf].name.clone()))
                }
                5 => {
                    changed.types[leaf].base_type =
                        Some(TypeRef::named(QualifiedName::new("urn:missing", "Missing")))
                }
                6 => {
                    changed.types[leaf].base_type = Some(TypeRef::primitive(PrimitiveKind::Boolean))
                }
                7 => {
                    changed.types[leaf].kind =
                        TypeKind::Alias(TypeRef::named(changed.types[date].name.clone()))
                }
                8 => {
                    changed.types[date].constraints =
                        UnicodeStringProfile::DateAndTime.constraints()
                }
                9 => {
                    changed.types[date].constraints.lexical.white_space =
                        Some(ams_gra_oms_ir::WhiteSpacePolicy::Preserve)
                }
                10 => changed.types[date].constraints.length = Some(9),
                11 => {
                    changed.types[date].constraints.lexical.pattern_groups[0].alternatives[0]
                        .expression = ".*".into()
                }
                _ => unreachable!(),
            }
            assert!(generate(&changed, world).is_err(), "mutation {mutation}");
        }
    }
}

#[test]
fn runtime_absent_without_admitted_profiles() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../xsd-frontend/tests/fixtures/track.xsd"),
    )
    .unwrap();
    assert!(
        !generate(&schema, GenerationWorld::ClosedSchemaSet)
            .unwrap()
            .contains("unicode31")
    );
}
