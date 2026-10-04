use ams_gra_oms_codegen_core::{
    AlternatingAsciiProfile, StructuredAsciiFacets, StructuredAsciiSegment,
    alternating_ascii_profile, alternating_ascii_rows,
};
use ams_gra_oms_ir::{ConstraintSet, PatternExpression, PatternGroup, WhiteSpacePolicy};

fn constraints(patterns: &[&str], facets: StructuredAsciiFacets) -> ConstraintSet {
    let mut c = ConstraintSet::default();
    match facets {
        StructuredAsciiFacets::Exact(n) => c.length = Some(n),
        StructuredAsciiFacets::Range(a, b) => {
            c.min_length = Some(a);
            c.max_length = Some(b);
        }
        StructuredAsciiFacets::Max(n) => c.max_length = Some(n),
    }
    c.lexical.pattern_groups.push(PatternGroup {
        alternatives: patterns
            .iter()
            .map(|p| PatternExpression::xml_schema(*p))
            .collect(),
    });
    c
}

#[test]
fn groups_are_intersection_and_branches_are_whole_value_union() {
    let branch = |s| vec![StructuredAsciiSegment::Literal(s)];
    let one = AlternatingAsciiProfile {
        facets: StructuredAsciiFacets::Exact(2),
        groups: vec![vec![branch("AA"), branch("BB")]],
    };
    assert!(one.accepts(b"AA"));
    assert!(one.accepts(b"BB"));
    assert!(!one.accepts(b"AABB"));
    let singleton = AlternatingAsciiProfile {
        facets: one.facets,
        groups: vec![vec![branch("AA")], vec![branch("BB")]],
    };
    assert!(!singleton.accepts(b"AA"));
    assert!(!singleton.accepts(b"BB"));
    let both = AlternatingAsciiProfile {
        facets: one.facets,
        groups: vec![
            vec![branch("AA"), branch("BB")],
            vec![branch("BB"), branch("CC")],
        ],
    };
    assert!(!both.accepts(b"AA"));
    assert!(!both.accepts(b"CC"));
    assert!(both.accepts(b"BB"));
    let mut reversed = both.clone();
    for group in &mut reversed.groups {
        group.reverse();
    }
    for input in [b"AA", b"BB", b"CC", b"DD"] {
        assert_eq!(both.accepts(input), reversed.accepts(input));
    }
}

#[test]
fn exact_profiles_and_fail_closed_neighbors() {
    assert_eq!(alternating_ascii_rows().len(), 16);
    for row in alternating_ascii_rows() {
        let c = constraints(row.patterns, row.profile.facets);
        assert_eq!(alternating_ascii_profile(&c), Some(&row.profile));
        let mut changed = c.clone();
        changed
            .lexical
            .pattern_groups
            .push(changed.lexical.pattern_groups[0].clone());
        assert!(alternating_ascii_profile(&changed).is_none());
        if row.patterns.len() > 1 {
            changed = c.clone();
            let p = changed.lexical.pattern_groups[0]
                .alternatives
                .pop()
                .unwrap();
            changed.lexical.pattern_groups.push(PatternGroup {
                alternatives: vec![p],
            });
            assert!(alternating_ascii_profile(&changed).is_none());
            changed = c.clone();
            changed.lexical.pattern_groups[0].alternatives.reverse();
            assert!(alternating_ascii_profile(&changed).is_none());
        }
        changed = c.clone();
        changed.lexical.white_space = Some(WhiteSpacePolicy::Preserve);
        assert!(alternating_ascii_profile(&changed).is_none());
        changed = c.clone();
        changed.lexical.pattern_groups[0].alternatives[0]
            .expression
            .push('|');
        assert!(alternating_ascii_profile(&changed).is_none());
        changed = c.clone();
        changed.max_length = Some(999);
        assert!(alternating_ascii_profile(&changed).is_none());
        for invalid in [
            b"\t".as_slice(),
            b"\n",
            b"\r",
            b"\0",
            b"\x7f",
            "١".as_bytes(),
        ] {
            assert!(!row.profile.accepts(invalid));
        }
    }
}

#[test]
fn all_frozen_release_rows_match_the_name_free_core_boundary() {
    let fixture = include_str!("../../../tests/fixtures/string/task060-pinned-rows.tsv");
    let mut admitted = 0;
    let mut deferred = 0;
    for line in fixture.lines().filter(|line| !line.starts_with('#')) {
        let cells: Vec<_> = line.split('\t').collect();
        let facets = if cells[2] != "-" {
            StructuredAsciiFacets::Exact(cells[2].parse().unwrap())
        } else {
            StructuredAsciiFacets::Range(cells[3].parse().unwrap(), cells[4].parse().unwrap())
        };
        let c = constraints(&cells[5..], facets);
        let expected = ![
            "IPv6_AddressType",
            "NITF_DateAndTimeType",
            "NITF_DateType",
            "NITF_MSTGTA_TargetLocationType",
        ]
        .contains(&cells[1]);
        assert_eq!(
            alternating_ascii_profile(&c).is_some(),
            expected,
            "{} {}",
            cells[0],
            cells[1]
        );
        let classified =
            ams_gra_oms_codegen_core::string_profile(ams_gra_oms_ir::PrimitiveKind::String, &c);
        if expected {
            assert!(matches!(
                classified,
                Ok(Some(
                    ams_gra_oms_codegen_core::StringProfile::AlternatingAscii(_)
                ))
            ));
        } else if cells[1] == "IPv6_AddressType" {
            assert_eq!(
                classified,
                Ok(Some(ams_gra_oms_codegen_core::StringProfile::Ipv6Address))
            );
        } else {
            assert!(classified.is_err(), "deferred {}", cells[1]);
        }
        if expected {
            admitted += 1;
        } else {
            deferred += 1;
        }
    }
    assert_eq!((admitted, deferred), (30, 8));
}

#[test]
fn semantic_group_and_alphabet_inventory() {
    use ams_gra_oms_codegen_core::structured_ascii_profiles;
    use std::collections::BTreeSet;
    let old: BTreeSet<Vec<(u8, u8)>> = structured_ascii_profiles()
        .iter()
        .flat_map(|(_, profile)| profile.segments.iter())
        .filter_map(|segment| match segment {
            StructuredAsciiSegment::Class(alphabet, _) => Some(alphabet.ranges().to_vec()),
            _ => None,
        })
        .collect();
    let current: BTreeSet<Vec<(u8, u8)>> = alternating_ascii_rows()
        .iter()
        .flat_map(|row| row.profile.groups.iter().flatten().flatten())
        .filter_map(|segment| match segment {
            StructuredAsciiSegment::Class(alphabet, _) => Some(alphabet.ranges().to_vec()),
            _ => None,
        })
        .collect();
    let reused = current.intersection(&old).count();
    let new = current.difference(&old).count();
    println!(
        "TASK060 ALPHABETS: total={} reused059={reused} new={new}",
        current.len()
    );
    let groups: usize = alternating_ascii_rows()
        .iter()
        .map(|row| row.profile.groups.len())
        .sum();
    let branches: usize = alternating_ascii_rows()
        .iter()
        .map(|row| row.profile.groups.iter().map(Vec::len).sum::<usize>())
        .sum();
    println!(
        "TASK060 SEMANTIC INVENTORY: profiles=16 groups={groups} expanded_branches={branches}"
    );
    assert_eq!(groups, 16);
    assert_eq!(branches, 724);
    assert_eq!((current.len(), reused, new), (18, 5, 13));
}

fn row(pattern: &str) -> &'static AlternatingAsciiProfile {
    &alternating_ascii_rows()
        .iter()
        .find(|r| r.patterns[0] == pattern)
        .unwrap()
        .profile
}

#[test]
fn high_value_profiles_and_independent_facets() {
    let notation = row("[A-Z0-9]{5}|UNKN|NONE");
    for text in ["ABC12", "UNKN", "NONE"] {
        assert!(notation.accepts(text.as_bytes()));
    }
    for text in ["UNK", "none", "ABC1", "ABC123", "UNKX", "AB!12", " UNKN"] {
        assert!(!notation.accepts(text.as_bytes()));
    }
    let origin = row(r"[A-Z][A-Z]|[E]|[\-]");
    for text in ["AA", "ZZ", "E", "-"] {
        assert!(origin.accepts(text.as_bytes()));
    }
    for text in ["A", "Z", "--", "E-", "aa", " "] {
        assert!(!origin.accepts(text.as_bytes()));
    }
    let grid: Vec<_> = alternating_ascii_rows()
        .iter()
        .filter(|r| r.patterns[0].contains("C-HJ-NP-X"))
        .collect();
    assert_eq!(grid[0].profile.groups[0].len(), 28);
    assert_eq!(grid[1].profile.groups[0].len(), 24);
    assert!(grid[0].profile.accepts_pattern(b"AAB"));
    assert!(!grid[0].profile.accepts(b"AAB"));
    assert!(grid[1].profile.accepts(b"AAB"));
    for g in &grid {
        assert!(g.profile.accepts(b"1CAA0000000000"));
        assert!(!g.profile.accepts(b"1CIA0000000000"));
    }
    let exemption = row("(X[1-8]( ){2})|25X[1-9]|[DNIO]|( ){4}");
    assert!(exemption.accepts_pattern(b"D"));
    assert!(!exemption.accepts(b"D"));
    assert!(!exemption.accepts(b"XXXX"));
    let ip = alternating_ascii_rows()
        .iter()
        .find(|r| r.profile.groups[0].len() == 625)
        .unwrap();
    for text in ["0.0.0.0", "9.10.199.255", "255.249.100.1"] {
        assert!(ip.profile.accepts(text.as_bytes()));
    }
    for text in [
        "00.0.0.0",
        "256.0.0.0",
        "1.2.3",
        "1.2.3.4.5",
        ".1.2.3.4",
        "1.2.3.4.",
        "+1.2.3.4",
    ] {
        assert!(!ip.profile.accepts(text.as_bytes()));
    }
}

#[test]
fn delimiter_factoring_requires_the_complete_semantic_product() {
    use ams_gra_oms_codegen_core::factor_delimited_ascii;
    let ip = alternating_ascii_rows()
        .iter()
        .find(|row| row.profile.groups[0].len() == 625)
        .unwrap();
    let branches = &ip.profile.groups[0];
    let product = factor_delimited_ascii(branches).unwrap();
    assert_eq!(
        (
            product.delimiter,
            product.components,
            product.alternatives.len()
        ),
        (b'.', 4, 5)
    );
    assert!(factor_delimited_ascii(&branches[..624]).is_none());
    let component = AlternatingAsciiProfile {
        groups: vec![product.alternatives],
        facets: StructuredAsciiFacets::Range(1, 3),
    };
    for text in [
        "0.0.0.0",
        "1.2.3.4",
        "9.99.199.249",
        "255.255.255.255",
        "256.0.0.1",
        "999.1.1.1",
        "01.2.3.4",
        "1.2.3",
        "1.2.3.4.5",
        ".1.2.3.4",
        "1.2.3.4.",
        "1..2.3",
        "+1.2.3.4",
        " 1.2.3.4",
        "١.2.3.4",
    ] {
        let pieces: Vec<_> = text.as_bytes().split(|b| *b == b'.').collect();
        let factored =
            pieces.len() == 4 && pieces.iter().all(|part| component.accepts_pattern(part));
        assert_eq!(
            factored,
            ip.profile.accepts_pattern(text.as_bytes()),
            "{text}"
        );
    }
}

#[test]
fn pinned_ipv4_expected_lexical_domain_is_not_a_platform_parser() {
    let ip = alternating_ascii_rows()
        .iter()
        .find(|row| row.profile.groups[0].len() == 625)
        .unwrap();
    for text in ["0.0.0.0", "1.2.3.4", "9.99.199.249", "255.255.255.255"] {
        assert!(ip.profile.accepts(text.as_bytes()), "{text}");
    }
    for text in [
        "256.0.0.1",
        "999.1.1.1",
        "01.2.3.4",
        "1.2.3",
        "1.2.3.4.5",
        ".1.2.3.4",
        "1.2.3.4.",
        "1..2.3",
        "+1.2.3.4",
        " 1.2.3.4",
        "1.2.3.4 ",
        "١.2.3.4",
    ] {
        assert!(!ip.profile.accepts(text.as_bytes()), "{text}");
    }
}
