//! Task 059: only whole pinned expression/facet rows classify.
use ams_gra_oms_codegen_core::{
    StringProfile, StructuredAsciiFacets, StructuredAsciiRepetition, StructuredAsciiSegment,
    string_profile, structured_ascii_profiles,
};
use ams_gra_oms_ir::{
    ConstraintSet, LexicalConstraintSet, NumericValue, PatternExpression, PatternGroup,
    PrimitiveKind, WhiteSpacePolicy,
};

fn constraints(pattern: &str, facets: StructuredAsciiFacets) -> ConstraintSet {
    let mut c = ConstraintSet {
        lexical: LexicalConstraintSet {
            pattern_groups: vec![PatternGroup {
                alternatives: vec![PatternExpression::xml_schema(pattern)],
            }],
            white_space: None,
        },
        ..ConstraintSet::default()
    };
    match facets {
        StructuredAsciiFacets::Exact(n) => c.length = Some(n),
        StructuredAsciiFacets::Range(a, b) => {
            c.min_length = Some(a);
            c.max_length = Some(b);
        }
        StructuredAsciiFacets::Max(n) => c.max_length = Some(n),
    }
    c
}
fn admitted(c: &ConstraintSet) -> bool {
    matches!(
        string_profile(PrimitiveKind::String, c),
        Ok(Some(StringProfile::StructuredAscii(_)))
    )
}
#[test]
fn every_pinned_row_classifies_without_shadowing_earlier_families() {
    let rows = structured_ascii_profiles();
    assert_eq!(rows.len(), 27);
    for &(pattern, profile) in rows {
        assert_eq!(
            string_profile(PrimitiveKind::String, &constraints(pattern, profile.facets)),
            Ok(Some(StringProfile::StructuredAscii(profile))),
            "{pattern}"
        );
        for segment in profile.segments {
            match segment {
                StructuredAsciiSegment::Literal(text) => {
                    assert!(text.bytes().all(|b| (0x20..=0x7e).contains(&b)))
                }
                StructuredAsciiSegment::Class(alphabet, repetition) => {
                    let ranges = alphabet.ranges();
                    assert!(!ranges.is_empty());
                    assert!(
                        ranges
                            .iter()
                            .all(|&(a, b)| a >= 0x20 && a <= b && b <= 0x7e)
                    );
                    assert!(ranges.windows(2).all(|r| r[0].1 + 1 < r[1].0));
                    if let StructuredAsciiRepetition::Bounded(a, b) = repetition {
                        assert!(a <= b)
                    }
                }
            }
        }
    }
    assert!(matches!(
        string_profile(
            PrimitiveKind::String,
            &constraints("[a-zA-Z]{0}", StructuredAsciiFacets::Exact(0))
        ),
        Ok(Some(StringProfile::BoundedAscii { .. }))
    ));
}
#[test]
fn exact_rows_have_fail_closed_neighbours() {
    for &(pattern, profile) in structured_ascii_profiles() {
        let mut c = constraints(pattern, profile.facets);
        c.lexical.white_space = Some(WhiteSpacePolicy::Preserve);
        assert!(!admitted(&c), "{pattern} explicit whiteSpace");
        c = constraints(pattern, profile.facets);
        c.min_inclusive = Some(NumericValue::Integer(0));
        assert!(!admitted(&c), "{pattern} numeric");
        c = constraints(pattern, profile.facets);
        c.lexical.pattern_groups[0]
            .alternatives
            .push(PatternExpression::xml_schema(pattern));
        assert!(!admitted(&c), "{pattern} alternatives");
        c = constraints(pattern, profile.facets);
        c.lexical.pattern_groups.push(PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(pattern)],
        });
        assert!(!admitted(&c), "{pattern} groups");
        c = constraints(pattern, profile.facets);
        c.lexical.pattern_groups[0].alternatives[0]
            .expression
            .push('Z');
        assert!(!admitted(&c), "{pattern} suffix");
        c = constraints(pattern, profile.facets);
        c.length = Some(999);
        assert!(!admitted(&c), "{pattern} facet");
        c = constraints(pattern, profile.facets);
        c.length = None;
        c.min_length = None;
        c.max_length = None;
        assert!(!admitted(&c), "{pattern} missing facets");
    }
    for pattern in [
        "IMX[0-9]{7}",
        "IMO[0-9]{8}",
        "IMO[1-9]{7}",
        "IMO[0-9]{7}|NONE",
        "[a-z-[aeiou]]+",
        "[^0-9]",
        "\\p{Lu}",
        ".*",
        "[a-z]+[a-z]+",
    ] {
        assert!(
            !admitted(&constraints(pattern, StructuredAsciiFacets::Exact(10))),
            "{pattern}"
        );
    }
    assert!(!admitted(&constraints(
        "[A-Za-z0-9]{1,4}",
        StructuredAsciiFacets::Range(1, 4)
    )));
}

#[test]
fn length_guided_prefix_is_exclusive_to_the_exact_pinned_row() {
    let rows: Vec<_> = structured_ascii_profiles()
        .iter()
        .filter(|(_, profile)| {
            profile.segments.iter().any(|segment| {
                matches!(
                    segment,
                    StructuredAsciiSegment::Class(
                        _,
                        StructuredAsciiRepetition::OptionalOneByTotalLength(_)
                    )
                )
            })
        })
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "1?[1-7][1-8]{2}");
    assert_eq!(rows[0].1.facets, StructuredAsciiFacets::Range(3, 4));
    assert!(matches!(
        rows[0].1.segments,
        [
            StructuredAsciiSegment::Class(
                _,
                StructuredAsciiRepetition::OptionalOneByTotalLength(4)
            ),
            StructuredAsciiSegment::Class(_, StructuredAsciiRepetition::One),
            StructuredAsciiSegment::Class(_, StructuredAsciiRepetition::Exact(2)),
        ]
    ));
    for pattern in [
        "2?[1-7][1-8]{2}",
        "1?[1-8][1-8]{2}",
        "1?[1-7][1-8]{3}",
        "1?[1-7][1-8]{2}|NONE",
    ] {
        assert!(
            !admitted(&constraints(pattern, StructuredAsciiFacets::Range(3, 4))),
            "{pattern}"
        );
    }
    assert!(!admitted(&constraints(
        "1?[1-7][1-8]{2}",
        StructuredAsciiFacets::Range(2, 5)
    )));
}
