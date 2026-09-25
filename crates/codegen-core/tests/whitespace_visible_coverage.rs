//! Task 041 coverage boundaries for the whitespace-visible String family.
//!
//! Every evidenced profile triple becomes baseline-renderable in all three
//! backends; every near-miss stays unsupported. These are asserted against
//! `CoverageAnalysis` rather than inferred, because coverage and the backend
//! validation gates must agree exactly -- that agreement is the reason the
//! shared classifier exists.

use ams_gra_oms_codegen_core::{
    BackendLanguage, CoverageAnalysis, GenerationWorld, WhitespaceVisiblePolicy,
    visible_ascii_pattern, whitespace_visible_pattern,
};
use ams_gra_oms_ir::{
    ConstraintSet, LexicalConstraintSet, NamespaceDecl, PatternExpression, PatternGroup,
    PrimitiveKind, QualifiedName, SchemaIr, SourceRef, TypeDecl, TypeKind, WhiteSpacePolicy,
};

const NS: &str = "urn:test:remarks";
const LANGUAGES: [BackendLanguage; 3] = [
    BackendLanguage::Ada,
    BackendLanguage::Rust,
    BackendLanguage::Cpp,
];

/// Every `(policy, min, max)` triple read from the pinned release bytes.
///
/// Duplicated from the classifier's own private table on purpose: this is an
/// INDEPENDENT statement of the evidence, so narrowing the table without also
/// changing the evidence breaks this test.
const EVIDENCED: [(WhitespaceVisiblePolicy, u64, u64); 5] = [
    // UCI 2.5 WhitespaceVisibleString1024Type / 4096Type.
    (WhitespaceVisiblePolicy::Collapse, 0, 1024),
    (WhitespaceVisiblePolicy::Collapse, 0, 4096),
    // UCI 2.5 QueryString4096Type.
    (WhitespaceVisiblePolicy::Preserve, 0, 4096),
    // UCI 2.6, all three declarations.
    (WhitespaceVisiblePolicy::Preserve, 1, 1024),
    (WhitespaceVisiblePolicy::Preserve, 1, 4096),
];

fn source() -> SourceRef {
    SourceRef {
        document: "remarks.ir".to_owned(),
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

/// A whitespace-visible member, in normalized IR form.
fn whitespace_visible(
    white_space: WhitespaceVisiblePolicy,
    min_length: u64,
    max_length: u64,
) -> ConstraintSet {
    ConstraintSet {
        min_length: Some(min_length),
        max_length: Some(max_length),
        lexical: LexicalConstraintSet {
            pattern_groups: vec![PatternGroup {
                alternatives: vec![PatternExpression::xml_schema(whitespace_visible_pattern(
                    min_length, max_length,
                ))],
            }],
            white_space: match white_space {
                WhitespaceVisiblePolicy::Collapse => Some(WhiteSpacePolicy::Collapse),
                WhitespaceVisiblePolicy::Preserve => None,
            },
        },
        ..ConstraintSet::default()
    }
}

/// Whether every declaration in the schema is baseline-renderable.
fn is_baseline(schema: &SchemaIr, language: BackendLanguage) -> bool {
    CoverageAnalysis::new(schema, GenerationWorld::ClosedSchemaSet)
        .expect("analysis must succeed")
        .backend_coverage(language)
        .expect("backend coverage must be measurable")
        .declarations_fully_renderable
        == schema.types.len()
}

/// EVERY evidenced triple is baseline-renderable in all three backends.
#[test]
fn every_evidenced_whitespace_visible_profile_is_baseline_renderable() {
    for (white_space, min_length, max_length) in EVIDENCED {
        // The declaration name is deliberately NOT a UCI local name: support
        // must arise from the primitive kind plus the effective constraints.
        let schema = schema(vec![primitive(
            "Candidate",
            PrimitiveKind::String,
            whitespace_visible(white_space, min_length, max_length),
        )]);
        for language in LANGUAGES {
            assert!(
                is_baseline(&schema, language),
                "{language:?} must treat {white_space:?} {min_length}..{max_length} as baseline"
            );
        }
    }
}

/// All five profiles coexist in ONE schema and are all renderable together.
///
/// A per-declaration check could pass while a generated unit holding several
/// members failed, so the whole family is measured as one schema too.
#[test]
fn the_whole_whitespace_visible_family_is_renderable_in_one_schema() {
    let types = EVIDENCED
        .iter()
        .enumerate()
        .map(|(index, &(white_space, min_length, max_length))| {
            primitive(
                &format!("Member{index}"),
                PrimitiveKind::String,
                whitespace_visible(white_space, min_length, max_length),
            )
        })
        .collect();
    let schema = schema(types);
    assert_eq!(schema.types.len(), 5);
    for language in LANGUAGES {
        assert!(
            is_baseline(&schema, language),
            "{language:?} must render the whole family together"
        );
    }
}

/// A SEMANTIC ALIAS gets identical support, whatever it is called.
///
/// `QueryString4096Type` is the authoritative instance of this: in UCI 2.6 it
/// carries exactly `WhitespaceVisibleString4096Type`'s profile despite sharing
/// none of its name.
#[test]
fn a_semantic_alias_under_any_name_is_equally_renderable() {
    let profile = whitespace_visible(WhitespaceVisiblePolicy::Preserve, 1, 4096);
    for name in [
        "WhitespaceVisibleString4096Type",
        "QueryString4096Type",
        "CompletelyUnrelatedName",
        "X",
    ] {
        let schema = schema(vec![primitive(
            name,
            PrimitiveKind::String,
            profile.clone(),
        )]);
        for language in LANGUAGES {
            assert!(
                is_baseline(&schema, language),
                "{language:?} must render {name} with the evidenced profile"
            );
        }
    }
}

/// A UCI-LOOKING name with the WRONG facets stays unsupported.
///
/// Support is decided by facets, which cuts both ways: the real declaration name
/// buys nothing when the constraints are not the evidenced ones.
#[test]
fn a_uci_name_with_wrong_facets_is_not_renderable() {
    // The real UCI name, but UCI 2.6's minimum paired with UCI 2.5's collapse
    // facet -- a combination no pinned release contains.
    let wrong = whitespace_visible(WhitespaceVisiblePolicy::Collapse, 1, 1024);
    let schema = schema(vec![primitive(
        "WhitespaceVisibleString1024Type",
        PrimitiveKind::String,
        wrong,
    )]);
    for language in LANGUAGES {
        assert!(
            !is_baseline(&schema, language),
            "{language:?} must not render a UCI-named declaration with unobserved facets"
        );
    }
}

/// Every near-miss stays out of coverage.
#[test]
fn whitespace_visible_near_misses_are_not_baseline() {
    let mut cases: Vec<(&str, ConstraintSet)> = vec![
        // Unobserved-but-ordinary bounds, with a perfectly agreeing quantifier.
        (
            "unobserved 0..512",
            whitespace_visible(WhitespaceVisiblePolicy::Collapse, 0, 512),
        ),
        (
            "unobserved 1..2048",
            whitespace_visible(WhitespaceVisiblePolicy::Preserve, 1, 2048),
        ),
        // The load-bearing one: an internally consistent synthetic profile whose
        // maxLength cannot be assumed representable by every backend's length
        // constant. Coverage must not report this as baseline-renderable,
        // because the emitted length literal is not guaranteed to compile
        // everywhere.
        (
            "unobserved u64::MAX bounds",
            whitespace_visible(WhitespaceVisiblePolicy::Preserve, 1, u64::MAX),
        ),
        // A MISSING whiteSpace facet where the evidence has one. The EXTRA-facet
        // cases follow below, because each needs a mutation.
        (
            "collapse facet dropped",
            whitespace_visible(WhitespaceVisiblePolicy::Preserve, 0, 1024),
        ),
    ];

    let mut explicit_preserve = whitespace_visible(WhitespaceVisiblePolicy::Preserve, 1, 1024);
    explicit_preserve.lexical.white_space = Some(WhiteSpacePolicy::Preserve);
    cases.push(("explicit whiteSpace = preserve", explicit_preserve));
    let mut replace = whitespace_visible(WhitespaceVisiblePolicy::Preserve, 1, 1024);
    replace.lexical.white_space = Some(WhiteSpacePolicy::Replace);
    cases.push(("whiteSpace = replace", replace));

    // Mismatched pattern and facet bounds, which must not be approximated.
    let mut mismatched = whitespace_visible(WhitespaceVisiblePolicy::Collapse, 0, 1024);
    mismatched.lexical.pattern_groups = vec![PatternGroup {
        alternatives: vec![PatternExpression::xml_schema(whitespace_visible_pattern(
            0, 4096,
        ))],
    }];
    cases.push(("facets disagree with the quantifier", mismatched));

    // Extra groups and extra alternatives.
    let mut two_groups = whitespace_visible(WhitespaceVisiblePolicy::Collapse, 0, 1024);
    two_groups.lexical.pattern_groups.push(PatternGroup {
        alternatives: vec![PatternExpression::xml_schema(whitespace_visible_pattern(
            0, 1024,
        ))],
    });
    cases.push(("a second pattern group", two_groups));
    let mut two_alternatives = whitespace_visible(WhitespaceVisiblePolicy::Collapse, 0, 1024);
    two_alternatives.lexical.pattern_groups[0]
        .alternatives
        .push(PatternExpression::xml_schema(whitespace_visible_pattern(
            0, 1024,
        )));
    cases.push(("a second expression", two_alternatives));

    // An unsupported extra facet: `length` alongside the bounds.
    let mut with_length = whitespace_visible(WhitespaceVisiblePolicy::Collapse, 0, 1024);
    with_length.length = Some(1024);
    cases.push(("a length facet", with_length));

    // TAB added to the class is a different lexical space.
    let mut with_tab = whitespace_visible(WhitespaceVisiblePolicy::Preserve, 1, 1024);
    with_tab.lexical.pattern_groups = vec![PatternGroup {
        alternatives: vec![PatternExpression::xml_schema(r"[ -~\n\r\t]{1,1024}")],
    }];
    cases.push(("TAB added to the class", with_tab));

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

/// The OLDER visible-ASCII controls keep their exact accepted/rejected sets.
///
/// Task 041's class is a superset of the visible-ASCII one, so this pins that
/// the two families did not blur: an unobserved visible-ASCII pair is still not
/// baseline even though the SAME pair is evidenced for the new family.
#[test]
fn the_visible_ascii_controls_are_unchanged() {
    let observed = ConstraintSet {
        min_length: Some(1),
        max_length: Some(256),
        lexical: LexicalConstraintSet {
            pattern_groups: vec![PatternGroup {
                alternatives: vec![PatternExpression::xml_schema(visible_ascii_pattern(1, 256))],
            }],
            white_space: None,
        },
        ..ConstraintSet::default()
    };
    let observed_schema = schema(vec![primitive("Callsign", PrimitiveKind::String, observed)]);
    for language in LANGUAGES {
        assert!(
            is_baseline(&observed_schema, language),
            "{language:?} must still render the visible-ASCII 1..256 profile"
        );
    }

    // 0..4096 is an evidenced WHITESPACE-VISIBLE pair but NOT a visible-ASCII
    // one, so the visible-ASCII pattern at those bounds must stay unsupported.
    let unobserved = ConstraintSet {
        min_length: Some(0),
        max_length: Some(4096),
        lexical: LexicalConstraintSet {
            pattern_groups: vec![PatternGroup {
                alternatives: vec![PatternExpression::xml_schema(visible_ascii_pattern(
                    0, 4096,
                ))],
            }],
            white_space: None,
        },
        ..ConstraintSet::default()
    };
    let unobserved_schema = schema(vec![primitive(
        "Candidate",
        PrimitiveKind::String,
        unobserved,
    )]);
    for language in LANGUAGES {
        assert!(
            !is_baseline(&unobserved_schema, language),
            "{language:?} must not borrow whitespace-visible bounds for visible ASCII"
        );
    }
}
