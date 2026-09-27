//! Task 052: `xs:hexBinary` lexical provenance at the frontend/IR boundary.
//!
//! The semantic value kind stays `PrimitiveKind::Binary` (octets). The XSD
//! primitive is kept as separate provenance on the DIRECT primitive reference
//! (a local element @type, or a named restriction's @base), and a named
//! restriction of a named restriction reaches it through `base_type`
//! ancestry via the shared IR query.

use ams_gra_oms_ir::{
    BinaryLexicalEncoding, FieldDecl, PrimitiveKind, QualifiedName, SchemaIr, TypeDecl, TypeKind,
    TypeRef, TypeRefTarget, declaration_binary_encoding, resolve_binary_encoding,
};
use ams_gra_oms_xsd_frontend::{FrontendError, load_schema_document};
use std::path::{Path, PathBuf};

const NS: &str = "urn:binary";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/binary-provenance")
        .join(name)
}

fn schema() -> SchemaIr {
    load_schema_document(&fixture("binary-provenance.xsd")).expect("fixture loads")
}

fn declaration<'a>(schema: &'a SchemaIr, name: &str) -> &'a TypeDecl {
    schema
        .types
        .iter()
        .find(|declaration| declaration.name == QualifiedName::new(NS, name))
        .unwrap_or_else(|| panic!("{name} is declared"))
}

fn members<'a>(schema: &'a SchemaIr, name: &str) -> &'a [FieldDecl] {
    match &declaration(schema, name).kind {
        TypeKind::Record { fields } => fields,
        TypeKind::Choice { alternatives } => alternatives,
        other => panic!("{name} is structural, got {other:?}"),
    }
}

/// A direct `xs:hexBinary` local element: semantic Binary + HexBinary
/// provenance on the primitive reference itself.
#[test]
fn task052_direct_hex_binary_field_carries_provenance() {
    let schema = schema();
    let direct = &members(&schema, "Holder")[0];
    assert_eq!(direct.name, "Direct");
    assert_eq!(
        direct.type_ref,
        TypeRef::binary(BinaryLexicalEncoding::HexBinary)
    );
    assert_eq!(
        direct.type_ref.target,
        TypeRefTarget::Primitive(PrimitiveKind::Binary)
    );
    assert_eq!(
        resolve_binary_encoding(&schema, &direct.type_ref),
        Some(BinaryLexicalEncoding::HexBinary)
    );
}

/// `Hex0 -> xs:hexBinary`, `Hex1 -> Hex0`, `Hex2 -> Hex1`: every link
/// resolves to HexBinary, constraint intersection does not erase it, and the
/// field that references `Hex2` carries no copied provenance.
#[test]
fn task052_named_restriction_chain_inherits_provenance() {
    let schema = schema();
    let hex0 = declaration(&schema, "Hex0");
    assert_eq!(hex0.kind, TypeKind::Primitive(PrimitiveKind::Binary));
    assert_eq!(
        hex0.base_type,
        Some(TypeRef::binary(BinaryLexicalEncoding::HexBinary))
    );
    for name in ["Hex0", "Hex1", "Hex2"] {
        assert_eq!(
            declaration_binary_encoding(&schema, declaration(&schema, name)),
            Some(BinaryLexicalEncoding::HexBinary),
            "{name}"
        );
    }
    let hex2 = declaration(&schema, "Hex2");
    assert_eq!(hex2.constraints.length, Some(4));
    assert_eq!(hex2.constraints.max_length, Some(8));
    assert_eq!(
        hex2.base_type,
        Some(TypeRef::named(QualifiedName::new(NS, "Hex1"))),
        "a named base is a plain named reference"
    );

    let named = &members(&schema, "Holder")[1];
    assert_eq!(
        named.type_ref,
        TypeRef::named(QualifiedName::new(NS, "Hex2"))
    );
    assert_eq!(named.type_ref.binary_encoding, None);
    assert_eq!(
        resolve_binary_encoding(&schema, &named.type_ref),
        Some(BinaryLexicalEncoding::HexBinary)
    );
}

/// The UCI 2.5 `AtomicValueType.HexBinaryValue` shape: a required Choice
/// alternative that is a direct `xs:hexBinary`.
#[test]
fn task052_choice_hex_binary_alternative_carries_provenance() {
    let schema = schema();
    let alternatives = members(&schema, "AtomicLike");
    assert_eq!(alternatives[1].name, "HexBinaryValue");
    assert_eq!(
        alternatives[1].type_ref,
        TypeRef::binary(BinaryLexicalEncoding::HexBinary)
    );
    assert_eq!(
        alternatives[0].type_ref.binary_encoding, None,
        "IntValue has no Binary provenance"
    );
}

/// `xs:base64Binary` stays an explicit unsupported frontend construct: it is
/// never silently mapped to Binary, with or without provenance.
#[test]
fn task052_base64_binary_remains_an_explicit_unsupported_construct() {
    for (label, body) in [
        (
            "field",
            r#"<xs:complexType name="P"><xs:sequence><xs:element name="B" type="xs:base64Binary"/></xs:sequence></xs:complexType>"#,
        ),
        (
            "restriction",
            r#"<xs:simpleType name="B"><xs:restriction base="xs:base64Binary"/></xs:simpleType>"#,
        ),
    ] {
        let path = std::env::temp_dir().join(format!(
            "ams-gra-oms-task052-base64-{label}-{}.xsd",
            std::process::id()
        ));
        std::fs::write(
            &path,
            format!(
                r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:b" elementFormDefault="qualified">{body}</xs:schema>"#
            ),
        )
        .expect("write");
        let error = load_schema_document(&path).expect_err(label);
        std::fs::remove_file(&path).expect("remove");
        assert!(
            matches!(&error, FrontendError::UnsupportedConstruct(message) if message.starts_with("xs:base64Binary at ")),
            "{label}: {error}"
        );
    }
}
