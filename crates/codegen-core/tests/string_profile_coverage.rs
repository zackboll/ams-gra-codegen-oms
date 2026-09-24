//! Task 037 coverage boundaries for named constrained String declarations.
//!
//! The supported schema-version profile becomes baseline-renderable; every
//! other constrained String shape, the ordinary unconstrained String, and every
//! **direct** primitive String field stay exactly as they were. These are the
//! distinctions the whole task rests on, so they are asserted against
//! `CoverageAnalysis` rather than inferred.

use ams_gra_oms_codegen_core::{
    BackendLanguage, CoverageAnalysis, GenerationWorld, UCI_SCHEMA_VERSION_PATTERN,
    UCI_UUID_LENGTH, UCI_UUID_PATTERN, visible_ascii_pattern,
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

/// A visible-ASCII family member with the given bounds, in normalized IR form.
///
/// The pattern text is derived from the bounds by the shared helper, which is
/// the property that makes membership exact: the quantifier and the facets
/// cannot disagree.
fn visible_ascii(min_length: u64, max_length: u64) -> ConstraintSet {
    ConstraintSet {
        min_length: Some(min_length),
        max_length: Some(max_length),
        lexical: LexicalConstraintSet {
            pattern_groups: vec![PatternGroup {
                alternatives: vec![PatternExpression::xml_schema(visible_ascii_pattern(
                    min_length, max_length,
                ))],
            }],
            white_space: None,
        },
        ..ConstraintSet::default()
    }
}

/// The authoritative UUID profile, in normalized IR form.
///
/// ONE pattern group holding ONE expression: the authoritative declaration has
/// a single `<xs:pattern>` element whose alternation is internal to its text.
fn uuid() -> ConstraintSet {
    ConstraintSet {
        length: Some(UCI_UUID_LENGTH),
        lexical: LexicalConstraintSet {
            pattern_groups: vec![PatternGroup {
                alternatives: vec![PatternExpression::xml_schema(UCI_UUID_PATTERN)],
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

/// The supported UUID profile is baseline-renderable in all three backends.
///
/// Coverage was not taught about UUIDs: it already asks `string_profile`, so
/// extending the shared classifier is what makes this true.
#[test]
fn the_supported_uuid_profile_is_baseline_renderable() {
    // The declaration name is deliberately not the UCI local name: support must
    // arise from the primitive kind plus the effective constraints alone.
    let schema = schema(vec![primitive("Identifier", PrimitiveKind::String, uuid())]);
    for language in LANGUAGES {
        assert!(
            is_baseline(&schema, language),
            "{language:?} must render the supported UUID profile"
        );
    }
}

/// Both String profiles coexist as baseline in one schema.
#[test]
fn both_string_profiles_are_baseline_together() {
    let schema = schema(vec![
        primitive("SchemaVersion", PrimitiveKind::String, schema_version()),
        primitive("Identifier", PrimitiveKind::String, uuid()),
    ]);
    for language in LANGUAGES {
        assert_eq!(
            renderable_declarations(&schema, language),
            2,
            "{language:?} must render both String profiles"
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

    // A `length + pattern` declaration that is NOT the Task 038 UUID profile.
    // The expression here is the general 8-4-4-4-12 hexadecimal shape, without
    // the authoritative `[1-5]` version and `[89abAB]` variant classes, so it
    // denotes a strictly larger lexical space and remains unsupported.
    let mut hex_like = ConstraintSet {
        length: Some(36),
        ..ConstraintSet::default()
    };
    hex_like.lexical.pattern_groups = vec![PatternGroup {
        alternatives: vec![PatternExpression::xml_schema(
            r"[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{12}",
        )],
    }];
    cases.push(("length + unconstrained hexadecimal pattern", hex_like));

    // The authoritative UUID pattern under a DIFFERENT length.
    let mut wrong_length = ConstraintSet {
        length: Some(37),
        ..ConstraintSet::default()
    };
    wrong_length.lexical.pattern_groups = vec![PatternGroup {
        alternatives: vec![PatternExpression::xml_schema(UCI_UUID_PATTERN)],
    }];
    cases.push(("UUID pattern, wrong length", wrong_length));

    // The authoritative UUID pattern split into two IR alternatives. The real
    // declaration has ONE pattern facet whose alternation is internal, so this
    // is a different declaration shape.
    let mut split_uuid = ConstraintSet {
        length: Some(36),
        ..ConstraintSet::default()
    };
    split_uuid.lexical.pattern_groups = vec![PatternGroup {
        alternatives: vec![
            PatternExpression::xml_schema("(0{8}(-0{4}){3}-0{12})"),
            PatternExpression::xml_schema(
                "([a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[1-5][a-fA-F0-9]{3}-[89abAB][a-fA-F0-9]{3}-[a-fA-F0-9]{12})",
            ),
        ],
    }];
    cases.push(("UUID pattern split into two alternatives", split_uuid));

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

/// Task 039: every visible-ASCII family member becomes baseline-renderable.
///
/// The classifier is the only thing that changed, which is the point: coverage
/// already asks `string_profile`, so extending the shared classifier is
/// sufficient and no coverage code mentions this profile.
#[test]
fn every_visible_ascii_family_member_is_baseline_renderable() {
    for (min_length, max_length) in [(1, 20), (1, 32), (1, 256), (1, 1024), (2, 4)] {
        let schema = schema(vec![primitive(
            "Candidate",
            PrimitiveKind::String,
            visible_ascii(min_length, max_length),
        )]);
        for language in LANGUAGES {
            assert!(
                is_baseline(&schema, language),
                "{language:?} must treat the {min_length}..{max_length} member as baseline"
            );
        }
    }
}

/// Task 039 near-misses stay NON-baseline in every backend.
#[test]
fn visible_ascii_near_misses_are_not_baseline() {
    let mut cases: Vec<(&str, ConstraintSet)> = Vec::new();

    // Same pattern, a different maxLength: a different type.
    let mut mismatched = visible_ascii(1, 256);
    mismatched.max_length = Some(128);
    cases.push(("mismatched bounds", mismatched));

    // The explicit whiteSpace of the WhitespaceVisibleString* family.
    let mut collapsed = visible_ascii(1, 256);
    collapsed.lexical.white_space = Some(WhiteSpacePolicy::Collapse);
    cases.push(("explicit whiteSpace=collapse", collapsed));

    // The fixed-`length` VisibleStringLength*/NITF_* shape.
    let mut fixed_length = ConstraintSet {
        length: Some(10),
        ..ConstraintSet::default()
    };
    fixed_length.lexical.pattern_groups = vec![PatternGroup {
        alternatives: vec![PatternExpression::xml_schema("[ -~]{10}")],
    }];
    cases.push(("length instead of min/max", fixed_length));

    // Task 041 note: `QueryString4096Type`'s `[ -~\n\r]{0,4096}` used to be
    // listed here, because at that time no profile implemented a class
    // containing LF and CR. Task 041 implements exactly that family, so it is
    // now legitimately baseline and has a POSITIVE control of its own in
    // `whitespace_visible_coverage.rs`. The visible-ASCII-specific property it
    // was protecting is asserted there and in the classifier's own tests: a
    // line-break class is never the visible-ASCII profile.

    // NATO_SpecialWordsType: ASCII-only, but a distinct lexical profile.
    let mut nato = visible_ascii(1, 256);
    nato.lexical.pattern_groups = vec![PatternGroup {
        alternatives: vec![PatternExpression::xml_schema(r"NATO:[a-zA-Z\-_]{1,256}")],
    }];
    cases.push(("the NATO special-words profile", nato));

    // An unobserved but perfectly ordinary pair. The shape agrees with itself
    // exactly; it is simply not a bound pair the authoritative family carries,
    // so coverage must not claim it.
    cases.push(("unobserved bounds 3..17", visible_ascii(3, 17)));

    // The load-bearing one: an internally consistent synthetic profile whose
    // maxLength cannot be assumed representable by every backend's length
    // constant. Coverage must not report this as baseline-renderable, because
    // the generated C++ `static constexpr std::size_t kMaxLength` for it is not
    // guaranteed to compile under the project's strict flags. This is the
    // readiness/generation agreement the shared classifier exists to preserve.
    cases.push(("unobserved u64::MAX bounds", visible_ascii(1, u64::MAX)));

    for (label, constraints) in cases {
        let schema = schema(vec![primitive(
            "Candidate",
            PrimitiveKind::String,
            constraints,
        )]);
        for language in LANGUAGES {
            assert!(
                !is_baseline(&schema, language),
                "{language:?} must NOT treat {label} as baseline"
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
