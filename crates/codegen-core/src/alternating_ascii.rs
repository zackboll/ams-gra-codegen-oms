//! Task 060 finite deterministic bodies. No regex source is evaluated here.
use crate::{
    StructuredAsciiAlphabet, StructuredAsciiFacets, StructuredAsciiRepetition,
    StructuredAsciiSegment,
};
use ams_gra_oms_ir::{ConstraintSet, PatternDialect};
use std::sync::OnceLock;

pub type AsciiBranch = Vec<StructuredAsciiSegment>;

/// Rendering-only factorization, proved from semantic bodies, never names/text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelimitedAsciiProduct {
    pub delimiter: u8,
    pub components: usize,
    pub alternatives: Vec<AsciiBranch>,
}

/// Recognize an exact Cartesian product of identical delimiter-free unions.
/// Missing/extra combinations decline optimization rather than widen acceptance.
#[must_use]
pub fn factor_delimited_ascii(branches: &[AsciiBranch]) -> Option<DelimitedAsciiProduct> {
    let first = branches.first()?;
    let delimiter = first.iter().find_map(|segment| match segment {
        StructuredAsciiSegment::Literal(text) if text.len() == 1 => Some(text.as_bytes()[0]),
        _ => None,
    })?;
    let split = |branch: &AsciiBranch| -> Option<Vec<AsciiBranch>> {
        let pieces: Vec<_> = branch.split(|segment| matches!(segment, StructuredAsciiSegment::Literal(text) if text.as_bytes() == [delimiter])).map(<[StructuredAsciiSegment]>::to_vec).collect();
        if pieces.len() < 2 || pieces.iter().any(|piece| piece.is_empty()) {
            return None;
        }
        for piece in &pieces {
            for segment in piece {
                match segment {
                    StructuredAsciiSegment::Literal(text)
                        if text.as_bytes().contains(&delimiter) =>
                    {
                        return None;
                    }
                    StructuredAsciiSegment::Class(alphabet, _)
                        if alphabet
                            .ranges()
                            .iter()
                            .any(|&(a, b)| (a..=b).contains(&delimiter)) =>
                    {
                        return None;
                    }
                    _ => {}
                }
            }
        }
        Some(pieces)
    };
    let components = split(first)?.len();
    let mut alternatives = Vec::new();
    let mut tuples = Vec::new();
    for branch in branches {
        let pieces = split(branch)?;
        if pieces.len() != components {
            return None;
        }
        let mut tuple = Vec::new();
        for piece in pieces {
            let index = alternatives
                .iter()
                .position(|old| *old == piece)
                .unwrap_or_else(|| {
                    alternatives.push(piece);
                    alternatives.len() - 1
                });
            tuple.push(index);
        }
        tuples.push(tuple);
    }
    tuples.sort();
    tuples.dedup();
    if tuples.len()
        != alternatives
            .len()
            .checked_pow(u32::try_from(components).ok()?)?
    {
        return None;
    }
    Some(DelimitedAsciiProduct {
        delimiter,
        components,
        alternatives,
    })
}

/// All groups must accept; within each group any whole-value branch may accept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlternatingAsciiProfile {
    pub groups: Vec<Vec<AsciiBranch>>,
    pub facets: StructuredAsciiFacets,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlternatingAsciiRow {
    pub patterns: &'static [&'static str],
    pub profile: AlternatingAsciiProfile,
}

impl AlternatingAsciiProfile {
    /// Reference semantic oracle for compiler-backed conformance tests.
    /// Each branch gets an independent cursor and consumes the entire input.
    #[must_use]
    pub fn accepts_pattern(&self, text: &[u8]) -> bool {
        self.groups
            .iter()
            .all(|group| group.iter().any(|branch| accepts_branch(branch, text)))
    }

    #[must_use]
    pub fn accepts(&self, text: &[u8]) -> bool {
        let length = text.len() as u64;
        let facets = match self.facets {
            StructuredAsciiFacets::Exact(n) => length == n,
            StructuredAsciiFacets::Range(a, b) => (a..=b).contains(&length),
            StructuredAsciiFacets::Max(n) => length <= n,
        };
        facets && self.accepts_pattern(text)
    }
}

fn accepts_branch(branch: &[StructuredAsciiSegment], text: &[u8]) -> bool {
    let mut pos = 0;
    for segment in branch {
        match segment {
            StructuredAsciiSegment::Literal(literal) => {
                if !text
                    .get(pos..)
                    .is_some_and(|rest| rest.starts_with(literal.as_bytes()))
                {
                    return false;
                }
                pos += literal.len();
            }
            StructuredAsciiSegment::Class(alphabet, repetition) => {
                let count = match repetition {
                    StructuredAsciiRepetition::One => 1,
                    StructuredAsciiRepetition::Exact(n) => usize::from(*n),
                    // Frozen Task 060 bodies expand optional/bounded repetitions
                    // into fixed deterministic bodies, never ambiguous partitions.
                    _ => return false,
                };
                let Some(part) = text.get(pos..pos + count) else {
                    return false;
                };
                if !part.iter().all(|byte| {
                    alphabet
                        .ranges()
                        .iter()
                        .any(|&(a, b)| (a..=b).contains(byte))
                }) {
                    return false;
                }
                pos += count;
            }
        }
    }
    pos == text.len()
}

/// Exact name-free classification, preserving normalized ordering and grouping.
#[must_use]
pub fn alternating_ascii_profile(c: &ConstraintSet) -> Option<&'static AlternatingAsciiProfile> {
    if c.lexical.white_space.is_some()
        || c.min_inclusive.is_some()
        || c.max_inclusive.is_some()
        || c.min_exclusive.is_some()
        || c.max_exclusive.is_some()
        || c.lexical.pattern_groups.len() != 1
    {
        return None;
    }
    let facets = match (c.length, c.min_length, c.max_length) {
        (Some(n), None, None) => StructuredAsciiFacets::Exact(n),
        (None, Some(a), Some(b)) => StructuredAsciiFacets::Range(a, b),
        _ => return None,
    };
    alternating_ascii_rows()
        .iter()
        .find(|row| {
            row.profile.facets == facets
                && c.lexical.pattern_groups[0].alternatives.len() == row.patterns.len()
                && c.lexical.pattern_groups[0]
                    .alternatives
                    .iter()
                    .zip(row.patterns)
                    .all(|(p, expected)| {
                        p.dialect == PatternDialect::XmlSchema && p.expression == *expected
                    })
        })
        .map(|row| &row.profile)
}

fn class(ranges: &'static [(u8, u8)], n: u8) -> StructuredAsciiSegment {
    StructuredAsciiSegment::Class(
        StructuredAsciiAlphabet::Finite(ranges),
        StructuredAsciiRepetition::Exact(n),
    )
}
fn literal(text: &'static str) -> StructuredAsciiSegment {
    StructuredAsciiSegment::Literal(text)
}
const DIGIT: &[(u8, u8)] = &[(b'0', b'9')];
const UPPER: &[(u8, u8)] = &[(b'A', b'Z')];
const SPACE: &[(u8, u8)] = &[(b' ', b' ')];
const NONZERO: &[(u8, u8)] = &[(b'1', b'9')];

fn military(optional: bool) -> Vec<AsciiBranch> {
    let zones = [
        vec![class(NONZERO, 1)],
        vec![class(&[(b'1', b'5')], 1), class(DIGIT, 1)],
        vec![literal("60")],
    ];
    let latitude = class(&[(b'C', b'H'), (b'J', b'N'), (b'P', b'X')], 1);
    let first = class(&[(b'A', b'H'), (b'J', b'N'), (b'P', b'Z')], 1);
    let second = class(&[(b'A', b'H'), (b'J', b'N'), (b'P', b'V')], 1);
    let mut branches = Vec::new();
    for mut zone in zones {
        zone.push(latitude);
        if optional {
            branches.push(zone.clone());
        }
        zone.extend([first, second]);
        for pairs in 0..=5 {
            let mut branch = zone.clone();
            if pairs > 0 {
                branch.push(class(DIGIT, 2 * pairs));
            }
            branches.push(branch);
        }
    }
    let polar = class(&[(b'A', b'B'), (b'Y', b'Z')], 1);
    if optional {
        branches.push(vec![polar]);
    }
    for pairs in 0..=5 {
        let mut branch = vec![
            polar,
            class(
                &[
                    (b'A', b'C'),
                    (b'F', b'H'),
                    (b'J', b'L'),
                    (b'P', b'U'),
                    (b'X', b'Z'),
                ],
                1,
            ),
            first,
        ];
        if pairs > 0 {
            branch.push(class(DIGIT, 2 * pairs));
        }
        branches.push(branch);
    }
    branches
}

fn ipv4() -> Vec<AsciiBranch> {
    let octets = [
        vec![class(DIGIT, 1)],
        vec![class(NONZERO, 1), class(DIGIT, 1)],
        vec![literal("1"), class(DIGIT, 2)],
        vec![literal("2"), class(&[(b'0', b'4')], 1), class(DIGIT, 1)],
        vec![literal("25"), class(&[(b'0', b'5')], 1)],
    ];
    let mut branches = vec![Vec::new()];
    for index in 0..4 {
        branches = branches
            .into_iter()
            .flat_map(|prefix| {
                octets.iter().map(move |octet| {
                    let mut branch = prefix.clone();
                    if index > 0 {
                        branch.push(literal("."));
                    }
                    branch.extend(octet);
                    branch
                })
            })
            .collect();
    }
    branches
}

fn padded_pairs(width: u8, pairs: u8) -> AsciiBranch {
    let mut branch = Vec::new();
    for _ in 0..pairs {
        branch.extend([class(UPPER, 2), literal(" ")]);
    }
    let padding = width - 3 * pairs;
    if padding > 0 {
        branch.push(class(SPACE, padding));
    }
    branch
}

/// Sixteen frozen expression/facet rows. Regex text is identity evidence only.
pub fn alternating_ascii_rows() -> &'static [AlternatingAsciiRow] {
    static ROWS: OnceLock<Vec<AlternatingAsciiRow>> = OnceLock::new();
    ROWS.get_or_init(build_rows)
}

fn build_rows() -> Vec<AlternatingAsciiRow> {
    let mut rows = Vec::new();
    let mut add = |patterns: &'static [&'static str], facets, branches| {
        rows.push(AlternatingAsciiRow {
            patterns,
            profile: AlternatingAsciiProfile {
                groups: vec![branches],
                facets,
            },
        })
    };
    use StructuredAsciiFacets::{Exact, Range};
    // Frozen row definitions follow; each body is authored semantically.
    add(
        &["[A-Z]{2}|[ ]{2}"],
        Exact(2),
        vec![vec![class(UPPER, 2)], vec![class(SPACE, 2)]],
    );
    add(
        &[
            "(([0-9]|[1-9][0-9]|1[0-9][0-9]|2[0-4][0-9]|25[0-5])\\.){3}([0-9]|[1-9][0-9]|1[0-9][0-9]|2[0-4][0-9]|25[0-5])",
        ],
        Range(7, 15),
        ipv4(),
    );
    add(
        &[
            "([1-9]|[1-5][0-9]|60)[C-HJ-NP-X]([A-HJ-NP-Z][A-HJ-NP-V]([0-9]{2}){0,5})?|[ABYZ]([A-CF-HJ-LP-UX-Z][A-HJ-NP-Z]([0-9]{2}){0,5})?",
        ],
        Range(14, 15),
        military(true),
    );
    add(
        &["((((U0)|[A-Z]{2})[0-9]{2})|UNKN)"],
        Exact(4),
        vec![
            vec![literal("U0"), class(DIGIT, 2)],
            vec![class(UPPER, 2), class(DIGIT, 2)],
            vec![literal("UNKN")],
        ],
    );
    add(
        &[
            " {11}",
            "([A-Z]{2} ){1} {8}",
            "([A-Z]{2} ){2} {5}",
            "([A-Z]{2} ){3} {2}",
            "([A-Z]{2} ){3}[A-Z]{2}",
        ],
        Exact(11),
        {
            let mut b = (0..=3).map(|n| padded_pairs(11, n)).collect::<Vec<_>>();
            let mut last = padded_pairs(9, 3);
            last.push(class(UPPER, 2));
            b.push(last);
            b
        },
    );
    add(
        &["(X[1-8]( ){2})|25X[1-9]|[DNIO]|( ){4}"],
        Exact(4),
        vec![
            vec![literal("X"), class(&[(b'1', b'8')], 1), class(SPACE, 2)],
            vec![literal("25X"), class(NONZERO, 1)],
            vec![class(&[(b'D', b'D'), (b'I', b'I'), (b'N', b'O')], 1)],
            vec![class(SPACE, 4)],
        ],
    );
    add(
        &["(DD|DE|GD|GE|O( )|X( )|( ){2})"],
        Exact(2),
        ["DD", "DE", "GD", "GE", "O ", "X ", "  "]
            .into_iter()
            .map(|s| vec![literal(s)])
            .collect(),
    );
    add(
        &["([0-9][A-Z]|[A-Z][0-9])"],
        Exact(2),
        vec![
            vec![class(DIGIT, 1), class(UPPER, 1)],
            vec![class(UPPER, 1), class(DIGIT, 1)],
        ],
    );
    add(
        &["[1-9][0-9]{4}|[ ]{5}"],
        Exact(5),
        vec![
            vec![class(NONZERO, 1), class(DIGIT, 4)],
            vec![class(SPACE, 5)],
        ],
    );
    add(
        &["[0-9][1-9][0-9]|[0-9]{2}[1-9]|[1-9][0-9]{2}"],
        Exact(3),
        vec![
            vec![class(DIGIT, 1), class(NONZERO, 1), class(DIGIT, 1)],
            vec![class(DIGIT, 2), class(NONZERO, 1)],
            vec![class(NONZERO, 1), class(DIGIT, 2)],
        ],
    );
    add(
        &["([3][1-3]\\.[0-9]{4})|( {7})"],
        Exact(7),
        vec![
            vec![
                literal("3"),
                class(&[(b'1', b'3')], 1),
                literal("."),
                class(DIGIT, 4),
            ],
            vec![class(SPACE, 7)],
        ],
    );
    add(
        &[
            " {20}",
            "([A-Z]{2} ){1} {17}",
            "([A-Z]{2} ){2} {14}",
            "([A-Z]{2} ){3} {11}",
            "([A-Z]{2} ){4} {8}",
            "([A-Z]{2} ){5} {5}",
            "([A-Z]{2} ){6} {2}",
            "([A-Z]{2} ){6}[A-Z]{2}",
        ],
        Exact(20),
        {
            let mut b = (0..=6).map(|n| padded_pairs(20, n)).collect::<Vec<_>>();
            let mut last = padded_pairs(18, 6);
            last.push(class(UPPER, 2));
            b.push(last);
            b
        },
    );
    add(
        &["(([01][0-9]|[2][0-3])([0-5][0-9])([0-5][0-9])Z)", "( ){7}"],
        Exact(7),
        vec![
            vec![
                class(&[(b'0', b'1')], 1),
                class(DIGIT, 1),
                class(&[(b'0', b'5')], 1),
                class(DIGIT, 1),
                class(&[(b'0', b'5')], 1),
                class(DIGIT, 1),
                literal("Z"),
            ],
            vec![
                literal("2"),
                class(&[(b'0', b'3')], 1),
                class(&[(b'0', b'5')], 1),
                class(DIGIT, 1),
                class(&[(b'0', b'5')], 1),
                class(DIGIT, 1),
                literal("Z"),
            ],
            vec![class(SPACE, 7)],
        ],
    );
    add(
        &["[A-Z0-9]{5}|UNKN|NONE"],
        Range(4, 5),
        vec![
            vec![class(&[(b'0', b'9'), (b'A', b'Z')], 5)],
            vec![literal("UNKN")],
            vec![literal("NONE")],
        ],
    );
    add(
        &["[A-Z][A-Z]|[E]|[\\-]"],
        Range(1, 2),
        vec![
            vec![class(UPPER, 2)],
            vec![literal("E")],
            vec![literal("-")],
        ],
    );
    add(
        &[
            "(([1-9]|[1-5][0-9]|60)[C-HJ-NP-X][A-HJ-NP-Z][A-HJ-NP-V]|[ABYZ][A-CF-HJ-LP-UX-Z][A-HJ-NP-Z])(([0-9]{2}){0,5})",
        ],
        Range(3, 15),
        military(false),
    );
    rows
}
