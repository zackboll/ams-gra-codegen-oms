//! Task 033 shared semantics for bound-only named floating restrictions.
//!
//! XML Schema expresses a constrained floating simple type as a named
//! restriction carrying numeric range facets. Task 015 already resolves those
//! facets through named restriction chains, so a backend never re-walks raw
//! XSD: [`ams_gra_oms_ir::TypeDecl::constraints`] is already the *effective*
//! domain. This module's single job is to decide whether that effective domain
//! is one the Task 033 backends can lower, and to hand back the bounds as
//! width-correct semantic values.
//!
//! The classification lives here, once, rather than three times in Ada, Rust,
//! and C++. Three independent readings of the same facet set is exactly how the
//! backends would drift into disagreeing about which declarations are
//! renderable -- and `CoverageAnalysis` would then be measuring a fourth
//! opinion. Every consumer asks [`floating_domain`].
//!
//! # What is supported
//!
//! Only `minInclusive`, `maxInclusive`, `minExclusive`, and `maxExclusive`, in
//! any one-sided, two-sided, or mixed inclusive/exclusive combination, on
//! [`PrimitiveKind::Float32`] and [`PrimitiveKind::Float64`]. Everything else a
//! [`ConstraintSet`] can carry -- `length`, `minLength`, `maxLength`, pattern
//! groups, `whiteSpace` -- fails closed. No constraint is ever silently
//! dropped: a facet this module cannot enforce is a rejection, not a warning.
//!
//! # Width is never coerced
//!
//! A `Float32` declaration accepts only [`NumericValue::Float32`] bounds and a
//! `Float64` declaration only [`NumericValue::Float64`]. A mismatched domain is
//! a defect in externally constructed IR and is rejected rather than converted:
//! widening a bound to `f64`, or narrowing one to `f32`, would move the
//! accepted set of a generated type. Nothing here passes through `i128`,
//! decimal, or string.
//!
//! # IEEE semantics are the language's, not ours
//!
//! The classifier reports bounds; it does not reinterpret comparison. Backends
//! emit ordinary `>=`/`>`/`<=`/`<` against these values, which gives the
//! correct IEEE-754 behaviour for free:
//!
//! * NaN compares false against every bound, so it is never a valid instance of
//!   a range-constrained type. It is not special-cased into acceptance, and
//!   unconstrained Task 022 floats still carry NaN unchanged.
//! * Infinities keep normal ordering. `+Infinity` satisfies a lower-only finite
//!   bound and fails an upper-only one; `-Infinity` is the mirror image; a
//!   two-sided finite range excludes both. Infinity is not rejected merely
//!   because a type is constrained.
//! * `+0.0` and `-0.0` compare equal, so `minInclusive = 0` admits both signs
//!   and `minExclusive = 0` rejects both.
//!
//! The bounds themselves must be finite. [`ams_gra_oms_ir::SchemaIr::validate`]
//! already rejects non-finite facets on the end-to-end path, but this module is
//! also reachable directly with hand-built IR, so it re-checks rather than
//! generating code from a NaN or infinite bound.

use ams_gra_oms_ir::{ConstraintSet, Float32Value, Float64Value, NumericValue, PrimitiveKind};

/// Whether a bound admits the bound value itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatingBoundKind {
    /// `minInclusive` / `maxInclusive`: the bound value is a legal instance.
    Inclusive,
    /// `minExclusive` / `maxExclusive`: the bound value is not a legal instance.
    Exclusive,
}

impl FloatingBoundKind {
    /// The comparison operator a lower bound of this kind lowers to.
    #[must_use]
    pub const fn lower_operator(self) -> &'static str {
        match self {
            Self::Inclusive => ">=",
            Self::Exclusive => ">",
        }
    }

    /// The comparison operator an upper bound of this kind lowers to.
    #[must_use]
    pub const fn upper_operator(self) -> &'static str {
        match self {
            Self::Inclusive => "<=",
            Self::Exclusive => "<",
        }
    }
}

/// One finite binary32 bound and its inclusivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Float32Bound {
    pub kind: FloatingBoundKind,
    pub value: Float32Value,
}

/// One finite binary64 bound and its inclusivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Float64Bound {
    pub kind: FloatingBoundKind,
    pub value: Float64Value,
}

/// A supported bound-only floating domain, at its declared width.
///
/// At least one bound is present: an all-`None` result is reported as
/// "unconstrained" by [`floating_domain`] returning `Ok(None)` instead, so a
/// backend never has to distinguish an empty domain from a missing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatingDomain {
    /// binary32, preserved as `f32` in every backend.
    Float32 {
        lower: Option<Float32Bound>,
        upper: Option<Float32Bound>,
    },
    /// binary64, preserved as `f64` in every backend.
    Float64 {
        lower: Option<Float64Bound>,
        upper: Option<Float64Bound>,
    },
}

impl FloatingDomain {
    /// Whether `value` satisfies this domain, using exactly the comparisons the
    /// backends emit.
    ///
    /// This is the host-side mirror of generated code, not a second
    /// implementation of the rule: it exists so the IEEE edge cases
    /// (NaN, one-sided infinity, signed zero) can be asserted in ordinary Rust
    /// tests, and so a caller that already has a value in hand can ask the same
    /// question without generating a type. `None` is returned when the value's
    /// width does not match the domain's, for the same reason a mismatched
    /// bound is rejected: widths are never coerced.
    #[must_use]
    pub fn accepts_f32(self, value: f32) -> Option<bool> {
        let Self::Float32 { lower, upper } = self else {
            return None;
        };
        let lower_ok = lower.is_none_or(|bound| match bound.kind {
            FloatingBoundKind::Inclusive => value >= bound.value.value(),
            FloatingBoundKind::Exclusive => value > bound.value.value(),
        });
        let upper_ok = upper.is_none_or(|bound| match bound.kind {
            FloatingBoundKind::Inclusive => value <= bound.value.value(),
            FloatingBoundKind::Exclusive => value < bound.value.value(),
        });
        Some(lower_ok && upper_ok)
    }

    /// Whether `value` satisfies this binary64 domain. See [`Self::accepts_f32`].
    #[must_use]
    pub fn accepts_f64(self, value: f64) -> Option<bool> {
        let Self::Float64 { lower, upper } = self else {
            return None;
        };
        let lower_ok = lower.is_none_or(|bound| match bound.kind {
            FloatingBoundKind::Inclusive => value >= bound.value.value(),
            FloatingBoundKind::Exclusive => value > bound.value.value(),
        });
        let upper_ok = upper.is_none_or(|bound| match bound.kind {
            FloatingBoundKind::Inclusive => value <= bound.value.value(),
            FloatingBoundKind::Exclusive => value < bound.value.value(),
        });
        Some(lower_ok && upper_ok)
    }
}

/// Classify a named floating declaration's effective constraints.
///
/// Returns `Ok(None)` for the default (unconstrained) [`ConstraintSet`], which
/// is what keeps Task 022 output byte-for-byte unchanged, and
/// `Ok(Some(domain))` for the supported bound-only subset.
///
/// # Errors
///
/// Returns a deterministic message when the declaration is not a Task 033
/// case: a non-floating primitive, a length or lexical facet, a bound whose
/// [`NumericValue`] domain does not match the declared width, a non-finite
/// bound, a lower bound above its upper bound, or an ambiguous same-side pair
/// (both `minInclusive` and `minExclusive`, or both `maxInclusive` and
/// `maxExclusive`). The ambiguous case is *not* resolved by preferring one
/// facet: the IR does not canonicalize it, authoritative UCI never produces it,
/// and guessing would silently change a generated type's accepted set.
pub fn floating_domain(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
) -> Result<Option<FloatingDomain>, &'static str> {
    if !matches!(kind, PrimitiveKind::Float32 | PrimitiveKind::Float64) {
        return Err("non-floating primitive");
    }
    if constraints.length.is_some()
        || constraints.min_length.is_some()
        || constraints.max_length.is_some()
    {
        return Err("length constraints on a floating declaration");
    }
    if constraints.lexical != Default::default() {
        return Err("lexical constraints on a floating declaration");
    }
    if constraints.min_inclusive.is_some() && constraints.min_exclusive.is_some() {
        return Err("ambiguous minInclusive and minExclusive lower bounds");
    }
    if constraints.max_inclusive.is_some() && constraints.max_exclusive.is_some() {
        return Err("ambiguous maxInclusive and maxExclusive upper bounds");
    }

    let lower = side(constraints.min_inclusive, constraints.min_exclusive);
    let upper = side(constraints.max_inclusive, constraints.max_exclusive);
    if lower.is_none() && upper.is_none() {
        return Ok(None);
    }

    match kind {
        PrimitiveKind::Float32 => {
            let lower = lower.map(float32_bound).transpose()?;
            let upper = upper.map(float32_bound).transpose()?;
            if let (Some(low), Some(high)) = (lower, upper)
                && low.value.value() > high.value.value()
            {
                return Err("contradictory floating bounds");
            }
            Ok(Some(FloatingDomain::Float32 { lower, upper }))
        }
        _ => {
            let lower = lower.map(float64_bound).transpose()?;
            let upper = upper.map(float64_bound).transpose()?;
            if let (Some(low), Some(high)) = (lower, upper)
                && low.value.value() > high.value.value()
            {
                return Err("contradictory floating bounds");
            }
            Ok(Some(FloatingDomain::Float64 { lower, upper }))
        }
    }
}

/// Collapse one side's inclusive/exclusive slots into at most one bound.
///
/// The caller has already rejected the both-present case, so this never has to
/// choose between them.
fn side(
    inclusive: Option<NumericValue>,
    exclusive: Option<NumericValue>,
) -> Option<(FloatingBoundKind, NumericValue)> {
    inclusive
        .map(|value| (FloatingBoundKind::Inclusive, value))
        .or_else(|| exclusive.map(|value| (FloatingBoundKind::Exclusive, value)))
}

fn float32_bound(
    (kind, value): (FloatingBoundKind, NumericValue),
) -> Result<Float32Bound, &'static str> {
    let NumericValue::Float32(value) = value else {
        return Err("Float32 declaration requires Float32 bounds");
    };
    if !value.value().is_finite() {
        return Err("non-finite floating bound");
    }
    Ok(Float32Bound { kind, value })
}

fn float64_bound(
    (kind, value): (FloatingBoundKind, NumericValue),
) -> Result<Float64Bound, &'static str> {
    let NumericValue::Float64(value) = value else {
        return Err("Float64 declaration requires Float64 bounds");
    };
    if !value.value().is_finite() {
        return Err("non-finite floating bound");
    }
    Ok(Float64Bound { kind, value })
}

/// Render a finite `f32` as a literal that round-trips to the same binary32.
///
/// The frontend stores the *semantic* value of a facet, not its original XML
/// spelling, so generated code cannot and must not try to reconstruct the
/// lexical text. What it must guarantee is that the literal it writes parses
/// back to bit-identical binary32. Rust's shortest round-trip `f32` formatting
/// provides that; the only adjustment is forcing a decimal point so the literal
/// is unambiguously floating in all three target languages.
#[must_use]
pub fn float32_literal(value: f32) -> String {
    debug_assert!(
        value.is_finite(),
        "bounds are checked finite before lowering"
    );
    decimalize(format!("{value:?}"))
}

/// Render a finite `f64` as a literal that round-trips to the same binary64.
#[must_use]
pub fn float64_literal(value: f64) -> String {
    debug_assert!(
        value.is_finite(),
        "bounds are checked finite before lowering"
    );
    decimalize(format!("{value:?}"))
}

/// Ensure a numeric literal reads as floating in Ada, Rust, and C++.
///
/// Rust's `{:?}` already emits a `.0` for integral values and may emit an
/// exponent such as `1e300`. Ada requires a digit on both sides of the point
/// and requires a point before an exponent, so `1e300` becomes `1.0e300`.
fn decimalize(text: String) -> String {
    if let Some(exponent) = text.find(['e', 'E']) {
        let (mantissa, suffix) = text.split_at(exponent);
        if mantissa.contains('.') {
            return text;
        }
        return format!("{mantissa}.0{suffix}");
    }
    if text.contains('.') {
        return text;
    }
    format!("{text}.0")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::{LexicalConstraintSet, PatternExpression, PatternGroup, WhiteSpacePolicy};

    fn f32_value(value: f32) -> NumericValue {
        NumericValue::Float32(Float32Value::from_value(value))
    }

    fn f64_value(value: f64) -> NumericValue {
        NumericValue::Float64(Float64Value::from_value(value))
    }

    /// The default constraint set is unconstrained at both widths, which is what
    /// preserves Task 022 output.
    #[test]
    fn default_constraints_are_unconstrained() {
        for kind in [PrimitiveKind::Float32, PrimitiveKind::Float64] {
            assert_eq!(floating_domain(kind, &ConstraintSet::default()), Ok(None));
        }
    }

    /// Every one-sided, two-sided, and mixed inclusive/exclusive shape the
    /// authoritative schema can produce classifies at its declared width.
    #[test]
    fn classifies_every_supported_bound_shape() {
        // AltitudeType's actual shape: Float64 lower-inclusive only.
        assert_eq!(
            floating_domain(
                PrimitiveKind::Float64,
                &ConstraintSet {
                    min_inclusive: Some(f64_value(-6_378_237.0)),
                    ..ConstraintSet::default()
                }
            ),
            Ok(Some(FloatingDomain::Float64 {
                lower: Some(Float64Bound {
                    kind: FloatingBoundKind::Inclusive,
                    value: Float64Value::from_value(-6_378_237.0),
                }),
                upper: None,
            }))
        );
        // DoublePositiveType's actual shape: the one authoritative exclusive.
        assert_eq!(
            floating_domain(
                PrimitiveKind::Float64,
                &ConstraintSet {
                    min_exclusive: Some(f64_value(0.0)),
                    ..ConstraintSet::default()
                }
            ),
            Ok(Some(FloatingDomain::Float64 {
                lower: Some(Float64Bound {
                    kind: FloatingBoundKind::Exclusive,
                    value: Float64Value::from_value(0.0),
                }),
                upper: None,
            }))
        );
        // Upper-only, at binary32.
        assert_eq!(
            floating_domain(
                PrimitiveKind::Float32,
                &ConstraintSet {
                    max_inclusive: Some(f32_value(1.0)),
                    ..ConstraintSet::default()
                }
            ),
            Ok(Some(FloatingDomain::Float32 {
                lower: None,
                upper: Some(Float32Bound {
                    kind: FloatingBoundKind::Inclusive,
                    value: Float32Value::from_value(1.0),
                }),
            }))
        );
        // Mixed two-sided: inclusive lower, exclusive upper.
        assert_eq!(
            floating_domain(
                PrimitiveKind::Float64,
                &ConstraintSet {
                    min_inclusive: Some(f64_value(0.0)),
                    max_exclusive: Some(f64_value(1.0)),
                    ..ConstraintSet::default()
                }
            ),
            Ok(Some(FloatingDomain::Float64 {
                lower: Some(Float64Bound {
                    kind: FloatingBoundKind::Inclusive,
                    value: Float64Value::from_value(0.0),
                }),
                upper: Some(Float64Bound {
                    kind: FloatingBoundKind::Exclusive,
                    value: Float64Value::from_value(1.0),
                }),
            }))
        );
    }

    /// An equal two-sided bound is a legal (single-value) domain, not a
    /// contradiction.
    #[test]
    fn equal_bounds_are_not_contradictory() {
        assert!(
            floating_domain(
                PrimitiveKind::Float64,
                &ConstraintSet {
                    min_inclusive: Some(f64_value(1.5)),
                    max_inclusive: Some(f64_value(1.5)),
                    ..ConstraintSet::default()
                }
            )
            .is_ok()
        );
    }

    /// Task 033 section 13: a same-side duplicate is ambiguous and fails
    /// closed rather than silently preferring one facet.
    #[test]
    fn rejects_ambiguous_same_side_bounds() {
        assert_eq!(
            floating_domain(
                PrimitiveKind::Float64,
                &ConstraintSet {
                    min_inclusive: Some(f64_value(0.0)),
                    min_exclusive: Some(f64_value(0.0)),
                    ..ConstraintSet::default()
                }
            ),
            Err("ambiguous minInclusive and minExclusive lower bounds")
        );
        assert_eq!(
            floating_domain(
                PrimitiveKind::Float64,
                &ConstraintSet {
                    max_inclusive: Some(f64_value(1.0)),
                    max_exclusive: Some(f64_value(1.0)),
                    ..ConstraintSet::default()
                }
            ),
            Err("ambiguous maxInclusive and maxExclusive upper bounds")
        );
    }

    /// Task 033 section 71: a bound in the wrong `NumericValue` domain is a
    /// defect, never a conversion.
    #[test]
    fn rejects_wrong_width_bounds() {
        assert_eq!(
            floating_domain(
                PrimitiveKind::Float32,
                &ConstraintSet {
                    min_inclusive: Some(f64_value(0.0)),
                    ..ConstraintSet::default()
                }
            ),
            Err("Float32 declaration requires Float32 bounds")
        );
        assert_eq!(
            floating_domain(
                PrimitiveKind::Float64,
                &ConstraintSet {
                    min_inclusive: Some(f32_value(0.0)),
                    ..ConstraintSet::default()
                }
            ),
            Err("Float64 declaration requires Float64 bounds")
        );
        // Integral bounds are equally wrong on a floating declaration.
        assert_eq!(
            floating_domain(
                PrimitiveKind::Float64,
                &ConstraintSet {
                    max_inclusive: Some(NumericValue::Integer(0)),
                    ..ConstraintSet::default()
                }
            ),
            Err("Float64 declaration requires Float64 bounds")
        );
    }

    /// Task 033 section 72: `SchemaIr::validate` rejects these earlier on the
    /// end-to-end path, but the helper is reachable directly and must not
    /// generate code from a non-finite bound.
    #[test]
    fn rejects_non_finite_bounds() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                floating_domain(
                    PrimitiveKind::Float64,
                    &ConstraintSet {
                        min_inclusive: Some(f64_value(value)),
                        ..ConstraintSet::default()
                    }
                ),
                Err("non-finite floating bound")
            );
        }
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                floating_domain(
                    PrimitiveKind::Float32,
                    &ConstraintSet {
                        max_exclusive: Some(f32_value(value)),
                        ..ConstraintSet::default()
                    }
                ),
                Err("non-finite floating bound")
            );
        }
    }

    /// Task 033 sections 34/35: no facet outside the numeric range subset is
    /// accepted, and none is silently discarded.
    #[test]
    fn rejects_length_and_lexical_constraints() {
        for constraints in [
            ConstraintSet {
                length: Some(4),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                min_length: Some(1),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                max_length: Some(8),
                ..ConstraintSet::default()
            },
        ] {
            assert_eq!(
                floating_domain(PrimitiveKind::Float64, &constraints),
                Err("length constraints on a floating declaration")
            );
        }
        for lexical in [
            LexicalConstraintSet {
                pattern_groups: vec![PatternGroup {
                    alternatives: vec![PatternExpression::xml_schema("[0-9]+\\.[0-9]+")],
                }],
                white_space: None,
            },
            LexicalConstraintSet {
                pattern_groups: Vec::new(),
                white_space: Some(WhiteSpacePolicy::Collapse),
            },
        ] {
            // A numeric bound alongside a lexical facet must not be partially
            // enforced: the whole declaration is rejected.
            assert_eq!(
                floating_domain(
                    PrimitiveKind::Float64,
                    &ConstraintSet {
                        min_inclusive: Some(f64_value(0.0)),
                        lexical,
                        ..ConstraintSet::default()
                    }
                ),
                Err("lexical constraints on a floating declaration")
            );
        }
    }

    #[test]
    fn rejects_contradictory_and_non_floating_input() {
        assert_eq!(
            floating_domain(
                PrimitiveKind::Float64,
                &ConstraintSet {
                    min_inclusive: Some(f64_value(1.0)),
                    max_inclusive: Some(f64_value(0.0)),
                    ..ConstraintSet::default()
                }
            ),
            Err("contradictory floating bounds")
        );
        assert_eq!(
            floating_domain(
                PrimitiveKind::Float32,
                &ConstraintSet {
                    min_inclusive: Some(f32_value(1.0)),
                    max_exclusive: Some(f32_value(0.0)),
                    ..ConstraintSet::default()
                }
            ),
            Err("contradictory floating bounds")
        );
        for kind in [
            PrimitiveKind::String,
            PrimitiveKind::SignedInteger,
            PrimitiveKind::Binary,
        ] {
            assert_eq!(
                floating_domain(kind, &ConstraintSet::default()),
                Err("non-floating primitive")
            );
        }
    }

    /// Task 033 section 18: the emitted literal must parse back to the same
    /// binary value at the same width. Original XSD spelling is explicitly not
    /// preserved, so round-trip is the whole contract.
    #[test]
    fn bound_literals_round_trip_at_their_width() {
        for value in [
            0.0f32,
            -0.0f32,
            1.0f32,
            -1.0f32,
            0.1f32,
            -6_378_237.0f32,
            f32::MIN,
            f32::MAX,
            f32::MIN_POSITIVE,
            std::f32::consts::PI,
        ] {
            let literal = float32_literal(value);
            let parsed: f32 = literal.parse().expect("literal must parse as f32");
            assert_eq!(
                parsed.to_bits(),
                value.to_bits(),
                "f32 literal {literal} must round-trip bit-identically"
            );
            assert!(
                literal.contains('.'),
                "f32 literal {literal} must read as floating in Ada"
            );
        }
        for value in [
            0.0f64,
            -0.0f64,
            1.0f64,
            0.1f64,
            -6_378_237.0f64,
            180.0f64,
            f64::MIN,
            f64::MAX,
            f64::MIN_POSITIVE,
            std::f64::consts::PI,
        ] {
            let literal = float64_literal(value);
            let parsed: f64 = literal.parse().expect("literal must parse as f64");
            assert_eq!(
                parsed.to_bits(),
                value.to_bits(),
                "f64 literal {literal} must round-trip bit-identically"
            );
            assert!(
                literal.contains('.'),
                "f64 literal {literal} must read as floating in Ada"
            );
        }
    }

    /// The operators are the ordinary IEEE comparisons, which is what makes NaN
    /// fail every bound without a special case, keeps one-sided infinity
    /// behaviour, and makes both zero signs compare equal.
    #[test]
    fn bound_kinds_map_to_ieee_comparisons() {
        assert_eq!(FloatingBoundKind::Inclusive.lower_operator(), ">=");
        assert_eq!(FloatingBoundKind::Exclusive.lower_operator(), ">");
        assert_eq!(FloatingBoundKind::Inclusive.upper_operator(), "<=");
        assert_eq!(FloatingBoundKind::Exclusive.upper_operator(), "<");
    }

    fn domain(constraints: ConstraintSet, kind: PrimitiveKind) -> FloatingDomain {
        floating_domain(kind, &constraints)
            .expect("supported domain")
            .expect("constrained domain")
    }

    fn lower_inclusive_zero() -> FloatingDomain {
        domain(
            ConstraintSet {
                min_inclusive: Some(f64_value(0.0)),
                ..ConstraintSet::default()
            },
            PrimitiveKind::Float64,
        )
    }

    fn lower_exclusive_zero() -> FloatingDomain {
        domain(
            ConstraintSet {
                min_exclusive: Some(f64_value(0.0)),
                ..ConstraintSet::default()
            },
            PrimitiveKind::Float64,
        )
    }

    fn upper_inclusive_one() -> FloatingDomain {
        domain(
            ConstraintSet {
                max_inclusive: Some(f64_value(1.0)),
                ..ConstraintSet::default()
            },
            PrimitiveKind::Float64,
        )
    }

    /// Task 033 section 15: NaN is a valid instance of no range-constrained
    /// type, on either side and under either inclusivity, and is never
    /// special-cased into acceptance.
    #[test]
    fn nan_satisfies_no_bound() {
        for domain in [
            lower_inclusive_zero(),
            lower_exclusive_zero(),
            upper_inclusive_one(),
            domain(
                ConstraintSet {
                    max_exclusive: Some(f64_value(1.0)),
                    ..ConstraintSet::default()
                },
                PrimitiveKind::Float64,
            ),
        ] {
            assert_eq!(domain.accepts_f64(f64::NAN), Some(false));
        }
        let float32 = domain(
            ConstraintSet {
                min_inclusive: Some(f32_value(0.0)),
                ..ConstraintSet::default()
            },
            PrimitiveKind::Float32,
        );
        assert_eq!(float32.accepts_f32(f32::NAN), Some(false));
    }

    /// Task 033 section 16: infinities keep ordinary IEEE ordering and are not
    /// rejected merely because a type is constrained.
    #[test]
    fn one_sided_infinity_keeps_ieee_ordering() {
        assert_eq!(
            lower_inclusive_zero().accepts_f64(f64::INFINITY),
            Some(true)
        );
        assert_eq!(
            lower_inclusive_zero().accepts_f64(f64::NEG_INFINITY),
            Some(false)
        );
        assert_eq!(
            upper_inclusive_one().accepts_f64(f64::NEG_INFINITY),
            Some(true)
        );
        assert_eq!(
            upper_inclusive_one().accepts_f64(f64::INFINITY),
            Some(false)
        );

        // A two-sided finite range excludes both infinities.
        let two_sided = domain(
            ConstraintSet {
                min_inclusive: Some(f64_value(0.0)),
                max_inclusive: Some(f64_value(1.0)),
                ..ConstraintSet::default()
            },
            PrimitiveKind::Float64,
        );
        assert_eq!(two_sided.accepts_f64(f64::INFINITY), Some(false));
        assert_eq!(two_sided.accepts_f64(f64::NEG_INFINITY), Some(false));
    }

    /// Task 033 section 17: both zero signs compare equal, so an inclusive zero
    /// lower admits each and an exclusive zero lower rejects each.
    #[test]
    fn negative_zero_follows_ieee_comparison() {
        assert_eq!(lower_inclusive_zero().accepts_f64(0.0), Some(true));
        assert_eq!(lower_inclusive_zero().accepts_f64(-0.0), Some(true));
        assert_eq!(lower_exclusive_zero().accepts_f64(0.0), Some(false));
        assert_eq!(lower_exclusive_zero().accepts_f64(-0.0), Some(false));
        // A genuinely positive value still passes the exclusive lower.
        assert_eq!(
            lower_exclusive_zero().accepts_f64(f64::MIN_POSITIVE),
            Some(true)
        );
    }

    /// Acceptance is asked at the domain's own width; a mismatched query is
    /// reported rather than converted.
    #[test]
    fn acceptance_is_width_specific() {
        assert_eq!(lower_inclusive_zero().accepts_f32(1.0), None);
        let float32 = domain(
            ConstraintSet {
                max_inclusive: Some(f32_value(1.0)),
                ..ConstraintSet::default()
            },
            PrimitiveKind::Float32,
        );
        assert_eq!(float32.accepts_f64(0.5), None);
        assert_eq!(float32.accepts_f32(1.0), Some(true));
        assert_eq!(float32.accepts_f32(1.5), Some(false));
    }
}
