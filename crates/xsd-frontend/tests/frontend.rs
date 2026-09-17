use ams_gra_oms_ir::{Cardinality, PrimitiveKind, QualifiedName, TypeKind, TypeRefTarget};
use ams_gra_oms_xsd_frontend::{FrontendError, load_schema_set};
use std::path::{Path, PathBuf};

const OMS_NS: &str = "urn:example:oms:track";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn normalizes_the_track_schema_into_language_neutral_ir() {
    let ir = load_schema_set(&fixture("track.xsd")).expect("fixture should parse");

    assert_eq!(ir.namespaces.len(), 1);
    assert_eq!(ir.namespaces[0].uri, OMS_NS);
    assert_eq!(ir.namespaces[0].preferred_prefix.as_deref(), Some("oms"));

    let track_id = ir
        .types
        .iter()
        .find(|declaration| declaration.name.local_name == "Track_Id")
        .expect("Track_Id should exist");
    assert_eq!(track_id.name.namespace_uri, OMS_NS);
    assert_eq!(
        track_id.kind,
        TypeKind::Primitive(PrimitiveKind::SignedInteger)
    );
    assert_eq!(track_id.constraints.min_inclusive, Some(1));
    assert_eq!(track_id.constraints.max_inclusive, Some(65_535));
    assert!(track_id.source.line.is_some());

    let quality = ir
        .types
        .iter()
        .find(|declaration| declaration.name.local_name == "Track_Quality")
        .expect("Track_Quality should exist");
    let TypeKind::Enumeration { variants } = &quality.kind else {
        panic!("Track_Quality should be an enumeration");
    };
    assert_eq!(
        variants
            .iter()
            .map(|variant| variant.wire_value.as_str())
            .collect::<Vec<_>>(),
        ["Unknown", "Tentative", "Confirmed"]
    );

    let track = ir
        .types
        .iter()
        .find(|declaration| declaration.name.local_name == "Track")
        .expect("Track should exist");
    let TypeKind::Record { fields } = &track.kind else {
        panic!("Track should be a record");
    };
    assert_eq!(fields.len(), 4);
    assert_named_type(&fields[0].type_ref.target, "Track_Id");
    assert_eq!(fields[0].cardinality, Cardinality::REQUIRED_ONE);
    assert_named_type(&fields[1].type_ref.target, "Track_Quality");
    assert_eq!(fields[1].cardinality, Cardinality::REQUIRED_ONE);
    assert_eq!(fields[2].name, "Callsign");
    assert_eq!(fields[2].cardinality, Cardinality::OPTIONAL_ONE);
    assert_eq!(
        fields[2].type_ref.target,
        TypeRefTarget::Primitive(PrimitiveKind::String)
    );
    assert_eq!(fields[3].name, "Sensor_Ids");
    assert_eq!(
        fields[3].cardinality,
        Cardinality {
            min_occurs: 0,
            max_occurs: Some(8),
        }
    );
    assert_eq!(
        fields[3].type_ref.target,
        TypeRefTarget::Primitive(PrimitiveKind::SignedInteger)
    );

    // The semantic IR has resolved names, primitives, and cardinality. It has
    // no backend container, serialization, transport, or language metadata.
    let debug_ir = format!("{ir:#?}");
    for backend_term in [
        "Vec<",
        "std::vector",
        "serde",
        "WebSocket",
        "OWP",
        "DDS",
        "Ada array",
    ] {
        assert!(!debug_ir.contains(backend_term));
    }
}

#[test]
fn parsing_is_deterministic() {
    let path = fixture("track.xsd");
    assert_eq!(
        load_schema_set(&path).expect("first parse should succeed"),
        load_schema_set(&path).expect("second parse should succeed")
    );
}

#[test]
fn unsupported_choice_fails_closed() {
    let error = load_schema_set(&fixture("unsupported-choice.xsd"))
        .expect_err("choice must not be silently approximated");
    assert_eq!(
        error,
        FrontendError::UnsupportedConstruct("xs:choice".to_owned())
    );
    assert_eq!(error.to_string(), "unsupported XSD construct: xs:choice");
}

fn assert_named_type(actual: &TypeRefTarget, local_name: &str) {
    assert_eq!(
        actual,
        &TypeRefTarget::Named(QualifiedName::new(OMS_NS, local_name))
    );
}
