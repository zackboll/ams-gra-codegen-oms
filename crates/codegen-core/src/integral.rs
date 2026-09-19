use ams_gra_oms_ir::{ConstraintSet, NumericValue, PrimitiveKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InclusiveIntegralDomain {
    Signed { min: i64, max: i64 },
    Unsigned { min: u64, max: u64 },
}

/// Classify the Task-020 inclusive integral subset. `None` is unconstrained.
pub fn inclusive_integral_domain(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
) -> Result<Option<InclusiveIntegralDomain>, &'static str> {
    if constraints.min_exclusive.is_some()
        || constraints.max_exclusive.is_some()
        || constraints.length.is_some()
        || constraints.min_length.is_some()
        || constraints.max_length.is_some()
        || constraints.lexical != Default::default()
    {
        return Err("exclusive, length, or lexical constraints");
    }
    let (Some(NumericValue::Integer(min)), Some(NumericValue::Integer(max))) =
        (constraints.min_inclusive, constraints.max_inclusive)
    else {
        return if constraints.min_inclusive.is_none() && constraints.max_inclusive.is_none() {
            Ok(None)
        } else {
            Err("non-finite inclusive integer bounds")
        };
    };
    if min > max {
        return Err("contradictory inclusive integer bounds");
    }
    match kind {
        PrimitiveKind::SignedInteger => Ok(Some(InclusiveIntegralDomain::Signed {
            min: i64::try_from(min).map_err(|_| "signed range outside i64")?,
            max: i64::try_from(max).map_err(|_| "signed range outside i64")?,
        })),
        PrimitiveKind::UnsignedInteger => Ok(Some(InclusiveIntegralDomain::Unsigned {
            min: u64::try_from(min).map_err(|_| "negative or oversized unsigned range")?,
            max: u64::try_from(max).map_err(|_| "negative or oversized unsigned range")?,
        })),
        _ => Err("non-integral primitive"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::{PatternExpression, PatternGroup, WhiteSpacePolicy};

    fn bounded(min: i128, max: i128) -> ConstraintSet {
        ConstraintSet {
            min_inclusive: Some(NumericValue::Integer(min)),
            max_inclusive: Some(NumericValue::Integer(max)),
            ..ConstraintSet::default()
        }
    }

    #[test]
    fn accepts_supported_integral_domains() {
        assert_eq!(
            inclusive_integral_domain(PrimitiveKind::SignedInteger, &ConstraintSet::default()),
            Ok(None)
        );
        assert_eq!(
            inclusive_integral_domain(PrimitiveKind::UnsignedInteger, &ConstraintSet::default()),
            Ok(None)
        );
        assert_eq!(
            inclusive_integral_domain(
                PrimitiveKind::SignedInteger,
                &bounded(i128::from(i64::MIN), i128::from(i64::MAX))
            ),
            Ok(Some(InclusiveIntegralDomain::Signed {
                min: i64::MIN,
                max: i64::MAX
            }))
        );
        assert_eq!(
            inclusive_integral_domain(
                PrimitiveKind::UnsignedInteger,
                &bounded(0, i128::from(u64::MAX))
            ),
            Ok(Some(InclusiveIntegralDomain::Unsigned {
                min: 0,
                max: u64::MAX
            }))
        );
        assert_eq!(
            inclusive_integral_domain(PrimitiveKind::SignedInteger, &bounded(-128, 127)),
            Ok(Some(InclusiveIntegralDomain::Signed {
                min: -128,
                max: 127
            }))
        );
        assert_eq!(
            inclusive_integral_domain(PrimitiveKind::UnsignedInteger, &bounded(0, 255)),
            Ok(Some(InclusiveIntegralDomain::Unsigned { min: 0, max: 255 }))
        );
    }

    #[test]
    fn rejects_unsupported_integral_constraints() {
        let overflow = i128::from(u64::MAX) + 1;
        assert_eq!(
            inclusive_integral_domain(
                PrimitiveKind::SignedInteger,
                &bounded(i128::from(i64::MIN) - 1, 0)
            ),
            Err("signed range outside i64")
        );
        assert_eq!(
            inclusive_integral_domain(
                PrimitiveKind::SignedInteger,
                &bounded(0, i128::from(i64::MAX) + 1)
            ),
            Err("signed range outside i64")
        );
        assert_eq!(
            inclusive_integral_domain(PrimitiveKind::UnsignedInteger, &bounded(-1, 0)),
            Err("negative or oversized unsigned range")
        );
        assert_eq!(
            inclusive_integral_domain(PrimitiveKind::UnsignedInteger, &bounded(0, overflow)),
            Err("negative or oversized unsigned range")
        );
        for constraints in [
            ConstraintSet {
                min_exclusive: Some(NumericValue::Integer(0)),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                max_exclusive: Some(NumericValue::Integer(1)),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                lexical: ams_gra_oms_ir::LexicalConstraintSet {
                    pattern_groups: vec![PatternGroup {
                        alternatives: vec![PatternExpression::xml_schema("[0-9]+")],
                    }],
                    white_space: None,
                },
                ..ConstraintSet::default()
            },
            ConstraintSet {
                lexical: ams_gra_oms_ir::LexicalConstraintSet {
                    pattern_groups: Vec::new(),
                    white_space: Some(WhiteSpacePolicy::Collapse),
                },
                ..ConstraintSet::default()
            },
            ConstraintSet {
                length: Some(1),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                min_length: Some(1),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                max_length: Some(1),
                ..ConstraintSet::default()
            },
        ] {
            assert_eq!(
                inclusive_integral_domain(PrimitiveKind::SignedInteger, &constraints),
                Err("exclusive, length, or lexical constraints")
            );
        }
        assert_eq!(
            inclusive_integral_domain(
                PrimitiveKind::SignedInteger,
                &ConstraintSet {
                    min_inclusive: Some(NumericValue::Integer(0)),
                    ..ConstraintSet::default()
                }
            ),
            Err("non-finite inclusive integer bounds")
        );
        assert_eq!(
            inclusive_integral_domain(
                PrimitiveKind::SignedInteger,
                &ConstraintSet {
                    max_inclusive: Some(NumericValue::Integer(0)),
                    ..ConstraintSet::default()
                }
            ),
            Err("non-finite inclusive integer bounds")
        );
        assert_eq!(
            inclusive_integral_domain(PrimitiveKind::SignedInteger, &bounded(1, 0)),
            Err("contradictory inclusive integer bounds")
        );
        assert_eq!(
            inclusive_integral_domain(PrimitiveKind::String, &bounded(0, 1)),
            Err("non-integral primitive")
        );
    }
}
