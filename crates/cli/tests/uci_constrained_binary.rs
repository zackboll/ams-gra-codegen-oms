//! Task 053: constrained named Binary inventory and model renderability on
//! the REAL pinned UCI roots.
//!
//! Runs only when the caller supplies a pinned root (a wrong root is a
//! failure, never a skip):
//!
//! * `AMS_GRA_UCI_2_5_ROOT`: open-arsenal/uci/standard v2.5 @ 093610b7...,
//!   root SHA-256 `ac943049...` (`scripts/fetch-pinned-uci-2.5.sh`);
//! * `AMS_GRA_UCI_2_6_ROOT`: v2.6 @ 78eb61b6..., root SHA-256 `af54ce72...`
//!   (`scripts/fetch-pinned-uci-2.6.sh`).
//!
//! For every NAMED Binary declaration it records the effective facets, the
//! restriction ancestry, the Task 052 lexical encoding and the shared Task 053
//! `binary_length_domain` classification, then proves each supported
//! declaration actually RENDERS in Ada, Rust and C++ (as a single-declaration
//! schema, so an unrelated blocker elsewhere in UCI cannot mask the result).
//! Exact expectations are asserted, so a normalization change is noticed.

use ams_gra_oms_codegen_core::{
    Backend, BinaryLengthDomain, GenerationWorld, binary_length_domain,
};
use ams_gra_oms_ir::{
    BinaryLexicalEncoding, PrimitiveKind, SchemaIr, TypeDecl, TypeKind, TypeRefTarget,
    declaration_binary_encoding,
};
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::path::PathBuf;
use std::process::Command;

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

/// `Name <- Base <- ... <- xs:hexBinary`, for the report only.
fn ancestry(schema: &SchemaIr, declaration: &TypeDecl) -> String {
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
            TypeRefTarget::Primitive(_) => links.push(match reference.binary_encoding {
                Some(BinaryLexicalEncoding::HexBinary) => "xs:hexBinary".to_owned(),
                other => format!("{other:?}"),
            }),
        }
    }
    links.join(" <- ")
}

fn classify(domain: Option<BinaryLengthDomain>) -> String {
    match domain {
        None => "unconstrained".to_owned(),
        Some(domain) => match (domain.exact(), domain.max_octets) {
            (Some(exact), _) => format!("exact length {exact}"),
            (None, Some(max)) if domain.min_octets > 0 => {
                format!("bounded {}..{max}", domain.min_octets)
            }
            (None, Some(max)) => format!("one-sided max {max}"),
            (None, None) => format!("one-sided min {}", domain.min_octets),
        },
    }
}

/// A schema holding exactly one declaration, for per-declaration rendering.
fn alone(schema: &SchemaIr, declaration: &TypeDecl) -> SchemaIr {
    let mut single = declaration.clone();
    // The base chain is already folded into the EFFECTIVE constraints; keep
    // only the primitive provenance so the single-declaration schema is
    // self-contained.
    single.base_type = Some(ams_gra_oms_ir::TypeRef::binary(
        declaration_binary_encoding(schema, declaration).expect("hexBinary"),
    ));
    SchemaIr {
        schema_version: None,
        namespaces: schema.namespaces.clone(),
        types: vec![single],
        messages: Vec::new(),
    }
}

fn inventory(label: &str, schema: &SchemaIr) -> Vec<String> {
    let mut rows = Vec::new();
    for declaration in &schema.types {
        let TypeKind::Primitive(kind @ PrimitiveKind::Binary) = declaration.kind else {
            continue;
        };
        let c = &declaration.constraints;
        let encoding = declaration_binary_encoding(schema, declaration);
        assert_eq!(encoding, Some(BinaryLexicalEncoding::HexBinary));
        // No Binary facet outside the length domain exists in pinned UCI.
        assert!(
            c.min_inclusive.is_none()
                && c.max_inclusive.is_none()
                && c.min_exclusive.is_none()
                && c.max_exclusive.is_none()
                && c.lexical == Default::default(),
            "{label} {}: unexpected non-length facet",
            declaration.name.local_name
        );
        let domain = binary_length_domain(kind, c)
            .unwrap_or_else(|error| panic!("{}: {error}", declaration.name.local_name));
        // Model renderability in all three backends.
        let single = alone(schema, declaration);
        let world = GenerationWorld::ClosedSchemaSet;
        for (backend, result) in [
            (
                "ada",
                ams_gra_oms_backend_ada::AdaBackend.generate(&single, world),
            ),
            (
                "rust",
                ams_gra_oms_backend_rust::RustBackend.generate(&single, world),
            ),
            (
                "cpp",
                ams_gra_oms_backend_cpp::CppBackend.generate(&single, world),
            ),
        ] {
            result.unwrap_or_else(|error| {
                panic!(
                    "{label} {} must render in {backend}: {}",
                    declaration.name.local_name, error.message
                )
            });
        }
        let row = format!(
            "{} | {}:{} | {} | lexical={:?} | length={:?} minLength={:?} maxLength={:?} | other=none | {} | model READY ada/rust/cpp",
            declaration.name.local_name,
            declaration
                .source
                .document
                .rsplit('/')
                .next()
                .unwrap_or_default(),
            declaration.source.line.unwrap_or_default(),
            ancestry(schema, declaration),
            encoding.expect("hexBinary"),
            c.length,
            c.min_length,
            c.max_length,
            classify(domain),
        );
        println!("{label} NAMED BINARY: {row}");
        rows.push(row);
    }
    rows
}

fn names(rows: &[String]) -> Vec<&str> {
    rows.iter()
        .map(|row| row.split(" | ").next().expect("name"))
        .collect()
}

fn with(rows: &[String], needle: &str) -> Vec<String> {
    rows.iter()
        .filter(|row| row.contains(needle))
        .cloned()
        .collect()
}

#[test]
fn task053_real_uci_2_5_constrained_binary_inventory() {
    let Some(schema) = pinned_root(
        "AMS_GRA_UCI_2_5_ROOT",
        "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27",
    ) else {
        return;
    };
    let rows = inventory("UCI 2.5", &schema);
    assert_eq!(
        names(&rows),
        ["AA_CodeType", "BDS_AddressType", "SHA_2_256_HashType"]
    );
    for (name, exact) in [
        ("AA_CodeType", 6),
        ("BDS_AddressType", 2),
        ("SHA_2_256_HashType", 32),
    ] {
        let row = with(&rows, &format!("{name} |"));
        assert!(row[0].contains(&format!("exact length {exact}")), "{row:?}");
        assert!(
            row[0].contains(&format!("{name} <- xs:hexBinary |")),
            "{row:?}"
        );
    }
    println!("UCI 2.5 CONSTRAINED BINARY INVENTORY: PASSED");
}

#[test]
fn task053_real_uci_2_6_constrained_binary_inventory() {
    let Some(schema) = pinned_root(
        "AMS_GRA_UCI_2_6_ROOT",
        "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b",
    ) else {
        return;
    };
    let rows = inventory("UCI 2.6", &schema);
    assert_eq!(
        names(&rows),
        [
            "AA_CodeType",
            "BDS_AddressType",
            "HexBinaryType",
            "IFF_RegisterType",
            "SHA_2_256_HashType"
        ]
    );
    for (name, class, chain) in [
        (
            "AA_CodeType",
            "exact length 6",
            "AA_CodeType <- xs:hexBinary",
        ),
        (
            "BDS_AddressType",
            "exact length 2",
            "BDS_AddressType <- xs:hexBinary",
        ),
        (
            "HexBinaryType",
            "unconstrained",
            "HexBinaryType <- xs:hexBinary",
        ),
        (
            "IFF_RegisterType",
            "exact length 7",
            "IFF_RegisterType <- HexBinaryType <- xs:hexBinary",
        ),
        (
            "SHA_2_256_HashType",
            "exact length 32",
            "SHA_2_256_HashType <- xs:hexBinary",
        ),
    ] {
        let row = with(&rows, &format!("{name} |"));
        assert!(row[0].contains(&format!(" | {class} | ")), "{row:?}");
        assert!(row[0].contains(&format!(" | {chain} | ")), "{row:?}");
    }
    println!("UCI 2.6 CONSTRAINED BINARY INVENTORY: PASSED");
}

/// Every message whose payload closure reaches a constrained named Binary,
/// classified through the REAL selected-service readiness (projection +
/// coverage + preflight) for every backend under `closed-schema`:
///
/// * A -- READY: the constrained Binary was its last model blocker;
/// * B -- NOT READY with a declaration blocker other than the Binary;
/// * C -- NOT READY for a payload-reference / topology / preflight reason.
fn message_impact(label: &str, schema: &SchemaIr) -> Vec<(String, String)> {
    use ams_gra_oms_codegen_core::{
        BackendLanguage, CoverageAnalysis, ServiceMessageBlocker, analyze_service_readiness,
        resolve_service_plan,
    };
    let world = GenerationWorld::ClosedSchemaSet;
    let constrained: Vec<_> = schema
        .types
        .iter()
        .filter(|declaration| {
            matches!(declaration.kind, TypeKind::Primitive(PrimitiveKind::Binary))
                && binary_length_domain(PrimitiveKind::Binary, &declaration.constraints)
                    .is_ok_and(|domain| domain.is_some())
        })
        .map(|declaration| declaration.name.clone())
        .collect();
    let analysis = CoverageAnalysis::new(schema, world).expect("coverage");
    let mut reaching = Vec::new();
    for message in &schema.messages {
        let TypeRefTarget::Named(payload) = &message.payload_type.target else {
            continue;
        };
        let closure = analysis.dependency_closure(payload).expect("closure");
        let hits: Vec<_> = closure
            .iter()
            .filter(|declaration| constrained.contains(&declaration.name))
            .map(|declaration| declaration.name.local_name.clone())
            .collect();
        if !hits.is_empty() {
            reaching.push((
                message.name.local_name.clone(),
                hits.join(","),
                closure.len(),
            ));
        }
    }
    // One contract PER message: a projection/topology failure of one
    // message must not hide the verdict of another.
    let mut classes = Vec::new();
    for (message, hits, size) in &reaching {
        let contract = format!(
            "contract_version: \"0.1\"\nservice:\n  name: task053-impact\n  version: \"0.1.0\"\n  kind: service\nstandards:\n  oms_version: \"2.5\"\n  uci_schema_version: \"2.5\"\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: output\n        mandate: mandatory\n        message: {message}\n        topic: t\n        timing:\n          kind: asynchronous\n"
        );
        let contract = ams_gra_oms_service_contract::parse_yaml(&contract).expect("contract");
        let plan = resolve_service_plan(&contract, schema).expect("plan");
        let mut per_backend = Vec::new();
        for language in BackendLanguage::ALL {
            per_backend.push(
                match analyze_service_readiness(&plan, schema, language, world) {
                    Err(error) => {
                        let text = format!("{error:?}");
                        let reason = if text.contains("cyclic generated value dependencies") {
                            "cyclic value dependencies".to_owned()
                        } else {
                            text.chars().take(80).collect()
                        };
                        format!("{language:?}=C({reason})")
                    }
                    Ok(readiness) if readiness.is_ready() => format!("{language:?}=A"),
                    Ok(readiness) => match readiness.blocked_messages.first().map(|b| &b.blocker) {
                        Some(ServiceMessageBlocker::Declaration(name)) => {
                            assert!(
                                !constrained.contains(name),
                                "{message}: a constrained Binary is still a blocker"
                            );
                            format!("{language:?}=B({})", name.local_name)
                        }
                        Some(other) => format!("{language:?}=C({other})"),
                        None => format!("{language:?}=C(preflight/API)"),
                    },
                },
            );
        }
        let line = format!(
            "{message} [closure {size}; via {hits}] {}",
            per_backend.join(" ")
        );
        println!("{label} MESSAGE IMPACT: {line}");
        classes.push((message.clone(), per_backend.join(" ")));
    }
    classes
}

#[test]
fn task053_real_uci_constrained_binary_message_impact() {
    for (label, variable, sha256) in [
        (
            "UCI 2.5",
            "AMS_GRA_UCI_2_5_ROOT",
            "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27",
        ),
        (
            "UCI 2.6",
            "AMS_GRA_UCI_2_6_ROOT",
            "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b",
        ),
    ] {
        let Some(schema) = pinned_root(variable, sha256) else {
            continue;
        };
        let classes = message_impact(label, &schema);
        let ready = classes
            .iter()
            .filter(|(_, verdicts)| verdicts.contains("=A"))
            .count();
        println!(
            "{label} MESSAGE IMPACT SUMMARY: reaching={} category-A-in-any-backend={ready}",
            classes.len()
        );
        // Recorded Task 053 outcome (identical in both releases): no message
        // becomes READY; four move to a non-Binary model blocker (B) and
        // `Response` fails projection on a cyclic value dependency (C).
        //
        // Task 054: Ada's first blockers here used to be the reserved-member
        // owners `AltitudeRangePairType` (`Range`) and `DateTimeRangeType`
        // (`Begin`/`End`). Those members are now escaped, so Ada reaches the
        // same next (non-Binary, non-naming) blocker as Rust and C++. The
        // A/B/C classes are unchanged.
        //
        // Task 058: `AircraftIdentifierType` and
        // `AlphanumericStringLength4Type` are bounded-ASCII String profiles
        // now. `IFF_Activity` / `IFF_Command` became READY in every backend
        // (they are Task 058 category-A), and `ProductMetadata` advanced to
        // its next non-Binary blocker, `FileNameType`.
        let b = |blocker: &str| format!("Ada=B({blocker}) Rust=B({blocker}) Cpp=B({blocker})");
        let a = "Ada=A Rust=A Cpp=A".to_owned();
        assert_eq!(
            classes,
            [
                ("FileMetadata".to_owned(), b("FileNameType")),
                ("IFF_Activity".to_owned(), a.clone()),
                ("IFF_Command".to_owned(), a),
                ("ProductMetadata".to_owned(), b("FileNameType")),
                (
                    "Response".to_owned(),
                    "Ada=C(cyclic value dependencies) Rust=C(cyclic value dependencies) \
                     Cpp=C(cyclic value dependencies)"
                        .to_owned()
                ),
            ],
            "{label}"
        );
        println!("{label} CONSTRAINED BINARY MESSAGE IMPACT: PASSED");
    }
}
