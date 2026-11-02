//! Independently mutate normalized effective facets and restriction edges.
use ams_gra_oms_backend_python::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_ir::{ConstraintSet, PrimitiveKind, TypeRef, WhiteSpacePolicy};
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::path::Path;

#[test]
fn effective_profile_and_every_named_edge_are_proven() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/python-whitespace-visible/oam.xsd"),
    )
    .unwrap();
    let index = |name| {
        schema
            .types
            .iter()
            .position(|d| d.name.local_name == name)
            .unwrap()
    };
    let parent = index("FoldShort");
    let child = index("Derived");
    for world in [
        GenerationWorld::ClosedSchemaSet,
        GenerationWorld::OpenExtensions,
    ] {
        assert!(generate(&schema, world).is_ok());
        for variant in 0..16 {
            let mut bad = schema.clone();
            match variant {
                0 => bad.types[parent].constraints = ConstraintSet::default(),
                1 => bad.types[child].constraints = ConstraintSet::default(),
                2 => {
                    bad.types[parent].base_type =
                        Some(TypeRef::named(bad.types[child].name.clone()))
                }
                3 => bad.types[child].base_type = Some(TypeRef::primitive(PrimitiveKind::Boolean)),
                4 => {
                    let mut missing = bad.types[parent].name.clone();
                    missing.local_name = "Missing".into();
                    bad.types[child].base_type = Some(TypeRef::named(missing));
                }
                5 => bad.types[child].is_abstract = true,
                6 => {
                    bad.types[child].constraints.lexical.white_space =
                        Some(WhiteSpacePolicy::Preserve)
                }
                7 => {
                    bad.types[child].constraints.lexical.white_space =
                        Some(WhiteSpacePolicy::Replace)
                }
                8 => bad.types[child].constraints.length = Some(1024),
                9 => bad.types[child].constraints.min_length = Some(1),
                10 => bad.types[child].constraints.max_length = Some(4096),
                11 => {
                    let group = bad.types[child].constraints.lexical.pattern_groups[0].clone();
                    bad.types[child]
                        .constraints
                        .lexical
                        .pattern_groups
                        .push(group);
                }
                12 => {
                    let alternative = bad.types[child].constraints.lexical.pattern_groups[0]
                        .alternatives[0]
                        .clone();
                    bad.types[child].constraints.lexical.pattern_groups[0]
                        .alternatives
                        .push(alternative);
                }
                // All endpoints independently classify, but these edges are invalid:
                // policy change (same max), widened max, and widened minimum.
                13 => {
                    bad.types[child].constraints = bad.types[index("KeepShort")].constraints.clone()
                }
                14 => {
                    bad.types[child].constraints = bad.types[index("FoldLong")].constraints.clone()
                }
                15 => {
                    bad.types[parent].constraints =
                        bad.types[index("KeepLong")].constraints.clone();
                    bad.types[child].constraints =
                        bad.types[index("KeepEmpty")].constraints.clone();
                }
                _ => unreachable!(),
            }
            assert!(
                generate(&bad, world).is_err(),
                "variant={variant} world={world:?}"
            );
        }
    }
}
