//! Task 052: xs:hexBinary provenance inventory on the REAL pinned UCI roots.
//!
//! Runs only when the caller supplies a pinned root:
//!
//! * `AMS_GRA_UCI_2_5_ROOT`: open-arsenal/uci/standard v2.5 @ 093610b7...,
//!   root SHA-256 `ac943049...` (fetched by `scripts/fetch-pinned-uci-2.5.sh`);
//! * `AMS_GRA_UCI_2_6_ROOT`: v2.6 @ 78eb61b6..., root SHA-256 `af54ce72...`.
//!
//! The SHA-256 is verified first; a wrong root is a failure, never a skip.
//! Every Binary value reference and every named Binary declaration must
//! resolve to HexBinary through the ONE shared IR query: zero unknown, zero
//! Base64Binary. Exact counts are asserted, so a normalization change that
//! moves them is noticed.

use ams_gra_oms_ir::{
    BinaryLexicalEncoding, Cardinality, ConstraintSet, PrimitiveKind, SchemaIr, TypeDecl, TypeKind,
    TypeRef, TypeRefTarget, declaration_binary_encoding, resolve_binary_encoding,
};
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::path::PathBuf;
use std::process::Command;

#[derive(Default)]
struct Inventory {
    /// `Owner.Member (min..max)` for every DIRECT `Primitive(Binary)` member.
    direct: Vec<String>,
    /// `Name <- base ... <- Some(HexBinary) [facets]` per named Binary.
    named: Vec<String>,
    /// Members that reference a named Binary declaration.
    named_references: Vec<String>,
    /// Named Binary declarations with any non-default constraint.
    constrained: Vec<String>,
}

fn pinned_root(variable: &str, sha256: &str) -> Option<SchemaIr> {
    let Some(root) = std::env::var_os(variable) else {
        eprintln!("SKIPPED: {variable} is not set");
        return None;
    };
    let root = PathBuf::from(root);
    let digest = Command::new("sha256sum")
        .arg(&root)
        .output()
        .expect("sha256sum runs");
    assert_eq!(
        String::from_utf8_lossy(&digest.stdout)
            .split_whitespace()
            .next(),
        Some(sha256),
        "{variable} {} is not the pinned root",
        root.display()
    );
    Some(load_schema_set(&root).expect("pinned UCI root loads"))
}

fn cardinality(cardinality: Cardinality) -> String {
    format!(
        "{}..{}",
        cardinality.min_occurs,
        cardinality
            .max_occurs
            .map_or_else(|| "unbounded".to_owned(), |max| max.to_string())
    )
}

/// The base chain of a named Binary, for the report only (the assertion
/// uses the shared IR query, never this walk).
fn chain(schema: &SchemaIr, declaration: &TypeDecl) -> String {
    let mut links = vec![declaration.name.local_name.clone()];
    let mut base = declaration.base_type.clone();
    while let Some(reference) = base {
        base = None;
        match &reference.target {
            TypeRefTarget::Named(name) => {
                links.push(name.local_name.clone());
                base = schema
                    .types
                    .iter()
                    .find(|candidate| &candidate.name == name)
                    .and_then(|candidate| candidate.base_type.clone());
            }
            TypeRefTarget::Primitive(_) => {
                links.push(format!("{:?}", reference.binary_encoding));
            }
        }
    }
    links.join(" <- ")
}

fn inventory(schema: &SchemaIr) -> Inventory {
    let mut inventory = Inventory::default();
    for declaration in &schema.types {
        let owner = &declaration.name.local_name;
        match &declaration.kind {
            TypeKind::Primitive(PrimitiveKind::Binary) => {
                assert_eq!(
                    declaration_binary_encoding(schema, declaration),
                    Some(BinaryLexicalEncoding::HexBinary),
                    "named Binary {owner}"
                );
                let c = &declaration.constraints;
                if *c != ConstraintSet::default() {
                    inventory.constrained.push(owner.clone());
                }
                inventory.named.push(format!(
                    "{} [length={:?} minLength={:?} maxLength={:?}]",
                    chain(schema, declaration),
                    c.length,
                    c.min_length,
                    c.max_length
                ));
            }
            TypeKind::Record { fields }
            | TypeKind::Choice {
                alternatives: fields,
            } => {
                for field in fields {
                    let resolved = resolve_binary_encoding(schema, &field.type_ref);
                    let place = format!(
                        "{owner}.{} ({})",
                        field.name,
                        cardinality(field.cardinality)
                    );
                    match &field.type_ref.target {
                        TypeRefTarget::Primitive(PrimitiveKind::Binary) => {
                            assert_eq!(resolved, Some(BinaryLexicalEncoding::HexBinary), "{place}");
                            inventory.direct.push(place);
                        }
                        TypeRefTarget::Named(_) if resolved.is_some() => {
                            assert_eq!(resolved, Some(BinaryLexicalEncoding::HexBinary), "{place}");
                            inventory.named_references.push(place);
                        }
                        _ => assert_eq!(resolved, None, "{place}"),
                    }
                }
            }
            _ => {}
        }
    }
    inventory
}

fn report(label: &str, inventory: &Inventory) {
    for line in &inventory.direct {
        println!("{label} direct HexBinary: {line}");
    }
    for line in &inventory.named {
        println!("{label} named HexBinary: {line}");
    }
    for line in &inventory.named_references {
        println!("{label} named-Binary reference: {line}");
    }
    println!(
        "{label} BINARY PROVENANCE: direct={} named={} named-references={} \
         constrained-named={} unknown=0 base64=0",
        inventory.direct.len(),
        inventory.named.len(),
        inventory.named_references.len(),
        inventory.constrained.len()
    );
}

#[test]
fn task052_real_uci_2_5_binary_provenance_inventory() {
    let Some(schema) = pinned_root(
        "AMS_GRA_UCI_2_5_ROOT",
        "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27",
    ) else {
        return;
    };
    let inventory = inventory(&schema);
    report("UCI 2.5", &inventory);
    assert_eq!(
        inventory.direct,
        [
            "AtomicValueType.HexBinaryValue (1..1)",
            "IFF_ModeSelectionType.EHS_BDS_Registers (0..unbounded)",
            "LogMDT.Data (0..1)",
            "SpectralBandType.SpectralImage (0..1)",
            "SubsystemStreamMDT.SubsystemStreamBinary (0..1)",
        ]
    );
    assert_eq!(inventory.named.len(), 3);
    assert_eq!(
        inventory.constrained,
        ["AA_CodeType", "BDS_AddressType", "SHA_2_256_HashType"]
    );
    // AtomicValueType.HexBinaryValue: a required Choice alternative, direct
    // Binary with HexBinary provenance, wire QName in the OAM namespace.
    let atomic = schema
        .types
        .iter()
        .find(|declaration| declaration.name.local_name == "AtomicValueType")
        .expect("AtomicValueType");
    let TypeKind::Choice { alternatives } = &atomic.kind else {
        panic!("AtomicValueType is a Choice");
    };
    let hex = alternatives
        .iter()
        .find(|alternative| alternative.name == "HexBinaryValue")
        .expect("HexBinaryValue");
    assert_eq!(
        hex.type_ref,
        TypeRef::binary(BinaryLexicalEncoding::HexBinary)
    );
    assert_eq!(
        hex.wire_name().namespace_uri,
        Some("https://www.vdl.afrl.af.mil/programs/oam")
    );
    assert_eq!(hex.cardinality, Cardinality::REQUIRED_ONE);
    println!("UCI 2.5 BINARY PROVENANCE INVENTORY: PASSED");
}

#[test]
fn task052_real_uci_2_6_binary_provenance_inventory() {
    let Some(schema) = pinned_root(
        "AMS_GRA_UCI_2_6_ROOT",
        "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b",
    ) else {
        return;
    };
    let inventory = inventory(&schema);
    report("UCI 2.6", &inventory);
    assert!(inventory.direct.is_empty());
    assert_eq!(inventory.named.len(), 5);
    // HexBinaryType is the only unconstrained named Binary.
    assert_eq!(
        inventory.constrained,
        [
            "AA_CodeType",
            "BDS_AddressType",
            "IFF_RegisterType",
            "SHA_2_256_HashType"
        ]
    );
    println!("UCI 2.6 BINARY PROVENANCE INVENTORY: PASSED");
}
