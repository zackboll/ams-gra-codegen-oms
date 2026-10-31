//! Independent effective-facet and ancestry adversaries, without XSD lowering.
use ams_gra_oms_backend_python::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_ir::{ConstraintSet, PrimitiveKind, TypeKind, TypeRef, WhiteSpacePolicy};
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::path::Path;

#[test]
fn effective_facets_and_named_ancestry_fail_closed() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/python-visible-ascii/oam.xsd"),
    )
    .unwrap();
    let parent = schema
        .types
        .iter()
        .position(|d| d.name.local_name == "Text256")
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
        assert!(generate(&schema, world).is_ok());
        for variant in 0..15 {
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
                6 => {
                    // Both domains are proven, but the child widens its parent.
                    bad.types[parent].constraints = bad
                        .types
                        .iter()
                        .find(|d| d.name.local_name == "Text20")
                        .unwrap()
                        .constraints
                        .clone();
                }
                7 => bad.types[child].is_abstract = true,
                8 => {
                    bad.types[child].constraints.lexical.white_space =
                        Some(WhiteSpacePolicy::Collapse)
                }
                9 => {
                    bad.types[child].constraints.lexical.white_space =
                        Some(WhiteSpacePolicy::Replace)
                }
                10 => {
                    bad.types[child].constraints.lexical.white_space =
                        Some(WhiteSpacePolicy::Preserve)
                }
                11 => {
                    let alternative = bad.types[child].constraints.lexical.pattern_groups[0]
                        .alternatives[0]
                        .clone();
                    bad.types[child].constraints.lexical.pattern_groups[0]
                        .alternatives
                        .push(alternative);
                }
                12 => {
                    let group = bad.types[child].constraints.lexical.pattern_groups[0].clone();
                    bad.types[child]
                        .constraints
                        .lexical
                        .pattern_groups
                        .push(group);
                }
                13 => bad.types[child].constraints.length = Some(256),
                14 => {
                    bad.types[child].kind =
                        TypeKind::Alias(TypeRef::named(bad.types[parent].name.clone()))
                }
                _ => unreachable!(),
            }
            assert!(generate(&bad, world).is_err(), "variant {variant}");
        }
        // A second distinct pattern would require a conjunction proof. The IR
        // and classifier intentionally reject it, even for contained lengths.
        let mut conjunction = schema.clone();
        let group = schema
            .types
            .iter()
            .find(|d| d.name.local_name == "Text32")
            .unwrap()
            .constraints
            .lexical
            .pattern_groups[0]
            .clone();
        conjunction.types[child]
            .constraints
            .lexical
            .pattern_groups
            .push(group);
        assert!(generate(&conjunction, world).is_err());
    }
}
