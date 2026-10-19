//! C3 semantic plan and parent-model preservation controls.
use ams_gra_oms_codegen_core::{
    Backend, GenerationWorld, ada_codec_views, ada_codec_views_files, plan_type_emissions,
    validate_ada_codec_views_name,
};

const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;

fn schema() -> ams_gra_oms_ir::SchemaIr {
    ams_gra_oms_xsd_frontend::load_schema_document(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/service-generate/codec-ada-copy-views.xsd"),
    )
    .unwrap()
}

#[test]
fn views_follow_emissions_and_all_opaque_storage_shapes() {
    let schema = schema();
    let emissions = plan_type_emissions(&schema, CLOSED).unwrap();
    let views = ada_codec_views(&schema, &emissions, CLOSED).unwrap();
    assert_eq!(
        views.binary_carriers,
        ["Digest", "EmptyDigest", "ExactDigest"]
    );
    for (container, vectors) in [
        (
            "LosslessEnvelope_UB0_Sequence",
            Some("LosslessEnvelope_UB0_Vectors"),
        ),
        (
            "LosslessEnvelope_UB1_Additional",
            Some("LosslessEnvelope_UB1_Additional_Vectors"),
        ),
        ("LosslessEnvelope_B0_Sequence", None),
        ("LosslessEnvelope_B1_Sequence", None),
    ] {
        assert!(
            views
                .sequences
                .iter()
                .any(|v| v.container == container && v.vectors.as_deref() == vectors)
        );
    }
    assert!(
        !views
            .sequences
            .iter()
            .any(|v| v.container.ends_with("Required_Array"))
    );
    // A projection excluding the Binary declaration cannot fabricate its visitor.
    let projected: Vec<_> = emissions.into_iter().filter(|e| !matches!(e,
        ams_gra_oms_codegen_core::TypeEmission::Declaration(d) if d.name.local_name == "Digest"
    )).collect();
    assert!(
        !ada_codec_views(&schema, &projected, CLOSED)
            .unwrap()
            .binary_carriers
            .contains(&"Digest".into())
    );
    validate_ada_codec_views_name(&schema, &projected, CLOSED).unwrap();
}

#[test]
fn child_layout_and_parent_only_generation_are_separate() {
    assert_eq!(
        ada_codec_views_files(&["Urn".into(), "Custom".into()]),
        ["urn-custom-codec_views.ads", "urn-custom-codec_views.adb"]
    );
    let files = ams_gra_oms_backend_ada::AdaBackend
        .generate(&schema(), CLOSED)
        .unwrap();
    assert!(
        files
            .iter()
            .all(|f| !f.relative_path.to_string_lossy().contains("codec_views"))
    );
}

#[test]
fn child_name_collides_case_insensitively_but_callback_names_do_not() {
    for name in [
        "Codec_Views",
        "CODEC_VIEWS",
        "codec_views",
        "Visit_Binary",
        "Visit_Element",
    ] {
        let mut schema = schema();
        let mut declaration = schema
            .types
            .iter()
            .find(|d| d.name.local_name == "Digest")
            .unwrap()
            .clone();
        declaration.name.local_name = name.into();
        schema.types.push(declaration);
        let plan = plan_type_emissions(&schema, CLOSED).unwrap();
        assert_eq!(
            validate_ada_codec_views_name(&schema, &plan, CLOSED).is_err(),
            name.eq_ignore_ascii_case("Codec_Views"),
            "{name}"
        );
    }
}

#[test]
fn child_collision_is_not_invented_for_an_ancestry_only_record() {
    let mut schema = schema();
    let base = schema
        .types
        .iter_mut()
        .find(|d| d.name.local_name == "Base")
        .unwrap();
    let old_name = base.name.clone();
    base.name.local_name = "Codec_Views".into();
    base.is_abstract = true;
    let new_name = base.name.clone();
    for d in &mut schema.types {
        if d.base_type
            .as_ref()
            .is_some_and(|r| r.target == ams_gra_oms_ir::TypeRefTarget::Named(old_name.clone()))
        {
            d.base_type.as_mut().unwrap().target =
                ams_gra_oms_ir::TypeRefTarget::Named(new_name.clone());
        }
    }
    let plan = plan_type_emissions(&schema, CLOSED).unwrap();
    validate_ada_codec_views_name(&schema, &plan, CLOSED).unwrap();
}
