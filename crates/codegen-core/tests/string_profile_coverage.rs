//! Task 037 coverage boundaries for named constrained String declarations.
//!
//! The supported schema-version profile becomes baseline-renderable; every
//! other constrained String shape, the ordinary unconstrained String, and every
//! **direct** primitive String field stay exactly as they were. These are the
//! distinctions the whole task rests on, so they are asserted against
//! `CoverageAnalysis` rather than inferred.

use ams_gra_oms_codegen_core::{
    BackendLanguage, CoverageAnalysis, GenerationWorld, UCI_SCHEMA_VERSION_PATTERN,
};
use ams_gra_oms_ir::{
    ConstraintSet, LexicalConstraintSet, NamespaceDecl, PatternExpression, PatternGroup,
    PrimitiveKind, QualifiedName, SchemaIr, SourceRef, TypeDecl, TypeKind, WhiteSpacePolicy,
};

const NS: &str = "urn:test:versioning";
const LANGUAGES: [BackendLanguage; 3] = [
    BackendLanguage::Ada,
    BackendLanguage::Rust,
    BackendLanguage::Cpp,
];

fn source() -> SourceRef {
    SourceRef {
        document: "versioning.ir".to_owned(),
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
fn schema_version() -> ConstraintSet {
    ConstraintSet {
        min_length: Some(7),
        max_length: Some(57),
        lexical: LexicalConstraintSet {
            pattern_groups: vec![PatternGroup {
                alternatives: vec![PatternExpression::xml_schema(UCI_SCHEMA_VERSION_PATTERN)],
            }],
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
fn the_supported_schema_version_profile_is_baseline_renderable() {
    // The declaration name is deliberately not a UCI local name: support must
    // arise from the primitive kind plus the effective constraints alone.
    let schema = schema(vec![primitive(
        "SchemaVersion",
        PrimitiveKind::String,
        schema_version(),
    )]);
    for language in LANGUAGES {
        assert!(
            is_baseline(&schema, language),
            "{language:?} must render the supported schema-version profile"
        );
    }
}

/// Every unsupported constrained-String profile stays fail-closed.
///
/// This is what keeps Task 037 a *profile* slice rather than a blanket
/// "constrained String" capability. Each case is a real UCI facet shape.
#[test]
fn unsupported_constrained_string_profiles_are_not_baseline() {
    let mut cases: Vec<(&str, ConstraintSet)> = Vec::new();

    // The conventional-looking four-group spelling is a DIFFERENT pattern and
    // is rejected by the authoritative type itself.
    let mut four_group = schema_version();
    four_group.lexical.pattern_groups = vec![PatternGroup {
        alternatives: vec![PatternExpression::xml_schema(
            r"[0-9]{3}\.[0-9]{3}\.[0-9]{3}\.[0-9]{3}",
        )],
    }];
    cases.push(("four-group pattern", four_group));

    // Same pattern, different bounds: the carrier would not enforce them.
    let mut narrow = schema_version();
    narrow.max_length = Some(20);
    cases.push(("same pattern, different maxLength", narrow));

    // The `length + pattern` family, which includes the UUID type. This is the
    // largest unimplemented String family and the measured next blocker.
    let mut uuid_like = ConstraintSet {
        length: Some(36),
        ..ConstraintSet::default()
    };
    uuid_like.lexical.pattern_groups = vec![PatternGroup {
        alternatives: vec![PatternExpression::xml_schema(
            r"[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{12}",
        )],
    }];
    cases.push(("length + pattern", uuid_like));

    // Multiple alternatives in one group are OR-ed; not implemented.
    let mut alternatives = schema_version();
    alternatives.lexical.pattern_groups[0]
        .alternatives
        .push(PatternExpression::xml_schema(r"[0-9]{3}"));
    cases.push(("multiple alternatives", alternatives));

    // Multiple restriction-level groups are AND-ed; not implemented.
    let mut groups = schema_version();
    groups.lexical.pattern_groups.push(PatternGroup {
        alternatives: vec![PatternExpression::xml_schema(UCI_SCHEMA_VERSION_PATTERN)],
    });
    cases.push(("multiple pattern groups", groups));

    // An explicit `whiteSpace` facet is a real narrowing for `xs:string`; this
    // is the `WhitespaceVisibleString*` family.
    let mut collapsed = schema_version();
    collapsed.lexical.white_space = Some(WhiteSpacePolicy::Collapse);
    cases.push(("explicit whiteSpace", collapsed));

    // A minLength/maxLength-only profile, with no pattern at all.
    let mut bounds_only = schema_version();
    bounds_only.lexical.pattern_groups.clear();
    cases.push(("minLength/maxLength only", bounds_only));

    for (label, constraints) in cases {
        let schema = schema(vec![primitive(
            "Candidate",
            PrimitiveKind::String,
            constraints,
        )]);
        for language in LANGUAGES {
            assert!(
                !is_baseline(&schema, language),
                "{language:?} must NOT treat the {label} profile as baseline"
            );
        }
    }
}

/// An ordinary unconstrained named String declaration is unchanged.
///
/// No backend emits a wrapper for one, so Task 037 must not opportunistically
/// promote it to baseline merely because it shares a primitive kind with the
/// supported profile.
#[test]
fn an_unconstrained_named_string_declaration_is_unchanged() {
    let schema = schema(vec![primitive(
        "Label",
        PrimitiveKind::String,
        ConstraintSet::default(),
    )]);
    for language in LANGUAGES {
        assert!(
            !is_baseline(&schema, language),
            "{language:?} must not newly treat an unconstrained String as baseline"
        );
    }
}
