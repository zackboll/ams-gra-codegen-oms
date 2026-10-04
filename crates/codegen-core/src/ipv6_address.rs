//! Exact factored language of the pinned UCI IPv6 String expression.
//! No RFC address interpretation, regex engine, normalization, or host parser.

use ams_gra_oms_ir::{ConstraintSet, LexicalConstraintSet, PatternExpression, PatternGroup};

/// Exact normalized expression read independently from both pinned roots.
pub const UCI_IPV6_PATTERN: &str = r"((:|[0-9a-fA-F]{0,4}):)([0-9a-fA-F]{0,4}:){0,5}((([0-9a-fA-F]{0,4}:)?(:|[0-9a-fA-F]{0,4}))|(((25[0-5]|2[0-4][0-9]|[01]?[0-9]?[0-9])\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9]?[0-9])))";

/// Exact effective facets, including absence of additional authored facets.
#[must_use]
pub fn ipv6_address_constraints() -> ConstraintSet {
    ConstraintSet {
        min_length: Some(2),
        max_length: Some(45),
        lexical: LexicalConstraintSet {
            pattern_groups: vec![PatternGroup {
                alternatives: vec![PatternExpression::xml_schema(UCI_IPV6_PATTERN)],
            }],
            white_space: None,
        },
        ..ConstraintSet::default()
    }
}

/// Language-neutral bounded scanner model. Every offset is within 0..=45.
pub struct Ipv6AddressModel;

impl Ipv6AddressModel {
    /// Validate the full lexical spelling, including independent length facets.
    #[must_use]
    pub fn accepts(text: &[u8]) -> bool {
        if !(2..=45).contains(&text.len()) {
            return false;
        }
        // Prefix (C | H) C: both alternatives matter when text starts with ::.
        Self::hex_colon(text, 0).is_some_and(|end| Self::after_prefix(text, end))
            || (text.starts_with(b"::") && Self::after_prefix(text, 2))
    }

    fn hex_colon(text: &[u8], start: usize) -> Option<usize> {
        let mut end = start;
        while end < text.len() && text[end].is_ascii_hexdigit() {
            end += 1;
        }
        (end - start <= 4 && text.get(end) == Some(&b':')).then_some(end + 1)
    }

    fn after_prefix(text: &[u8], mut start: usize) -> bool {
        for count in 0..=5 {
            if Self::suffix(&text[start..]) {
                return true;
            }
            if count == 5 {
                break;
            }
            let Some(end) = Self::hex_colon(text, start) else {
                break;
            };
            start = end;
        }
        false
    }

    fn hex(text: &[u8]) -> bool {
        text.len() <= 4 && text.iter().all(u8::is_ascii_hexdigit)
    }

    fn suffix(text: &[u8]) -> bool {
        if Self::hex(text) || text == b":" {
            return true;
        }
        if let Some(end) = Self::hex_colon(text, 0) {
            let tail = &text[end..];
            if Self::hex(tail) || tail == b":" {
                return true;
            }
        }
        Self::embedded_ipv4(text)
    }

    fn embedded_ipv4(text: &[u8]) -> bool {
        let mut count = 0;
        for octet in text.split(|b| *b == b'.') {
            if octet.is_empty() || octet.len() > 3 || !octet.iter().all(u8::is_ascii_digit) {
                return false;
            }
            let value = octet
                .iter()
                .fold(0_u16, |n, b| n * 10 + u16::from(b - b'0'));
            if value > 255 {
                return false;
            }
            count += 1;
        }
        count == 4
    }
}

#[cfg(test)]
#[path = "../../../tests/task062_cases.rs"]
mod corpus;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_ipv6_semantics_match_independent_partition_corpus() {
        let cases = corpus::cases();
        assert!(cases.len() > 20_000);
        for (text, expected) in &cases {
            assert_eq!(
                corpus::independent_accepts(text),
                *expected,
                "oracle {text:?}"
            );
            assert_eq!(
                Ipv6AddressModel::accepts(text.as_bytes()),
                *expected,
                "model {text:?}"
            );
        }
        println!(
            "TASK062 INDEPENDENT SEMANTIC CORPUS: PASSED ({} cases)",
            cases.len()
        );
    }

    #[test]
    fn exact_ipv6_profile_and_all_facet_neighbors_fail_closed() {
        use crate::{StringProfile, string_profile};
        use ams_gra_oms_ir::{PrimitiveKind, WhiteSpacePolicy};
        let exact = ipv6_address_constraints();
        assert_eq!(
            string_profile(PrimitiveKind::String, &exact),
            Ok(Some(StringProfile::Ipv6Address))
        );
        let mut neighbors = Vec::new();
        for min in [None, Some(1), Some(3)] {
            let mut c = exact.clone();
            c.min_length = min;
            neighbors.push(c);
        }
        for max in [None, Some(44), Some(46)] {
            let mut c = exact.clone();
            c.max_length = max;
            neighbors.push(c);
        }
        for white in [
            WhiteSpacePolicy::Preserve,
            WhiteSpacePolicy::Replace,
            WhiteSpacePolicy::Collapse,
        ] {
            let mut c = exact.clone();
            c.lexical.white_space = Some(white);
            neighbors.push(c);
        }
        let mut c = exact.clone();
        c.length = Some(2);
        neighbors.push(c);
        let mut c = exact.clone();
        c.min_inclusive = Some(ams_gra_oms_ir::NumericValue::Integer(0));
        neighbors.push(c);
        let mut c = exact.clone();
        c.lexical.pattern_groups[0].alternatives[0]
            .expression
            .push('|');
        neighbors.push(c);
        let mut c = exact.clone();
        c.lexical.pattern_groups[0]
            .alternatives
            .push(PatternExpression::xml_schema(".*"));
        neighbors.push(c);
        let mut c = exact.clone();
        c.lexical
            .pattern_groups
            .push(c.lexical.pattern_groups[0].clone());
        neighbors.push(c);
        for c in neighbors {
            assert!(string_profile(PrimitiveKind::String, &c).is_err(), "{c:?}");
        }
        assert_eq!(
            string_profile(PrimitiveKind::String, &ConstraintSet::default()),
            Ok(None)
        );
        assert_eq!(string_profile(PrimitiveKind::Boolean, &exact), Ok(None));
        // PatternDialect currently has only XmlSchema; foreign dialects cannot
        // be constructed in normalized IR and therefore cannot be admitted.
    }
}
