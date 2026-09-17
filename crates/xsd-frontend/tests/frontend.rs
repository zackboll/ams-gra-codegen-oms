use ams_gra_oms_ir::{Cardinality, PrimitiveKind, QualifiedName, TypeKind, TypeRefTarget};
use ams_gra_oms_xsd_frontend::{FrontendError, load_schema_document, load_schema_set};
use std::path::{Path, PathBuf};

const OMS_NS: &str = "urn:example:oms:track";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn normalizes_the_track_schema_into_language_neutral_ir() {
    let ir = load_schema_document(&fixture("track.xsd")).expect("fixture should parse");

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
        load_schema_document(&path).expect("first parse should succeed"),
        load_schema_document(&path).expect("second parse should succeed")
    );
}

#[test]
fn accepts_and_normalizes_element_form_default_deterministically() {
    let path = fixture("element-form-default/root.xsd");
    let ir = load_schema_set(&path).expect("qualified and unqualified forms should parse");
    assert_eq!(ir, load_schema_set(&path).expect("repeat should match"));
    assert_eq!(
        ir.types
            .iter()
            .map(|declaration| declaration.name.local_name.as_str())
            .collect::<Vec<_>>(),
        ["Envelope", "Value_Type"]
    );

    let TypeKind::Record { fields } = &ir.types[0].kind else {
        panic!("Envelope should be a record");
    };
    assert_eq!(
        fields[0].type_ref.target,
        TypeRefTarget::Named(QualifiedName::new("urn:example:element-form", "Value_Type"))
    );
    assert!(
        ir.types[0]
            .source
            .document
            .ends_with("element-form-default/root.xsd")
    );
    assert_eq!(ir.types[0].source.line, Some(8));
    assert!(
        ir.types[1]
            .source
            .document
            .ends_with("element-form-default/types.xsd")
    );
    assert_eq!(ir.types[1].source.line, Some(7));
}

#[test]
fn rejects_invalid_element_form_default_value() {
    let error = load_schema_document(&fixture("errors/invalid-element-form-default.xsd"))
        .expect_err("invalid form default must fail");
    assert!(matches!(error, FrontendError::InvalidInput(_)));
    let message = error.to_string();
    assert!(message.contains("invalid-element-form-default.xsd"));
    assert!(message.contains("must be qualified or unqualified, got sometimes"));
}

#[test]
fn element_form_support_does_not_allow_other_schema_attributes() {
    let error = load_schema_document(&fixture("errors/unsupported-attribute-form-default.xsd"))
        .expect_err("attributeFormDefault remains outside this feature");
    assert!(matches!(error, FrontendError::UnsupportedConstruct(_)));
    let message = error.to_string();
    assert!(message.contains("xs:schema @attributeFormDefault"));
    assert!(message.contains("unsupported-attribute-form-default.xsd:2:1"));
}

#[test]
fn standalone_loading_rejects_dependencies() {
    for (name, construct, position) in [
        ("schema-set/root.xsd", "xs:include", ":6:3"),
        ("errors/import-mismatch-root.xsd", "xs:import", ":1:84"),
    ] {
        let error = load_schema_document(&fixture(name))
            .expect_err("standalone loading must not traverse dependencies");
        let message = error.to_string();
        assert!(message.contains(construct));
        assert!(message.contains(name));
        assert!(message.contains(position));
    }
}

#[test]
fn recursively_loads_a_deduplicated_deterministic_schema_set() {
    let path = fixture("schema-set/root.xsd");
    let ir = load_schema_set(&path).expect("schema set should load");
    assert_eq!(ir, load_schema_set(&path).expect("repeat should match"));
    assert_eq!(ir.schema_version.as_deref(), Some("3"));
    assert_eq!(
        ir.namespaces
            .iter()
            .map(|namespace| (
                namespace.uri.as_str(),
                namespace.preferred_prefix.as_deref()
            ))
            .collect::<Vec<_>>(),
        [
            ("urn:example:oms:track", Some("track")),
            ("urn:example:oms:sensor", Some("sensor")),
        ]
    );
    assert_eq!(
        ir.types
            .iter()
            .map(|declaration| declaration.name.local_name.as_str())
            .collect::<Vec<_>>(),
        ["Track", "Track_Quality", "Sensor_Id"]
    );
    assert_eq!(
        ir.types
            .iter()
            .filter(|declaration| declaration.name.local_name == "Sensor_Id")
            .count(),
        1,
        "the shared dependency must be represented once"
    );

    let track = &ir.types[0];
    let TypeKind::Record { fields } = &track.kind else {
        panic!("Track should be a record");
    };
    assert_eq!(
        fields[0].type_ref.target,
        TypeRefTarget::Named(QualifiedName::new("urn:example:oms:track", "Track_Quality"))
    );
    assert_eq!(
        fields[1].type_ref.target,
        TypeRefTarget::Named(QualifiedName::new("urn:example:oms:sensor", "Sensor_Id"))
    );
    assert!(ir.types[0].source.document.ends_with("schema-set/root.xsd"));
    assert!(fields[0].source.document.ends_with("schema-set/root.xsd"));
    assert!(fields[1].source.document.ends_with("schema-set/root.xsd"));
    assert!(
        ir.types[1]
            .source
            .document
            .ends_with("schema-set/common.xsd")
    );
    assert!(
        ir.types[2]
            .source
            .document
            .ends_with("schema-set/sensor.xsd")
    );
}

#[test]
fn codegen_order_fixture_retains_schema_discovery_order() {
    let ir = load_schema_set(&fixture(
        "../../../../tests/fixtures/codegen-order/root.xsd",
    ))
    .expect("codegen order schema set should load");
    assert_eq!(
        ir.types
            .iter()
            .map(|declaration| declaration.name.local_name.as_str())
            .collect::<Vec<_>>(),
        ["Record_First_In_Source", "Included_Id", "Included_Quality"]
    );
}

#[test]
fn dependency_cycle_terminates_and_loads_each_document_once() {
    let ir = load_schema_set(&fixture("cycle/a.xsd")).expect("cycle should be benign");
    assert_eq!(
        ir.types
            .iter()
            .map(|declaration| declaration.name.local_name.as_str())
            .collect::<Vec<_>>(),
        ["A", "B"]
    );
}

#[test]
fn qname_prefixes_are_resolved_in_their_source_documents() {
    let ir = load_schema_set(&fixture("local-prefix/root.xsd")).expect("set should load");
    for (record_name, namespace, type_name) in [
        ("Root_Record", "urn:prefix:root", "Root_Id"),
        ("Other_Record", "urn:prefix:other", "Other_Id"),
    ] {
        let record = ir
            .types
            .iter()
            .find(|declaration| declaration.name.local_name == record_name)
            .expect("record should exist");
        let TypeKind::Record { fields } = &record.kind else {
            panic!("expected record");
        };
        assert_eq!(
            fields[0].type_ref.target,
            TypeRefTarget::Named(QualifiedName::new(namespace, type_name))
        );
    }
}

#[test]
fn schema_set_failures_are_explicit_and_contextual() {
    for (name, expected) in [
        (
            "include-missing-location.xsd",
            "xs:include is missing required attribute schemaLocation",
        ),
        (
            "include-mismatch-root.xsd",
            "xs:include target namespace mismatch",
        ),
        (
            "import-missing-namespace.xsd",
            "xs:import is missing required attribute namespace",
        ),
        (
            "import-missing-location.xsd",
            "xs:import is missing required attribute schemaLocation",
        ),
        (
            "import-mismatch-root.xsd",
            "xs:import target namespace mismatch",
        ),
        ("missing-file.xsd", "absent.xsd"),
        ("remote.xsd", "only local filesystem paths are allowed"),
        ("remote-http.xsd", "only local filesystem paths are allowed"),
        (
            "unresolved-root.xsd",
            "unresolved type reference {urn:other}Absent",
        ),
        (
            "duplicate-root.xsd",
            "duplicate type declaration {urn:duplicate}Repeated",
        ),
        ("chameleon-root.xsd", "targetNamespace"),
    ] {
        let path = fixture(&format!("errors/{name}"));
        let error = load_schema_set(&path).expect_err(name);
        let message = error.to_string();
        assert!(message.contains(expected), "{name}: {message}");
        assert!(
            message.contains(name)
                || message.contains("no-namespace.xsd")
                || message.contains("other.xsd")
                || message.contains("duplicate-child.xsd"),
            "missing source context: {message}"
        );
    }
}

#[test]
fn unsupported_choice_fails_closed() {
    let error = load_schema_document(&fixture("unsupported-choice.xsd"))
        .expect_err("choice must not be silently approximated");
    assert!(matches!(error, FrontendError::UnsupportedConstruct(_)));
    assert_eq!(
        error.to_string(),
        format!(
            "unsupported XSD construct: xs:choice at {}:7:5",
            fixture("unsupported-choice.xsd").display()
        )
    );
}

fn assert_named_type(actual: &TypeRefTarget, local_name: &str) {
    assert_eq!(
        actual,
        &TypeRefTarget::Named(QualifiedName::new(OMS_NS, local_name))
    );
}
