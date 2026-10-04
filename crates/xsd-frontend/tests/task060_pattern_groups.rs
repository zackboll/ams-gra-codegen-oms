//! Task 060: prove that normalization preserves OR within / AND across levels.
use ams_gra_oms_ir::{PatternDialect, PrimitiveKind, WhiteSpacePolicy};
use ams_gra_oms_xsd_frontend::load_schema_document;

#[test]
fn union_alternatives_and_inherited_intersection_are_not_flattened() {
    let path =
        std::env::temp_dir().join(format!("task060-pattern-groups-{}.xsd", std::process::id()));
    std::fs::write(
        &path,
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:task060" targetNamespace="urn:task060">
  <xs:simpleType name="Base"><xs:restriction base="xs:string"><xs:pattern value="AA|BB"/><xs:pattern value="CC"/></xs:restriction></xs:simpleType>
  <xs:simpleType name="Derived"><xs:restriction base="t:Base"><xs:pattern value="BB"/><xs:pattern value="DD"/></xs:restriction></xs:simpleType>
</xs:schema>"#,
    )
    .unwrap();
    let result = load_schema_document(&path);
    std::fs::remove_file(&path).unwrap();
    let schema = result.unwrap();
    let base = schema
        .types
        .iter()
        .find(|d| d.name.local_name == "Base")
        .unwrap();
    let derived = schema
        .types
        .iter()
        .find(|d| d.name.local_name == "Derived")
        .unwrap();
    let groups = &derived.constraints.lexical.pattern_groups;
    assert_eq!(base.constraints.lexical.pattern_groups.len(), 1);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0], base.constraints.lexical.pattern_groups[0]);
    let expressions: Vec<Vec<_>> = groups
        .iter()
        .map(|g| {
            g.alternatives
                .iter()
                .map(|p| {
                    assert_eq!(p.dialect, PatternDialect::XmlSchema);
                    p.expression.as_str()
                })
                .collect()
        })
        .collect();
    assert_eq!(expressions, [vec!["AA|BB", "CC"], vec!["BB", "DD"]]);
    assert_eq!(derived.constraints.lexical.white_space, None);
    assert_eq!(
        derived
            .constraints
            .lexical
            .effective_white_space(PrimitiveKind::String),
        WhiteSpacePolicy::Preserve
    );

    // An explicit finite-literal oracle for this fixture only. This is not a
    // regex evaluator: these exact alternatives contain only whole literals.
    let base_union = ["AA", "BB", "CC"];
    let derived_union = ["BB", "DD"];
    for (text, accepted) in [
        ("AA", false),
        ("BB", true),
        ("CC", false),
        ("DD", false),
        ("", false),
        ("BBx", false),
    ] {
        assert_eq!(
            base_union.contains(&text) && derived_union.contains(&text),
            accepted,
            "{text}"
        );
    }
    // Flattening the two restriction groups as OR would admit all three of
    // AA, CC and DD; none belongs to the derived intersection.
    for text in ["AA", "CC", "DD"] {
        assert!(base_union.contains(&text) || derived_union.contains(&text));
        assert!(!(base_union.contains(&text) && derived_union.contains(&text)));
    }
}
