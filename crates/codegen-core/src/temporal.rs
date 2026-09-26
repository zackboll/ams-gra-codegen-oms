//! Task 036 shared semantics for named XML Schema temporal declarations.
//!
//! This module answers exactly one question, once, for `backend-ada`,
//! `backend-rust`, `backend-cpp`, and `CoverageAnalysis`:
//!
//! > Is this named declaration a temporal declaration that Task 036 fully
//! > implements?
//!
//! Four independent readings of the same facet set is precisely how the three
//! backends would drift into disagreeing about which declarations are
//! renderable, while coverage measured a fourth opinion. Every consumer calls
//! [`temporal_profile`].
//!
//! # The one supported profile
//!
//! [`TemporalProfile::DateTimeZulu`]: a [`PrimitiveKind::DateTime`]
//! declaration whose *effective* [`ConstraintSet`] carries exactly one pattern
//! group, holding exactly one XML-Schema-dialect alternative whose expression
//! is the literal three characters `.+Z`, and nothing else at all. This is the
//! authoritative UCI profile, reconfirmed from the pinned release bytes:
//!
//! ```xml
//! <xs:simpleType name="DateTimeType">
//!   <xs:restriction base="xs:dateTime">
//!     <xs:pattern value=".+Z"/>
//!   </xs:restriction>
//! </xs:simpleType>
//! ```
//!
//! Nothing here tests a UCI local name. Support arises solely from the
//! primitive kind plus the effective constraint shape, so a differently named
//! declaration with the same profile is equally supported and `DateTimeType`
//! carrying a different profile is equally unsupported.
//!
//! # Why `.+Z` is interpreted rather than matched by a regex engine
//!
//! Task 036 deliberately introduces no XML Schema regular-expression engine.
//! Instead it *proves*, for this one expression, that pattern-matching reduces
//! to a decision the generated code can make directly.
//!
//! XML Schema 1.0 Part 2 §3.2.7.1 gives the `dateTime` lexical space as
//!
//! ```text
//! '-'? yyyy '-' mm '-' dd 'T' hh ':' mm ':' ss ('.' s+)? (zzzzzz)?
//! ```
//!
//! and §3.2.7.3 gives the timezone as `(('+' | '-') hh ':' mm) | 'Z'`. Under
//! the XML Schema regular-expression language (Appendix F), `.` is the
//! `WildcardEsc` production [37a], equivalent to the character class
//! `[^\n\r]`, and patterns are anchored: a literal is pattern-valid only if the
//! *entire* literal is matched.
//!
//! So for a string `s` that has **already** been accepted by the `dateTime`
//! lexical validator, `.+Z` matches `s` exactly when:
//!
//! * `s` is non-empty and ends in a literal `Z` -- the `Z` in the pattern is a
//!   literal character, and `.+` must consume the whole prefix; and
//! * the consumed prefix contains no `#xA` or `#xD`.
//!
//! The second condition is free. `dateTime` has a *fixed* intrinsic
//! `whiteSpace` of `collapse` (§4.3.6: for every atomic datatype other than
//! `string` and its restrictions the value is `collapse` and "cannot be changed
//! by a schema author"), and Datatype Valid applies the pattern to the literal
//! *after* that normalization. A collapsed literal contains no tab, line feed,
//! or carriage return at all. Separately, the `dateTime` grammar above admits
//! only digits, `-`, `T`, `:`, `.`, `+`, and `Z` -- no line terminator can
//! survive lexical validation anyway.
//!
//! Therefore, on the post-normalization, lexically-valid `dateTime` domain:
//!
//! ```text
//! pattern ".+Z"  ==  the normalized lexical form ends in literal 'Z'
//! ```
//!
//! and by §3.2.7.3 the only `dateTime` timezone spelling ending in `Z` *is*
//! `Z`, the Zulu/UTC form. That is the equivalence Task 036 implements, and it
//! is asserted of this expression alone. It is **not** generalized: any other
//! pattern text, a second alternative, a second group, or a non-XML-Schema
//! dialect all fail closed, because the proof above does not carry over.
//!
//! # Why the result distinguishes three outcomes
//!
//! Diagnostics must be able to say *why* a declaration is unsupported.
//! `Ok(Some(_))` is the implemented profile; `Ok(None)` means "not a Task 036
//! temporal declaration at all" (an integer, a string, a record); and
//! `Err(_)` means "this genuinely is a temporal declaration, but its shape is
//! outside the implemented subset" -- an unconstrained `DateTime`, a different
//! pattern, or any `Time`/`Duration`. Collapsing the last two would make a
//! backend report a `Duration` as though it were not temporal.
//!
//! # What this classifier does not decide
//!
//! [`temporal_profile`] only classifies named declarations. Direct primitive
//! references use the separate [`direct_temporal_profile`] decision; an
//! unconstrained named declaration must never inherit the direct field policy.

use ams_gra_oms_ir::{ConstraintSet, PatternDialect, PrimitiveKind};
use ams_gra_oms_ir::{SchemaIr, TypeKind, TypeRefTarget};

use crate::{
    EffectiveValueMember, GenerationWorld, TypeEmission, abstract_value_projection_for_ref,
    effective_choice_alternatives, effective_record_fields, field_storage_semantics,
    name_preflight_plan,
};

/// The authoritative UCI Zulu lexical restriction, verbatim.
///
/// Reconfirmed from the pinned release bytes at
/// `UCI_MessageDefinitions_v2_5_0.xsd:117043` (UCI 2.5) and
/// `UCI_SecurityMarkings_v2_6_0.xsd:1208` (UCI 2.6).
const UCI_ZULU_PATTERN: &str = ".+Z";

/// A named temporal declaration shape that Task 036 fully implements.
///
/// Each future variant must arrive with its own validator and its own
/// conformance corpus; this is not a place to accumulate near-misses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalProfile {
    /// [`PrimitiveKind::DateTime`] restricted by exactly the UCI `.+Z` pattern.
    ///
    /// Generated code stores the whitespace-normalized, lexically valid XML
    /// Schema `dateTime` spelling whose timezone is therefore `Z`.
    DateTimeZulu,
}

/// The supported direct primitive temporal value domain (not a named type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectTemporalProfile {
    /// Base XML Schema 1.0 dateTime, including absent and numeric timezones.
    DateTime,
}

/// A direct temporal value that cannot be represented without dropping facets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectTemporalProfileError {
    TimeUnsupported,
    DurationUnsupported,
    DateTimeUnsupportedConstraints,
}

/// Classify a direct field or alternative primitive reference, never a named
/// declaration. The intrinsic dateTime whitespace collapse is implemented by
/// the carrier; any explicit field-local facet is rejected rather than ignored.
///
/// # Errors
/// Returns an error for unsupported temporal kinds or any constrained DateTime.
pub fn direct_temporal_profile(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
) -> Result<Option<DirectTemporalProfile>, DirectTemporalProfileError> {
    match kind {
        PrimitiveKind::DateTime if *constraints == ConstraintSet::default() => {
            Ok(Some(DirectTemporalProfile::DateTime))
        }
        PrimitiveKind::DateTime => Err(DirectTemporalProfileError::DateTimeUnsupportedConstraints),
        PrimitiveKind::Time => Err(DirectTemporalProfileError::TimeUnsupported),
        PrimitiveKind::Duration => Err(DirectTemporalProfileError::DurationUnsupported),
        _ => Ok(None),
    }
}

/// Whether an emitted structural member stores a supported direct DateTime.
/// Inheritance, absent-only elision, abstract projection and the requested
/// generation world are resolved against the same emission surfaces as name
/// preflight. An unused abstract ancestor is not a generated storage owner.
#[must_use]
pub fn schema_emits_direct_date_time(schema: &SchemaIr, world: GenerationWorld) -> bool {
    let plan = name_preflight_plan(schema, world);
    plan.surfaces().iter().any(|emission| {
        let TypeEmission::Declaration(declaration) = emission else {
            return false;
        };
        if !emission.emits_own_top_level_name() {
            return false;
        }
        let members = match declaration.kind {
            TypeKind::Record { .. } => effective_record_fields(schema, &declaration.name),
            TypeKind::Choice { .. } => effective_choice_alternatives(schema, &declaration.name),
            _ => return false,
        };
        members.is_ok_and(|members| {
            members.into_iter().any(|member| {
                matches!(
                    member.type_ref.target,
                    TypeRefTarget::Primitive(PrimitiveKind::DateTime)
                ) && direct_temporal_profile(PrimitiveKind::DateTime, &member.constraints)
                    == Ok(Some(DirectTemporalProfile::DateTime))
                    && matches!(
                        field_storage_semantics(schema, member, world),
                        Ok(EffectiveValueMember::Stored(_))
                    )
                    && abstract_value_projection_for_ref(schema, &member.type_ref, world).is_ok()
            })
        })
    })
}

/// Why a temporal declaration falls outside the Task 036 implemented subset.
///
/// Every variant describes a declaration that *is* temporal. A non-temporal
/// declaration is reported as `Ok(None)` instead, never as an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalProfileError {
    /// `PrimitiveKind::Time`; the UCI `TimeType` profile is follow-up work.
    TimeUnsupported,
    /// `PrimitiveKind::Duration`; the UCI `DurationType` profile is follow-up.
    DurationUnsupported,
    /// A `DateTime` with no lexical restriction at all.
    ///
    /// Unconstrained `dateTime` admits every timezone spelling, including none,
    /// which is a strictly larger lexical space than the one Task 036's
    /// validated carrier promises. It is not silently narrowed to Zulu.
    DateTimeUnconstrained,
    /// A `DateTime` whose constraint shape is not the supported Zulu profile.
    ///
    /// Covers a different pattern text, multiple alternatives in one group,
    /// multiple restriction-level groups, a non-XML-Schema dialect, an explicit
    /// `whiteSpace` facet, or any neighbouring numeric/length facet.
    DateTimeUnsupportedConstraints,
}

impl TemporalProfileError {
    /// A short, stable diagnostic fragment for backend error messages.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::TimeUnsupported => "unsupported temporal declaration: Time",
            Self::DurationUnsupported => "unsupported temporal declaration: Duration",
            Self::DateTimeUnconstrained => {
                "unsupported temporal declaration: DateTime without the UCI Zulu pattern"
            }
            Self::DateTimeUnsupportedConstraints => {
                "unsupported temporal declaration: DateTime with unsupported constraints"
            }
        }
    }
}

impl std::fmt::Display for TemporalProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.reason())
    }
}

/// Whether a primitive kind is one of XML Schema's temporal value spaces.
///
/// Used to keep "not temporal" and "temporal but unsupported" apart at call
/// sites that only need the coarse question.
#[must_use]
pub const fn is_temporal_primitive(kind: PrimitiveKind) -> bool {
    matches!(
        kind,
        PrimitiveKind::DateTime | PrimitiveKind::Time | PrimitiveKind::Duration
    )
}

/// Classify a named temporal declaration against the Task 036 subset.
///
/// * `Ok(None)` -- not a temporal declaration; Task 036 has no opinion.
/// * `Ok(Some(profile))` -- an implemented profile; backends may render it.
/// * `Err(reason)` -- temporal, but outside the implemented subset.
///
/// `constraints` must be the declaration's **effective** constraint set. Task
/// 015 already resolves named restriction chains, so this never re-walks raw
/// XSD; it reads the normalized IR that every backend already holds.
///
/// # Errors
///
/// Returns [`TemporalProfileError`] when the declaration is temporal but its
/// primitive kind or effective constraint shape is not the one implemented
/// profile. No facet is ever silently ignored: a constraint this module cannot
/// enforce is a rejection, not a warning.
pub fn temporal_profile(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
) -> Result<Option<TemporalProfile>, TemporalProfileError> {
    match kind {
        PrimitiveKind::Time => Err(TemporalProfileError::TimeUnsupported),
        PrimitiveKind::Duration => Err(TemporalProfileError::DurationUnsupported),
        PrimitiveKind::DateTime => date_time_profile(constraints).map(Some),
        _ => Ok(None),
    }
}

/// Decide whether a `DateTime`'s effective constraints are the Zulu profile.
///
/// The shape is checked structurally against the normalized Task 016 IR --
/// group count, alternative count, dialect, expression text -- never against a
/// rendered debug string, which would silently accept a formatting change.
fn date_time_profile(constraints: &ConstraintSet) -> Result<TemporalProfile, TemporalProfileError> {
    // Any neighbouring facet is disqualifying. `dateTime` does admit
    // min/maxInclusive and min/maxExclusive per §3.2.7.6, but Task 036's
    // carrier stores a lexical spelling and performs no value-space
    // comparison, so it cannot enforce an ordering bound. Length facets are
    // not even applicable to `dateTime`, and their presence means the IR did
    // not come from a well-formed temporal restriction.
    if constraints.min_inclusive.is_some()
        || constraints.max_inclusive.is_some()
        || constraints.min_exclusive.is_some()
        || constraints.max_exclusive.is_some()
        || constraints.length.is_some()
        || constraints.min_length.is_some()
        || constraints.max_length.is_some()
    {
        return Err(TemporalProfileError::DateTimeUnsupportedConstraints);
    }
    // §4.3.6 fixes `dateTime`'s intrinsic `whiteSpace` at `collapse` and
    // forbids a schema author from restating it. An explicit facet in the IR
    // is therefore either redundant or wrong; either way it is not the
    // authoritative profile, so it fails closed rather than being assumed
    // harmless.
    if constraints.lexical.white_space.is_some() {
        return Err(TemporalProfileError::DateTimeUnsupportedConstraints);
    }
    let groups = &constraints.lexical.pattern_groups;
    // No pattern at all is a *distinct* outcome from a wrong pattern: an
    // unconstrained `DateTime` is a coherent schema construct that Task 036
    // simply does not represent, and saying so precisely matters.
    if groups.is_empty() {
        return Err(TemporalProfileError::DateTimeUnconstrained);
    }
    // Exactly one restriction level, carrying exactly one alternative. Two
    // groups are AND-ed and two alternatives are OR-ed (§4.3.4.3); the
    // equivalence proof in this module's header covers neither, so both fail
    // closed rather than being approximated.
    let [group] = groups.as_slice() else {
        return Err(TemporalProfileError::DateTimeUnsupportedConstraints);
    };
    let [alternative] = group.alternatives.as_slice() else {
        return Err(TemporalProfileError::DateTimeUnsupportedConstraints);
    };
    if alternative.dialect != PatternDialect::XmlSchema
        || alternative.expression != UCI_ZULU_PATTERN
    {
        return Err(TemporalProfileError::DateTimeUnsupportedConstraints);
    }
    Ok(TemporalProfile::DateTimeZulu)
}

/// Whether a schema emits at least one Task 036 temporal value carrier.
///
/// Several generator decisions are conditional on this and must agree exactly:
///
/// * C++ adds `#include <string_view>` only when a carrier exists;
/// * Ada emits a package **body** only when a carrier exists, so a schema with
///   no Task 036 feature keeps its existing single-file `.ads` output;
/// * the shared name model registers Ada's `Create` / `Value` only for the
///   declarations that really generate them.
///
/// Reading the classifier keeps all of those tied to one predicate. A schema
/// containing only *unsupported* temporal declarations returns `false`,
/// because those fail closed in the backend and produce no output at all.
#[must_use]
pub fn schema_emits_temporal_carrier(schema: &ams_gra_oms_ir::SchemaIr) -> bool {
    schema.types.iter().any(|declaration| {
        matches!(declaration.kind, ams_gra_oms_ir::TypeKind::Primitive(kind)
            if temporal_profile(kind, &declaration.constraints)
                .is_ok_and(|profile| profile.is_some()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::{
        LexicalConstraintSet, NumericValue, PatternExpression, PatternGroup, WhiteSpacePolicy,
    };

    #[test]
    fn direct_date_time_is_distinct_from_named_zulu() {
        let bare = ConstraintSet::default();
        assert_eq!(
            direct_temporal_profile(PrimitiveKind::DateTime, &bare),
            Ok(Some(DirectTemporalProfile::DateTime))
        );
        assert_eq!(
            temporal_profile(PrimitiveKind::DateTime, &bare),
            Err(TemporalProfileError::DateTimeUnconstrained)
        );
        assert_eq!(
            temporal_profile(PrimitiveKind::DateTime, &zulu()),
            Ok(Some(TemporalProfile::DateTimeZulu))
        );
        assert_eq!(
            direct_temporal_profile(PrimitiveKind::DateTime, &zulu()),
            Err(DirectTemporalProfileError::DateTimeUnsupportedConstraints)
        );
    }

    #[test]
    fn direct_neighbors_and_field_facets_fail_closed() {
        let bare = ConstraintSet::default();
        assert_eq!(
            direct_temporal_profile(PrimitiveKind::Time, &bare),
            Err(DirectTemporalProfileError::TimeUnsupported)
        );
        assert_eq!(
            direct_temporal_profile(PrimitiveKind::Duration, &bare),
            Err(DirectTemporalProfileError::DurationUnsupported)
        );
        assert_eq!(
            direct_temporal_profile(PrimitiveKind::String, &bare),
            Ok(None)
        );
        for mut constraints in [
            ConstraintSet {
                min_inclusive: Some(NumericValue::Integer(0)),
                ..bare.clone()
            },
            ConstraintSet {
                max_exclusive: Some(NumericValue::Integer(1)),
                ..bare.clone()
            },
            zulu(),
        ] {
            assert_eq!(
                direct_temporal_profile(PrimitiveKind::DateTime, &constraints),
                Err(DirectTemporalProfileError::DateTimeUnsupportedConstraints)
            );
            constraints.lexical.white_space = Some(WhiteSpacePolicy::Collapse);
            assert_eq!(
                direct_temporal_profile(PrimitiveKind::DateTime, &constraints),
                Err(DirectTemporalProfileError::DateTimeUnsupportedConstraints)
            );
        }
    }

    /// The authoritative UCI profile, built from the normalized IR shape.
    fn zulu() -> ConstraintSet {
        with_patterns(vec![PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(UCI_ZULU_PATTERN)],
        }])
    }

    fn with_patterns(groups: Vec<PatternGroup>) -> ConstraintSet {
        ConstraintSet {
            lexical: LexicalConstraintSet {
                pattern_groups: groups,
                white_space: None,
            },
            ..ConstraintSet::default()
        }
    }

    /// The single supported profile, recognized from kind + effective facets.
    #[test]
    fn uci_zulu_date_time_is_the_supported_profile() {
        assert_eq!(
            temporal_profile(PrimitiveKind::DateTime, &zulu()),
            Ok(Some(TemporalProfile::DateTimeZulu))
        );
    }

    /// An unconstrained `DateTime` is temporal but unrepresented, and must be
    /// reported distinctly from a *wrongly* constrained one so diagnostics can
    /// tell an author which boundary they hit.
    #[test]
    fn unconstrained_date_time_is_temporal_but_unsupported() {
        assert_eq!(
            temporal_profile(PrimitiveKind::DateTime, &ConstraintSet::default()),
            Err(TemporalProfileError::DateTimeUnconstrained)
        );
    }

    /// Task 036: `Time` and `Duration` remain unsupported, and are not
    /// opportunistically admitted because the machinery looks similar. The
    /// authoritative `TimeType` carries the *same* `.+Z` text, which is exactly
    /// why this must be asserted with that pattern present.
    #[test]
    fn time_and_duration_remain_unsupported() {
        assert_eq!(
            temporal_profile(PrimitiveKind::Time, &zulu()),
            Err(TemporalProfileError::TimeUnsupported)
        );
        assert_eq!(
            temporal_profile(PrimitiveKind::Time, &ConstraintSet::default()),
            Err(TemporalProfileError::TimeUnsupported)
        );
        assert_eq!(
            temporal_profile(PrimitiveKind::Duration, &ConstraintSet::default()),
            Err(TemporalProfileError::DurationUnsupported)
        );
        assert_eq!(
            temporal_profile(PrimitiveKind::Duration, &zulu()),
            Err(TemporalProfileError::DurationUnsupported)
        );
    }

    /// A non-temporal primitive is not this module's business at all, and must
    /// not be reported as an unsupported *temporal* declaration.
    #[test]
    fn non_temporal_primitives_are_not_task_036_declarations() {
        for kind in [
            PrimitiveKind::Boolean,
            PrimitiveKind::SignedInteger,
            PrimitiveKind::UnsignedInteger,
            PrimitiveKind::Decimal,
            PrimitiveKind::Float32,
            PrimitiveKind::Float64,
            PrimitiveKind::String,
            PrimitiveKind::Binary,
        ] {
            assert_eq!(temporal_profile(kind, &zulu()), Ok(None));
            assert_eq!(temporal_profile(kind, &ConstraintSet::default()), Ok(None));
            assert!(!is_temporal_primitive(kind));
        }
        for kind in [
            PrimitiveKind::DateTime,
            PrimitiveKind::Time,
            PrimitiveKind::Duration,
        ] {
            assert!(is_temporal_primitive(kind));
        }
    }

    /// No generic regex translation: a pattern that is not *exactly* the
    /// authoritative expression fails closed, including ones a human would
    /// call equivalent or stricter. The equivalence proof in this module's
    /// header is claimed for `.+Z` alone, so nothing else may ride on it.
    #[test]
    fn a_different_pattern_fails_closed() {
        for expression in [
            ".*Z",
            ".+z",
            ".+Z ",
            " .+Z",
            "..+Z",
            ".+Z|.+z",
            "[^\\n\\r]+Z",
            "\\d{4}-\\d{2}-\\d{2}T.+Z",
            ".+",
            "",
            "Z",
        ] {
            assert_eq!(
                temporal_profile(
                    PrimitiveKind::DateTime,
                    &with_patterns(vec![PatternGroup {
                        alternatives: vec![PatternExpression::xml_schema(expression)],
                    }])
                ),
                Err(TemporalProfileError::DateTimeUnsupportedConstraints),
                "pattern {expression:?} must not be accepted"
            );
        }
    }

    /// Alternatives within one group are OR-ed (XSD 1.0 Part 2 section
    /// 4.3.4.3), which widens the accepted lexical space beyond the proven
    /// equivalence even when one alternative is the authoritative expression.
    #[test]
    fn multiple_alternatives_fail_closed() {
        assert_eq!(
            temporal_profile(
                PrimitiveKind::DateTime,
                &with_patterns(vec![PatternGroup {
                    alternatives: vec![
                        PatternExpression::xml_schema(UCI_ZULU_PATTERN),
                        PatternExpression::xml_schema(".+z"),
                    ],
                }])
            ),
            Err(TemporalProfileError::DateTimeUnsupportedConstraints)
        );
        // A degenerate group carrying no alternative is not the profile, and
        // is specifically not silently re-read as "unconstrained".
        assert_eq!(
            temporal_profile(
                PrimitiveKind::DateTime,
                &with_patterns(vec![PatternGroup {
                    alternatives: Vec::new(),
                }])
            ),
            Err(TemporalProfileError::DateTimeUnsupportedConstraints)
        );
    }

    /// Groups from different restriction levels are AND-ed. Even when one of
    /// them is the authoritative expression, the conjunction denotes a
    /// narrower lexical space than the validator implements.
    #[test]
    fn multiple_pattern_groups_fail_closed() {
        assert_eq!(
            temporal_profile(
                PrimitiveKind::DateTime,
                &with_patterns(vec![
                    PatternGroup {
                        alternatives: vec![PatternExpression::xml_schema(UCI_ZULU_PATTERN)],
                    },
                    PatternGroup {
                        alternatives: vec![PatternExpression::xml_schema(UCI_ZULU_PATTERN)],
                    },
                ])
            ),
            Err(TemporalProfileError::DateTimeUnsupportedConstraints)
        );
    }

    /// A neighbouring facet is never dropped. `dateTime` genuinely admits the
    /// ordering facets (section 3.2.7.6), and a lexical carrier that performs
    /// no value-space comparison cannot enforce them, so accepting one would
    /// mean silently generating a type that ignores part of its own schema.
    #[test]
    fn neighbouring_facets_fail_closed() {
        let bound = NumericValue::Integer(0);
        for constraints in [
            ConstraintSet {
                min_inclusive: Some(bound),
                ..zulu()
            },
            ConstraintSet {
                max_inclusive: Some(bound),
                ..zulu()
            },
            ConstraintSet {
                min_exclusive: Some(bound),
                ..zulu()
            },
            ConstraintSet {
                max_exclusive: Some(bound),
                ..zulu()
            },
            ConstraintSet {
                length: Some(20),
                ..zulu()
            },
            ConstraintSet {
                min_length: Some(1),
                ..zulu()
            },
            ConstraintSet {
                max_length: Some(40),
                ..zulu()
            },
        ] {
            assert_eq!(
                temporal_profile(PrimitiveKind::DateTime, &constraints),
                Err(TemporalProfileError::DateTimeUnsupportedConstraints)
            );
        }
    }

    /// Section 4.3.6 fixes `dateTime`'s `whiteSpace` at `collapse` and forbids
    /// a schema author from restating it, so an explicit facet cannot legally
    /// appear. Its presence is not the authoritative profile even when the
    /// value merely repeats the intrinsic one.
    #[test]
    fn an_explicit_white_space_facet_fails_closed() {
        for policy in [
            WhiteSpacePolicy::Collapse,
            WhiteSpacePolicy::Replace,
            WhiteSpacePolicy::Preserve,
        ] {
            let mut constraints = zulu();
            constraints.lexical.white_space = Some(policy);
            assert_eq!(
                temporal_profile(PrimitiveKind::DateTime, &constraints),
                Err(TemporalProfileError::DateTimeUnsupportedConstraints)
            );
        }
    }

    /// Recognition reads the structured Task 016 IR -- dialect and expression
    /// text -- rather than a rendered debug string that a formatting change
    /// could silently alter.
    #[test]
    fn recognition_reads_the_structured_pattern_representation() {
        let constraints = zulu();
        assert_eq!(constraints.lexical.pattern_groups.len(), 1);
        let group = &constraints.lexical.pattern_groups[0];
        assert_eq!(group.alternatives.len(), 1);
        assert_eq!(group.alternatives[0].dialect, PatternDialect::XmlSchema);
        assert_eq!(group.alternatives[0].expression, ".+Z");
    }

    /// Diagnostic text stays distinguishable per outcome, so a backend error
    /// can name which boundary was hit rather than emitting one vague string.
    #[test]
    fn every_error_reason_is_distinct() {
        let reasons = [
            TemporalProfileError::TimeUnsupported.reason(),
            TemporalProfileError::DurationUnsupported.reason(),
            TemporalProfileError::DateTimeUnconstrained.reason(),
            TemporalProfileError::DateTimeUnsupportedConstraints.reason(),
        ];
        for (index, left) in reasons.iter().enumerate() {
            for right in &reasons[index + 1..] {
                assert_ne!(left, right);
            }
        }
        assert_eq!(
            TemporalProfileError::TimeUnsupported.to_string(),
            TemporalProfileError::TimeUnsupported.reason()
        );
    }
}
