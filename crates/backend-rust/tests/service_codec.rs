//! Task 050: direct `generate_service_codec` calls. The renderer re-runs the
//! shared codec preflight, so a caller that skipped readiness still fails
//! closed, and every Rust spelling matches the generated model's.

use ams_gra_oms_backend_rust::{generate, generate_service_codec};
use ams_gra_oms_codegen_core::{
    GenerationWorld, build_service_api_model, project_service_generation_schema,
    resolve_service_plan,
};
use ams_gra_oms_service_contract::parse_yaml;
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::path::Path;

const WORLD: GenerationWorld = GenerationWorld::ClosedSchemaSet;

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate")
        .join(name)
}

fn render(stem: &str) -> (Result<String, String>, String) {
    let schema = load_schema_set(&fixture(&format!("{stem}.xsd"))).expect("schema");
    let contract =
        parse_yaml(&std::fs::read_to_string(fixture(&format!("{stem}.yaml"))).expect("contract"))
            .expect("parse");
    let plan = resolve_service_plan(&contract, &schema).expect("plan");
    let projection = project_service_generation_schema(&plan, &schema, WORLD).expect("projection");
    let model = build_service_api_model(&plan, projection.schema(), WORLD).expect("api model");
    let rust_model = generate(projection.schema(), WORLD).expect("model renders");
    (
        generate_service_codec(&model, projection.schema(), WORLD).map_err(|error| error.message),
        rust_model,
    )
}

#[test]
fn direct_codec_generation_fails_closed_on_binary() {
    let (codec, _model) = render("codec-binary");
    let message = codec.expect_err("Binary has no evidenced encoding");
    assert!(
        message.starts_with("service codec boundary: BlobPayload.Data is Binary"),
        "{message}"
    );
}

#[test]
fn direct_codec_generation_fails_closed_outside_oam() {
    let (codec, _model) = render("runtime-test");
    assert!(
        codec
            .expect_err("non-OAM")
            .contains("outside the OAM namespace")
    );
}

#[test]
fn codec_references_the_model_renderer_spellings() {
    let (codec, model) = render("codec-oam");
    let codec = codec.expect("codec renders");
    // Enum variants and wire values: the Rust identifier is the model's own
    // (Task 044 remapping), and the JSON string is the XSD wire value.
    for (variant, wire) in [
        ("NORMAL", "NORMAL"),
        ("Value5G", "5G"),
        ("SOMEVALUE", "SOME_VALUE"),
        ("ValueSelf", "Self"),
    ] {
        assert!(model.contains(&format!("    {variant},\n")), "{variant}");
        assert!(
            codec.contains(&format!(
                "super::model::SignalCode::{variant} => \"{wire}\""
            )),
            "{variant}"
        );
        assert!(
            codec.contains(&format!(
                "\"{wire}\" => Ok(super::model::SignalCode::{variant})"
            )),
            "{wire}"
        );
    }
    // Field members: the model's snake_case field, the XSD member name.
    assert!(model.contains("    pub uuid: UniversallyUniqueIdentifierType,"));
    assert!(codec.contains("object.insert(\"UUID\".to_owned(), { let x = &value.uuid;"));
    // Choice and abstract-value variants use the model's variant names.
    assert!(model.contains("    Levels(BoundedVec<Level, 1, 3>),"));
    assert!(codec.contains("super::model::SourceChoice::Levels(x) => (\"Levels\","));
    assert!(model.contains("    BoxShape(BoxShape),"));
    assert!(codec.contains("super::model::ShapeBase::BoxShape(x) => with_type("));
    // No numeric bound is duplicated: Level's -10..10 lives only in the model.
    assert!(model.contains("pub const MIN: i64 = -10;"));
    assert!(!codec.contains("-10") && !codec.contains("180"));
    // Deterministic.
    assert_eq!(render("codec-oam").0.expect("again"), codec);
}
