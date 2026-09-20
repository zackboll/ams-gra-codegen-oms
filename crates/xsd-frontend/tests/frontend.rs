use ams_gra_oms_ir::{
    Cardinality, Float32Value, Float64Value, NumericValue, PatternDialect, PrimitiveKind,
    QualifiedName, SchemaIr, TypeKind, TypeRefTarget, WhiteSpacePolicy,
};
use ams_gra_oms_xsd_frontend::{
    FrontendError, load_schema_document, load_schema_set, load_schema_set_with_overlays,
};
use std::fs;
use std::path::{Path, PathBuf};

const OMS_NS: &str = "urn:example:oms:track";
const INHERITANCE_NS: &str = "urn:inheritance";

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
    assert_eq!(
        track_id.constraints.min_inclusive,
        Some(NumericValue::Integer(1))
    );
    assert_eq!(
        track_id.constraints.max_inclusive,
        Some(NumericValue::Integer(65_535))
    );
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
fn normalizes_complex_inheritance_without_flattening() {
    let ir = load_schema_document(&fixture("complex-inheritance.xsd"))
        .expect("inheritance fixture should parse");
    let declaration = |name: &str| {
        ir.types
            .iter()
            .find(|declaration| declaration.name.local_name == name)
            .unwrap_or_else(|| panic!("{name} should exist"))
    };

    let base = declaration("Base");
    assert!(base.is_abstract);
    assert!(base.base_type.is_none());
    let TypeKind::Record { fields } = &base.kind else {
        panic!("Base should be a record");
    };
    assert_eq!(
        fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["BaseField"]
    );

    let middle = declaration("Middle");
    assert_named_inheritance_type(&middle.base_type.as_ref().unwrap().target, "Base");
    let TypeKind::Record { fields } = &middle.kind else {
        panic!("Middle should be a record");
    };
    assert_eq!(
        fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["MiddleField"]
    );
    assert!(!fields.iter().any(|field| field.name == "BaseField"));
    assert_eq!(fields[0].cardinality, Cardinality::OPTIONAL_ONE);
    assert!(fields[0].nillable);

    let leaf = declaration("Leaf");
    assert_eq!(leaf.documentation.as_deref(), Some("Leaf documentation."));
    assert!(leaf.source.document.ends_with("complex-inheritance.xsd"));
    assert_named_inheritance_type(&leaf.base_type.as_ref().unwrap().target, "Middle");
    let TypeKind::Record { fields } = &leaf.kind else {
        panic!("Leaf should be a record");
    };
    assert_eq!(
        fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["LeafField"]
    );
    assert_eq!(
        fields[0].documentation.as_deref(),
        Some("Leaf field documentation.")
    );
    assert_eq!(
        fields[0].constraints.min_inclusive,
        Some(NumericValue::Integer(-2_147_483_648))
    );
    assert_eq!(
        fields[0].constraints.max_inclusive,
        Some(NumericValue::Integer(2_147_483_647))
    );
    assert!(
        !fields
            .iter()
            .any(|field| matches!(field.name.as_str(), "BaseField" | "MiddleField"))
    );

    let choice = declaration("ChoiceLeaf");
    assert_named_inheritance_type(&choice.base_type.as_ref().unwrap().target, "Base");
    let TypeKind::Choice { alternatives } = &choice.kind else {
        panic!("ChoiceLeaf should be a choice");
    };
    assert_eq!(
        alternatives
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["Text", "Number"]
    );
    assert_eq!(alternatives[1].cardinality.max_occurs, None);

    let empty = declaration("EmptyLeaf");
    assert!(!empty.is_abstract);
    assert_named_inheritance_type(&empty.base_type.as_ref().unwrap().target, "Base");
    assert!(matches!(&empty.kind, TypeKind::Record { fields } if fields.is_empty()));

    assert_named_inheritance_type(&ir.messages[0].payload_type.target, "Leaf");
}

#[test]
fn complex_inheritance_fails_closed_on_unsupported_shapes() {
    for (label, body, expected) in [
        (
            "primitive-base",
            r#"<xs:extension base="xs:string"/>"#,
            "extension primitive base",
        ),
        (
            "restriction",
            r#"<xs:restriction base="t:Base"/>"#,
            "xs:restriction",
        ),
        (
            "attribute",
            r#"<xs:extension base="t:Base"><xs:attribute name="value" type="xs:string"/></xs:extension>"#,
            "xs:attribute",
        ),
        (
            "content-annotation",
            r#"<xs:annotation><xs:documentation>lost</xs:documentation></xs:annotation><xs:extension base="t:Base"/>"#,
            "multiple content-model children",
        ),
        (
            "extension-annotation",
            r#"<xs:extension base="t:Base"><xs:annotation><xs:documentation>lost</xs:documentation></xs:annotation></xs:extension>"#,
            "xs:annotation",
        ),
    ] {
        let path = write_temporary_schema(
            label,
            &format!(
                r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:test" targetNamespace="urn:test">
  <xs:complexType name="Base"><xs:sequence/></xs:complexType>
  <xs:complexType name="Derived"><xs:complexContent>{body}</xs:complexContent></xs:complexType>
</xs:schema>
"#
            ),
        );
        let error = load_schema_document(&path).expect_err(label);
        fs::remove_file(path).expect("temporary schema should be removable");
        assert!(matches!(error, FrontendError::UnsupportedConstruct(_)));
        assert!(error.to_string().contains(expected), "{label}: {error}");
    }
}

#[test]
fn complex_inheritance_reports_invalid_bases_and_abstract_values() {
    for (label, declarations, expected) in [
        (
            "unresolved-base",
            r#"<xs:complexType name="Derived"><xs:complexContent><xs:extension base="t:Absent"/></xs:complexContent></xs:complexType>"#,
            "unresolved type reference {urn:test}Absent in base type",
        ),
        (
            "invalid-abstract",
            r#"<xs:complexType name="Derived" abstract="yes"><xs:sequence/></xs:complexType>"#,
            "abstract is not an XSD boolean: yes",
        ),
    ] {
        let path = write_temporary_schema(
            label,
            &format!(
                r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:test" targetNamespace="urn:test">
  {declarations}
</xs:schema>
"#
            ),
        );
        let error = load_schema_document(&path).expect_err(label);
        fs::remove_file(path).expect("temporary schema should be removable");
        assert!(matches!(error, FrontendError::InvalidInput(_)));
        assert!(error.to_string().contains(expected), "{label}: {error}");
    }
}

#[test]
fn normalizes_binary_floating_primitives_by_namespace_uri() {
    let ir = load_schema_document(&fixture("floating-primitives.xsd"))
        .expect("floating primitive fixture should parse");
    let measurements = ir
        .types
        .iter()
        .find(|declaration| declaration.name.local_name == "Measurements")
        .expect("Measurements should exist");
    let TypeKind::Record { fields } = &measurements.kind else {
        panic!("Measurements should be a record");
    };

    assert_eq!(fields.len(), 2);
    assert_eq!(
        fields[0].type_ref.target,
        TypeRefTarget::Primitive(PrimitiveKind::Float32)
    );
    assert_eq!(fields[0].cardinality, Cardinality::REQUIRED_ONE);
    assert!(!fields[0].nillable);
    assert_eq!(
        fields[0].documentation.as_deref(),
        Some("Binary32 estimate.")
    );
    assert_eq!(
        fields[1].type_ref.target,
        TypeRefTarget::Primitive(PrimitiveKind::Float64)
    );
    assert_ne!(
        fields[1].type_ref.target,
        TypeRefTarget::Primitive(PrimitiveKind::Decimal)
    );
    assert_eq!(fields[1].cardinality, Cardinality::OPTIONAL_ONE);
    assert!(fields[1].nillable);
}

#[test]
fn uci_type_versions_are_validated_and_discarded() {
    let path = write_temporary_schema(
        "type-versions",
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
    xmlns:t="urn:type-versions"
    xmlns:uci="https://www.vdl.afrl.af.mil/programs/oam"
    targetNamespace="urn:type-versions">
  <xs:simpleType name="Count" uci:version="001.000.000.000">
    <xs:annotation><xs:documentation>Count documentation.</xs:documentation></xs:annotation>
    <xs:restriction base="xs:integer"><xs:minInclusive value="1"/></xs:restriction>
  </xs:simpleType>
  <xs:simpleType name="State" uci:version="002.000.000.000">
    <xs:restriction base="xs:string">
      <xs:enumeration value="Ready"><xs:annotation><xs:documentation>Ready state.</xs:documentation></xs:annotation></xs:enumeration>
    </xs:restriction>
  </xs:simpleType>
  <xs:complexType name="VersionedType" uci:version="003.000.000.000">
    <xs:annotation><xs:documentation>Type documentation.</xs:documentation></xs:annotation>
    <xs:sequence>
      <xs:element name="Count" type="t:Count" minOccurs="0" maxOccurs="4" nillable="true"/>
    </xs:sequence>
  </xs:complexType>
</xs:schema>
"#,
    );
    let versioned = load_schema_document(&path).expect("versioned types should parse");
    let without_versions = fs::read_to_string(&path)
        .expect("temporary schema should be readable")
        .replace(" uci:version=\"001.000.000.000\"", "")
        .replace(" uci:version=\"002.000.000.000\"", "")
        .replace(" uci:version=\"003.000.000.000\"", "");
    fs::write(&path, without_versions).expect("temporary schema should be replaceable");
    let unversioned = load_schema_document(&path).expect("generic types should still parse");
    fs::remove_file(path).expect("temporary schema should be removable");

    assert_eq!(versioned, unversioned);
    assert_eq!(
        versioned.types[0].constraints.min_inclusive,
        Some(NumericValue::Integer(1))
    );
    assert_eq!(
        versioned.types[0].documentation.as_deref(),
        Some("Count documentation.")
    );
    let TypeKind::Enumeration { variants } = &versioned.types[1].kind else {
        panic!("State should remain an enumeration");
    };
    assert_eq!(variants[0].documentation.as_deref(), Some("Ready state."));
    let TypeKind::Record { fields } = &versioned.types[2].kind else {
        panic!("VersionedType should remain a record");
    };
    assert_eq!(
        versioned.types[2].documentation.as_deref(),
        Some("Type documentation.")
    );
    assert_eq!(fields[0].cardinality.min_occurs, 0);
    assert_eq!(fields[0].cardinality.max_occurs, Some(4));
    assert!(fields[0].nillable);
    assert_eq!(
        fields[0].type_ref.target,
        TypeRefTarget::Named(QualifiedName::new("urn:type-versions", "Count"))
    );
}

#[test]
fn invalid_uci_type_version_metadata_fails_closed() {
    for (label, declaration, category, expected) in [
        (
            "empty-complex-version",
            r#"<xs:complexType name="Value" uci:version=""><xs:sequence/></xs:complexType>"#,
            "invalid",
            "xs:complexType UCI version attribute must not be empty",
        ),
        (
            "empty-simple-version",
            r#"<xs:simpleType name="Value" uci:version="   "><xs:restriction base="xs:integer"/></xs:simpleType>"#,
            "invalid",
            "xs:simpleType UCI version attribute must not be empty",
        ),
        (
            "wrong-complex-version-namespace",
            r#"<xs:complexType name="Value" other:version="001.000.000.000"><xs:sequence/></xs:complexType>"#,
            "unsupported",
            "xs:complexType @version",
        ),
        (
            "unqualified-simple-version",
            r#"<xs:simpleType name="Value" version="001.000.000.000"><xs:restriction base="xs:integer"/></xs:simpleType>"#,
            "unsupported",
            "xs:simpleType @version",
        ),
        (
            "unexpected-namespaced-type-attribute",
            r#"<xs:complexType name="Value" other:metadata="value"><xs:sequence/></xs:complexType>"#,
            "unsupported",
            "xs:complexType @metadata",
        ),
    ] {
        let path = write_temporary_schema(
            label,
            &format!(
                r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
    xmlns:uci="https://www.vdl.afrl.af.mil/programs/oam"
    xmlns:other="urn:not-uci" targetNamespace="urn:invalid-version">
  {declaration}
</xs:schema>
"#
            ),
        );
        let error = load_schema_document(&path).expect_err(label);
        fs::remove_file(path).expect("temporary schema should be removable");
        match category {
            "invalid" => assert!(matches!(error, FrontendError::InvalidInput(_))),
            "unsupported" => {
                assert!(matches!(error, FrontendError::UnsupportedConstruct(_)))
            }
            _ => unreachable!(),
        }
        assert!(error.to_string().contains(expected), "{label}: {error}");
    }
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
    assert_eq!(
        ir.types[0].constraints.min_inclusive,
        Some(NumericValue::Integer(1))
    );
    assert_eq!(
        ir.types[0].constraints.max_inclusive,
        Some(NumericValue::Integer(4))
    );

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
fn normalizes_choices_scalars_integer_ranges_and_unbounded_cardinality() {
    let path = fixture("choice-scalars.xsd");
    let ir = load_schema_document(&path).expect("Task 012 fixture should parse");
    let value = ir
        .types
        .iter()
        .find(|ty| ty.name.local_name == "Value")
        .unwrap();
    assert_eq!(
        value.documentation.as_deref(),
        Some("Choice documentation.")
    );
    let TypeKind::Choice { alternatives } = &value.kind else {
        panic!("expected choice")
    };
    assert_eq!(
        alternatives
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["IntegerValue", "PreciseValue", "NamedValue"]
    );
    assert_eq!(
        alternatives[0].documentation.as_deref(),
        Some("Integer alternative.")
    );
    assert_eq!(
        alternatives[0].constraints.min_inclusive,
        Some(NumericValue::Integer(-2_147_483_648))
    );
    assert_eq!(
        alternatives[0].constraints.max_inclusive,
        Some(NumericValue::Integer(2_147_483_647))
    );
    assert_eq!(
        alternatives[1].cardinality,
        Cardinality {
            min_occurs: 0,
            max_occurs: None
        }
    );
    assert!(alternatives[1].nillable);
    assert_eq!(
        alternatives[2].cardinality,
        Cardinality {
            min_occurs: 1,
            max_occurs: None
        }
    );
    assert_eq!(
        alternatives[2].type_ref.target,
        TypeRefTarget::Named(QualifiedName::new("urn:choice-scalars", "Named"))
    );
    assert!(
        alternatives
            .iter()
            .all(|field| field.source.document == path.display().to_string()
                && field.source.line.is_some())
    );

    let scalars = ir
        .types
        .iter()
        .find(|ty| ty.name.local_name == "Scalars")
        .unwrap();
    let TypeKind::Record { fields } = &scalars.kind else {
        panic!("expected record")
    };
    let expected = [
        (
            PrimitiveKind::SignedInteger,
            Some(NumericValue::Integer(-128)),
            Some(NumericValue::Integer(127)),
        ),
        (
            PrimitiveKind::SignedInteger,
            Some(NumericValue::Integer(-32_768)),
            Some(NumericValue::Integer(32_767)),
        ),
        (
            PrimitiveKind::SignedInteger,
            Some(NumericValue::Integer(-9_223_372_036_854_775_808)),
            Some(NumericValue::Integer(9_223_372_036_854_775_807)),
        ),
        (
            PrimitiveKind::UnsignedInteger,
            Some(NumericValue::Integer(0)),
            Some(NumericValue::Integer(255)),
        ),
        (
            PrimitiveKind::UnsignedInteger,
            Some(NumericValue::Integer(0)),
            Some(NumericValue::Integer(65_535)),
        ),
        (
            PrimitiveKind::UnsignedInteger,
            Some(NumericValue::Integer(0)),
            Some(NumericValue::Integer(4_294_967_295)),
        ),
        (PrimitiveKind::Boolean, None, None),
        (PrimitiveKind::DateTime, None, None),
        (PrimitiveKind::Binary, None, None),
    ];
    for (field, (kind, min, max)) in fields.iter().zip(expected) {
        assert_eq!(field.type_ref.target, TypeRefTarget::Primitive(kind));
        assert_eq!(
            (
                field.constraints.min_inclusive,
                field.constraints.max_inclusive
            ),
            (min, max)
        );
    }
    let port = ir
        .types
        .iter()
        .find(|ty| ty.name.local_name == "Port")
        .unwrap();
    assert_eq!(
        port.kind,
        TypeKind::Primitive(PrimitiveKind::UnsignedInteger)
    );
    assert_eq!(
        (
            port.constraints.min_inclusive,
            port.constraints.max_inclusive
        ),
        (
            Some(NumericValue::Integer(1)),
            Some(NumericValue::Integer(65_535))
        )
    );
    let positive = ir
        .types
        .iter()
        .find(|ty| ty.name.local_name == "PositiveInt")
        .unwrap();
    assert_eq!(
        (
            positive.constraints.min_exclusive,
            positive.constraints.max_inclusive
        ),
        (
            Some(NumericValue::Integer(0)),
            Some(NumericValue::Integer(1000))
        )
    );
    assert_eq!(ir, load_schema_document(&path).unwrap());
}

#[test]
fn task_012_structural_and_range_errors_fail_closed() {
    for (label, declaration, expected) in [
        (
            "choice-bounds",
            r#"<xs:complexType name="V"><xs:choice minOccurs="0"><xs:element name="A" type="xs:string"/></xs:choice></xs:complexType>"#,
            "choice @minOccurs",
        ),
        (
            "nested-choice",
            r#"<xs:complexType name="V"><xs:choice><xs:sequence/></xs:choice></xs:complexType>"#,
            "xs:sequence",
        ),
        (
            "contradictory-byte",
            r#"<xs:simpleType name="V"><xs:restriction base="xs:byte"><xs:minInclusive value="1000"/></xs:restriction></xs:simpleType>"#,
            "contradictory numeric constraints",
        ),
    ] {
        let path = write_temporary_schema(
            label,
            &format!(
                r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:errors">{declaration}</xs:schema>"#
            ),
        );
        let error = load_schema_document(&path).expect_err(label);
        fs::remove_file(path).unwrap();
        assert!(error.to_string().contains(expected), "{label}: {error}");
    }
}

#[test]
fn global_elements_normalize_as_ordered_documented_messages() {
    let path = fixture("global-elements.xsd");
    let ir = load_schema_document(&path).expect("global messages should parse");

    assert_eq!(
        ir.types.len(),
        1,
        "message elements are not duplicate types"
    );
    assert_eq!(ir.types[0].name.local_name, "TrackType");
    assert_eq!(
        ir.messages
            .iter()
            .map(|message| message.name.local_name.as_str())
            .collect::<Vec<_>>(),
        ["TrackMessage", "TrackUpdate"]
    );
    for message in &ir.messages {
        assert_eq!(message.name.namespace_uri, "urn:messages");
        assert_eq!(
            message.payload_type.target,
            TypeRefTarget::Named(QualifiedName::new("urn:messages", "TrackType"))
        );
        assert_eq!(message.source.document, path.display().to_string());
        assert!(message.source.line.is_some());
    }
    assert_eq!(
        ir.messages[0].documentation.as_deref(),
        Some("Track message purpose.\n\nAdditional message detail.")
    );
    assert_eq!(ir.messages[1].documentation, None);
    assert_eq!(
        ir.types[0].documentation.as_deref(),
        Some("Payload shape documentation.")
    );
    assert_eq!(
        ir,
        load_schema_document(&path).expect("message parsing should be deterministic")
    );
}

#[test]
fn schema_set_messages_follow_document_discovery_then_source_order() {
    let path = fixture("global-elements-set/root.xsd");
    let ir = load_schema_set(&path).expect("schema-set messages should parse");
    assert_eq!(
        ir.messages
            .iter()
            .map(|message| message.name.local_name.as_str())
            .collect::<Vec<_>>(),
        ["RootMessage", "IncludedMessage", "ImportedMessage"]
    );
    assert_eq!(
        ir.messages[1].payload_type.target,
        TypeRefTarget::Named(QualifiedName::new("urn:message-set", "IncludedType"))
    );
    assert_eq!(
        ir.messages[2].payload_type.target,
        TypeRefTarget::Named(QualifiedName::new("urn:imported-messages", "ImportedType"))
    );
}

#[test]
fn invalid_global_messages_fail_semantic_validation() {
    for (name, expected) in [
        (
            "duplicate-message-root.xsd",
            "duplicate message declaration {urn:duplicate-message}Repeated",
        ),
        (
            "unresolved-message.xsd",
            "unresolved type reference {urn:unresolved-message}MissingType in message payload",
        ),
    ] {
        let error = load_schema_set(&fixture(&format!("errors/{name}"))).expect_err(name);
        assert!(error.to_string().contains(expected), "{name}: {error}");
    }
}

#[test]
fn primitive_global_message_payload_is_rejected() {
    let path = fixture("errors/primitive-message-payload.xsd");
    let error = load_schema_document(&path)
        .expect_err("a primitive payload must not produce a MessageDecl");

    assert!(matches!(error, FrontendError::UnsupportedConstruct(_)));
    assert_eq!(
        error.to_string(),
        format!(
            "unsupported XSD construct: UCI message payload must reference a named schema type at {}:5:3",
            path.display()
        )
    );
}

#[test]
fn unsupported_primitives_fail_closed() {
    let label = "unsupported-date-field";
    let declaration = r#"<xs:complexType name="Value"><xs:sequence><xs:element name="Date" type="xs:date"/></xs:sequence></xs:complexType>"#;
    let path = write_temporary_schema(
        label,
        &format!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:unsupported-primitive">
  {declaration}
</xs:schema>
"#
        ),
    );
    let error = load_schema_document(&path).expect_err(label);
    fs::remove_file(path).expect("temporary schema should be removable");
    assert!(matches!(error, FrontendError::UnsupportedConstruct(_)));
    assert!(error.to_string().contains("xs:date"), "{label}: {error}");
}

#[test]
fn zero_facet_floating_restrictions_normalize_without_new_constraints() {
    for (label, base, kind) in [
        ("float-alias", "xs:float", PrimitiveKind::Float32),
        ("double-alias", "xs:double", PrimitiveKind::Float64),
    ] {
        let path = write_temporary_schema(
            label,
            &format!(
                r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:floating-alias">
  <xs:simpleType name="Value"><xs:restriction base="{base}"/></xs:simpleType>
</xs:schema>
"#
            ),
        );
        let ir = load_schema_document(&path).expect("zero-facet alias should normalize");
        fs::remove_file(path).expect("temporary schema should be removable");
        assert_eq!(ir.types[0].kind, TypeKind::Primitive(kind));
        assert_eq!(
            ir.types[0].base_type.as_ref().unwrap().target,
            TypeRefTarget::Primitive(kind)
        );
        assert_eq!(ir.types[0].constraints, Default::default());
    }
}

#[test]
fn normalizes_floating_ranges_with_width_correct_values() {
    for (label, base, facets, expected) in [
        (
            "double-integer-looking",
            "xs:double",
            r#"<xs:minInclusive value="-6378237"/>"#,
            (
                Some(NumericValue::Float64(Float64Value::from_value(-6378237.0))),
                None,
                None,
            ),
        ),
        (
            "double-fraction",
            "xs:double",
            r#"<xs:minInclusive value="-1.5"/><xs:maxInclusive value="2.25"/>"#,
            (
                Some(NumericValue::Float64(Float64Value::from_value(-1.5))),
                Some(NumericValue::Float64(Float64Value::from_value(2.25))),
                None,
            ),
        ),
        (
            "double-exclusive-zero",
            "xs:double",
            r#"<xs:minExclusive value="0.0"/>"#,
            (
                None,
                None,
                Some(NumericValue::Float64(Float64Value::from_value(0.0))),
            ),
        ),
        (
            "float-unit",
            "xs:float",
            r#"<xs:minInclusive value="0"/><xs:maxInclusive value="1"/>"#,
            (
                Some(NumericValue::Float32(Float32Value::from_value(0.0))),
                Some(NumericValue::Float32(Float32Value::from_value(1.0))),
                None,
            ),
        ),
    ] {
        let path = write_temporary_schema(
            label,
            &format!(
                r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:floating-range">
  <xs:simpleType name="Value"><xs:restriction base="{base}">{facets}</xs:restriction></xs:simpleType>
</xs:schema>
"#
            ),
        );
        let declaration = load_schema_document(&path).unwrap().types.remove(0);
        fs::remove_file(path).unwrap();
        assert_eq!(
            (
                declaration.constraints.min_inclusive,
                declaration.constraints.max_inclusive,
                declaration.constraints.min_exclusive,
            ),
            expected
        );
    }

    let path = write_temporary_schema(
        "equivalent-floating-spellings",
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:floating-spellings">
  <xs:simpleType name="IntegerSpelling"><xs:restriction base="xs:double"><xs:minInclusive value="0"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="DecimalSpelling"><xs:restriction base="xs:double"><xs:minInclusive value="0.0"/></xs:restriction></xs:simpleType>
</xs:schema>
"#,
    );
    let ir = load_schema_document(&path).unwrap();
    fs::remove_file(path).unwrap();
    assert_eq!(
        ir.types[0].constraints.min_inclusive,
        ir.types[1].constraints.min_inclusive
    );
}

#[test]
fn named_restrictions_resolve_forward_multilevel_constraints_without_reordering() {
    let path = write_temporary_schema(
        "named-restrictions",
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:named" targetNamespace="urn:named">
  <xs:simpleType name="Leaf"><xs:restriction base="t:Middle"><xs:maxInclusive value="100.0"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="Middle"><xs:restriction base="t:Root"/></xs:simpleType>
  <xs:simpleType name="Root"><xs:restriction base="xs:double"><xs:minInclusive value="0.0"/></xs:restriction></xs:simpleType>
</xs:schema>
"#,
    );
    let ir = load_schema_document(&path).unwrap();
    fs::remove_file(path).unwrap();
    ir.validate()
        .expect("frontend-normalized named restrictions should be valid IR");
    assert_eq!(
        ir.types
            .iter()
            .map(|declaration| declaration.name.local_name.as_str())
            .collect::<Vec<_>>(),
        ["Leaf", "Middle", "Root"]
    );
    assert_eq!(
        ir.types[0].kind,
        TypeKind::Primitive(PrimitiveKind::Float64)
    );
    assert_eq!(
        ir.types[0].base_type.as_ref().unwrap().target,
        TypeRefTarget::Named(QualifiedName::new("urn:named", "Middle"))
    );
    assert_eq!(
        ir.types[0].constraints.min_inclusive,
        Some(NumericValue::Float64(Float64Value::from_value(0.0)))
    );
    assert_eq!(
        ir.types[0].constraints.max_inclusive,
        Some(NumericValue::Float64(Float64Value::from_value(100.0)))
    );
}

#[test]
fn named_restrictions_intersect_lengths_and_reject_cycles_or_structural_bases() {
    let path = write_temporary_schema(
        "named-length",
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:named-length" targetNamespace="urn:named-length">
  <xs:simpleType name="Base"><xs:restriction base="xs:string"><xs:minLength value="4"/><xs:maxLength value="32"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="Derived"><xs:restriction base="t:Base"><xs:length value="8"/><xs:maxLength value="16"/></xs:restriction></xs:simpleType>
</xs:schema>
"#,
    );
    let ir = load_schema_document(&path).unwrap();
    fs::remove_file(path).unwrap();
    assert_eq!(ir.types[1].constraints.length, Some(8));
    assert_eq!(ir.types[1].constraints.min_length, Some(4));
    assert_eq!(ir.types[1].constraints.max_length, Some(16));

    for (label, declarations, expected) in [
        (
            "named-self-cycle",
            r#"<xs:simpleType name="A"><xs:restriction base="t:A"/></xs:simpleType>"#,
            "named simple-restriction cycle",
        ),
        (
            "named-multi-cycle",
            r#"<xs:simpleType name="A"><xs:restriction base="t:B"/></xs:simpleType><xs:simpleType name="B"><xs:restriction base="t:A"/></xs:simpleType>"#,
            "named simple-restriction cycle",
        ),
        (
            "named-structural-base",
            r#"<xs:simpleType name="A"><xs:restriction base="t:B"/></xs:simpleType><xs:complexType name="B"><xs:sequence/></xs:complexType>"#,
            "non-simple base",
        ),
    ] {
        let path = write_temporary_schema(
            label,
            &format!(
                r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:named-invalid" targetNamespace="urn:named-invalid">{declarations}</xs:schema>"#
            ),
        );
        let error = load_schema_document(&path).expect_err(label);
        fs::remove_file(path).unwrap();
        assert!(error.to_string().contains(expected), "{error}");
    }
}

/// Build and load a two-level named restriction chain over one primitive.
fn restriction_step_result(
    label: &str,
    base: &str,
    base_facets: &str,
    derived_facets: &str,
) -> Result<SchemaIr, FrontendError> {
    let path = write_temporary_schema(
        label,
        &format!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:steps" targetNamespace="urn:steps">
  <xs:simpleType name="Base"><xs:restriction base="{base}">{base_facets}</xs:restriction></xs:simpleType>
  <xs:simpleType name="Derived"><xs:restriction base="t:Base">{derived_facets}</xs:restriction></xs:simpleType>
</xs:schema>
"#
        ),
    );
    let result = load_schema_document(&path);
    fs::remove_file(path).unwrap();
    result
}

/// Corrective cleanup: an illegal authored restriction STEP must be rejected
/// even though effective intersection would silently normalize it away.
///
/// `Base minInclusive = 10` then `Derived minInclusive = 0` intersects back to
/// `>= 10`, so the generated domain never widens -- but the derivation is
/// invalid XSD, and normalization must not destroy that evidence.
#[test]
fn numeric_restriction_steps_reject_weakened_bounds_across_primitive_families() {
    for (base, low, high, weak_min, strong_min, weak_max, strong_max) in [
        ("xs:long", "10", "100", "0", "20", "1000", "50"),
        // `xs:unsignedInt` carries an intrinsic 0 ..= 4294967295 domain, so
        // the weak upper bound stays inside it and is rejected for weakening
        // rather than for exceeding the width.
        ("xs:unsignedInt", "10", "100", "0", "20", "1000", "50"),
        ("xs:float", "10.0", "100.0", "0.0", "20.0", "1000.0", "50.0"),
        (
            "xs:double",
            "10.0",
            "100.0",
            "0.0",
            "20.0",
            "1000.0",
            "50.0",
        ),
    ] {
        let base_facets =
            format!(r#"<xs:minInclusive value="{low}"/><xs:maxInclusive value="{high}"/>"#);

        // Strengthening in either direction is legal.
        for (label, facets) in [
            (
                "step-strong-min",
                format!(r#"<xs:minInclusive value="{strong_min}"/>"#),
            ),
            (
                "step-strong-max",
                format!(r#"<xs:maxInclusive value="{strong_max}"/>"#),
            ),
        ] {
            assert!(
                restriction_step_result(label, base, &base_facets, &facets).is_ok(),
                "{base} {label}: strengthening is a legal restriction"
            );
        }

        // Weakening either bound is not, even though intersection hides it.
        for (label, facets, expected) in [
            (
                "step-weak-min",
                format!(r#"<xs:minInclusive value="{weak_min}"/>"#),
                "weakens the inherited lower bound",
            ),
            (
                "step-weak-max",
                format!(r#"<xs:maxInclusive value="{weak_max}"/>"#),
                "weakens the inherited upper bound",
            ),
        ] {
            let error = restriction_step_result(label, base, &base_facets, &facets)
                .expect_err("a weakened bound must be rejected");
            assert!(error.to_string().contains(expected), "{base}: {error}");
        }
    }
}

/// Inclusive/exclusive strength at an EQUAL value. `> 10` is strictly stronger
/// than `>= 10`, so tightening that way is legal and loosening is not.
#[test]
fn equal_value_inclusive_exclusive_strength_is_directional() {
    assert!(
        restriction_step_result(
            "step-incl-to-excl",
            "xs:long",
            r#"<xs:minInclusive value="10"/>"#,
            r#"<xs:minExclusive value="10"/>"#
        )
        .is_ok(),
        "minInclusive 10 -> minExclusive 10 strengthens the domain"
    );

    let error = restriction_step_result(
        "step-excl-to-incl",
        "xs:long",
        r#"<xs:minExclusive value="10"/>"#,
        r#"<xs:minInclusive value="10"/>"#,
    )
    .expect_err("minExclusive 10 -> minInclusive 10 readmits 10");
    assert!(
        error
            .to_string()
            .contains("weakens the inherited lower bound"),
        "{error}"
    );
}

/// One restriction step cannot declare both spellings of the same bound.
#[test]
fn a_restriction_step_cannot_declare_both_bound_spellings() {
    for (label, facets, expected) in [
        (
            "both-min",
            r#"<xs:minInclusive value="1"/><xs:minExclusive value="2"/>"#,
            "both minInclusive and minExclusive",
        ),
        (
            "both-max",
            r#"<xs:maxInclusive value="9"/><xs:maxExclusive value="8"/>"#,
            "both maxInclusive and maxExclusive",
        ),
    ] {
        let error = restriction_step_result(label, "xs:long", "", facets)
            .expect_err("a doubled bound spelling must be rejected");
        assert!(error.to_string().contains(expected), "{label}: {error}");
    }
}

/// The same rule for String/Binary length bounds.
#[test]
fn length_restriction_steps_reject_weakened_bounds() {
    let base = r#"<xs:minLength value="4"/><xs:maxLength value="32"/>"#;
    for (label, facets, expected) in [
        (
            "weak-min-length",
            r#"<xs:minLength value="2"/>"#,
            Some("weakens the inherited minimum length"),
        ),
        (
            "weak-max-length",
            r#"<xs:maxLength value="64"/>"#,
            Some("weakens the inherited maximum length"),
        ),
        (
            "length-below-interval",
            r#"<xs:length value="2"/>"#,
            Some("outside the inherited length bounds"),
        ),
        (
            "length-above-interval",
            r#"<xs:length value="64"/>"#,
            Some("outside the inherited length bounds"),
        ),
        ("strong-min-length", r#"<xs:minLength value="8"/>"#, None),
        ("strong-max-length", r#"<xs:maxLength value="16"/>"#, None),
        ("length-within-interval", r#"<xs:length value="8"/>"#, None),
    ] {
        let result = restriction_step_result(label, "xs:string", base, facets);
        match expected {
            Some(expected) => {
                let error = result.expect_err(label);
                assert!(error.to_string().contains(expected), "{label}: {error}");
            }
            None => assert!(result.is_ok(), "{label} must remain legal"),
        }
    }

    // An inherited EXACT length cannot be changed to a different exact length.
    let error = restriction_step_result(
        "exact-length-change",
        "xs:hexBinary",
        r#"<xs:length value="8"/>"#,
        r#"<xs:length value="4"/>"#,
    )
    .expect_err("changing an inherited exact length must be rejected");
    assert!(
        error
            .to_string()
            .contains("changes the inherited exact length"),
        "{error}"
    );
    // Restating the same exact length is a no-op restriction and stays legal.
    assert!(
        restriction_step_result(
            "exact-length-same",
            "xs:hexBinary",
            r#"<xs:length value="8"/>"#,
            r#"<xs:length value="8"/>"#
        )
        .is_ok()
    );
}

/// Validation is against the EFFECTIVE IMMEDIATE BASE, not the original
/// primitive. `Leaf` below weakens a bound `Middle` introduced, which the
/// root primitive never had.
#[test]
fn restriction_steps_are_validated_against_the_effective_immediate_base() {
    let path = write_temporary_schema(
        "multilevel-step",
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:steps" targetNamespace="urn:steps">
  <xs:simpleType name="Root"><xs:restriction base="xs:long"><xs:minInclusive value="0"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="Middle"><xs:restriction base="t:Root"><xs:minInclusive value="50"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="Leaf"><xs:restriction base="t:Middle"><xs:minInclusive value="10"/></xs:restriction></xs:simpleType>
</xs:schema>
"#,
    );
    let error = load_schema_document(&path).expect_err("Leaf weakens Middle's bound");
    fs::remove_file(path).unwrap();
    assert!(
        error
            .to_string()
            .contains("weakens the inherited lower bound"),
        "10 is legal against Root but not against Middle: {error}"
    );

    // The same chain strengthening at every step remains valid, and the
    // effective intersection is preserved rather than replaced.
    let path = write_temporary_schema(
        "multilevel-step-valid",
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:steps" targetNamespace="urn:steps">
  <xs:simpleType name="Root"><xs:restriction base="xs:long"><xs:minInclusive value="0"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="Middle"><xs:restriction base="t:Root"><xs:minInclusive value="50"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="Leaf"><xs:restriction base="t:Middle"><xs:minInclusive value="75"/></xs:restriction></xs:simpleType>
</xs:schema>
"#,
    );
    let ir = load_schema_document(&path).expect("a monotonically strengthening chain is valid");
    fs::remove_file(path).unwrap();
    let leaf = ir
        .types
        .iter()
        .find(|declaration| declaration.name.local_name == "Leaf")
        .expect("Leaf must be declared");
    assert_eq!(
        leaf.constraints.min_inclusive,
        Some(NumericValue::Integer(75))
    );
}

#[test]
fn floating_special_range_values_and_contradictions_fail_closed() {
    for (label, base, facets, expected) in [
        (
            "double-nan",
            "xs:double",
            r#"<xs:minInclusive value="NaN"/>"#,
            "finite",
        ),
        (
            "double-inf",
            "xs:double",
            r#"<xs:maxInclusive value="INF"/>"#,
            "finite",
        ),
        (
            "double-inverted",
            "xs:double",
            r#"<xs:minInclusive value="10.0"/><xs:maxInclusive value="5.0"/>"#,
            "contradictory",
        ),
        (
            "double-exclusive-equal",
            "xs:double",
            r#"<xs:minExclusive value="1.0"/><xs:maxInclusive value="1.0"/>"#,
            "contradictory",
        ),
        (
            "float-exclusive-equal",
            "xs:float",
            r#"<xs:minInclusive value="2.0"/><xs:maxExclusive value="2.0"/>"#,
            "contradictory",
        ),
    ] {
        let path = write_temporary_schema(
            label,
            &format!(
                r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:floating-invalid"><xs:simpleType name="Value"><xs:restriction base="{base}">{facets}</xs:restriction></xs:simpleType></xs:schema>"#
            ),
        );
        let error = load_schema_document(&path).expect_err(label);
        fs::remove_file(path).unwrap();
        assert!(error.to_string().contains(expected), "{error}");
    }
}

#[test]
fn normalizes_temporal_and_scalar_restrictions() {
    let ir = load_schema_document(&fixture("scalar-restrictions.xsd"))
        .expect("scalar restriction fixture should parse");
    let declaration = |name: &str| {
        ir.types
            .iter()
            .find(|declaration| declaration.name.local_name == name)
            .unwrap_or_else(|| panic!("{name} should exist"))
    };

    let binary = declaration("BinaryExact");
    assert_eq!(binary.kind, TypeKind::Primitive(PrimitiveKind::Binary));
    assert_eq!(
        binary.base_type.as_ref().unwrap().target,
        TypeRefTarget::Primitive(PrimitiveKind::Binary)
    );
    assert_eq!(binary.constraints.length, Some(6));

    let string = declaration("StringRule");
    assert_eq!(string.kind, TypeKind::Primitive(PrimitiveKind::String));
    assert_eq!(string.constraints.min_length, Some(2));
    assert_eq!(string.constraints.max_length, Some(8));
    let pattern_groups = &string.constraints.lexical.pattern_groups;
    assert_eq!(pattern_groups.len(), 1);
    assert_eq!(
        pattern_groups[0]
            .alternatives
            .iter()
            .map(|pattern| (pattern.dialect, pattern.expression.as_str()))
            .collect::<Vec<_>>(),
        [
            (PatternDialect::XmlSchema, "[A-Z]+"),
            (PatternDialect::XmlSchema, ".*Z")
        ]
    );

    let duration = declaration("DurationAlias");
    assert_eq!(duration.kind, TypeKind::Primitive(PrimitiveKind::Duration));
    assert_eq!(
        duration.base_type.as_ref().unwrap().target,
        TypeRefTarget::Primitive(PrimitiveKind::Duration)
    );
    assert_eq!(duration.constraints, Default::default());
    assert_eq!(
        declaration("StringAlias").kind,
        TypeKind::Primitive(PrimitiveKind::String)
    );

    let annotated_pattern = declaration("AnnotatedPattern");
    let plain_pattern = declaration("PlainPattern");
    assert_eq!(
        annotated_pattern.kind,
        TypeKind::Primitive(PrimitiveKind::String)
    );
    assert_eq!(
        annotated_pattern.constraints.lexical.pattern_groups[0].alternatives[0].expression,
        r"NATO:[a-zA-Z\-_]{1,256}"
    );
    assert_eq!(annotated_pattern.constraints, plain_pattern.constraints);

    let TypeKind::Record { fields } = &declaration("TemporalRecord").kind else {
        panic!("TemporalRecord should be a record");
    };
    assert_eq!(
        fields[0].type_ref.target,
        TypeRefTarget::Primitive(PrimitiveKind::Duration)
    );
    assert_eq!(fields[0].cardinality, Cardinality::REQUIRED_ONE);
    assert_eq!(
        fields[1].type_ref.target,
        TypeRefTarget::Primitive(PrimitiveKind::Time)
    );
    assert_eq!(fields[1].cardinality, Cardinality::OPTIONAL_ONE);
    assert!(fields[1].nillable);
    assert_eq!(fields[1].documentation.as_deref(), Some("Time of day."));
    assert_ne!(PrimitiveKind::Time, PrimitiveKind::DateTime);
    assert_ne!(PrimitiveKind::Duration, PrimitiveKind::DateTime);
    assert_ne!(PrimitiveKind::Duration, PrimitiveKind::String);
}

#[test]
fn scalar_restrictions_reject_invalid_lengths_and_lexical_facets() {
    for (label, base, facets, category, expected) in [
        (
            "duplicate-length",
            "xs:hexBinary",
            r#"<xs:length value="4"/><xs:length value="8"/>"#,
            "invalid",
            "duplicate xs:length facet",
        ),
        (
            "negative-length",
            "xs:string",
            r#"<xs:length value="-1"/>"#,
            "invalid",
            "length facet is not a non-negative integer: -1",
        ),
        (
            "fractional-min-length",
            "xs:string",
            r#"<xs:minLength value="1.5"/>"#,
            "invalid",
            "minLength facet is not a non-negative integer: 1.5",
        ),
        (
            "text-max-length",
            "xs:string",
            r#"<xs:maxLength value="abc"/>"#,
            "invalid",
            "maxLength facet is not a non-negative integer: abc",
        ),
        (
            "contradictory-min",
            "xs:string",
            r#"<xs:length value="4"/><xs:minLength value="5"/>"#,
            "invalid",
            "contradictory length constraints",
        ),
        (
            "contradictory-max",
            "xs:hexBinary",
            r#"<xs:length value="6"/><xs:maxLength value="5"/>"#,
            "invalid",
            "contradictory length constraints",
        ),
        (
            "binary-pattern",
            "xs:hexBinary",
            r#"<xs:pattern value="[0-9A-F]+"/>"#,
            "unsupported",
            "xs:pattern",
        ),
        (
            "unexpected-length-child",
            "xs:string",
            r#"<xs:length value="4"><xs:unexpected/></xs:length>"#,
            "unsupported",
            "xs:unexpected",
        ),
        (
            "misplaced-pattern-annotation",
            "xs:string",
            r#"<xs:pattern value="[A-Z]+"><xs:unexpected/><xs:annotation><xs:documentation>Too late.</xs:documentation></xs:annotation></xs:pattern>"#,
            "unsupported",
            "xs:annotation outside leading position",
        ),
        (
            "pattern-appinfo",
            "xs:string",
            r#"<xs:pattern value="[A-Z]+"><xs:annotation><xs:appinfo/></xs:annotation></xs:pattern>"#,
            "unsupported",
            "xs:appinfo",
        ),
    ] {
        let path = write_temporary_schema(
            label,
            &format!(
                r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:scalar-error">
  <xs:simpleType name="Value"><xs:restriction base="{base}">{facets}</xs:restriction></xs:simpleType>
</xs:schema>
"#
            ),
        );
        let error = load_schema_document(&path).expect_err(label);
        fs::remove_file(path).expect("temporary schema should be removable");
        match category {
            "invalid" => assert!(
                matches!(error, FrontendError::InvalidInput(_)),
                "{label}: {error}"
            ),
            "unsupported" => assert!(
                matches!(error, FrontendError::UnsupportedConstruct(_)),
                "{label}: {error}"
            ),
            _ => unreachable!(),
        }
        assert!(error.to_string().contains(expected), "{label}: {error}");
    }
}

#[test]
fn generic_global_element_without_uci_version_is_not_a_message() {
    let path = fixture("errors/generic-global-element.xsd");
    let error = load_schema_document(&path).expect_err("generic global must not become a message");
    assert!(matches!(error, FrontendError::InvalidInput(_)));
    assert_eq!(
        error.to_string(),
        format!(
            "invalid schema input: {}: xs:element is missing required UCI version attribute at 4:3",
            path.display()
        )
    );
}

#[test]
fn empty_global_message_version_is_invalid() {
    let path = fixture("errors/global-element-empty-version.xsd");
    let error = load_schema_document(&path).expect_err("empty UCI version must be rejected");
    assert!(matches!(error, FrontendError::InvalidInput(_)));
    assert_eq!(
        error.to_string(),
        format!(
            "invalid schema input: {}: xs:element UCI version attribute must not be empty at 5:3",
            path.display()
        )
    );
}

#[test]
fn unsupported_global_element_variants_fail_closed() {
    for (name, expected) in [
        (
            "global-element-anonymous-type.xsd",
            "xs:anonymous global element type",
        ),
        (
            "global-element-substitution-group.xsd",
            "xs:element @substitutionGroup",
        ),
    ] {
        let error = load_schema_document(&fixture(&format!("errors/{name}"))).expect_err(name);
        assert!(matches!(error, FrontendError::UnsupportedConstruct(_)));
        assert!(error.to_string().contains(expected), "{name}: {error}");
    }
}

#[test]
fn normalizes_lexical_constraints_by_restriction_level() {
    let path = write_temporary_schema(
        "lexical-constraints",
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:lexical" targetNamespace="urn:lexical">
  <xs:simpleType name="Base"><xs:restriction base="xs:string"><xs:pattern value="A"/><xs:pattern value="B"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="Derived"><xs:restriction base="t:Base"><xs:pattern value="C"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="DateTimeZulu"><xs:restriction base="xs:dateTime"><xs:pattern value=".+Z"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="TimeZulu"><xs:restriction base="xs:time"><xs:pattern value=".+Z"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="Serial"><xs:restriction base="xs:int"><xs:minInclusive value="1"/><xs:maxInclusive value="999"/><xs:pattern value="[0-9]{1,3}"/></xs:restriction></xs:simpleType>
</xs:schema>
"#,
    );
    let ir = load_schema_document(&path).expect("lexical restrictions should normalize");
    fs::remove_file(path).expect("temporary schema should be removable");
    let declaration = |name: &str| {
        ir.types
            .iter()
            .find(|declaration| declaration.name.local_name == name)
            .unwrap()
    };
    let expressions = |name: &str| {
        declaration(name)
            .constraints
            .lexical
            .pattern_groups
            .iter()
            .map(|group| {
                group
                    .alternatives
                    .iter()
                    .map(|pattern| (pattern.dialect, pattern.expression.as_str()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };

    assert_eq!(
        expressions("Base"),
        [vec![
            (PatternDialect::XmlSchema, "A"),
            (PatternDialect::XmlSchema, "B")
        ]]
    );
    assert_eq!(
        expressions("Derived"),
        [
            vec![
                (PatternDialect::XmlSchema, "A"),
                (PatternDialect::XmlSchema, "B")
            ],
            vec![(PatternDialect::XmlSchema, "C")]
        ]
    );
    assert_eq!(
        declaration("Derived").base_type.as_ref().unwrap().target,
        TypeRefTarget::Named(QualifiedName::new("urn:lexical", "Base"))
    );
    for (name, kind) in [
        ("DateTimeZulu", PrimitiveKind::DateTime),
        ("TimeZulu", PrimitiveKind::Time),
    ] {
        assert_eq!(declaration(name).kind, TypeKind::Primitive(kind));
        assert_eq!(
            expressions(name),
            [vec![(PatternDialect::XmlSchema, ".+Z")]]
        );
    }
    let serial = declaration("Serial");
    assert_eq!(
        serial.constraints.min_inclusive,
        Some(NumericValue::Integer(1))
    );
    assert_eq!(
        serial.constraints.max_inclusive,
        Some(NumericValue::Integer(999))
    );
    assert_eq!(
        expressions("Serial"),
        [vec![(PatternDialect::XmlSchema, "[0-9]{1,3}")]]
    );
}

#[test]
fn normalizes_and_strictly_parses_string_white_space() {
    let path = write_temporary_schema(
        "white-space",
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:white-space" targetNamespace="urn:white-space">
  <xs:simpleType name="Preserved"><xs:restriction base="xs:string"><xs:whiteSpace value="preserve"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="Replaced"><xs:restriction base="xs:string"><xs:whiteSpace value="replace"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="Combined"><xs:restriction base="xs:string"><xs:minLength value="0"/><xs:maxLength value="1024"/><xs:whiteSpace value="collapse"/><xs:pattern value="[ -~\n\r]{0,1024}"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="Derived"><xs:restriction base="t:Replaced"><xs:whiteSpace value="collapse"/></xs:restriction></xs:simpleType>
</xs:schema>
"#,
    );
    let ir = load_schema_document(&path).expect("whiteSpace restrictions should normalize");
    fs::remove_file(path).expect("temporary schema should be removable");
    let declaration = |name: &str| {
        ir.types
            .iter()
            .find(|item| item.name.local_name == name)
            .unwrap()
    };
    assert_eq!(
        declaration("Preserved").constraints.lexical.white_space,
        Some(WhiteSpacePolicy::Preserve)
    );
    assert_eq!(
        declaration("Replaced").constraints.lexical.white_space,
        Some(WhiteSpacePolicy::Replace)
    );
    let combined = declaration("Combined");
    assert_eq!(combined.constraints.min_length, Some(0));
    assert_eq!(combined.constraints.max_length, Some(1024));
    assert_eq!(
        combined.constraints.lexical.white_space,
        Some(WhiteSpacePolicy::Collapse)
    );
    assert_eq!(
        combined.constraints.lexical.pattern_groups[0].alternatives[0].expression,
        r"[ -~\n\r]{0,1024}"
    );
    assert_eq!(
        declaration("Derived").constraints.lexical.white_space,
        Some(WhiteSpacePolicy::Collapse)
    );

    for invalid in ["Collapse", "trim", "", "unknown"] {
        let path = write_temporary_schema(
            "invalid-white-space",
            &format!(
                r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:invalid"><xs:simpleType name="Invalid"><xs:restriction base="xs:string"><xs:whiteSpace value="{invalid}"/></xs:restriction></xs:simpleType></xs:schema>
"#
            ),
        );
        let error = load_schema_document(&path).expect_err("invalid whiteSpace must fail");
        fs::remove_file(path).expect("temporary schema should be removable");
        assert!(error.to_string().contains("invalid whiteSpace facet value"));
    }
}

// ---------------------------------------------------------------------------
// Task 029 -- additive same-namespace schema overlays
// ---------------------------------------------------------------------------

const OVERLAY_NS: &str = "urn:overlay";

fn overlay_fixture(name: &str) -> PathBuf {
    fixture("schema-overlay").join(name)
}

fn type_names(ir: &SchemaIr) -> Vec<&str> {
    ir.types
        .iter()
        .map(|declaration| declaration.name.local_name.as_str())
        .collect()
}

fn immediate_base(ir: &SchemaIr, local_name: &str) -> Option<TypeRefTarget> {
    ir.types
        .iter()
        .find(|declaration| declaration.name.local_name == local_name)?
        .base_type
        .as_ref()
        .map(|base| base.target.clone())
}

/// Section 27: the public root alone declares the extension point and no
/// descendant at all. The private document is deliberately NOT xs:include'd,
/// so it can only enter through explicit overlay composition.
#[test]
fn overlay_fixture_without_overlay_has_no_private_descendant() {
    let ir = load_schema_set(&overlay_fixture("public.xsd")).expect("public root should load");
    assert_eq!(type_names(&ir), ["ExtensionBase", "Container"]);
    assert_eq!(
        ir.types
            .iter()
            .filter(|declaration| {
                declaration.base_type.as_ref().is_some_and(|base| {
                    base.target
                        == TypeRefTarget::Named(QualifiedName::new(OVERLAY_NS, "ExtensionBase"))
                })
            })
            .count(),
        0,
        "the public root alone must have zero known concrete descendants"
    );
}

/// Section 28: the same root plus the private overlay yields one namespace and
/// the private descendant, with primary declarations retaining their order and
/// overlay declarations appended after them.
#[test]
fn overlay_adds_private_descendant_after_primary_declarations() {
    let ir = load_schema_set_with_overlays(
        &overlay_fixture("public.xsd"),
        &[overlay_fixture("private-a.xsd")],
    )
    .expect("root plus overlay should load");

    assert_eq!(ir.namespaces.len(), 1);
    assert_eq!(ir.namespaces[0].uri, OVERLAY_NS);
    assert_eq!(type_names(&ir), ["ExtensionBase", "Container", "PrivateA"]);
    assert_eq!(
        immediate_base(&ir, "PrivateA"),
        Some(TypeRefTarget::Named(QualifiedName::new(
            OVERLAY_NS,
            "ExtensionBase"
        ))),
        "section 14: an overlay type may extend a primary-root type"
    );

    let private = ir
        .types
        .iter()
        .find(|declaration| declaration.name.local_name == "PrivateA")
        .expect("PrivateA should exist");
    assert_eq!(private.name.namespace_uri, OVERLAY_NS);
    assert!(
        private.source.document.contains("private-a.xsd"),
        "section 37: provenance stays available through the existing SourceRef"
    );
}

/// Section 31: several overlays compose in caller-provided order, and reversing
/// the CLI order deterministically reverses their declaration order. Overlay
/// paths are never sorted, because argument order is the reproducible input and
/// absolute filesystem paths vary by machine.
#[test]
fn two_overlays_follow_caller_order_and_are_never_sorted() {
    let forward = load_schema_set_with_overlays(
        &overlay_fixture("public.xsd"),
        &[
            overlay_fixture("private-a.xsd"),
            overlay_fixture("private-b.xsd"),
        ],
    )
    .expect("two overlays should compose");
    assert_eq!(
        type_names(&forward),
        ["ExtensionBase", "Container", "PrivateA", "PrivateB"]
    );

    let reversed = load_schema_set_with_overlays(
        &overlay_fixture("public.xsd"),
        &[
            overlay_fixture("private-b.xsd"),
            overlay_fixture("private-a.xsd"),
        ],
    )
    .expect("reversed overlays should also compose");
    assert_eq!(
        type_names(&reversed),
        ["ExtensionBase", "Container", "PrivateB", "PrivateA"]
    );
}

/// Sections 8/21: one canonical document is loaded once, whether the caller
/// repeats the overlay path or spells the same file differently.
#[test]
fn repeated_overlay_path_loads_the_document_once() {
    let ir = load_schema_set_with_overlays(
        &overlay_fixture("public.xsd"),
        &[
            overlay_fixture("private-a.xsd"),
            overlay_fixture("private-a.xsd"),
            overlay_fixture(".").join("private-a.xsd"),
        ],
    )
    .expect("repeating one overlay must not duplicate declarations");
    assert_eq!(type_names(&ir), ["ExtensionBase", "Container", "PrivateA"]);
}

/// Section 32: an overlay the root already reaches through xs:include is still
/// loaded exactly once; canonical-path identity, not input route, decides.
#[test]
fn overlay_already_reachable_from_root_is_not_duplicated() {
    let ir = load_schema_set_with_overlays(
        &overlay_fixture("including-root.xsd"),
        &[overlay_fixture("private-a.xsd")],
    )
    .expect("already-included overlay should be accepted");
    assert_eq!(type_names(&ir), ["ExtensionBase", "Container", "PrivateA"]);
}

/// Section 12: the primary root remains authoritative for the schema version
/// even when an overlay declares a different one.
#[test]
fn overlay_schema_version_never_replaces_the_root_version() {
    let ir = load_schema_set_with_overlays(
        &overlay_fixture("versioned-root.xsd"),
        &[overlay_fixture("versioned-overlay.xsd")],
    )
    .expect("differing versions should still load");
    assert_eq!(ir.schema_version.as_deref(), Some("1"));
    assert_eq!(type_names(&ir), ["RootCarrier", "OverlayCarrier"]);
}

/// Sections 13/33: an overlay binding the shared namespace to a different
/// lexical prefix resolves normally and does not change the presentation
/// metadata recorded from the primary root.
#[test]
fn overlay_prefix_difference_has_no_semantic_effect() {
    let ir = load_schema_set_with_overlays(
        &overlay_fixture("public.xsd"),
        &[overlay_fixture("private-a.xsd")],
    )
    .expect("differently prefixed overlay should load");
    assert_eq!(ir.namespaces.len(), 1);
    assert_eq!(
        ir.namespaces[0].preferred_prefix.as_deref(),
        Some("pub"),
        "the primary root's first-seen prefix stays authoritative"
    );
}

/// Section 14: a pending named simple restriction declared in an overlay may
/// restrict a compatible named base loaded from the primary root, because
/// resolution runs only after all documents are assembled.
#[test]
fn overlay_named_restriction_resolves_against_a_root_base() {
    let ir = load_schema_set_with_overlays(
        &overlay_fixture("restriction-root.xsd"),
        &[overlay_fixture("restriction-overlay.xsd")],
    )
    .expect("overlay restriction of a root base should resolve");
    let narrowed = ir
        .types
        .iter()
        .find(|declaration| declaration.name.local_name == "OverlayNarrowedInt")
        .expect("OverlayNarrowedInt should exist");
    assert_eq!(
        narrowed.kind,
        TypeKind::Primitive(PrimitiveKind::SignedInteger)
    );
    assert_eq!(
        narrowed.constraints.min_inclusive,
        Some(NumericValue::Integer(10))
    );
    assert_eq!(
        narrowed.constraints.max_inclusive,
        Some(NumericValue::Integer(20))
    );
}

/// Section 15: a root reference that is unresolved on its own may be satisfied
/// by an explicitly supplied overlay. Resolution never special-cases whether a
/// declaration arrived from the root or from an overlay.
#[test]
fn root_reference_may_be_satisfied_by_an_explicit_overlay() {
    let error = load_schema_set(&overlay_fixture("root-needing-overlay.xsd"))
        .expect_err("the root alone leaves the reference unresolved");
    assert!(
        error.to_string().contains("SuppliedByOverlayType"),
        "root-only diagnostic must name the missing type: {error}"
    );

    let ir = load_schema_set_with_overlays(
        &overlay_fixture("root-needing-overlay.xsd"),
        &[overlay_fixture("supplying-overlay.xsd")],
    )
    .expect("the explicitly supplied overlay completes the schema set");
    assert_eq!(type_names(&ir), ["RootReferrer", "SuppliedByOverlayType"]);
}

/// Sections 16/35: overlays are additive only. A qualified name redeclared by
/// an overlay fails existing duplicate validation with no precedence and no
/// shadowing, whether it collides with the root or with an earlier overlay.
#[test]
fn overlays_cannot_override_an_existing_declaration() {
    let root_collision = load_schema_set_with_overlays(
        &overlay_fixture("public.xsd"),
        &[overlay_fixture("duplicate-of-public.xsd")],
    )
    .expect_err("an overlay must not override a root declaration")
    .to_string();
    assert!(
        root_collision.contains("Container"),
        "duplicate diagnostic must name the type: {root_collision}"
    );

    let overlay_collision = load_schema_set_with_overlays(
        &overlay_fixture("public.xsd"),
        &[
            overlay_fixture("private-a.xsd"),
            overlay_fixture("duplicate-private-a.xsd"),
        ],
    )
    .expect_err("a later overlay must not override an earlier overlay")
    .to_string();
    assert!(
        overlay_collision.contains("PrivateA"),
        "duplicate diagnostic must name the type: {overlay_collision}"
    );
}

/// Section 34: a top-level overlay in a different target namespace is rejected
/// by the frontend with an explicit overlay diagnostic. It must not be blamed
/// on an xs:include the caller never wrote, and must not reach a backend.
#[test]
fn different_namespace_overlay_is_rejected_with_an_overlay_diagnostic() {
    let error = load_schema_set_with_overlays(
        &overlay_fixture("public.xsd"),
        &[overlay_fixture("other-namespace.xsd")],
    )
    .expect_err("a cross-namespace overlay root must be rejected")
    .to_string();
    assert!(
        error.contains("schema overlay target namespace mismatch"),
        "diagnostic must identify the overlay: {error}"
    );
    assert!(
        error.contains("other-namespace.xsd"),
        "diagnostic must identify the file: {error}"
    );
    assert!(
        error.contains(OVERLAY_NS) && error.contains("urn:other"),
        "diagnostic must show expected and actual namespaces: {error}"
    );
    assert!(
        !error.contains("xs:include"),
        "must not misattribute the overlay to an xs:include: {error}"
    );
}

/// Section 17: a missing overlay path fails loudly with the overlay path
/// visible, and is never silently skipped.
#[test]
fn missing_overlay_path_fails_with_the_path_visible() {
    let error = load_schema_set_with_overlays(
        &overlay_fixture("public.xsd"),
        &[overlay_fixture("absent-overlay.xsd")],
    )
    .expect_err("a missing overlay must fail the load")
    .to_string();
    assert!(
        error.contains("absent-overlay.xsd"),
        "diagnostic must name the missing overlay: {error}"
    );
}

/// Section 18: a malformed overlay fails the entire schema-set load with
/// source-path context, leaving no partial IR.
#[test]
fn malformed_overlay_fails_the_whole_schema_set() {
    let error = load_schema_set_with_overlays(
        &overlay_fixture("public.xsd"),
        &[overlay_fixture("malformed-overlay.xsd")],
    )
    .expect_err("a malformed overlay must fail the load");
    assert!(matches!(error, FrontendError::InvalidInput(_)));
    assert!(
        error.to_string().contains("malformed-overlay.xsd"),
        "diagnostic must retain source path context: {error}"
    );
}

/// Sections 5/57: `load_schema_set` keeps behaving exactly as before, because
/// it is now just the empty-overlay case of the single loading path.
#[test]
fn load_schema_set_matches_the_empty_overlay_case() {
    for name in ["public.xsd", "including-root.xsd"] {
        let root_only =
            load_schema_set(&overlay_fixture(name)).expect("root-only load should succeed");
        let empty_overlays = load_schema_set_with_overlays(&overlay_fixture(name), &[])
            .expect("empty overlay list should behave identically");
        assert_eq!(type_names(&root_only), type_names(&empty_overlays));
        assert_eq!(root_only.schema_version, empty_overlays.schema_version);
        assert_eq!(root_only.namespaces, empty_overlays.namespaces);
        assert_eq!(root_only.messages.len(), empty_overlays.messages.len());
    }
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

fn assert_named_inheritance_type(actual: &TypeRefTarget, local_name: &str) {
    assert_eq!(
        actual,
        &TypeRefTarget::Named(QualifiedName::new(INHERITANCE_NS, local_name))
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
