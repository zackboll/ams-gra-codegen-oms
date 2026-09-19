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
