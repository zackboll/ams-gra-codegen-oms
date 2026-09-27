//! Task 053: coverage READY == service-check READY == actual generation, for
//! every named Binary constraint shape, in all three backends.
//!
//! Each case is hand-built IR (so shapes the XSD frontend rejects earlier,
//! such as a Binary `pattern`, still reach the shared classifier), with a
//! payload record that references the Binary either by NAME or as a DIRECT
//! primitive with field-local facets. No READY verdict may reach a renderer
//! that then rejects the declaration, and vice versa.
//!
//! Lexical provenance is deliberately UNKNOWN in the named cases: model
//! capability does not depend on `xs:hexBinary` (only the codec does).

use ams_gra_oms_codegen_core::{
    Backend, BackendLanguage, CoverageAnalysis, GenerationWorld, analyze_service_readiness,
    resolve_service_plan,
};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, FieldDecl, LexicalConstraintSet, MessageDecl, NamespaceDecl,
    NumericValue, PatternExpression, PatternGroup, PrimitiveKind, QualifiedName, SchemaIr,
    SourceRef, TypeDecl, TypeKind, TypeRef,
};
use ams_gra_oms_service_contract::parse_yaml;

const NS: &str = "urn:test:blob";
const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;

fn source() -> SourceRef {
    SourceRef {
        document: "blob.ir".to_owned(),
        line: Some(1),
    }
}

fn lengths(length: Option<u64>, min: Option<u64>, max: Option<u64>) -> ConstraintSet {
    ConstraintSet {
        length,
        min_length: min,
        max_length: max,
        ..ConstraintSet::default()
    }
}

fn field(type_ref: TypeRef, constraints: ConstraintSet) -> FieldDecl {
    FieldDecl {
        name: "Data".to_owned(),
        wire_namespace_uri: Some(NS.to_owned()),
        type_ref,
        cardinality: Cardinality::REQUIRED_ONE,
        nillable: false,
        constraints,
        documentation: None,
        source: source(),
    }
}

fn declaration(name: &str, kind: TypeKind, constraints: ConstraintSet) -> TypeDecl {
    TypeDecl {
        name: QualifiedName::new(NS, name),
        is_abstract: false,
        base_type: None,
        kind,
        constraints,
        documentation: None,
        source: source(),
    }
}

/// `Payload { Data : Blob }` with `Blob` a named Binary carrying
/// `constraints`, or `Payload { Data : Binary + field constraints }`.
fn schema(named: Option<ConstraintSet>, direct: ConstraintSet) -> SchemaIr {
    let mut types = Vec::new();
    let data = match named {
        Some(constraints) => {
            types.push(declaration(
                "Blob",
                TypeKind::Primitive(PrimitiveKind::Binary),
                constraints,
            ));
            field(TypeRef::named(QualifiedName::new(NS, "Blob")), direct)
        }
        None => field(TypeRef::primitive(PrimitiveKind::Binary), direct),
    };
    types.push(declaration(
        "Payload",
        TypeKind::Record { fields: vec![data] },
        ConstraintSet::default(),
    ));
    SchemaIr {
        schema_version: None,
        namespaces: vec![NamespaceDecl {
            uri: NS.to_owned(),
            preferred_prefix: None,
        }],
        types,
        messages: vec![MessageDecl {
            name: QualifiedName::new(NS, "BlobMessage"),
            payload_type: TypeRef::named(QualifiedName::new(NS, "Payload")),
            documentation: None,
            source: source(),
        }],
    }
}

const CONTRACT: &str = r#"contract_version: "0.1"
service:
  name: blob
  version: "0.1.0"
  kind: service
standards:
  oms_version: "2.5"
  uci_schema_version: "2.5"
functions:
  - id: blob
    name: Blob
    category: specific
    applicability: applicable
    exchanges:
      - id: output-blob
        kind: oms_message
        direction: output
        mandate: mandatory
        message: BlobMessage
        topic: blob-topic
        timing:
          kind: asynchronous
"#;

fn generates(schema: &SchemaIr, language: BackendLanguage) -> Result<(), String> {
    let result = match language {
        BackendLanguage::Ada => ams_gra_oms_backend_ada::AdaBackend.generate(schema, CLOSED),
        BackendLanguage::Rust => ams_gra_oms_backend_rust::RustBackend.generate(schema, CLOSED),
        BackendLanguage::Cpp => ams_gra_oms_backend_cpp::CppBackend.generate(schema, CLOSED),
    };
    result.map(|_| ()).map_err(|error| error.message)
}

fn verdicts(schema: &SchemaIr, language: BackendLanguage) -> (bool, bool, Result<(), String>) {
    schema.validate().expect("valid IR");
    let coverage = CoverageAnalysis::new(schema, CLOSED)
        .expect("coverage")
        .backend_coverage(language)
        .expect("backend coverage");
    let covered = coverage.declarations_fully_renderable == coverage.declarations_total
        && coverage.message_closures_renderable == 1;
    let contract = parse_yaml(CONTRACT).expect("contract");
    let plan = resolve_service_plan(&contract, schema).expect("plan");
    let ready = analyze_service_readiness(&plan, schema, language, CLOSED)
        .expect("readiness")
        .is_ready();
    (covered, ready, generates(schema, language))
}

fn pattern() -> ConstraintSet {
    ConstraintSet {
        length: Some(4),
        lexical: LexicalConstraintSet {
            pattern_groups: vec![PatternGroup {
                alternatives: vec![PatternExpression::xml_schema("[0-9A-F]*")],
            }],
            white_space: None,
        },
        ..ConstraintSet::default()
    }
}

#[test]
fn task053_named_binary_renderability_parity() {
    let numeric = ConstraintSet {
        min_inclusive: Some(NumericValue::Integer(0)),
        ..ConstraintSet::default()
    };
    let cases: [(&str, Option<ConstraintSet>, ConstraintSet, Option<&str>); 11] = [
        (
            "unconstrained named",
            Some(ConstraintSet::default()),
            ConstraintSet::default(),
            None,
        ),
        (
            "exact named",
            Some(lengths(Some(4), None, None)),
            ConstraintSet::default(),
            None,
        ),
        (
            "exact zero named",
            Some(lengths(Some(0), None, None)),
            ConstraintSet::default(),
            None,
        ),
        (
            "min-only named",
            Some(lengths(None, Some(2), None)),
            ConstraintSet::default(),
            None,
        ),
        (
            "max-only named",
            Some(lengths(None, None, Some(6))),
            ConstraintSet::default(),
            None,
        ),
        (
            "min+max named",
            Some(lengths(None, Some(2), Some(6))),
            ConstraintSet::default(),
            None,
        ),
        (
            "pattern named",
            Some(pattern()),
            ConstraintSet::default(),
            Some("Binary pattern constraints on Blob"),
        ),
        (
            "numeric named",
            Some(numeric),
            ConstraintSet::default(),
            Some("numeric Binary constraints on Blob"),
        ),
        // Direct field-local facets stay fail-closed (no per-field carrier),
        // attributed to the FIELD-local facet, not to Binary itself.
        (
            "direct length",
            None,
            lengths(Some(4), None, None),
            Some("field constraints on Data"),
        ),
        (
            "direct minLength",
            None,
            lengths(None, Some(2), None),
            Some("field constraints on Data"),
        ),
        (
            "direct maxLength",
            None,
            lengths(None, None, Some(6)),
            Some("field constraints on Data"),
        ),
    ];
    for (label, named, direct, expected_error) in cases {
        let schema = schema(named, direct);
        for language in BackendLanguage::ALL {
            let (covered, ready, generated) = verdicts(&schema, language);
            let supported = expected_error.is_none();
            assert_eq!(covered, supported, "{label} {language:?}: coverage");
            assert_eq!(ready, supported, "{label} {language:?}: service-check");
            match (expected_error, &generated) {
                (None, Ok(())) => {}
                (Some(reason), Err(message)) => {
                    assert!(message.contains(reason), "{label} {language:?}: {message}")
                }
                _ => panic!("{label} {language:?}: generation {generated:?} disagrees"),
            }
        }
    }
    println!("CONSTRAINED BINARY RENDERABILITY PARITY: PASSED");
}

/// A direct Binary field's local facets are also fail-closed on OPTIONAL
/// and REPEATED occurrences; no per-field checked carrier exists.
#[test]
fn task053_direct_field_local_binary_facets_remain_unsupported_in_every_occurrence() {
    for cardinality in [
        Cardinality::OPTIONAL_ONE,
        Cardinality {
            min_occurs: 0,
            max_occurs: Some(3),
        },
        Cardinality {
            min_occurs: 0,
            max_occurs: None,
        },
    ] {
        let mut schema = schema(None, lengths(Some(4), None, None));
        let TypeKind::Record { fields } = &mut schema.types[0].kind else {
            panic!("payload record");
        };
        fields[0].cardinality = cardinality;
        for language in BackendLanguage::ALL {
            let (covered, ready, generated) = verdicts(&schema, language);
            assert!(!covered && !ready, "{cardinality:?} {language:?}");
            assert!(generated.is_err(), "{cardinality:?} {language:?}");
        }
    }
}
