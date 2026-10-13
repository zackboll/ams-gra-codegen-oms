use ams_gra_oms_schema_docs::generate;
use ams_gra_oms_xsd_frontend::load_schema_set_with_overlays;
use serde_json::Value;
use std::path::Path;

#[test]
fn graph_matches_independent_ir_values_and_exact_identities() {
    let schema = load_schema_set_with_overlays(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/explorer.xsd"),
        &[],
    )
    .unwrap();
    let files = generate(&schema).unwrap();
    assert_eq!(files, generate(&schema).unwrap());
    let data = &files
        .iter()
        .find(|f| f.relative_path.to_str() == Some("assets/explorer-data.js"))
        .unwrap()
        .contents;
    let graph: Value = serde_json::from_str(
        data.strip_prefix("window.schemaExplorerData = ")
            .unwrap()
            .strip_suffix(";\n")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(graph["version"], 1);
    assert_eq!(graph["types"].as_array().unwrap().len(), schema.types.len());
    let root = graph["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["qname"][1] == "Root")
        .unwrap();
    assert_eq!(
        root["segments"][0]["owner"],
        serde_json::json!(["urn:explorer", "Base"])
    );
    assert_eq!(
        root["segments"][1]["owner"],
        serde_json::json!(["urn:explorer", "Root"])
    );
    assert_eq!(
        root["members"][0]["count"],
        serde_json::json!({"min":"0","max":"1"})
    );
    assert_eq!(root["members"][0]["nillable"], true);
    assert_eq!(
        root["members"][1]["count"],
        serde_json::json!({"min":"0","max":null})
    );
    assert_eq!(
        root["members"][2]["count"],
        serde_json::json!({"min":"2","max":"8"})
    );
    assert_eq!(
        root["members"][3]["count"],
        serde_json::json!({"min":"0","max":"0"})
    );
    assert_eq!(
        root["members"][4]["count"],
        serde_json::json!({"min":"9007199254740993","max":"18446744073709551615"})
    );
    for declaration in &schema.types {
        let exported = graph["types"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| {
                t["qname"][0] == declaration.name.namespace_uri
                    && t["qname"][1] == declaration.name.local_name
            })
            .unwrap();
        assert_eq!(exported["abstract"], declaration.is_abstract);
        if let ams_gra_oms_ir::TypeKind::Record { fields }
        | ams_gra_oms_ir::TypeKind::Choice {
            alternatives: fields,
        } = &declaration.kind
        {
            for (f, m) in fields.iter().zip(exported["members"].as_array().unwrap()) {
                assert_eq!(m["name"], f.name);
                assert_eq!(m["namespace"], serde_json::json!(f.wire_namespace_uri));
                assert_eq!(m["count"]["min"], f.cardinality.min_occurs.to_string());
                assert_eq!(
                    m["count"]["max"],
                    serde_json::json!(f.cardinality.max_occurs.map(|n| n.to_string()))
                );
            }
        }
        assert!(
            files
                .iter()
                .any(|f| f.relative_path.to_str() == exported["url"].as_str())
        );
    }
    assert!(data.contains("</script>"));
    assert!(data.contains("__proto__"));
}

#[test]
fn ir_only_alias_list_unqualified_and_duplicate_inheritance_are_exported() {
    use ams_gra_oms_ir::{Cardinality, QualifiedName, TypeKind, TypeRef};
    let mut schema = load_schema_set_with_overlays(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/explorer.xsd"),
        &[],
    )
    .unwrap();
    let alias = schema
        .types
        .iter_mut()
        .find(|t| t.name.local_name == "Alias")
        .unwrap();
    alias.kind = TypeKind::Alias(TypeRef::named(QualifiedName::new("urn:explorer", "Enum")));
    alias.base_type = None;
    let list = schema
        .types
        .iter_mut()
        .find(|t| t.name.local_name == "Items")
        .unwrap();
    list.kind = TypeKind::List {
        item_type: TypeRef::named(QualifiedName::new("urn:explorer", "Alias")),
        cardinality: Cardinality {
            min_occurs: 2,
            max_occurs: Some(8),
        },
    };
    list.base_type = None;
    let root = schema
        .types
        .iter_mut()
        .find(|t| t.name.local_name == "Root")
        .unwrap();
    if let TypeKind::Record { fields } = &mut root.kind {
        fields[0].name = "Inherited".into();
        fields[0].wire_namespace_uri = None;
    }
    let mut other = schema
        .types
        .iter()
        .find(|t| t.name.local_name == "Child")
        .unwrap()
        .clone();
    other.name = QualifiedName::new("urn:other", "Child");
    schema.namespaces.push(ams_gra_oms_ir::NamespaceDecl {
        uri: "urn:other".into(),
        preferred_prefix: None,
    });
    schema.types.push(other);
    let files = generate(&schema).unwrap();
    let data = &files
        .iter()
        .find(|f| f.relative_path.to_str() == Some("assets/explorer-data.js"))
        .unwrap()
        .contents;
    let graph: Value = serde_json::from_str(
        data.strip_prefix("window.schemaExplorerData = ")
            .unwrap()
            .strip_suffix(";\n")
            .unwrap(),
    )
    .unwrap();
    let types = graph["types"].as_array().unwrap();
    assert_eq!(types.iter().filter(|t| t["qname"][1] == "Child").count(), 2);
    let root = types.iter().find(|t| t["qname"][1] == "Root").unwrap();
    assert_eq!(root["members"][0]["namespace"], Value::Null);
    assert_eq!(root["segments"].as_array().unwrap().len(), 2);
    let list = types.iter().find(|t| t["qname"][1] == "Items").unwrap();
    assert_eq!(
        list["item"]["named"],
        serde_json::json!(["urn:explorer", "Alias"])
    );
    assert_eq!(list["itemCount"], serde_json::json!({"min":"2","max":"8"}));
    let alias = types.iter().find(|t| t["qname"][1] == "Alias").unwrap();
    assert_eq!(alias["kind"], "Alias");
    assert_eq!(
        alias["target"]["named"],
        serde_json::json!(["urn:explorer", "Enum"])
    );
}
