//! Task 058: the bounded-ASCII String family through the shared classifier
//! AND `CoverageAnalysis` (the same verdict readiness uses), with every
//! negative neighbour failing closed and every earlier String profile
//! classified exactly as before.

use ams_gra_oms_codegen_core::{
    BackendLanguage, BoundedAsciiAlphabet, BoundedAsciiLength, CoverageAnalysis, GenerationWorld,
    NATO_SPECIAL_WORDS_PATTERN, StringProfile, StringProfileError, UCI_SCHEMA_VERSION_PATTERN,
    UCI_UUID_PATTERN, bounded_ascii_profiles, string_profile, visible_ascii_pattern,
};
use ams_gra_oms_ir::{
    ConstraintSet, LexicalConstraintSet, NamespaceDecl, NumericValue, PatternDialect,
    PatternExpression, PatternGroup, PrimitiveKind, QualifiedName, SchemaIr, SourceRef, TypeDecl,
    TypeKind, WhiteSpacePolicy,
};

const NS: &str = "urn:test:bounded";

fn exact(pattern: &str, length: u64) -> ConstraintSet {
    ConstraintSet {
        length: Some(length),
        lexical: LexicalConstraintSet {
            pattern_groups: vec![PatternGroup {
                alternatives: vec![PatternExpression::xml_schema(pattern)],
            }],
            white_space: None,
        },
        ..ConstraintSet::default()
    }
}

fn range(pattern: &str, min_length: u64, max_length: u64) -> ConstraintSet {
    ConstraintSet {
        min_length: Some(min_length),
        max_length: Some(max_length),
        lexical: LexicalConstraintSet {
            pattern_groups: vec![PatternGroup {
                alternatives: vec![PatternExpression::xml_schema(pattern)],
            }],
            white_space: None,
        },
        ..ConstraintSet::default()
    }
}

fn classify(constraints: &ConstraintSet) -> Result<Option<StringProfile>, StringProfileError> {
    string_profile(PrimitiveKind::String, constraints)
}

fn declaration(name: &str, constraints: ConstraintSet) -> TypeDecl {
    TypeDecl {
        name: QualifiedName::new(NS, name),
        is_abstract: false,
        base_type: None,
        kind: TypeKind::Primitive(PrimitiveKind::String),
        constraints,
        documentation: None,
        source: SourceRef {
            document: "bounded.ir".to_owned(),
            line: Some(1),
        },
    }
}

fn baseline(name: &str, constraints: ConstraintSet) -> bool {
    let schema = SchemaIr {
        schema_version: None,
        namespaces: vec![NamespaceDecl {
            uri: NS.to_owned(),
            preferred_prefix: None,
        }],
        types: vec![declaration(name, constraints)],
        messages: Vec::new(),
    };
    let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
    let verdicts: Vec<bool> = BackendLanguage::ALL
        .iter()
        .map(|&language| {
            analysis
                .backend_coverage(language)
                .unwrap()
                .declarations_fully_renderable
                == 1
        })
        .collect();
    assert!(
        verdicts.iter().all(|v| *v == verdicts[0]),
        "backends disagree on {name}"
    );
    verdicts[0]
}

/// The whole pinned domain: 60 rows, 19 alphabets, every one classifying to
/// itself and baseline in every backend.
#[test]
fn every_pinned_row_classifies_and_is_baseline() {
    let rows = bounded_ascii_profiles();
    assert_eq!(rows.len(), 60);
    let alphabets: std::collections::BTreeSet<_> = rows.iter().map(|row| row.0).collect();
    assert_eq!(alphabets.len(), 19);
    assert_eq!(BoundedAsciiAlphabet::ALL.len(), 19);
    for &(alphabet, spelling, length) in rows {
        let pattern = format!("{spelling}{}", length.quantifier());
        let constraints = match length {
            BoundedAsciiLength::Exact(n) => exact(&pattern, n),
            BoundedAsciiLength::Range {
                min_length,
                max_length,
            } => range(&pattern, min_length, max_length),
        };
        assert_eq!(
            classify(&constraints),
            Ok(Some(StringProfile::BoundedAscii { alphabet, length })),
            "{pattern}"
        );
        assert!(baseline("AnyName", constraints), "{pattern}");
    }
}

/// Alphabet member sets: all ASCII, printable, well-ordered, and never
/// containing a whitespace control or DEL.
#[test]
fn every_alphabet_is_a_disjoint_ordered_subset_of_visible_ascii() {
    for alphabet in BoundedAsciiAlphabet::ALL {
        let ranges = alphabet.ranges();
        assert!(!ranges.is_empty());
        for window in ranges.windows(2) {
            assert!(window[0].1 + 1 < window[1].0, "{alphabet:?} not canonical");
        }
        for &(low, high) in ranges {
            assert!(low <= high && low >= 0x20 && high <= 0x7E, "{alphabet:?}");
        }
        for byte in [0x00, 0x09, 0x0A, 0x0D, 0x7F, 0x80, 0xFF] {
            assert!(!alphabet.contains(byte), "{alphabet:?} {byte:#x}");
        }
    }
}

/// The two task-named boundary declarations, under their real IR shapes.
#[test]
fn empty_type_and_aircraft_identifier_shapes_are_members() {
    assert_eq!(
        classify(&exact("[a-zA-Z]{0}", 0)),
        Ok(Some(StringProfile::BoundedAscii {
            alphabet: BoundedAsciiAlphabet::Letters,
            length: BoundedAsciiLength::Exact(0),
        }))
    );
    assert_eq!(
        classify(&exact("[A-Z0-9 ]{8}", 8)),
        Ok(Some(StringProfile::BoundedAscii {
            alphabet: BoundedAsciiAlphabet::UpperAlphanumericSpace,
            length: BoundedAsciiLength::Exact(8),
        }))
    );
    // Membership is by facets, never by name, in both directions.
    assert!(baseline("EmptyType", exact("[a-zA-Z]{0}", 0)));
    assert!(baseline("TotallyDifferent", exact("[A-Z0-9 ]{8}", 8)));
    assert!(!baseline(
        "AircraftIdentifierType",
        exact("[A-Z0-9 ]{9}", 9)
    ));
    assert!(!baseline("EmptyType", exact("[a-zA-Z]{1}", 1)));
}

/// Section 8 negative neighbours: every one fails closed AND is non-baseline.
#[test]
fn negative_neighbours_fail_closed() {
    let mut cases: Vec<(&str, ConstraintSet)> = vec![
        ("same pattern, wrong length", exact("[A-Z0-9 ]{8}", 9)),
        ("quantifier/length disagree", exact("[A-Z0-9 ]{9}", 8)),
        (
            "same pattern, wrong minLength",
            range("[a-zA-Z0-9]{1,20}", 2, 20),
        ),
        (
            "same pattern, wrong maxLength",
            range("[a-zA-Z0-9]{1,20}", 1, 21),
        ),
        ("same bounds, different pattern", exact("[A-Z0-9_]{8}", 8)),
        (
            "unobserved spelling of a member set",
            exact("[0-9A-Z ]{8}", 8),
        ),
        ("unobserved alphabet", range("[a-f0-9]{1,20}", 1, 20)),
        (
            "observed alphabet, arbitrary bound",
            range("[a-zA-Z0-9]{1,21}", 1, 21),
        ),
        (
            "observed alphabet, huge bound",
            range("[a-zA-Z0-9]{1,18446744073709551615}", 1, u64::MAX),
        ),
        (
            "exact shape with {M,N} quantifier",
            exact("[a-zA-Z0-9]{4,4}", 4),
        ),
        (
            "range shape with {N} quantifier",
            range("[a-zA-Z0-9]{4}", 4, 4),
        ),
        ("non-ASCII class", range("[a-zA-Z\u{e9}]{1,20}", 1, 20)),
        ("class with TAB", range("[a-zA-Z0-9\t]{1,20}", 1, 20)),
        ("class subtraction", range("[a-z-[aeiou]]{1,20}", 1, 20)),
        ("unicode category", range("\\p{Lu}{1,20}", 1, 20)),
        ("multi-char escape", range("\\d{1,20}", 1, 20)),
        ("wildcard", range(".{1,20}", 1, 20)),
        (
            "unobserved alternation neighbor",
            range("[A-Z0-9]{5}|UNKN|NOPE", 4, 5),
        ),
        ("unbounded *", range("[0-7]*", 1, 16)),
    ];
    // Extra facets on an admitted row.
    let mut both = exact("[A-Z0-9 ]{8}", 8);
    both.min_length = Some(8);
    cases.push(("length plus minLength", both));
    let mut numeric = exact("[A-Z0-9 ]{8}", 8);
    numeric.min_inclusive = Some(NumericValue::Integer(0));
    cases.push(("extra numeric facet", numeric));
    for policy in [
        WhiteSpacePolicy::Preserve,
        WhiteSpacePolicy::Replace,
        WhiteSpacePolicy::Collapse,
    ] {
        let mut ws = exact("[A-Z0-9 ]{8}", 8);
        ws.lexical.white_space = Some(policy);
        cases.push(("explicit whiteSpace", ws));
    }
    let mut two_groups = exact("[A-Z0-9 ]{8}", 8);
    two_groups.lexical.pattern_groups.push(PatternGroup {
        alternatives: vec![PatternExpression::xml_schema("[A-Z0-9 ]{8}")],
    });
    cases.push(("extra pattern group", two_groups));
    let mut two_alternatives = exact("[A-Z0-9 ]{8}", 8);
    two_alternatives.lexical.pattern_groups[0]
        .alternatives
        .push(PatternExpression::xml_schema("[A-Z]{8}"));
    cases.push(("multiple alternatives", two_alternatives));
    let mut no_pattern = exact("[A-Z0-9 ]{8}", 8);
    no_pattern.lexical.pattern_groups.clear();
    cases.push(("length without pattern", no_pattern));
    for (label, constraints) in cases {
        assert_eq!(
            classify(&constraints),
            Err(StringProfileError::UnsupportedConstraints),
            "{label}"
        );
        assert!(!baseline("AircraftIdentifierType", constraints), "{label}");
    }
    // Only one dialect exists in the IR today; the classifier requires it.
    assert_eq!(
        exact("[A-Z0-9 ]{8}", 8).lexical.pattern_groups[0].alternatives[0].dialect,
        PatternDialect::XmlSchema
    );
}

/// Existing String profiles are classified exactly as before; none becomes
/// `BoundedAscii`, and no bounded-ASCII row is one of them.
#[test]
fn the_bounded_ascii_family_never_shadows_an_existing_profile() {
    assert_eq!(
        classify(&range(UCI_SCHEMA_VERSION_PATTERN, 7, 57)),
        Ok(Some(StringProfile::UciSchemaVersion))
    );
    assert_eq!(
        classify(&exact(UCI_UUID_PATTERN, 36)),
        Ok(Some(StringProfile::UniversallyUniqueIdentifier))
    );
    assert_eq!(
        classify(&range(&visible_ascii_pattern(1, 256), 1, 256)),
        Ok(Some(StringProfile::VisibleAscii {
            min_length: 1,
            max_length: 256
        }))
    );
    assert_eq!(
        classify(&range(NATO_SPECIAL_WORDS_PATTERN, 6, 261)),
        Ok(Some(StringProfile::NatoSpecialWords))
    );
    // [ -~] under min/max stays VisibleAscii (or fails closed), never
    // BoundedAscii; [ -~] under `length` is BoundedAscii.
    assert!(matches!(
        classify(&range("[ -~]{1,20}", 1, 20)),
        Ok(Some(StringProfile::VisibleAscii { .. }))
    ));
    assert_eq!(
        classify(&range("[ -~]{2,2}", 2, 2)),
        Err(StringProfileError::UnsupportedConstraints)
    );
    // No full row expression is an existing profile's expression, and no row
    // is a min/max `[ -~]` shape (the Task 039 VisibleAscii family).
    for &(_, spelling, length) in bounded_ascii_profiles() {
        let expression = format!("{spelling}{}", length.quantifier());
        for existing in [
            UCI_SCHEMA_VERSION_PATTERN,
            UCI_UUID_PATTERN,
            NATO_SPECIAL_WORDS_PATTERN,
        ] {
            assert_ne!(expression, existing);
        }
        assert!(
            !(spelling == "[ -~]" && matches!(length, BoundedAsciiLength::Range { .. })),
            "{expression}"
        );
    }
}

/// TEST-ONLY expansion of one pinned class spelling into its member set.
///
/// Deliberately minimal: it understands exactly the constructs the 21 pinned
/// spellings use (single characters, `\`-escaped single characters, `a-b`
/// ranges) and panics on anything else. Production code never parses a class;
/// this exists only to prove the hand-written numeric `ranges()` table and the
/// pinned spellings describe the SAME member set.
fn expand_pinned_class(spelling: &str) -> std::collections::BTreeSet<u8> {
    let body = spelling
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .expect("a bracketed class");
    let mut atoms = Vec::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            atoms.push((chars.next().expect("escaped char"), true));
        } else {
            atoms.push((c, false));
        }
    }
    let mut members = std::collections::BTreeSet::new();
    let mut i = 0;
    while i < atoms.len() {
        let (low, _) = atoms[i];
        if i + 2 < atoms.len() && atoms[i + 1] == ('-', false) {
            let (high, _) = atoms[i + 2];
            assert!(low <= high, "{spelling}");
            members.extend(low as u8..=high as u8);
            i += 3;
        } else {
            assert!(low.is_ascii(), "{spelling}");
            members.insert(low as u8);
            i += 1;
        }
    }
    members
}

#[test]
fn every_pinned_spelling_denotes_exactly_its_alphabets_ranges() {
    let mut spellings = std::collections::BTreeSet::new();
    for &(alphabet, spelling, _) in bounded_ascii_profiles() {
        spellings.insert(spelling);
        let from_ranges: std::collections::BTreeSet<u8> = (0u8..=0x7F)
            .filter(|&byte| alphabet.contains(byte))
            .collect();
        assert_eq!(
            expand_pinned_class(spelling),
            from_ranges,
            "{alphabet:?} vs {spelling}"
        );
    }
    assert_eq!(spellings.len(), 20, "distinct pinned spellings");
}
