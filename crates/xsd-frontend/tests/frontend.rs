use ams_gra_oms_ir::{Cardinality, PrimitiveKind, QualifiedName, TypeKind, TypeRefTarget};
use ams_gra_oms_xsd_frontend::{FrontendError, load_schema_document, load_schema_set};
use std::fs;
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
fn annotations_normalize_into_existing_ir_documentation_fields() {
    let ir = load_schema_document(&fixture("annotations.xsd")).expect("annotations should parse");
    assert_eq!(
        ir.types.len(),
        3,
        "schema annotation must not add declarations"
    );
    assert_eq!(ir.types[0].name.local_name, "Count");
    assert_eq!(
        ir.types[0].documentation.as_deref(),
        Some("A formatted count type.\n\nSecond paragraph.")
    );
    assert_eq!(ir.types[0].constraints.min_inclusive, Some(1));
    assert_eq!(ir.types[0].constraints.max_inclusive, Some(4));

    let TypeKind::Enumeration { variants } = &ir.types[1].kind else {
        panic!("State should be an enumeration");
    };
    assert_eq!(ir.types[1].documentation.as_deref(), Some("State type."));
    assert_eq!(variants[0].wire_value, "Ready");
    assert_eq!(variants[0].documentation.as_deref(), Some("Ready to go."));

    let TypeKind::Record { fields } = &ir.types[2].kind else {
        panic!("Example should be a record");
    };
    assert_eq!(
        ir.types[2].documentation.as_deref(),
        Some("Example description")
    );
    assert_eq!(
        fields[0].documentation.as_deref(),
        Some("Field description.\n\nMore field detail.")
    );
    assert_eq!(fields[0].cardinality, Cardinality::REQUIRED_ONE);
    assert!(!fields[0].nillable);
    assert_named_annotation_type(&fields[0].type_ref.target, "Count");
    assert!(ir.types[2].source.document.ends_with("annotations.xsd"));
    assert!(fields[0].source.document.ends_with("annotations.xsd"));
}

#[test]
fn documentation_whitespace_normalization_is_formatting_independent() {
    let compact = write_temporary_schema(
        "compact",
        "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" targetNamespace=\"urn:whitespace\"><xs:simpleType name=\"Value\"><xs:annotation><xs:documentation>Same logical text.</xs:documentation></xs:annotation><xs:restriction base=\"xs:integer\"/></xs:simpleType></xs:schema>\n",
    );
    let formatted = write_temporary_schema(
        "formatted",
        "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" targetNamespace=\"urn:whitespace\">\n  <xs:simpleType name=\"Value\">\n    <xs:annotation><xs:documentation>\n      Same   logical\n      text.\n    </xs:documentation></xs:annotation>\n    <xs:restriction base=\"xs:integer\"/>\n  </xs:simpleType>\n</xs:schema>\n",
    );
    let compact_ir = load_schema_document(&compact).expect("compact schema should parse");
    let formatted_ir = load_schema_document(&formatted).expect("formatted schema should parse");
    fs::remove_file(compact).expect("compact temporary schema should be removable");
    fs::remove_file(formatted).expect("formatted temporary schema should be removable");
    assert_eq!(
        compact_ir.types[0].documentation,
        formatted_ir.types[0].documentation
    );
    assert_eq!(
        compact_ir.types[0].documentation.as_deref(),
        Some("Same logical text.")
    );
}

#[test]
fn unsupported_annotation_content_and_position_fail_closed() {
    for (name, construct, position) in [
        ("annotation-appinfo.xsd", "xs:appinfo", ":3:5"),
        ("annotation-unknown-child.xsd", "xs:other", ":3:5"),
        (
            "misplaced-annotation.xsd",
            "xs:annotation outside leading position",
            ":4:5",
        ),
    ] {
        let error = load_schema_document(&fixture(&format!("errors/{name}")))
            .expect_err("unsupported annotation syntax must fail");
        assert!(matches!(error, FrontendError::UnsupportedConstruct(_)));
        let message = error.to_string();
        assert!(message.contains(construct), "{name}: {message}");
        assert!(message.contains(position), "{name}: {message}");
    }
}

#[test]
fn enumeration_child_diagnostic_has_one_xsd_prefix() {
    let error = load_schema_document(&fixture("errors/enumeration-child.xsd"))
        .expect_err("unexpected enumeration child must fail");
    assert!(matches!(error, FrontendError::UnsupportedConstruct(_)));
    let message = error.to_string();
    assert!(message.contains("xs:enumeration child"));
    assert!(!message.contains("xs:xs:"), "{message}");
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
    assert_eq!(ir.types[0].source.line, Some(9));
    assert!(
        ir.types[1]
            .source
            .document
            .ends_with("element-form-default/types.xsd")
    );
    assert_eq!(ir.types[1].source.line, Some(7));
}

#[test]
fn accepts_attribute_form_defaults_and_discards_them_during_normalization() {
    let qualified_source = fs::read_to_string(fixture("attribute-form-default/qualified.xsd"))
        .expect("qualified fixture should be readable");
    let unqualified_source = fs::read_to_string(fixture("attribute-form-default/unqualified.xsd"))
        .expect("unqualified fixture should be readable");
    let temporary_path = std::env::temp_dir().join(format!(
        "ams-gra-attribute-form-default-{}.xsd",
        std::process::id()
    ));

    fs::write(&temporary_path, qualified_source).expect("temporary schema should be writable");
    let qualified =
        load_schema_document(&temporary_path).expect("qualified attribute form should parse");
    fs::write(&temporary_path, unqualified_source).expect("temporary schema should be replaceable");
    let unqualified =
        load_schema_document(&temporary_path).expect("unqualified attribute form should parse");
    fs::remove_file(&temporary_path).expect("temporary schema should be removable");

    assert_eq!(qualified, unqualified);
    assert_eq!(qualified.types.len(), 1);
    assert_eq!(qualified.types[0].name.local_name, "Value_Type");
}

#[test]
fn rejects_invalid_element_form_default_value() {
    let error = load_schema_document(&fixture("errors/invalid-element-form-default.xsd"))
        .expect_err("invalid form default must fail");
    assert!(matches!(error, FrontendError::InvalidInput(_)));
    let message = error.to_string();
    assert!(message.contains("invalid-element-form-default.xsd"));
    assert!(message.contains(
        "xs:schema @elementFormDefault must be qualified or unqualified, got sometimes at 2:1"
    ));
}

#[test]
fn rejects_invalid_attribute_form_default_value_with_source_position() {
    let error = load_schema_document(&fixture("errors/invalid-attribute-form-default.xsd"))
        .expect_err("invalid attribute form default must fail");
    assert!(matches!(error, FrontendError::InvalidInput(_)));
    assert_eq!(
        error.to_string(),
        format!(
            "invalid schema input: {}: xs:schema @attributeFormDefault must be qualified or unqualified, got invalid at 2:1",
            fixture("errors/invalid-attribute-form-default.xsd").display()
        )
    );
}

#[test]
fn form_default_support_does_not_allow_other_schema_attributes() {
    let error = load_schema_document(&fixture("errors/unsupported-schema-attribute.xsd"))
        .expect_err("unrelated schema attributes must remain unsupported");
    assert!(matches!(error, FrontendError::UnsupportedConstruct(_)));
    let message = error.to_string();
    assert!(message.contains("xs:schema @id"));
    assert!(message.contains("unsupported-schema-attribute.xsd:2:1"));
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

fn assert_named_annotation_type(actual: &TypeRefTarget, local_name: &str) {
    assert_eq!(
        actual,
        &TypeRefTarget::Named(QualifiedName::new("urn:annotations", local_name))
    );
}

fn write_temporary_schema(label: &str, contents: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "ams-gra-annotation-{label}-{}.xsd",
        std::process::id()
    ));
    fs::write(&path, contents).expect("temporary schema should be writable");
    path
}
