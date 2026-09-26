//! The single shared classification of Ada's per-field optional wrapper.
//!
//! Task 034 introduced a generated per-emitted-field discriminated record for
//! non-nillable `0..1` **named**-target Record fields. Task 035 broadens the
//! same representation to direct primitive targets whose *value* lowering the
//! Ada backend already implements.
//!
//! Before Task 035 this predicate existed twice -- once in `backend-ada` for
//! generation and once in `backend_names` for the generated-name preflight --
//! and the two had to be kept in lockstep by hand. They now both call this
//! module, so there is exactly one occurrence model and no third copy.
//!
//! # Why `ConstraintSet::default()` is not the integral gate
//!
//! It is tempting to require `field.constraints == ConstraintSet::default()`
//! for every direct primitive. That is wrong for integers, because the XSD
//! frontend deliberately normalizes a built-in integer type's *domain* into
//! semantic inclusive bounds. A bare
//!
//! ```xml
//! <xs:element name="Version" type="xs:unsignedInt" minOccurs="0"/>
//! ```
//!
//! carries no author-written facet at all, yet normalizes to
//! `UnsignedInteger` plus `minInclusive = 0`, `maxInclusive = 4294967295`.
//! So `constraints != default` does **not** mean "user-authored field-local
//! restriction"; for integral primitives it usually just means "this is what
//! `xs:unsignedInt` *is*".
//!
//! Rather than adding constraint provenance to Schema IR, this module asks the
//! question that actually matters:
//!
//! > can the existing Ada field-value lowering represent this exact field
//! > domain without semantic loss?
//!
//! For integral primitives that answer already exists as Task 020's
//! [`inclusive_integral_domain`], which is the same function
//! `backend-ada::ada_field_base` uses to render *required* direct integral
//! fields. Reusing it keeps one integral-domain policy: any domain Ada can
//! already spell as a required component it can also spell inside the wrapper,
//! and any shape Task 020 rejects (exclusive bounds, length, lexical facets,
//! half-open or contradictory ranges) stays fail-closed here too.

use ams_gra_oms_ir::{Cardinality, ConstraintSet, FieldDecl, PrimitiveKind, TypeRefTarget};

use crate::integral::inclusive_integral_domain;
use crate::temporal::{DirectTemporalProfile, direct_temporal_profile};

/// Whether Ada stores this Record field in a generated
/// `{Owner}_{Field}_Optional` discriminated wrapper.
///
/// True for exactly the Task 034 + Task 035 subset:
///
/// * cardinality is exactly `0..1`;
/// * the field is **not** nillable -- nil is a third state this two-state
///   wrapper cannot express, so it stays fail-closed;
/// * the target is either a **named** type with default field-local
///   constraints (Task 034), or a **direct primitive** whose value
///   representation Ada already implements (Task 035).
///
/// `Primitive(String)` is deliberately **false**: direct optional Strings keep
/// using the shared `Optional_String`, so Task 035 causes no churn in already
/// generated output.
///
/// This says nothing about whether a *named* target declaration is itself
/// renderable. That remains the job of the backend's own validation and of the
/// target's capability rules, so an optional reference to an unsupported
/// declaration still fails and is attributed to the target rather than to
/// optionality.
#[must_use]
pub fn ada_record_field_uses_optional_wrapper(field: &FieldDecl) -> bool {
    if field.cardinality != Cardinality::OPTIONAL_ONE || field.nillable {
        return false;
    }
    match field.type_ref.target {
        // Task 034. A field-local facet on an optional named value has no
        // lowering, so it remains fail-closed.
        TypeRefTarget::Named(_) => field.constraints == ConstraintSet::default(),
        TypeRefTarget::Primitive(kind) => {
            ada_optional_direct_primitive_representable(kind, &field.constraints)
        }
    }
}

/// Whether Ada's existing value lowering can represent this direct primitive
/// field domain inside the Task 034 wrapper.
///
/// Split out from [`ada_record_field_uses_optional_wrapper`] so the
/// *occurrence* question and the *primitive value* question stay visibly
/// separate. A primitive kind Ada cannot represent at all does not become
/// representable merely because an optional occurrence now has a storage shape.
#[must_use]
pub fn ada_optional_direct_primitive_representable(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
) -> bool {
    match kind {
        // Task 020 already lowers these exact domains for required direct
        // integral fields, including the bounds the frontend synthesizes for a
        // built-in like `xs:unsignedInt`. `Ok(None)` is unconstrained,
        // `Ok(Some(_))` is a supported inclusive range, and `Err(_)` is a shape
        // Ada has no representation for -- which stays unsupported.
        PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger => {
            inclusive_integral_domain(kind, constraints).is_ok()
        }
        // These have no field-local constraint lowering in the Ada backend at
        // all, so a facet on one is genuine unsupported restriction and must
        // not be silently dropped into an unconstrained wrapper.
        PrimitiveKind::Boolean
        | PrimitiveKind::Float32
        | PrimitiveKind::Float64
        | PrimitiveKind::Binary => *constraints == ConstraintSet::default(),
        // Unchanged: the shared `Optional_String` already represents this, so
        // no per-field wrapper is generated and no output churns.
        PrimitiveKind::String => false,
        PrimitiveKind::DateTime => {
            direct_temporal_profile(kind, constraints) == Ok(Some(DirectTemporalProfile::DateTime))
        }
        PrimitiveKind::Time | PrimitiveKind::Duration | PrimitiveKind::Decimal => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::{
        LexicalConstraintSet, NumericValue, PatternExpression, PatternGroup, QualifiedName,
        SourceRef, TypeRef,
    };

    fn field(target: TypeRefTarget, cardinality: Cardinality) -> FieldDecl {
        FieldDecl {
            name: "Value".to_owned(),
            type_ref: TypeRef { target },
            cardinality,
            nillable: false,
            constraints: ConstraintSet::default(),
            documentation: None,
            source: SourceRef {
                document: "test".to_owned(),
                line: None,
            },
        }
    }

    fn primitive(kind: PrimitiveKind) -> FieldDecl {
        field(TypeRefTarget::Primitive(kind), Cardinality::OPTIONAL_ONE)
    }

    fn named() -> FieldDecl {
        field(
            TypeRefTarget::Named(QualifiedName {
                namespace_uri: "urn:test".to_owned(),
                local_name: "Target".to_owned(),
            }),
            Cardinality::OPTIONAL_ONE,
        )
    }

    fn bounded(min: i128, max: i128) -> ConstraintSet {
        ConstraintSet {
            min_inclusive: Some(NumericValue::Integer(min)),
            max_inclusive: Some(NumericValue::Integer(max)),
            ..ConstraintSet::default()
        }
    }

    /// Every built-in integer domain the frontend synthesizes, including the
    /// exact shape a bare `xs:unsignedInt minOccurs="0"` produces -- which is
    /// what `MissionID_Type.Version` normalizes to. These are emphatically not
    /// `ConstraintSet::default()`, and must still be supported.
    #[test]
    fn synthesized_builtin_integer_domains_are_supported() {
        for (kind, min, max) in [
            (PrimitiveKind::SignedInteger, -128, 127),
            (PrimitiveKind::SignedInteger, -32_768, 32_767),
            (PrimitiveKind::SignedInteger, -2_147_483_648, 2_147_483_647),
            (
                PrimitiveKind::SignedInteger,
                i128::from(i64::MIN),
                i128::from(i64::MAX),
            ),
            (PrimitiveKind::UnsignedInteger, 0, 255),
            (PrimitiveKind::UnsignedInteger, 0, 65_535),
            (PrimitiveKind::UnsignedInteger, 0, 4_294_967_295),
        ] {
            let mut candidate = primitive(kind);
            candidate.constraints = bounded(min, max);
            assert_ne!(
                candidate.constraints,
                ConstraintSet::default(),
                "{kind:?} {min}..{max} is a synthesized domain, not a default set"
            );
            assert!(
                ada_record_field_uses_optional_wrapper(&candidate),
                "{kind:?} {min}..{max} must reuse the Task 020 integral lowering"
            );
        }
    }

    /// An unconstrained integral optional is `Ok(None)` and also supported.
    #[test]
    fn unconstrained_integral_optionals_are_supported() {
        for kind in [PrimitiveKind::SignedInteger, PrimitiveKind::UnsignedInteger] {
            assert!(ada_record_field_uses_optional_wrapper(&primitive(kind)));
        }
    }

    /// Integral shapes Task 020 cannot represent stay fail-closed, so no facet
    /// is silently lost into an unconstrained wrapper.
    #[test]
    fn integral_shapes_task_020_rejects_stay_unsupported() {
        for constraints in [
            ConstraintSet {
                min_exclusive: Some(NumericValue::Integer(0)),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                max_exclusive: Some(NumericValue::Integer(9)),
                ..ConstraintSet::default()
            },
            // Half-open: a lone bound has no Ada range spelling.
            ConstraintSet {
                max_inclusive: Some(NumericValue::Integer(9)),
                ..ConstraintSet::default()
            },
            bounded(9, 0),
            ConstraintSet {
                length: Some(4),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                max_length: Some(4),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                lexical: LexicalConstraintSet {
                    pattern_groups: vec![PatternGroup {
                        alternatives: vec![PatternExpression::xml_schema("[0-9]+")],
                    }],
                    white_space: None,
                },
                ..ConstraintSet::default()
            },
        ] {
            let mut candidate = primitive(PrimitiveKind::UnsignedInteger);
            candidate.constraints = constraints.clone();
            assert!(
                !ada_record_field_uses_optional_wrapper(&candidate),
                "{constraints:?} must stay fail-closed"
            );
        }
    }

    /// Non-integral direct primitives have no field-local constraint lowering,
    /// so any facet keeps them unsupported.
    #[test]
    fn non_integral_primitives_require_default_constraints() {
        for kind in [
            PrimitiveKind::Boolean,
            PrimitiveKind::Float32,
            PrimitiveKind::Float64,
            PrimitiveKind::Binary,
        ] {
            assert!(ada_record_field_uses_optional_wrapper(&primitive(kind)));
            let mut constrained = primitive(kind);
            constrained.constraints = bounded(0, 9);
            assert!(
                !ada_record_field_uses_optional_wrapper(&constrained),
                "{kind:?} with a field-local facet must stay fail-closed"
            );
        }
    }

    /// Direct optional String keeps the shared `Optional_String`, so it must
    /// never claim a per-field wrapper.
    #[test]
    fn direct_string_never_uses_a_per_field_wrapper() {
        assert!(!ada_record_field_uses_optional_wrapper(&primitive(
            PrimitiveKind::String
        )));
    }

    /// The existing optional wrapper composes with the validated direct value.
    #[test]
    fn direct_date_time_uses_the_optional_wrapper_only_without_facets() {
        let mut candidate = primitive(PrimitiveKind::DateTime);
        assert!(ada_record_field_uses_optional_wrapper(&candidate));
        candidate.constraints = bounded(0, 9);
        assert!(!ada_record_field_uses_optional_wrapper(&candidate));
    }

    /// Occurrence storage does not grant support for other unsupported kinds.
    #[test]
    fn temporal_and_decimal_remain_unsupported() {
        for kind in [
            PrimitiveKind::Time,
            PrimitiveKind::Duration,
            PrimitiveKind::Decimal,
        ] {
            assert!(
                !ada_record_field_uses_optional_wrapper(&primitive(kind)),
                "{kind:?} has no Ada value representation at any cardinality"
            );
        }
    }

    /// Nillability is a third state the two-state wrapper cannot express.
    #[test]
    fn nillable_optionals_stay_unsupported() {
        for mut candidate in [primitive(PrimitiveKind::UnsignedInteger), named()] {
            candidate.nillable = true;
            assert!(!ada_record_field_uses_optional_wrapper(&candidate));
        }
    }

    /// Only `0..1` is implied; no other occurrence family is.
    #[test]
    fn only_optional_one_uses_the_wrapper() {
        for cardinality in [
            Cardinality::REQUIRED_ONE,
            Cardinality {
                min_occurs: 0,
                max_occurs: None,
            },
            Cardinality {
                min_occurs: 2,
                max_occurs: Some(5),
            },
        ] {
            let candidate = field(
                TypeRefTarget::Primitive(PrimitiveKind::UnsignedInteger),
                cardinality,
            );
            assert!(!ada_record_field_uses_optional_wrapper(&candidate));
        }
    }

    /// Task 034 named-target behavior is preserved exactly.
    #[test]
    fn named_optionals_keep_task_034_behavior() {
        assert!(ada_record_field_uses_optional_wrapper(&named()));
        let mut constrained = named();
        constrained.constraints = bounded(0, 9);
        assert!(!ada_record_field_uses_optional_wrapper(&constrained));
    }
}
