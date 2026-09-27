//! Task 051: every operation that clones or projects a `FieldDecl` preserves
//! its wire namespace unchanged. Wire identity lives on the field semantic
//! object, never in a codec-only side table.

use ams_gra_oms_codegen_core::{
    GenerationWorld, effective_choice_alternatives, effective_record_fields,
    project_abstract_value, project_service_generation_schema, resolve_service_plan,
};
use ams_gra_oms_ir::{FieldDecl, QualifiedName, SchemaIr, TypeKind};
use ams_gra_oms_service_contract::parse_yaml;
use ams_gra_oms_xsd_frontend::{load_schema_set, load_schema_set_with_overlays};
use std::path::{Path, PathBuf};

fn fixture(directory: &str, name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(directory)
        .join(name)
}

fn project(schema: &SchemaIr, stem: &str) -> SchemaIr {
    let contract = parse_yaml(
        &std::fs::read_to_string(fixture("service-generate", &format!("{stem}.yaml")))
            .expect("contract"),
    )
    .expect("parse");
    let plan = resolve_service_plan(&contract, schema).expect("plan");
    project_service_generation_schema(&plan, schema, GenerationWorld::ClosedSchemaSet)
        .expect("projection")
        .schema()
        .clone()
}

/// `(owner, name, wire namespace)` for every LOCAL member of every structural type.
fn local_members(schema: &SchemaIr) -> Vec<(QualifiedName, String, Option<String>)> {
    let mut out = Vec::new();
    for declaration in &schema.types {
        let fields: &[FieldDecl] = match &declaration.kind {
            TypeKind::Record { fields } => fields,
            TypeKind::Choice { alternatives } => alternatives,
            _ => continue,
        };
        for field in fields {
            out.push((
                declaration.name.clone(),
                field.name.clone(),
                field.wire_namespace_uri.clone(),
            ));
        }
    }
    out
}

fn wire(fields: &[&FieldDecl]) -> Vec<(String, Option<String>)> {
    fields
        .iter()
        .map(|field| (field.name.clone(), field.wire_namespace_uri.clone()))
        .collect()
}

/// Selected-service projection keeps each projected member's namespace
/// exactly as normalized, for qualified and unqualified members alike.
#[test]
fn task051_selected_projection_preserves_wire_namespaces() {
    for stem in [
        "runtime-test",
        "codec-choice",
        "codec-inherit",
        "codec-shape",
        "codec-unqualified",
        "codec-form-qualified",
        "codec-form-unqualified",
    ] {
        let full =
            load_schema_set(&fixture("service-generate", &format!("{stem}.xsd"))).expect("schema");
        let projected = project(&full, stem);
        let before = local_members(&full);
        let after = local_members(&projected);
        assert!(!after.is_empty(), "{stem}");
        for member in after {
            assert!(before.contains(&member), "{stem}: {member:?} changed");
        }
    }
}

/// Effective Record fields (inherited + added) and effective Choice
/// alternatives are the original FieldDecls, provenance intact.
#[test]
fn task051_effective_members_keep_their_own_provenance() {
    let schema = load_schema_set(&fixture("service-generate", "codec-inherit.xsd")).unwrap();
    let schema = project(&schema, "codec-inherit");
    let derived = QualifiedName::new("urn:inherit", "Derived");
    assert_eq!(
        wire(&effective_record_fields(&schema, &derived).unwrap()),
        [
            ("BaseValue".to_owned(), Some("urn:inherit".to_owned())),
            ("ExtraValue".to_owned(), Some("urn:inherit".to_owned())),
        ]
    );

    let schema = load_schema_set(&fixture("service-generate", "codec-choice.xsd")).unwrap();
    let schema = project(&schema, "codec-choice");
    let selection = QualifiedName::new("urn:choice", "Selection");
    assert_eq!(
        wire(&effective_choice_alternatives(&schema, &selection).unwrap()),
        [
            ("Alpha".to_owned(), Some("urn:choice".to_owned())),
            ("Beta".to_owned(), Some("urn:choice".to_owned())),
        ]
    );

    // Mixed provenance across inheritance (frontend matrix fixture): a type
    // in an unqualified-default document extending a qualified-default base.
    let matrix = load_schema_set(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/member-qname/matrix.xsd"),
    )
    .unwrap();
    let mixed = QualifiedName::new("urn:matrix", "DerivedFromQualified");
    assert_eq!(
        wire(&effective_record_fields(&matrix, &mixed).unwrap()),
        [
            ("Inherited".to_owned(), Some("urn:matrix".to_owned())),
            ("LocallyUnqualified".to_owned(), None),
            ("Added".to_owned(), None),
        ]
    );
}

/// Abstract-value projection: every concrete descendant keeps its members'
/// namespaces.
#[test]
fn task051_abstract_value_projection_preserves_wire_namespaces() {
    let schema = load_schema_set(&fixture("service-generate", "codec-shape.xsd")).unwrap();
    let schema = project(&schema, "codec-shape");
    let projection =
        project_abstract_value(&schema, &QualifiedName::new("urn:shape", "ShapeBase")).unwrap();
    assert_eq!(projection.concrete_descendants.len(), 2);
    for descendant in projection.concrete_descendants {
        for field in effective_record_fields(&schema, &descendant.name).unwrap() {
            assert_eq!(
                field.wire_name().namespace_uri,
                Some("urn:shape"),
                "{}.{}",
                descendant.name.local_name,
                field.name
            );
        }
    }
}

/// Private overlays keep the overlay document's own member semantics.
#[test]
fn task051_private_overlay_members_keep_wire_namespaces() {
    let root = fixture("service-plan", "root.xsd");
    let schema =
        load_schema_set_with_overlays(&root, &[root.with_file_name("private-overlay.xsd")])
            .expect("overlay set");
    let private = schema
        .types
        .iter()
        .find(|declaration| declaration.name == QualifiedName::new("urn:test", "PrivateReportType"))
        .expect("overlay type");
    let TypeKind::Record { fields } = &private.kind else {
        panic!("record");
    };
    assert_eq!(fields.len(), 2);
    for field in fields {
        assert_eq!(
            field.wire_name().namespace_uri,
            Some("urn:test"),
            "{}",
            field.name
        );
    }
}
