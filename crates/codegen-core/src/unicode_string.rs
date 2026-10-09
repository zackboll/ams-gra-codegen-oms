//! Task066 exact String languages; Unicode 3.1.0 is a fixed project policy.
//! No regex parser, calendar/coordinate interpretation, or host Unicode oracle.
use ams_gra_oms_ir::{ConstraintSet, LexicalConstraintSet, PatternExpression, PatternGroup};

/// UnicodeData-3.1.0.txt General_Category=Nd, merged contiguous ranges.
/// Source: https://www.unicode.org/Public/3.1-Update/UnicodeData-3.1.0.txt
/// SHA256: 8e57884da0da3a66782b8a6332a601fccbcae9ed0060e12501aca02fa56ffecd
pub const UNICODE31_ND: &[(u32, u32)] = &[
    (0x0030, 0x0039),
    (0x0660, 0x0669),
    (0x06F0, 0x06F9),
    (0x0966, 0x096F),
    (0x09E6, 0x09EF),
    (0x0A66, 0x0A6F),
    (0x0AE6, 0x0AEF),
    (0x0B66, 0x0B6F),
    (0x0BE7, 0x0BEF),
    (0x0C66, 0x0C6F),
    (0x0CE6, 0x0CEF),
    (0x0D66, 0x0D6F),
    (0x0E50, 0x0E59),
    (0x0ED0, 0x0ED9),
    (0x0F20, 0x0F29),
    (0x1040, 0x1049),
    (0x1369, 0x1371),
    (0x17E0, 0x17E9),
    (0x1810, 0x1819),
    (0xFF10, 0xFF19),
    (0x1D7CE, 0x1D7FF),
];

/// Frozen decimal-digit membership, deliberately not `char::is_numeric`.
#[must_use]
pub fn unicode31_decimal_digit(scalar: u32) -> bool {
    UNICODE31_ND
        .iter()
        .any(|&(first, last)| (first..=last).contains(&scalar))
}

/// Exact profiles, identified by facets rather than declaration names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnicodeStringProfile {
    DateAndTime,
    Date,
    TargetLocation,
}

/// Only the semantic tokens needed by these three authored languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnicodeStringToken {
    /// An explicit authored ASCII class or literal.
    Ascii(&'static str),
    /// XML Schema `\d` under the authorized Unicode 3.1 policy.
    DecimalDigit,
}

impl UnicodeStringProfile {
    pub const ALL: [Self; 3] = [Self::DateAndTime, Self::Date, Self::TargetLocation];

    #[must_use]
    pub const fn id(self) -> u8 {
        match self {
            Self::DateAndTime => 0,
            Self::Date => 1,
            Self::TargetLocation => 2,
        }
    }

    #[must_use]
    pub const fn length(self) -> usize {
        match self {
            Self::DateAndTime => 12,
            Self::Date => 8,
            Self::TargetLocation => 21,
        }
    }

    /// Complete effective constraints, including absent facets and alternative order.
    #[must_use]
    pub fn constraints(self) -> ConstraintSet {
        let alternatives = match self {
            Self::DateAndTime => [
                r"(([12]\d\d\d)((0[1-9])|(1[012]))(0[1-9]|[12][0-9]|3[01])([01][0-9]|[2][0-3])([0-5][0-9]))",
                " {12}",
            ],
            Self::Date => [
                r"([12]\d\d\d((0[1-9])|(1[012]))(0[1-9]|[12][0-9]|3[01]))",
                " {8}",
            ],
            Self::TargetLocation => [
                r"([0-8]\d([0-5]\d){2}\.\d{2}(N|S)(0\d{2}|1[0-7]\d)([0-5]\d){2}\.\d{2}(E|W))",
                r"([\+\-]{1}[0-8]\d\.\d{6}[\+\-]{1}(0\d{2}|1[0-7]\d)\.\d{6})",
            ],
        };
        ConstraintSet {
            length: Some(self.length() as u64),
            lexical: LexicalConstraintSet {
                pattern_groups: vec![PatternGroup {
                    alternatives: alternatives
                        .into_iter()
                        .map(PatternExpression::xml_schema)
                        .collect(),
                }],
                white_space: None,
            },
            ..ConstraintSet::default()
        }
    }

    /// Finite authored alternatives expanded as fixed scalar sequences.
    #[must_use]
    pub fn branches(self) -> Vec<Vec<UnicodeStringToken>> {
        use UnicodeStringToken::{Ascii as A, DecimalDigit as D};
        const DIGITS: &str = "0123456789";
        let mut result = Vec::new();
        if self == Self::TargetLocation {
            for longitude in [[A("0"), D, D], [A("1"), A("01234567"), D]] {
                let mut sexagesimal = vec![
                    A("012345678"),
                    D,
                    A("012345"),
                    D,
                    A("012345"),
                    D,
                    A("."),
                    D,
                    D,
                    A("NS"),
                ];
                sexagesimal.extend(longitude);
                sexagesimal.extend([A("012345"), D, A("012345"), D, A("."), D, D, A("EW")]);
                result.push(sexagesimal);
                let mut decimal = vec![A("+-"), A("012345678"), D, A(".")];
                decimal.extend([D; 6]);
                decimal.push(A("+-"));
                decimal.extend(longitude);
                decimal.push(A("."));
                decimal.extend([D; 6]);
                result.push(decimal);
            }
        } else {
            for month in [[A("0"), A("123456789")], [A("1"), A("012")]] {
                for day in [
                    [A("0"), A("123456789")],
                    [A("12"), A(DIGITS)],
                    [A("3"), A("01")],
                ] {
                    let mut date = vec![A("12"), D, D, D];
                    date.extend(month);
                    date.extend(day);
                    if self == Self::Date {
                        result.push(date);
                    } else {
                        for hour in [[A("01"), A(DIGITS)], [A("2"), A("0123")]] {
                            let mut time = date.clone();
                            time.extend(hour);
                            time.extend([A("012345"), A(DIGITS)]);
                            result.push(time);
                        }
                    }
                }
            }
            result.push(vec![A(" "); self.length()]);
        }
        result
    }

    /// Validate valid UTF-8 text, counting scalars, preserving lexical spelling.
    #[must_use]
    pub fn accepts(self, text: &str) -> bool {
        let scalars: Vec<_> = text.chars().map(u32::from).collect();
        scalars.len() == self.length()
            && self.branches().iter().any(|branch| {
                branch.iter().zip(&scalars).all(|(token, &c)| match token {
                    UnicodeStringToken::Ascii(set) => set.bytes().any(|b| u32::from(b) == c),
                    UnicodeStringToken::DecimalDigit => unicode31_decimal_digit(c),
                })
            })
    }
}

/// One shared emission predicate, also used by generated-name preflight.
#[must_use]
pub fn schema_emits_unicode_string(schema: &ams_gra_oms_ir::SchemaIr) -> bool {
    schema.types.iter().any(|d| {
        matches!(
            d.kind,
            ams_gra_oms_ir::TypeKind::Primitive(ams_gra_oms_ir::PrimitiveKind::String)
        ) && UnicodeStringProfile::ALL
            .iter()
            .any(|p| d.constraints == p.constraints())
    })
}
