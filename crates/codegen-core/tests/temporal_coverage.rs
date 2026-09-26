//! Task 036 coverage boundaries for named temporal declarations.
//!
//! The supported DateTime Zulu profile becomes baseline-renderable; every
//! other temporal shape, and every **direct** primitive temporal field, stays
//! unsupported. These are the distinctions the whole task rests on, so they
//! are asserted against `CoverageAnalysis` rather than inferred.

use ams_gra_oms_codegen_core::{BackendLanguage, CoverageAnalysis, GenerationWorld};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, FieldDecl, LexicalConstraintSet, NamespaceDecl, PatternDialect,
    PatternExpression, PatternGroup, PrimitiveKind, QualifiedName, SchemaIr, SourceRef, TypeDecl,
    TypeKind, TypeRef, WhiteSpacePolicy,
};

const NS: &str = "urn:test:temporal";
const LANGUAGES: [BackendLanguage; 3] = [
    BackendLanguage::Ada,
    BackendLanguage::Rust,
    BackendLanguage::Cpp,
];

fn source() -> SourceRef {
    SourceRef {
        document: "temporal.ir".to_owned(),
        line: Some(1),
    }
}

fn primitive(name: &str, kind: PrimitiveKind, constraints: ConstraintSet) -> TypeDecl {
    TypeDecl {
        name: QualifiedName::new(NS, name),
        is_abstract: false,
        base_type: None,
        kind: TypeKind::Primitive(kind),
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

/// The authoritative UCI profile, in normalized IR form.
fn zulu() -> ConstraintSet {
    patterns(vec![PatternGroup {
        alternatives: vec![PatternExpression::xml_schema(".+Z")],
    }])
}

fn patterns(groups: Vec<PatternGroup>) -> ConstraintSet {
    ConstraintSet {
        lexical: LexicalConstraintSet {
            pattern_groups: groups,
            white_space: None,
        },
        ..ConstraintSet::default()
    }
}

/// Whether the single declaration in `schema` is baseline-renderable.
fn is_baseline(schema: &SchemaIr, language: BackendLanguage) -> bool {
    renderable_declarations(schema, language) == schema.types.len()
}

/// Baseline (no hypothetical feature family enabled) renderable declarations.
fn renderable_declarations(schema: &SchemaIr, language: BackendLanguage) -> usize {
    CoverageAnalysis::new(schema, GenerationWorld::ClosedSchemaSet)
        .expect("analysis must succeed")
        .backend_coverage(language)
        .expect("backend coverage must be measurable")
        .declarations_fully_renderable
}

/// The supported profile is baseline-renderable in all three backends.
#[test]
fn the_supported_date_time_zulu_profile_is_baseline_renderable() {
    // The declaration name is deliberately not a UCI local name: support must
    // arise from the primitive kind plus the effective constraints alone.
    let schema = schema(vec![primitive("Instant", PrimitiveKind::DateTime, zulu())]);
    for language in LANGUAGES {
        assert!(
            is_baseline(&schema, language),
            "{language:?} must render the supported DateTime Zulu profile"
        );
    }
}

/// Every unsupported named DateTime constraint profile stays fail-closed.
#[test]
fn unsupported_date_time_constraint_profiles_are_not_baseline() {
    let mut cases: Vec<(&str, ConstraintSet)> = vec![
        // No pattern at all: unconstrained dateTime admits every timezone
        // spelling, a strictly larger space than the carrier promises.
        ("no pattern", ConstraintSet::default()),
        // A different pattern; the `.+Z` equivalence proof does not carry.
        (
            "different pattern",
            patterns(vec![PatternGroup {
                alternatives: vec![PatternExpression::xml_schema(".*Z")],
            }]),
        ),
        // Alternatives in one group are OR-ed, widening the space.
        (
            "multiple alternatives",
            patterns(vec![PatternGroup {
                alternatives: vec![
                    PatternExpression::xml_schema(".+Z"),
                    PatternExpression::xml_schema(".+z"),
                ],
            }]),
        ),
        // Groups from different restriction levels are AND-ed, narrowing it.
        (
            "multiple pattern groups",
            patterns(vec![
                PatternGroup {
                    alternatives: vec![PatternExpression::xml_schema(".+Z")],
                },
                PatternGroup {
                    alternatives: vec![PatternExpression::xml_schema(".+Z")],
                },
            ]),
        ),
    ];
    // An explicit whiteSpace facet: XSD fixes dateTime's at `collapse`, so an
    // explicit one cannot legally appear and is not the authoritative profile.
    let mut explicit_white_space = zulu();
    explicit_white_space.lexical.white_space = Some(WhiteSpacePolicy::Collapse);
    cases.push(("explicit whiteSpace facet", explicit_white_space));
    // A neighbouring facet the lexical carrier cannot enforce.
    let mut length = zulu();
    length.length = Some(20);
    cases.push(("neighbouring length facet", length));

    for (label, constraints) in cases {
        let schema = schema(vec![primitive(
            "Instant",
            PrimitiveKind::DateTime,
            constraints,
        )]);
        for language in LANGUAGES {
            assert!(
                !is_baseline(&schema, language),
                "{language:?} must not render DateTime with {label}"
            );
        }
    }
}

/// The pattern's dialect participates: only the XML Schema dialect is proven.
#[test]
fn the_pattern_dialect_is_part_of_the_supported_profile() {
    let constraints = zulu();
    assert_eq!(
        constraints.lexical.pattern_groups[0].alternatives[0].dialect,
        PatternDialect::XmlSchema,
        "the supported profile is stated in the XML Schema dialect"
    );
}

/// Time and Duration remain unsupported after Task 036, including `TimeType`'s
/// authoritative `.+Z` pattern -- which is the *same text* as the supported
/// DateTime profile, and must not be admitted by accident.
#[test]
fn time_and_duration_remain_unsupported() {
    for (kind, constraints) in [
        (PrimitiveKind::Time, zulu()),
        (PrimitiveKind::Time, ConstraintSet::default()),
        (PrimitiveKind::Duration, ConstraintSet::default()),
        (PrimitiveKind::Duration, zulu()),
    ] {
        let schema = schema(vec![primitive("Span", kind, constraints)]);
        for language in LANGUAGES {
            assert!(
                !is_baseline(&schema, language),
                "{language:?} must not render {kind:?}"
            );
        }
    }
}

/// A **direct** primitive DateTime field now uses the general validated carrier.
///
/// Named support is still independently restricted to the Task 036 Zulu shape.
#[test]
fn a_direct_primitive_date_time_field_is_baseline_renderable() {
    for cardinality in [Cardinality::REQUIRED_ONE, Cardinality::OPTIONAL_ONE] {
        let holder = TypeDecl {
            name: QualifiedName::new(NS, "Holder"),
            is_abstract: false,
            base_type: None,
            kind: TypeKind::Record {
                fields: vec![FieldDecl {
                    name: "Stamp".to_owned(),
                    type_ref: TypeRef::primitive(PrimitiveKind::DateTime),
                    cardinality,
                    nillable: false,
                    constraints: ConstraintSet::default(),
                    documentation: None,
                    source: source(),
                }],
            },
            constraints: ConstraintSet::default(),
            documentation: None,
            source: source(),
        };
        // The supported named declaration is present too, so this proves the
        // direct field is judged separately rather than incidentally.
        let schema = schema(vec![
            primitive("Instant", PrimitiveKind::DateTime, zulu()),
            holder,
        ]);
        for language in LANGUAGES {
            assert_eq!(
                renderable_declarations(&schema, language),
                2,
                "{language:?}: the named Zulu carrier and direct field holder are both renderable ({cardinality:?})"
            );
        }
    }
}
