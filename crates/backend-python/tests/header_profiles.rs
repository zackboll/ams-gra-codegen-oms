//! Admission tests mutate normalized facets independently of runtime validators.
use ams_gra_oms_backend_python::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_ir::{PrimitiveKind, TypeKind};
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::path::Path;

#[test]
fn normalized_neighbours_and_pattern_alternatives_stay_unsupported() {
    let mut schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/python-header-profiles/oam.xsd"),
    )
    .unwrap();
    schema.messages.clear();
    schema.types.retain(|declaration| {
        matches!(declaration.kind, TypeKind::Primitive(PrimitiveKind::String))
            && declaration.name.local_name != "NearUUID"
    });
    assert_eq!(schema.types.len(), 2);
    for index in 0..schema.types.len() {
        // Exact support is name-free, including names that could hide builtins.
        let mut renamed = schema.clone();
        renamed.types[index].name.local_name = "all".into();
        assert!(generate(&renamed, GenerationWorld::ClosedSchemaSet).is_ok());
        let original = schema.types[index].constraints.clone();
        let mut mutations = Vec::new();
        let mut alternative = original.clone();
        let duplicate = alternative.lexical.pattern_groups[0].alternatives[0].clone();
        alternative.lexical.pattern_groups[0]
            .alternatives
            .push(duplicate);
        mutations.push(alternative);
        let mut conjunction = original.clone();
        conjunction
            .lexical
            .pattern_groups
            .push(conjunction.lexical.pattern_groups[0].clone());
        mutations.push(conjunction);
        let mut expression = original.clone();
        expression.lexical.pattern_groups[0].alternatives[0]
            .expression
            .push('|');
        mutations.push(expression);
        let mut bound = original.clone();
        if let Some(length) = &mut bound.length {
            *length += 1;
        } else {
            *bound.min_length.as_mut().unwrap() += 1;
        }
        mutations.push(bound);
        for constraints in mutations {
            let mut candidate = schema.clone();
            candidate.types[index].constraints = constraints;
            for world in [
                GenerationWorld::ClosedSchemaSet,
                GenerationWorld::OpenExtensions,
            ] {
                assert!(generate(&candidate, world).is_err());
            }
        }
    }
}
