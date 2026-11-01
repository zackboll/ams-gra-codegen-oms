//! PY-011: classifier admission and named-ancestry adversaries on normalized IR.
use ams_gra_oms_backend_python::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_ir::{ConstraintSet, PrimitiveKind, TypeKind, TypeRef, WhiteSpacePolicy};
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::path::Path;

#[test]
fn exact_profile_and_named_ancestry_fail_closed() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/python-nato-special-words/oam.xsd"),
    )
    .unwrap();
    let parent = schema
        .types
        .iter()
        .position(|d| d.name.local_name == "SpecialWord")
        .unwrap();
    let child = schema
        .types
        .iter()
        .position(|d| d.name.local_name == "Restricted")
        .unwrap();
    for world in [
        GenerationWorld::ClosedSchemaSet,
        GenerationWorld::OpenExtensions,
    ] {
        let first = generate(&schema, world).unwrap();
        assert_eq!(first, generate(&schema, world).unwrap());
        assert_eq!(first.matches("def _check_nato_special_words").count(), 1);
        for variant in 0..14 {
            let mut bad = schema.clone();
            match variant {
                0 => bad.types[parent].constraints = ConstraintSet::default(),
                1 => bad.types[parent].kind = TypeKind::Primitive(PrimitiveKind::Boolean),
                2 => {
                    bad.types[parent].base_type =
                        Some(TypeRef::named(bad.types[child].name.clone()))
                }
                3 => bad.types[child].constraints = ConstraintSet::default(),
                4 => bad.types[child].base_type = Some(TypeRef::primitive(PrimitiveKind::Boolean)),
                5 => {
                    let mut missing = bad.types[parent].name.clone();
                    missing.local_name = "Missing".into();
                    bad.types[child].base_type = Some(TypeRef::named(missing));
                }
                6 => bad.types[child].is_abstract = true,
                7 => {
                    bad.types[child].constraints.lexical.white_space =
                        Some(WhiteSpacePolicy::Collapse)
                }
                8 => {
                    bad.types[child].constraints.lexical.white_space =
                        Some(WhiteSpacePolicy::Preserve)
                }
                9 => {
                    let alternative = bad.types[child].constraints.lexical.pattern_groups[0]
                        .alternatives[0]
                        .clone();
                    bad.types[child].constraints.lexical.pattern_groups[0]
                        .alternatives
                        .push(alternative);
                }
                10 => {
                    let group = bad.types[child].constraints.lexical.pattern_groups[0].clone();
                    bad.types[child]
                        .constraints
                        .lexical
                        .pattern_groups
                        .push(group);
                }
                11 => bad.types[child].constraints.length = Some(6),
                12 => {
                    bad.types[child].kind =
                        TypeKind::Alias(TypeRef::named(bad.types[parent].name.clone()))
                }
                13 => bad.types[parent].constraints.max_length = Some(262),
                _ => unreachable!(),
            }
            assert!(generate(&bad, world).is_err(), "variant {variant}");
        }
    }
}

#[test]
fn helper_is_absent_without_profile() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/python-visible-ascii/oam.xsd"),
    )
    .unwrap();
    assert!(
        !generate(&schema, GenerationWorld::ClosedSchemaSet)
            .unwrap()
            .contains("nato_special_words")
    );
}
