//! Task 051: local element wire-namespace provenance.
//!
//! XML Schema 1.0 Structures 2E section 3.3.2: a local element's
//! `{target namespace}` is the enclosing schema document's `targetNamespace`
//! when its `form` is `qualified` (or `form` is absent and that document's
//! `elementFormDefault` is `qualified`), otherwise *absent*. Section 3.15.2:
//! `elementFormDefault` defaults to `unqualified`.

use ams_gra_oms_ir::{FieldDecl, FieldWireName, QualifiedName, SchemaIr, TypeKind, TypeRefTarget};
use ams_gra_oms_xsd_frontend::{FrontendError, load_schema_document, load_schema_set};
use std::path::{Path, PathBuf};

const MATRIX: &str = "urn:matrix";
const IMPORTED: &str = "urn:matrix:imported";
const QUALIFIED_IMPORT: &str = "urn:matrix:qualified-import";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/member-qname")
        .join(name)
}

fn matrix() -> SchemaIr {
    load_schema_set(&fixture("matrix.xsd")).expect("matrix loads")
}

/// Local (declared-here) members of `type_name` in `namespace`.
fn members<'a>(schema: &'a SchemaIr, namespace: &str, type_name: &str) -> &'a [FieldDecl] {
    let declaration = schema
        .types
        .iter()
        .find(|declaration| declaration.name == QualifiedName::new(namespace, type_name))
        .unwrap_or_else(|| panic!("{type_name} is declared"));
    match &declaration.kind {
        TypeKind::Record { fields } => fields,
        TypeKind::Choice { alternatives } => alternatives,
        other => panic!("{type_name} is structural, got {other:?}"),
    }
}

fn wire(schema: &SchemaIr, namespace: &str, type_name: &str) -> Vec<(String, Option<String>)> {
    members(schema, namespace, type_name)
        .iter()
        .map(|field| {
            let FieldWireName {
                namespace_uri,
                local_name,
            } = field.wire_name();
            (local_name.to_owned(), namespace_uri.map(str::to_owned))
        })
        .collect()
}

fn q(namespace: &str) -> Option<String> {
    Some(namespace.to_owned())
}

/// A (no default, no form -> absent) and D (default unqualified + local
/// form="qualified" -> targetNamespace), plus a local form="unqualified" that
/// restates the default.
#[test]
fn task051_default_unqualified_document_with_local_form_overrides() {
    let schema = matrix();
    assert_eq!(
        wire(&schema, MATRIX, "DefaultRecord"),
        [
            ("Plain".to_owned(), None),
            ("ForcedQualified".to_owned(), q(MATRIX)),
            ("ForcedUnqualified".to_owned(), None),
            ("Foreign".to_owned(), q(MATRIX)),
        ]
    );
}

/// The element's own namespace and its TYPE's namespace are independent.
#[test]
fn task051_element_namespace_is_not_the_type_namespace() {
    let schema = matrix();
    let foreign = &members(&schema, MATRIX, "DefaultRecord")[3];
    assert_eq!(foreign.name, "Foreign");
    assert_eq!(foreign.wire_name().namespace_uri, Some(MATRIX));
    assert_eq!(
        foreign.type_ref.target,
        TypeRefTarget::Named(QualifiedName::new(IMPORTED, "ImportedRecord"))
    );
}

/// F. Choice alternatives follow exactly the same rule as sequence members,
/// in both an unqualified-default and a qualified-default document.
#[test]
fn task051_choice_alternatives_use_the_same_rule() {
    let schema = matrix();
    assert_eq!(
        wire(&schema, MATRIX, "DefaultChoice"),
        [("Left".to_owned(), None), ("Right".to_owned(), q(MATRIX))]
    );
    assert_eq!(
        wire(&schema, MATRIX, "QualifiedChoice"),
        [("Up".to_owned(), q(MATRIX)), ("Down".to_owned(), None)]
    );
}

/// C (schema qualified, no form -> targetNamespace), E (schema qualified +
/// form="unqualified" -> absent), and H: the INCLUDED document's own
/// elementFormDefault applies to the fields it declares, not the root's.
#[test]
fn task051_included_document_uses_its_own_default() {
    let schema = matrix();
    assert_eq!(
        wire(&schema, MATRIX, "QualifiedBase"),
        [
            ("Inherited".to_owned(), q(MATRIX)),
            ("LocallyUnqualified".to_owned(), None),
        ]
    );
}

/// G. Extension-added members use the DECLARING document's rule, in both
/// directions; inherited members keep their own provenance.
#[test]
fn task051_extension_members_use_the_declaring_document() {
    let schema = matrix();
    // Declared in matrix.xsd (default unqualified), base from included.xsd.
    assert_eq!(
        wire(&schema, MATRIX, "DerivedFromQualified"),
        [("Added".to_owned(), None)]
    );
    // Declared in included.xsd (qualified), base from matrix.xsd.
    assert_eq!(
        wire(&schema, MATRIX, "QualifiedDerived"),
        [("Extra".to_owned(), q(MATRIX))]
    );
}

/// I. Imported documents keep their own targetNamespace and default,
/// independently of the importing root and of each other.
#[test]
fn task051_imported_documents_keep_their_own_semantics() {
    let schema = matrix();
    // B. explicit elementFormDefault="unqualified".
    assert_eq!(
        wire(&schema, IMPORTED, "ImportedRecord"),
        [
            ("Loose".to_owned(), None),
            ("Tight".to_owned(), q(IMPORTED))
        ]
    );
    assert_eq!(
        wire(&schema, QUALIFIED_IMPORT, "QualifiedImport"),
        [("Member".to_owned(), q(QUALIFIED_IMPORT))]
    );
    // Every qualified member names a namespace the set declares.
    schema.validate().expect("valid IR");
}

/// The normalized IR stores namespace URIs only: no prefix spelling and no
/// Clark-notation string ever reaches a FieldDecl.
#[test]
fn task051_ir_stores_uris_not_prefixes_or_clark_strings() {
    let schema = matrix();
    for declaration in &schema.types {
        let fields = match &declaration.kind {
            TypeKind::Record { fields } => fields.as_slice(),
            TypeKind::Choice { alternatives } => alternatives.as_slice(),
            _ => continue,
        };
        for field in fields {
            assert!(!field.name.contains(['{', '}', ':']), "{}", field.name);
            if let Some(uri) = &field.wire_namespace_uri {
                assert!(
                    [MATRIX, IMPORTED, QUALIFIED_IMPORT].contains(&uri.as_str()),
                    "{uri}"
                );
            }
        }
    }
}

/// Loading is deterministic.
#[test]
fn task051_qualification_is_deterministic() {
    assert_eq!(matrix(), matrix());
}

/// Local `ref=` stays unsupported and fails explicitly; it is never read as
/// name/type.
#[test]
fn task051_local_element_ref_is_still_rejected() {
    let error = load_schema_document(&fixture("local-ref.xsd")).expect_err("ref= is unsupported");
    assert!(matches!(error, FrontendError::UnsupportedConstruct(_)));
    let message = error.to_string();
    assert!(message.contains("xs:element @ref"), "{message}");
    assert!(message.contains("local-ref.xsd"), "{message}");
}

/// Only `qualified` and `unqualified` are valid local form values.
#[test]
fn task051_invalid_local_form_is_invalid_input() {
    let error =
        load_schema_document(&fixture("invalid-local-form.xsd")).expect_err("invalid local form");
    assert!(matches!(error, FrontendError::InvalidInput(_)));
    let message = error.to_string();
    assert!(
        message.contains("xs:element @form must be qualified or unqualified, got sometimes at 6:7"),
        "{message}"
    );
}
