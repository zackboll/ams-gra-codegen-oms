//! Normalized-IR boundaries that cannot all be authored by the XSD frontend.
use ams_gra_oms_backend_python::{generate, generate_service_codec};
use ams_gra_oms_codegen_core::{GenerationWorld, build_service_api_model, resolve_service_plan};
use ams_gra_oms_ir::{BinaryLexicalEncoding, ConstraintSet, TypeKind, TypeRef, TypeRefTarget};
use ams_gra_oms_service_contract::parse_yaml;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::path::Path;

#[test]
fn effective_choice_ancestry_and_codec_provenance_fail_closed() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/python-choice");
    let original = load_schema_document(&fixture.join("oam.xsd")).unwrap();
    let contract =
        parse_yaml(&std::fs::read_to_string(fixture.join("service.yaml")).unwrap()).unwrap();
    for world in [
        GenerationWorld::ClosedSchemaSet,
        GenerationWorld::OpenExtensions,
    ] {
        let mut schema = original.clone();
        // An empty Record ancestor contributes no product segment. This is the
        // shared projection's legitimate structural-ancestry case, not flattening.
        let mut base = schema
            .types
            .iter()
            .find(|ty| ty.name.local_name == "Detail")
            .unwrap()
            .clone();
        base.name.local_name = "EmptyBase".to_owned();
        base.kind = TypeKind::Record { fields: Vec::new() };
        base.is_abstract = true;
        schema
            .types
            .iter_mut()
            .find(|ty| ty.name.local_name == "Selection")
            .unwrap()
            .base_type = Some(TypeRef::named(base.name.clone()));
        schema.types.push(base.clone());
        assert!(generate(&schema, world).is_ok());

        let base = schema.types.last_mut().unwrap();
        base.kind = original
            .types
            .iter()
            .find(|ty| ty.name.local_name == "Detail")
            .unwrap()
            .kind
            .clone();
        let error = generate(&schema, world).unwrap_err().to_string();
        assert!(
            error.contains("Record") || error.contains("record"),
            "{error}"
        );

        let mut schema = original.clone();
        let owner = schema
            .types
            .iter()
            .find(|ty| ty.name.local_name == "Owner")
            .unwrap()
            .name
            .clone();
        schema
            .types
            .iter_mut()
            .find(|ty| ty.name.local_name == "Selection")
            .unwrap()
            .base_type = Some(TypeRef::named(owner));
        assert!(
            generate(&schema, world).is_err(),
            "multiple Choice segments"
        );

        for encoding in [None, Some(BinaryLexicalEncoding::Base64Binary)] {
            let mut schema = original.clone();
            let selection = schema
                .types
                .iter_mut()
                .find(|ty| ty.name.local_name == "Selection")
                .unwrap();
            let TypeKind::Choice { alternatives } = &mut selection.kind else {
                panic!("fixture")
            };
            let field = alternatives
                .iter_mut()
                .find(|field| field.name == "DirectBinary")
                .unwrap();
            field.type_ref.binary_encoding = encoding;
            assert!(generate(&schema, world).is_ok());
            let plan = resolve_service_plan(&contract, &schema).unwrap();
            let model = build_service_api_model(&plan, &schema, world).unwrap();
            let error = generate_service_codec(&model, &schema)
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("Selection.DirectBinary")
                    && error.contains("Binary lexical provenance"),
                "{error}"
            );
        }

        let mut schema = original.clone();
        let selection = schema
            .types
            .iter_mut()
            .find(|ty| ty.name.local_name == "Selection")
            .unwrap();
        let TypeKind::Choice { alternatives } = &mut selection.kind else {
            panic!("fixture")
        };
        alternatives[0].wire_namespace_uri = None;
        assert!(generate(&schema, world).is_ok());
        let plan = resolve_service_plan(&contract, &schema).unwrap();
        let model = build_service_api_model(&plan, &schema, world).unwrap();
        assert!(
            generate_service_codec(&model, &schema)
                .unwrap_err()
                .to_string()
                .contains("unqualified element member")
        );

        for kind in ["alias", "facets"] {
            let mut schema = original.clone();
            let selection = schema
                .types
                .iter_mut()
                .find(|ty| ty.name.local_name == "Selection")
                .unwrap();
            let TypeKind::Choice { alternatives } = &mut selection.kind else {
                panic!("fixture")
            };
            if kind == "facets" {
                alternatives[0].constraints.max_length = Some(3);
            } else {
                let TypeRefTarget::Named(name) = &alternatives
                    .iter()
                    .find(|field| field.name == "Binary")
                    .unwrap()
                    .type_ref
                    .target
                else {
                    panic!("fixture")
                };
                let name = name.clone();
                let binary = schema.types.iter_mut().find(|ty| ty.name == name).unwrap();
                binary.kind = TypeKind::Alias(TypeRef::binary(BinaryLexicalEncoding::HexBinary));
                binary.base_type = None;
                binary.constraints = ConstraintSet::default();
            }
            assert!(generate(&schema, world).is_err(), "{kind}");
        }
    }
}
