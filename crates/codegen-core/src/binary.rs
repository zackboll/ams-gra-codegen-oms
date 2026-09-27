//! Task 053: the ONE language-neutral Binary value-domain classifier.
//!
//! A Binary VALUE is an owned sequence of octets (Task 025). XML Schema 2E
//! 4.3.1-4.3.3 measure `length`, `minLength` and `maxLength` of `hexBinary`,
//! `base64Binary` and their restrictions in **octets**, so a length-only
//! restriction is fully described by an inclusive octet-count interval. That
//! interval is computed from the EFFECTIVE `ConstraintSet` the frontend already
//! intersected along the restriction ancestry; nothing here walks ancestry or
//! reparses XSD.
//!
//! Three dimensions stay independent:
//!
//! * semantic value -- `PrimitiveKind::Binary` (octets);
//! * value-space constraint -- [`BinaryLengthDomain`] (this module);
//! * lexical provenance -- `ams_gra_oms_ir::declaration_binary_encoding`
//!   (Task 052).
//!
//! The classifier never consults lexical provenance, so a Binary declaration
//! with unknown provenance may still be MODEL renderable; only the codec
//! requires `xs:hexBinary`.
//!
//! Every backend, coverage/readiness, the Ada name model and the Rust codec
//! renderer consume this one function; there is no backend-specific reading
//! of Binary constraints.

use ams_gra_oms_ir::{ConstraintSet, LexicalConstraintSet, PrimitiveKind, WhiteSpacePolicy};
use std::fmt;

/// The legal octet counts of a length-constrained Binary value, inclusive.
///
/// `max_octets == None` means unbounded above.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryLengthDomain {
    pub min_octets: u64,
    pub max_octets: Option<u64>,
}

impl BinaryLengthDomain {
    /// Whether `octets` is a legal value length.
    #[must_use]
    pub fn contains(self, octets: u64) -> bool {
        octets >= self.min_octets && self.max_octets.is_none_or(|max| octets <= max)
    }

    /// The single legal length, when the domain is exact.
    #[must_use]
    pub fn exact(self) -> Option<u64> {
        (self.max_octets == Some(self.min_octets)).then_some(self.min_octets)
    }
}

/// Why a constraint set is outside the supported Binary value domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryConstraintError {
    /// The classifier was asked about a non-Binary primitive.
    NotBinary(PrimitiveKind),
    /// `minInclusive` / `maxInclusive` / `minExclusive` / `maxExclusive`.
    NumericFacet,
    /// `pattern` constrains the LEXICAL form, which stored octets cannot
    /// enforce.
    Pattern,
    /// An explicit `whiteSpace` other than Binary's fixed intrinsic collapse.
    WhiteSpace(WhiteSpacePolicy),
    /// Length facets admitting no octet count. The IR already rejects these;
    /// the classifier re-checks rather than trusts.
    ContradictoryLength,
}

impl fmt::Display for BinaryConstraintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotBinary(kind) => write!(formatter, "non-Binary primitive {kind:?}"),
            Self::NumericFacet => formatter.write_str("numeric Binary constraints"),
            Self::Pattern => formatter.write_str("Binary pattern constraints"),
            Self::WhiteSpace(policy) => {
                write!(formatter, "Binary whiteSpace {policy:?} constraints")
            }
            Self::ContradictoryLength => {
                formatter.write_str("contradictory Binary length constraints")
            }
        }
    }
}

/// Classify a Binary constraint set.
///
/// * `Ok(None)` -- unconstrained Binary (existing, byte-identical output);
/// * `Ok(Some(domain))` -- fully supported length-only constrained Binary;
/// * `Err(_)` -- at least one facet the octet-length domain cannot represent.
///
/// Every field of the `ConstraintSet` is consumed explicitly (exhaustive
/// destructuring), so a future facet cannot reach a backend unclassified.
///
/// # Errors
///
/// See [`BinaryConstraintError`].
pub fn binary_length_domain(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
) -> Result<Option<BinaryLengthDomain>, BinaryConstraintError> {
    if kind != PrimitiveKind::Binary {
        return Err(BinaryConstraintError::NotBinary(kind));
    }
    let ConstraintSet {
        min_inclusive,
        max_inclusive,
        min_exclusive,
        max_exclusive,
        length,
        min_length,
        max_length,
        lexical:
            LexicalConstraintSet {
                pattern_groups,
                white_space,
            },
    } = constraints;
    if min_inclusive.is_some()
        || max_inclusive.is_some()
        || min_exclusive.is_some()
        || max_exclusive.is_some()
    {
        return Err(BinaryConstraintError::NumericFacet);
    }
    if !pattern_groups.is_empty() {
        return Err(BinaryConstraintError::Pattern);
    }
    // XML Schema 2E 4.3.6: Binary's whiteSpace is fixed `collapse`. An
    // explicit `collapse` restates the intrinsic lexical policy and removes no
    // octet sequence from the value space, so it is accepted.
    match white_space {
        None => {}
        Some(policy) if *policy == PrimitiveKind::Binary.intrinsic_white_space_policy() => {}
        Some(policy) => return Err(BinaryConstraintError::WhiteSpace(*policy)),
    }
    if length.is_none() && min_length.is_none() && max_length.is_none() {
        return Ok(None);
    }
    // The effective minimum is the largest lower bound and the effective
    // maximum the smallest upper bound; every present facet participates.
    let min_octets = [*length, *min_length]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or(0);
    let max_octets = [*length, *max_length].into_iter().flatten().min();
    if max_octets.is_some_and(|max| max < min_octets)
        || length.is_some_and(|exact| exact != min_octets || Some(exact) != max_octets)
    {
        return Err(BinaryConstraintError::ContradictoryLength);
    }
    Ok(Some(BinaryLengthDomain {
        min_octets,
        max_octets,
    }))
}

/// Whether a NAMED Binary declaration emits a checked, validated carrier:
/// exactly `binary_length_domain(..) == Ok(Some(_))`.
#[must_use]
pub fn is_constrained_binary_carrier(kind: PrimitiveKind, constraints: &ConstraintSet) -> bool {
    kind == PrimitiveKind::Binary
        && binary_length_domain(kind, constraints).is_ok_and(|domain| domain.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::{NumericValue, PatternExpression, PatternGroup};

    const BINARY: PrimitiveKind = PrimitiveKind::Binary;

    fn lengths(length: Option<u64>, min: Option<u64>, max: Option<u64>) -> ConstraintSet {
        ConstraintSet {
            length,
            min_length: min,
            max_length: max,
            ..ConstraintSet::default()
        }
    }

    fn domain(min_octets: u64, max_octets: Option<u64>) -> Option<BinaryLengthDomain> {
        Some(BinaryLengthDomain {
            min_octets,
            max_octets,
        })
    }

    fn with_pattern(mut constraints: ConstraintSet) -> ConstraintSet {
        constraints.lexical.pattern_groups.push(PatternGroup {
            alternatives: vec![PatternExpression::xml_schema("[0-9A-F]*")],
        });
        constraints
    }

    #[test]
    fn task053_unconstrained_binary_is_none() {
        assert_eq!(
            binary_length_domain(BINARY, &ConstraintSet::default()),
            Ok(None)
        );
    }

    #[test]
    fn task053_length_shapes_normalize_to_octet_intervals() {
        for (constraints, expected) in [
            (lengths(Some(4), None, None), domain(4, Some(4))),
            (lengths(None, Some(2), Some(6)), domain(2, Some(6))),
            (lengths(None, Some(2), None), domain(2, None)),
            (lengths(None, None, Some(6)), domain(0, Some(6))),
            (lengths(Some(0), None, None), domain(0, Some(0))),
            // A derived effective set may keep compatible inherited bounds.
            (lengths(Some(4), Some(2), Some(6)), domain(4, Some(4))),
            (lengths(None, Some(u64::MAX), None), domain(u64::MAX, None)),
        ] {
            assert_eq!(binary_length_domain(BINARY, &constraints), Ok(expected));
        }
    }

    #[test]
    fn task053_explicit_collapse_is_the_intrinsic_policy() {
        let mut exact = lengths(Some(4), None, None);
        exact.lexical.white_space = Some(WhiteSpacePolicy::Collapse);
        assert_eq!(binary_length_domain(BINARY, &exact), Ok(domain(4, Some(4))));
        let mut bare = ConstraintSet::default();
        bare.lexical.white_space = Some(WhiteSpacePolicy::Collapse);
        assert_eq!(binary_length_domain(BINARY, &bare), Ok(None));
    }

    #[test]
    fn task053_unsupported_facets_are_rejected() {
        for policy in [WhiteSpacePolicy::Preserve, WhiteSpacePolicy::Replace] {
            let mut constraints = lengths(Some(4), None, None);
            constraints.lexical.white_space = Some(policy);
            assert_eq!(
                binary_length_domain(BINARY, &constraints),
                Err(BinaryConstraintError::WhiteSpace(policy))
            );
        }
        for constraints in [
            with_pattern(lengths(Some(4), None, None)),
            with_pattern(ConstraintSet::default()),
        ] {
            assert_eq!(
                binary_length_domain(BINARY, &constraints),
                Err(BinaryConstraintError::Pattern)
            );
        }
        let zero = Some(NumericValue::Integer(0));
        for constraints in [
            ConstraintSet {
                min_inclusive: zero,
                ..lengths(Some(4), None, None)
            },
            ConstraintSet {
                max_inclusive: zero,
                ..ConstraintSet::default()
            },
            ConstraintSet {
                min_exclusive: zero,
                ..ConstraintSet::default()
            },
            ConstraintSet {
                max_exclusive: zero,
                ..ConstraintSet::default()
            },
        ] {
            assert_eq!(
                binary_length_domain(BINARY, &constraints),
                Err(BinaryConstraintError::NumericFacet)
            );
        }
        for constraints in [
            lengths(None, Some(7), Some(6)),
            lengths(Some(4), Some(5), None),
            lengths(Some(4), None, Some(3)),
        ] {
            assert_eq!(
                binary_length_domain(BINARY, &constraints),
                Err(BinaryConstraintError::ContradictoryLength)
            );
        }
        assert_eq!(
            binary_length_domain(PrimitiveKind::String, &lengths(Some(4), None, None)),
            Err(BinaryConstraintError::NotBinary(PrimitiveKind::String))
        );
    }

    #[test]
    fn task053_domain_membership_is_inclusive() {
        let bounded = BinaryLengthDomain {
            min_octets: 2,
            max_octets: Some(6),
        };
        assert!(!bounded.contains(1));
        assert!(bounded.contains(2));
        assert!(bounded.contains(6));
        assert!(!bounded.contains(7));
        assert_eq!(bounded.exact(), None);
        assert!(
            BinaryLengthDomain {
                min_octets: 2,
                max_octets: None
            }
            .contains(u64::MAX)
        );
        assert_eq!(
            BinaryLengthDomain {
                min_octets: 4,
                max_octets: Some(4)
            }
            .exact(),
            Some(4)
        );
    }

    #[test]
    fn task053_carrier_predicate_matches_the_classifier() {
        assert!(!is_constrained_binary_carrier(
            BINARY,
            &ConstraintSet::default()
        ));
        assert!(is_constrained_binary_carrier(
            BINARY,
            &lengths(Some(0), None, None)
        ));
        assert!(!is_constrained_binary_carrier(
            PrimitiveKind::String,
            &lengths(Some(4), None, None)
        ));
        assert!(!is_constrained_binary_carrier(
            BINARY,
            &with_pattern(lengths(Some(4), None, None))
        ));
    }
}
