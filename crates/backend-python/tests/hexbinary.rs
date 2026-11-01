//! Binary value space must not imply a hexadecimal lexical encoding.
use ams_gra_oms_backend_python::{generate, generate_service_codec};
use ams_gra_oms_codegen_core::{GenerationWorld, build_service_api_model, resolve_service_plan};
use ams_gra_oms_ir::{
    BinaryLexicalEncoding, ConstraintSet, NumericValue, PrimitiveKind, TypeKind, TypeRef,
    WhiteSpacePolicy,
};
use ams_gra_oms_service_contract::parse_yaml;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::path::Path;

#[test]
fn unknown_and_base64_provenance_fail_before_codec_artifacts() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/python-hexbinary");
    let original = load_schema_document(&fixture.join("oam.xsd")).unwrap();
    let contract =
        parse_yaml(&std::fs::read_to_string(fixture.join("service.yaml")).unwrap()).unwrap();
    for world in [
        GenerationWorld::ClosedSchemaSet,
        GenerationWorld::OpenExtensions,
    ] {
        for named in [false, true] {
            for encoding in [None, Some(BinaryLexicalEncoding::Base64Binary)] {
                let mut schema = original.clone();
                let reference = encoding.map_or_else(
                    || TypeRef::primitive(PrimitiveKind::Binary),
                    TypeRef::binary,
                );
                if named {
                    schema
                        .types
                        .iter_mut()
                        .find(|ty| ty.name.local_name == "HexBinaryType")
                        .unwrap()
                        .base_type = Some(reference);
                } else {
                    let payload = schema
                        .types
                        .iter_mut()
                        .find(|ty| ty.name.local_name == "Payload")
                        .unwrap();
                    let TypeKind::Record { fields } = &mut payload.kind else {
                        panic!("fixture")
                    };
                    fields
                        .iter_mut()
                        .find(|field| field.name == "Required")
                        .unwrap()
                        .type_ref = reference;
                }
                assert!(generate(&schema, world).is_ok());
                let plan = resolve_service_plan(&contract, &schema).unwrap();
                let model = build_service_api_model(&plan, &schema, world).unwrap();
                let error = generate_service_codec(&model, &schema)
                    .unwrap_err()
                    .to_string();
                assert!(error.contains("Binary lexical provenance"), "{error}");
            }
        }
    }
}

#[test]
fn unsupported_effective_domains_aliases_and_field_facets_fail_closed() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/python-hexbinary");
    let original = load_schema_document(&fixture.join("oam.xsd")).unwrap();
    let mut whitespace = ConstraintSet::default();
    whitespace.lexical.white_space = Some(WhiteSpacePolicy::Preserve);
    for constraints in [
        ConstraintSet {
            length: Some(2),
            max_length: Some(1),
            ..ConstraintSet::default()
        },
        ConstraintSet {
            min_inclusive: Some(NumericValue::Integer(0)),
            ..ConstraintSet::default()
        },
        whitespace,
    ] {
        let mut schema = original.clone();
        schema
            .types
            .iter_mut()
            .find(|ty| ty.name.local_name == "HexBinaryType")
            .unwrap()
            .constraints = constraints;
        assert!(generate(&schema, GenerationWorld::ClosedSchemaSet).is_err());
    }
    for named in [false, true] {
        let mut schema = original.clone();
        let base_name = schema
            .types
            .iter()
            .find(|ty| ty.name.local_name == "HexBinaryType")
            .unwrap()
            .name
            .clone();
        let declaration = schema
            .types
            .iter_mut()
            .find(|ty| ty.name.local_name == "Exact")
            .unwrap();
        declaration.kind = TypeKind::Alias(if named {
            TypeRef::named(base_name)
        } else {
            TypeRef::binary(BinaryLexicalEncoding::HexBinary)
        });
        declaration.base_type = None;
        declaration.constraints = ConstraintSet::default();
        assert!(generate(&schema, GenerationWorld::ClosedSchemaSet).is_err());
    }
    let mut schema = original.clone();
    let payload = schema
        .types
        .iter_mut()
        .find(|ty| ty.name.local_name == "Payload")
        .unwrap();
    let TypeKind::Record { fields } = &mut payload.kind else {
        panic!("fixture")
    };
    fields
        .iter_mut()
        .find(|field| field.name == "Required")
        .unwrap()
        .constraints
        .length = Some(2);
    let error = generate(&schema, GenerationWorld::ClosedSchemaSet)
        .unwrap_err()
        .to_string();
    assert!(error.contains("field-local Binary length"), "{error}");
}
