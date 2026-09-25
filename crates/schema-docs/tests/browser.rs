use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, EnumVariant, FieldDecl, NumericValue, PatternExpression,
    PatternGroup, QualifiedName, SourceRef, TypeDecl, TypeKind, TypeRef,
};
use ams_gra_oms_schema_docs::{escape, generate};
use ams_gra_oms_xsd_frontend::load_schema_set_with_overlays;
use std::path::Path;

fn fixture() -> ams_gra_oms_ir::SchemaIr {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/service-plan/root.xsd");
    load_schema_set_with_overlays(&root, &[root.with_file_name("private-overlay.xsd")]).unwrap()
}

fn duplicate_inherited_member_schema() -> ams_gra_oms_ir::SchemaIr {
    use ams_gra_oms_ir::{NamespaceDecl, PrimitiveKind, SchemaIr};
    let ns = "urn:docs-collision";
    let source = SourceRef {
        document: "collision.ir".into(),
        line: Some(1),
    };
    let declaration = |name: &str, base: Option<TypeRef>| TypeDecl {
        name: QualifiedName::new(ns, name),
        is_abstract: false,
        base_type: base,
        kind: TypeKind::Record {
            fields: vec![FieldDecl {
                name: "Same".into(),
                type_ref: TypeRef::primitive(PrimitiveKind::String),
                cardinality: Cardinality::REQUIRED_ONE,
                nillable: false,
                constraints: ConstraintSet::default(),
                documentation: None,
                source: source.clone(),
            }],
        },
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source.clone(),
    };
    SchemaIr {
        schema_version: None,
        namespaces: vec![NamespaceDecl {
            uri: ns.into(),
            preferred_prefix: Some("c".into()),
        }],
        types: vec![
            declaration("Base", None),
            declaration(
                "Derived",
                Some(TypeRef::named(QualifiedName::new(ns, "Base"))),
            ),
        ],
        messages: vec![],
    }
}

#[test]
fn valid_inherited_duplicate_names_are_browsable_without_weakening_codegen() {
    use ams_gra_oms_codegen_core::{StructuralProjectionError, project_structural_type};
    let schema = duplicate_inherited_member_schema();
    assert!(
        schema.validate().is_ok(),
        "duplicate inherited names are valid IR"
    );
    let name = QualifiedName::new("urn:docs-collision", "Derived");
    assert!(matches!(
        project_structural_type(&schema, &name),
        Err(StructuralProjectionError::InheritedMemberNameCollision { .. })
    ));
    let files = generate(&schema).expect("valid normalized IR must remain browsable");
    let derived = page(&files, "Derived");
    let base_header = derived.find("}Base</a> — Record fields").unwrap();
    let derived_header = derived.find("}Derived</a> — Record fields").unwrap();
    assert!(base_header < derived_header);
    assert_eq!(derived.matches("<strong>Same</strong>").count(), 3); // declared + both ancestry segments
    assert!(derived[base_header..derived_header].contains("<strong>Same</strong>"));
    assert!(derived[derived_header..].contains("<strong>Same</strong>"));
    let ancestry = derived
        .split("<h2>Structural ancestry (base to derived)</h2><ol>")
        .nth(1)
        .unwrap()
        .split("</ol>")
        .next()
        .unwrap();
    assert!(ancestry.find("}Base</a>").unwrap() < ancestry.find("}Derived</a>").unwrap());
    assert_eq!(ancestry.matches("<li>").count(), 2);
}

#[test]
fn deterministic_pages_and_navigation() {
    let schema = fixture();
    let first = generate(&schema).unwrap();
    assert_eq!(first, generate(&schema).unwrap());
    assert_eq!(first.len(), schema.types.len() + 4);
    let pages: Vec<_> = first
        .iter()
        .filter(|f| f.relative_path.starts_with("types"))
        .collect();
    assert_eq!(pages.len(), schema.types.len());
    let names: std::collections::BTreeSet<_> = pages.iter().map(|p| &p.relative_path).collect();
    assert_eq!(names.len(), pages.len());
    for page in &pages {
        for link in page.contents.split("href=\"types/").skip(1) {
            let target = link.split('"').next().unwrap();
            assert!(names.iter().any(|p| p.to_string_lossy().ends_with(target)));
        }
    }
    let index = &first[0].contents;
    assert!(
        index.contains("Private"),
        "overlay declarations must appear"
    );
    assert!(index.contains("message-000000"));
}

#[test]
fn untrusted_text_and_pattern_groups_are_escaped() {
    let mut schema = fixture();
    let target = schema
        .types
        .iter_mut()
        .find(|t| matches!(t.kind, TypeKind::Primitive(_)))
        .unwrap();
    target.documentation =
        Some("<script>alert(\"x\")</script></style><script>...</script> & '".into());
    target.constraints.lexical.pattern_groups = vec![
        PatternGroup {
            alternatives: vec![
                PatternExpression::xml_schema("<a>"),
                PatternExpression::xml_schema("&b"),
            ],
        },
        PatternGroup {
            alternatives: vec![PatternExpression::xml_schema("'c'")],
        },
    ];
    let files = generate(&schema).unwrap();
    let page = files
        .iter()
        .find(|f| f.contents.contains("&lt;script&gt;alert"))
        .unwrap();
    assert!(!page.contents.contains("<script>alert"));
    assert!(page.contents.contains("&lt;/style&gt;&lt;script&gt;"));
    assert!(page.contents.contains("Pattern group 1 — one of:"));
    assert!(page.contents.contains("<p>AND</p>"));
    assert!(page.contents.contains("&lt;a&gt;"));
    assert_eq!(escape("&<>\"'"), "&amp;&lt;&gt;&quot;&#39;");
}

#[test]
fn search_data_and_offline_runtime() {
    let schema = fixture();
    let files = generate(&schema).unwrap();
    let search = files
        .iter()
        .find(|f| f.relative_path == Path::new("assets/search-index.js"))
        .unwrap();
    let data = search
        .contents
        .strip_prefix("window.schemaSearchIndex = ")
        .unwrap()
        .trim_end_matches(";\n");
    let records: serde_json::Value = serde_json::from_str(data).unwrap();
    for t in &schema.types {
        assert!(
            records
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["terms"].as_str().unwrap().contains(&t.name.local_name))
        );
    }
    for m in &schema.messages {
        assert!(
            records
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["terms"].as_str().unwrap().contains(&m.name.local_name))
        );
    }
    let js = files
        .iter()
        .find(|f| f.relative_path == Path::new("assets/search.js"))
        .unwrap();
    for forbidden in ["fetch(", "XMLHttpRequest", "http://", "https://"] {
        assert!(!js.contents.contains(forbidden));
    }
}

#[test]
fn distinct_namespaces_cannot_collide() {
    let mut schema = fixture();
    let name = schema.types[0].name.local_name.clone();
    schema.namespaces.push(ams_gra_oms_ir::NamespaceDecl {
        uri: "urn:other".into(),
        preferred_prefix: Some("other".into()),
    });
    let mut other = schema.types[0].clone();
    other.name = QualifiedName::new("urn:other", name);
    schema.types.push(other);
    let files = generate(&schema).unwrap();
    assert_eq!(files.len(), schema.types.len() + 4);
}

#[test]
fn inherited_mixed_compositors_and_message_reverse_link() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/xsd-frontend/tests/fixtures/complex-inheritance.xsd");
    let schema = load_schema_set_with_overlays(&root, &[]).unwrap();
    let files = generate(&schema).unwrap();
    let leaf = files
        .iter()
        .find(|f| f.contents.contains("<h1>Leaf</h1>"))
        .unwrap();
    assert!(leaf.contents.contains("BaseField"));
    assert!(leaf.contents.contains("MiddleField"));
    assert!(leaf.contents.contains("LeafField"));
    assert!(leaf.contents.contains("message-000000"));
    assert!(leaf.contents.contains("<h2>Uses</h2>"));
    assert!(leaf.contents.contains("Base type:"));
    let mixed = files
        .iter()
        .find(|f| f.contents.contains("<h1>ChoiceLeaf</h1>"))
        .unwrap();
    assert!(mixed.contents.contains("Record fields"));
    assert!(mixed.contents.contains("Choice alternatives"));
    assert!(mixed.contents.contains("BaseField"));
    assert!(mixed.contents.contains("0..1") || mixed.contents.contains("1..*"));
    let base = files
        .iter()
        .find(|f| f.contents.contains("<h1>Base</h1>"))
        .unwrap();
    assert!(base.contents.contains("<h2>Referenced by</h2>"));
    assert!(base.contents.contains("ChoiceLeaf"));
}

fn page<'a>(files: &'a [ams_gra_oms_codegen_core::GeneratedFile], name: &str) -> &'a str {
    &files
        .iter()
        .find(|f| f.contents.contains(&format!("<h1>{name}</h1>")))
        .unwrap()
        .contents
}

#[test]
fn direct_references_kinds_constraints_and_search_terms() {
    let mut schema = fixture();
    let ns = schema.types[0].name.namespace_uri.clone();
    let named = |name: &str| TypeRef::named(QualifiedName::new(&ns, name));
    let source = SourceRef {
        document: "fixture.xsd".into(),
        line: Some(42),
    };
    let new = |name: &str, kind| TypeDecl {
        name: QualifiedName::new(&ns, name),
        is_abstract: false,
        base_type: None,
        kind,
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source.clone(),
    };
    let field = |name: &str, target: &str| FieldDecl {
        name: name.into(),
        type_ref: named(target),
        cardinality: Cardinality::OPTIONAL_ONE,
        nillable: true,
        constraints: ConstraintSet::default(),
        documentation: Some("Member doc".into()),
        source: source.clone(),
    };
    let mut constrained = new(
        "DocsConstrained",
        TypeKind::Primitive(ams_gra_oms_ir::PrimitiveKind::String),
    );
    constrained.constraints.length = Some(5);
    constrained.constraints.min_length = Some(2);
    constrained.constraints.max_length = Some(8);
    constrained.constraints.min_length = None;
    constrained.constraints.max_length = None;
    schema.types.push(constrained);
    let mut numeric = new(
        "DocsNumeric",
        TypeKind::Primitive(ams_gra_oms_ir::PrimitiveKind::SignedInteger),
    );
    numeric.constraints.min_inclusive = Some(NumericValue::Integer(1));
    numeric.constraints.max_inclusive = Some(NumericValue::Integer(9));
    schema.types.push(numeric);
    schema
        .types
        .push(new("DocsAlias", TypeKind::Alias(named("DocsConstrained"))));
    schema.types.push(new(
        "DocsRecord",
        TypeKind::Record {
            fields: vec![field("SearchField", "DocsAlias")],
        },
    ));
    schema.types.push(new(
        "DocsChoice",
        TypeKind::Choice {
            alternatives: vec![field("SearchAlternative", "DocsRecord")],
        },
    ));
    schema.types.push(new(
        "DocsList",
        TypeKind::List {
            item_type: named("DocsChoice"),
            cardinality: Cardinality {
                min_occurs: 3,
                max_occurs: None,
            },
        },
    ));
    schema.types.push(new(
        "DocsEnum",
        TypeKind::Enumeration {
            variants: vec![EnumVariant {
                wire_value: "WireSearchValue".into(),
                documentation: Some("Enum doc".into()),
            }],
        },
    ));
    let files = generate(&schema).unwrap();
    let alias = page(&files, "DocsAlias");
    assert!(alias.contains("Alias target</h2><p><a href=\""));
    assert!(alias.contains("DocsConstrained</a>"));
    let record = page(&files, "DocsRecord");
    assert!(record.contains("SearchField"));
    assert!(record.contains("DocsAlias</a>"));
    assert!(record.contains("0..1 — nillable: true — source: fixture.xsd:42"));
    let choice = page(&files, "DocsChoice");
    assert!(choice.contains("SearchAlternative"));
    assert!(choice.contains("DocsRecord</a>"));
    let list = page(&files, "DocsList");
    assert!(list.contains("DocsChoice</a> — cardinality: 3..*"));
    assert!(page(&files, "DocsConstrained").contains("<h2>Referenced by</h2><ul><li><a href=\""));
    assert!(page(&files, "DocsConstrained").contains("length: 5"));
    assert!(page(&files, "DocsNumeric").contains("minInclusive: 1"));
    assert!(page(&files, "DocsNumeric").contains("maxInclusive: 9"));
    assert!(page(&files, "DocsEnum").contains("WireSearchValue"));
    let index = &files
        .iter()
        .find(|f| f.relative_path.to_str() == Some("assets/search-index.js"))
        .unwrap()
        .contents;
    for term in [
        "DocsAlias",
        &ns,
        "SearchField",
        "SearchAlternative",
        "WireSearchValue",
        "PrivateReport",
    ] {
        assert!(index.contains(term), "missing search term {term}");
    }
    let landing = &files[0].contents;
    assert!(landing.contains("Namespace"));
    assert!(landing.contains(&ns));
}
