//! Task 042 coverage boundaries for the NATO special-words String profile.
//!
//! The authoritative shape becomes baseline-renderable in all three backends,
//! under any declaration name; every near-miss stays unsupported. Asserted
//! against `CoverageAnalysis`, because coverage and the backend validation
//! gates must agree exactly.

use ams_gra_oms_codegen_core::{BackendLanguage, CoverageAnalysis, GenerationWorld};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, FieldDecl, LexicalConstraintSet, NamespaceDecl, NumericValue,
    PatternExpression, PatternGroup, PrimitiveKind, QualifiedName, SchemaIr, SourceRef, TypeDecl,
    TypeKind, TypeRef, TypeRefTarget, WhiteSpacePolicy,
};

const NS: &str = "urn:test:markings";
const LANGUAGES: [BackendLanguage; 3] = [
    BackendLanguage::Ada,
    BackendLanguage::Rust,
    BackendLanguage::Cpp,
];

fn source() -> SourceRef {
    SourceRef {
        document: "markings.ir".to_owned(),
        line: Some(1),
    }
}

fn primitive(name: &str, constraints: ConstraintSet) -> TypeDecl {
    TypeDecl {
        name: QualifiedName::new(NS, name),
        is_abstract: false,
        base_type: None,
        kind: TypeKind::Primitive(PrimitiveKind::String),
        constraints,
        documentation: None,
        source: source(),
    }
}

fn schema(types: Vec<TypeDecl>) -> SchemaIr {
    SchemaIr {
        schema_version: None,
        namespaces: vec![NamespaceDecl {
            uri: NS.to_owned(),
            preferred_prefix: None,
        }],
        types,
        messages: Vec::new(),
    }
}

/// The authoritative profile in normalized IR form, spelled out literally so
/// this file is an independent statement of the pinned evidence.
fn nato() -> ConstraintSet {
    ConstraintSet {
        min_length: Some(6),
        max_length: Some(261),
        lexical: LexicalConstraintSet {
            pattern_groups: vec![PatternGroup {
                alternatives: vec![PatternExpression::xml_schema(r"NATO:[a-zA-Z\-_]{1,256}")],
            }],
            white_space: None,
        },
        ..ConstraintSet::default()
    }
}

fn renderable(schema: &SchemaIr, language: BackendLanguage) -> usize {
    CoverageAnalysis::new(schema, GenerationWorld::ClosedSchemaSet)
        .expect("analysis must succeed")
        .backend_coverage(language)
        .expect("backend coverage must be measurable")
        .declarations_fully_renderable
}

fn is_baseline(schema: &SchemaIr, language: BackendLanguage) -> bool {
    renderable(schema, language) == schema.types.len()
}

/// The authoritative profile is baseline in every backend, whatever its name.
#[test]
fn the_authoritative_profile_is_baseline_under_any_name() {
    for name in ["Candidate", "NATO_SpecialWordsType", "ReleaseToken"] {
        let schema = schema(vec![primitive(name, nato())]);
        for language in LANGUAGES {
            assert!(
                is_baseline(&schema, language),
                "{language:?} must treat {name} as baseline"
            );
        }
    }
}

/// A UCI-looking name with different facets stays unsupported.
#[test]
fn a_uci_name_with_wrong_facets_is_not_renderable() {
    let mut wrong = nato();
    wrong.max_length = Some(256);
    let schema = schema(vec![primitive("NATO_SpecialWordsType", wrong)]);
    for language in LANGUAGES {
        assert!(!is_baseline(&schema, language), "{language:?}");
    }
}

/// A record holding the profile is renderable transitively, and an unrelated
/// unsupported sibling does not contaminate it.
#[test]
fn a_record_holding_the_profile_is_renderable_and_isolated() {
    let holder = TypeDecl {
        kind: TypeKind::Record {
            fields: vec![FieldDecl {
                name: "Word".to_owned(),
                type_ref: TypeRef {
                    target: TypeRefTarget::Named(QualifiedName::new(NS, "Word")),
                },
                cardinality: Cardinality::REQUIRED_ONE,
                nillable: false,
                constraints: ConstraintSet::default(),
                documentation: None,
                source: source(),
            }],
        },
        ..primitive("Holder", ConstraintSet::default())
    };
    let mut neighbour = nato();
    neighbour.min_length = Some(1);
    neighbour.max_length = Some(256);
    let schema = schema(vec![
        primitive("Word", nato()),
        holder,
        primitive("Neighbour", neighbour),
    ]);
    for language in LANGUAGES {
        // Word and Holder render; only the unsupported neighbour does not.
        assert_eq!(renderable(&schema, language), 2, "{language:?}");
    }
}

/// Every near-miss stays out of coverage in every backend.
#[test]
fn nato_near_misses_are_not_baseline() {
    let with_pattern = |expression: &str| {
        let mut constraints = nato();
        constraints.lexical.pattern_groups[0].alternatives[0] =
            PatternExpression::xml_schema(expression);
        constraints
    };
    let mut cases: Vec<(&str, ConstraintSet)> = vec![
        (
            "lower-case prefix",
            with_pattern(r"nato:[a-zA-Z\-_]{1,256}"),
        ),
        ("changed prefix", with_pattern(r"OTAN:[a-zA-Z\-_]{1,256}")),
        (
            "digits in class",
            with_pattern(r"NATO:[a-zA-Z0-9\-_]{1,256}"),
        ),
        ("suffix {1,255}", with_pattern(r"NATO:[a-zA-Z\-_]{1,255}")),
    ];
    let mut no_min = nato();
    no_min.min_length = None;
    cases.push(("absent minLength", no_min));
    let mut changed = nato();
    changed.max_length = Some(262);
    cases.push(("changed maxLength", changed));
    let mut length = nato();
    length.min_length = None;
    length.max_length = None;
    length.length = Some(261);
    cases.push(("length instead of min/max", length));
    let mut preserve = nato();
    preserve.lexical.white_space = Some(WhiteSpacePolicy::Preserve);
    cases.push(("explicit whiteSpace=preserve", preserve));
    let mut collapse = nato();
    collapse.lexical.white_space = Some(WhiteSpacePolicy::Collapse);
    cases.push(("explicit whiteSpace=collapse", collapse));
    let mut groups = nato();
    groups
        .lexical
        .pattern_groups
        .push(nato().lexical.pattern_groups[0].clone());
    cases.push(("a second group", groups));
    let mut numeric = nato();
    numeric.max_inclusive = Some(NumericValue::Integer(9));
    cases.push(("a numeric facet", numeric));
    let mut huge = with_pattern(r"NATO:[a-zA-Z\-_]{1,18446744073709551610}");
    huge.max_length = Some(u64::MAX);
    cases.push(("huge agreeing bounds", huge));

    for (label, constraints) in cases {
        let schema = schema(vec![primitive("Candidate", constraints)]);
        for language in LANGUAGES {
            assert!(
                !is_baseline(&schema, language),
                "{language:?} must NOT treat {label} as baseline"
            );
        }
    }
}
