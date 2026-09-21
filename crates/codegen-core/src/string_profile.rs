//! Task 037 shared semantics for named constrained XML Schema `string`
//! declarations.
//!
//! This module answers exactly one question, once, for `backend-ada`,
//! `backend-rust`, `backend-cpp`, and `CoverageAnalysis`:
//!
//! > Is this named declaration a constrained `string` declaration that Task
//! > 037 fully implements?
//!
//! It is the String-side analogue of Task 036's [`crate::temporal`]
//! classifier, and exists for the same reason: four independent readings of
//! the same facet set is precisely how three backends drift into disagreeing
//! about which declarations are renderable while coverage measures a fourth
//! opinion. Every consumer calls [`string_profile`].
//!
//! # The one supported profile
//!
//! [`StringProfile::UciSchemaVersion`]: a [`PrimitiveKind::String`]
//! declaration whose *effective* [`ConstraintSet`] carries exactly
//! `minLength = 7`, `maxLength = 57`, no `length`, no explicit `whiteSpace`,
//! and exactly one pattern group holding exactly one XML-Schema-dialect
//! alternative whose expression is [`UCI_SCHEMA_VERSION_PATTERN`] verbatim.
//!
//! This is the authoritative UCI profile, read from the pinned release bytes
//! of both tracked releases, which are byte-identical for this declaration:
//!
//! ```xml
//! <xs:simpleType name="UCI_SchemaVersionStringType" uci:version="000.001.000.000">
//!   <xs:annotation>
//!     <xs:documentation>String representing the UCI version.</xs:documentation>
//!   </xs:annotation>
//!   <xs:restriction base="xs:string">
//!     <xs:minLength value="7"/>
//!     <xs:maxLength value="57"/>
//!     <xs:pattern value="[0-9]{3}\.[0-9]{1,2}(\.[0-9]{1,2})([a-z]{1,2})?(_[a-zA-Z0-9\-]{1,45})?"/>
//!   </xs:restriction>
//! </xs:simpleType>
//! ```
//!
//! * UCI 2.5 `093610b7753944059360d3236770ab446d039556`:
//!   `UCI_MessageDefinitions_v2_5_0.xsd:145460`
//! * UCI 2.6 `78eb61b6112c8bffa40820c33124b57787fc5bd9`:
//!   `UCI_MessageDefinitions_v2_6_0.xsd:145708`
//!
//! The restriction chain depth is 1: the immediate and only base is the
//! primitive `xs:string`, so the normalized effective constraint set is the
//! single restriction step itself.
//!
//! Nothing here tests a UCI local name. Support arises solely from the
//! primitive kind plus the effective constraint shape, so a differently named
//! declaration with the same profile is equally supported, and a declaration
//! *named* `UCI_SchemaVersionStringType` carrying a different profile is
//! equally unsupported.
//!
//! # The profile is emphatically not `NNN.NNN.NNN.NNN`
//!
//! A UCI schema version is *conventionally* written like `000.001.000.000`,
//! and that spelling even appears as this declaration's own `uci:version`
//! attribute. It is nevertheless **not** the lexical space this type defines.
//! The authoritative pattern admits exactly three dot-separated numeric
//! groups, of widths 3, 1-2, and 1-2, followed by two optional suffixes. The
//! four-group form `000.001.000.000` is *rejected* by this very type. The
//! documentation on the referencing element agrees, giving `002.0` and
//! `001.9b` as the intended shape. Implementing the conventional-looking
//! four-group form would have produced a validator that rejects real UCI
//! versions and accepts values the schema forbids.
//!
//! # Why the pattern is interpreted rather than matched by a regex engine
//!
//! Task 037 deliberately introduces no XML Schema regular-expression engine.
//! Instead it *proves*, for this one expression, that pattern-matching reduces
//! to a bounded, deterministic, single-pass structural decision.
//!
//! The expression is a plain concatenation of five bounded pieces, using only
//! the XML Schema regex features `charClassExpr`, `quantifier` with explicit
//! `{n,m}` bounds, `?`, and the escaped literals `\.` and `\-`. It contains no
//! alternation, no backreference, no unbounded repetition, no nesting beyond
//! one level of grouping, and no ambiguity: each piece's alphabet is disjoint
//! from the literal that follows it, so a left-to-right scan never needs to
//! backtrack. Concretely, in left-to-right order:
//!
//! ```text
//! [0-9]{3}                  exactly 3 ASCII digits
//! \.                        one literal '.'
//! [0-9]{1,2}                1-2 ASCII digits
//! (\.[0-9]{1,2})            one literal '.' then 1-2 ASCII digits   (required)
//! ([a-z]{1,2})?             optional 1-2 ASCII lowercase letters
//! (_[a-zA-Z0-9\-]{1,45})?   optional '_' then 1-45 ASCII alnum/'-'
//! ```
//!
//! Note the third group is parenthesized but carries **no** quantifier, so it
//! is required, not optional. XML Schema patterns are anchored (§4.3.4.3: the
//! pattern must match the *entire* literal), which the generated validators
//! implement by requiring the scan to finish exactly at end-of-input.
//!
//! That equivalence is asserted of this expression alone. It is **not**
//! generalized: any other pattern text, a second alternative, a second group,
//! or a non-XML-Schema dialect all fail closed, because the proof above does
//! not carry over.
//!
//! # `whiteSpace`, and why nothing is trimmed
//!
//! `xs:string` is the one atomic datatype whose intrinsic `whiteSpace` is
//! `preserve` and *not* fixed (§4.3.6). This declaration adds no `whiteSpace`
//! facet, so the effective policy is `preserve`: normalization is the identity
//! and the pattern is applied to the literal exactly as written. The generated
//! carriers therefore store the caller's input **unchanged** and never trim or
//! collapse. A value with leading, trailing, or interior whitespace is not
//! repaired into a valid one -- it is rejected, because no whitespace
//! character appears in any of the pattern's character classes.
//!
//! # Why `length` is counted in bytes here, and why that is XSD-correct
//!
//! XML Schema `minLength`/`maxLength` on a `string` count **characters**
//! (§4.3.1-4.3.3), never bytes or UTF-16 code units. Counting bytes is only
//! sound when every accepted character is single-byte.
//!
//! Here it is, and provably so: the union of every character class in the
//! pattern is `[0-9]`, `[a-z]`, `[a-zA-Z0-9]`, `.`, `_`, and `-`. Every one of
//! those is ASCII, i.e. below U+0080, so every accepted value is pure ASCII and
//! its UTF-8 byte count equals its character count exactly. Because the
//! pattern is checked *before* any length decision matters, no non-ASCII input
//! can reach a length comparison in the first place. This reasoning is
//! specific to this profile and is re-derived, not assumed, for any future one.
//!
//! # `minLength`/`maxLength` are enforced even though the pattern implies them
//!
//! The pattern's own bounds are `3+1+1+2 = 7` minimum and
//! `3+1+2+3+2+46 = 57` maximum, which coincide exactly with the declared
//! `minLength = 7` and `maxLength = 57`. The facets are therefore formally
//! redundant for *this* expression.
//!
//! They are still enforced explicitly by the generated validators, and still
//! required exactly by this classifier. Recognizing the profile by pattern
//! alone would silently accept a neighbouring declaration carrying the same
//! pattern with, say, `maxLength = 20`, and then generate a carrier that
//! ignores that facet. Task 037 loses no facet silently: the effective
//! constraint set is matched in full, and every matched facet is enforced.
//!
//! # What this classifier does not decide
//!
//! It says nothing about *direct* `TypeRefTarget::Primitive(String)` fields, or
//! about field-local anonymous restrictions. Task 037 is a **named
//! declaration** slice: the carrier is emitted for a named declaration, and an
//! ordinary unconstrained `string` keeps its existing plain representation in
//! every backend.

use ams_gra_oms_ir::{ConstraintSet, PatternDialect, PrimitiveKind};

/// The authoritative UCI schema-version lexical restriction, verbatim.
///
/// Reconfirmed from the pinned release bytes at
/// `UCI_MessageDefinitions_v2_5_0.xsd:145460` (UCI 2.5) and
/// `UCI_MessageDefinitions_v2_6_0.xsd:145708` (UCI 2.6). The two releases are
/// byte-identical for this declaration.
///
/// This is a Rust raw string, so the `\.` and `\-` here are the *same two
/// characters* the XSD `value` attribute contains. It is compared for exact
/// equality and is never parsed as a regular expression.
pub const UCI_SCHEMA_VERSION_PATTERN: &str =
    r"[0-9]{3}\.[0-9]{1,2}(\.[0-9]{1,2})([a-z]{1,2})?(_[a-zA-Z0-9\-]{1,45})?";

/// The authoritative `minLength`, which is also the pattern's own minimum.
pub const UCI_SCHEMA_VERSION_MIN_LENGTH: u64 = 7;

/// The authoritative `maxLength`, which is also the pattern's own maximum.
pub const UCI_SCHEMA_VERSION_MAX_LENGTH: u64 = 57;

/// A named constrained-`string` shape that Task 037 fully implements.
///
/// Each future variant must arrive with its own evidence, its own validator,
/// and its own conformance corpus; this is not a place to accumulate
/// near-misses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringProfile {
    /// The UCI schema-version profile: `minLength 7`, `maxLength 57`, and the
    /// single authoritative dotted-numeric pattern, over `whiteSpace=preserve`.
    ///
    /// Generated code stores the caller's string unchanged once it has been
    /// accepted by both the pattern and the length facets.
    UciSchemaVersion,
}

/// Why a constrained `string` declaration falls outside the Task 037 subset.
///
/// Every variant describes a declaration that *is* a constrained `string`. An
/// unconstrained `string`, or a non-`string` declaration, is reported as
/// `Ok(None)` instead, never as an error -- those keep their existing
/// representation and Task 037 has no opinion about them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringProfileError {
    /// A constrained `string` whose effective facet shape is not the one
    /// supported profile.
    ///
    /// Covers a different pattern text, multiple alternatives in one group,
    /// multiple restriction-level groups, a non-XML-Schema dialect, a `length`
    /// facet, different `minLength`/`maxLength` values, an explicit
    /// `whiteSpace` facet, or any numeric facet. Task 037 enforces the
    /// authoritative profile exactly, so every neighbouring shape fails closed
    /// rather than being approximated by a validator that would ignore a facet.
    UnsupportedConstraints,
}

impl StringProfileError {
    /// A short, stable diagnostic fragment for backend error messages.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::UnsupportedConstraints => {
                "unsupported constrained String declaration: unsupported facet profile"
            }
        }
    }
}

impl std::fmt::Display for StringProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.reason())
    }
}

/// Whether an effective constraint set constrains a `string` at all.
///
/// Keeps "ordinary unconstrained `string`" and "constrained `string`" apart at
/// call sites that only need the coarse question. An unconstrained `string`
/// must keep its existing plain backend representation, so it is never routed
/// through the Task 037 carrier machinery.
#[must_use]
pub fn constrains_string(constraints: &ConstraintSet) -> bool {
    constraints.length.is_some()
        || constraints.min_length.is_some()
        || constraints.max_length.is_some()
        || constraints.lexical.white_space.is_some()
        || !constraints.lexical.pattern_groups.is_empty()
        || constraints.min_inclusive.is_some()
        || constraints.max_inclusive.is_some()
        || constraints.min_exclusive.is_some()
        || constraints.max_exclusive.is_some()
}

/// Classify a named `string` declaration against the Task 037 subset.
///
/// * `Ok(None)` -- not a constrained `string`; Task 037 has no opinion, and the
///   declaration keeps whatever representation it already had.
/// * `Ok(Some(profile))` -- an implemented profile; backends may render a
///   validated carrier for it.
/// * `Err(reason)` -- a constrained `string` outside the implemented subset.
///
/// `constraints` must be the declaration's **effective** constraint set. Task
/// 015 already resolves named restriction chains, so this never re-walks raw
/// XSD; it reads the normalized IR that every backend already holds.
///
/// # Errors
///
/// Returns [`StringProfileError`] when the declaration is a constrained
/// `string` whose effective facet shape is not the one implemented profile. No
/// facet is ever silently ignored: a constraint this module cannot enforce is a
/// rejection, not a warning.
pub fn string_profile(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
) -> Result<Option<StringProfile>, StringProfileError> {
    if kind != PrimitiveKind::String {
        return Ok(None);
    }
    // An ordinary `string` is not a Task 037 concern. This is what keeps
    // existing Rust `String` / C++ `std::string` / Ada `Unbounded_String`
    // output completely unchanged.
    if !constrains_string(constraints) {
        return Ok(None);
    }
    schema_version_profile(constraints).map(Some)
}

/// Decide whether a constrained `string`'s effective facets are the supported
/// profile.
///
/// The shape is checked structurally against the normalized Task 016 IR --
/// facet values, group count, alternative count, dialect, expression text --
/// never against a rendered debug string, which would silently accept a
/// formatting change, and never against the declaration's name.
fn schema_version_profile(
    constraints: &ConstraintSet,
) -> Result<StringProfile, StringProfileError> {
    // Numeric bounds are not applicable to `string` at all; their presence
    // means this IR did not come from a well-formed `string` restriction.
    if constraints.min_inclusive.is_some()
        || constraints.max_inclusive.is_some()
        || constraints.min_exclusive.is_some()
        || constraints.max_exclusive.is_some()
    {
        return Err(StringProfileError::UnsupportedConstraints);
    }
    // `length` is mutually exclusive with min/maxLength in the authoritative
    // profile, and the `length + pattern` family (65 UCI declarations,
    // including the UUID type) is explicitly *not* implemented here.
    if constraints.length.is_some() {
        return Err(StringProfileError::UnsupportedConstraints);
    }
    // The exact authoritative bounds. A same-pattern declaration with
    // different bounds is a different type and fails closed.
    if constraints.min_length != Some(UCI_SCHEMA_VERSION_MIN_LENGTH)
        || constraints.max_length != Some(UCI_SCHEMA_VERSION_MAX_LENGTH)
    {
        return Err(StringProfileError::UnsupportedConstraints);
    }
    // `xs:string`'s intrinsic `whiteSpace` is `preserve` and is *not* fixed, so
    // an explicit facet here would be a real, enforceable narrowing of the
    // lexical space -- exactly the `WhitespaceVisibleString*` family. Task 037
    // implements `preserve` only, so an explicit facet fails closed instead of
    // being assumed harmless.
    if constraints.lexical.white_space.is_some() {
        return Err(StringProfileError::UnsupportedConstraints);
    }
    // Exactly one restriction level, carrying exactly one alternative. Two
    // groups are AND-ed and two alternatives are OR-ed (§4.3.4.3); the
    // equivalence proof in this module's header covers neither, so both fail
    // closed rather than being approximated.
    let [group] = constraints.lexical.pattern_groups.as_slice() else {
        return Err(StringProfileError::UnsupportedConstraints);
    };
    let [alternative] = group.alternatives.as_slice() else {
        return Err(StringProfileError::UnsupportedConstraints);
    };
    if alternative.dialect != PatternDialect::XmlSchema
        || alternative.expression != UCI_SCHEMA_VERSION_PATTERN
    {
        return Err(StringProfileError::UnsupportedConstraints);
    }
    Ok(StringProfile::UciSchemaVersion)
}

/// Whether a schema emits at least one Task 037 String value carrier.
///
/// Mirrors [`crate::temporal::schema_emits_temporal_carrier`]. Several
/// generator decisions are conditional on this and must agree exactly; reading
/// the classifier keeps them all tied to one predicate. A schema containing
/// only *unsupported* constrained strings returns `false`, because those fail
/// closed in the backend and produce no output at all.
#[must_use]
pub fn schema_emits_string_profile_carrier(schema: &ams_gra_oms_ir::SchemaIr) -> bool {
    schema.types.iter().any(|declaration| {
        matches!(declaration.kind, ams_gra_oms_ir::TypeKind::Primitive(kind)
            if string_profile(kind, &declaration.constraints)
                .is_ok_and(|profile| profile.is_some()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::{
        LexicalConstraintSet, NumericValue, PatternExpression, PatternGroup, WhiteSpacePolicy,
    };

    /// The authoritative UCI profile, built from the normalized IR shape.
    fn schema_version() -> ConstraintSet {
        ConstraintSet {
            min_length: Some(UCI_SCHEMA_VERSION_MIN_LENGTH),
            max_length: Some(UCI_SCHEMA_VERSION_MAX_LENGTH),
            lexical: LexicalConstraintSet {
                pattern_groups: vec![PatternGroup {
                    alternatives: vec![PatternExpression::xml_schema(UCI_SCHEMA_VERSION_PATTERN)],
                }],
                white_space: None,
            },
            ..ConstraintSet::default()
        }
    }

    /// The single supported profile, recognized from kind + effective facets.
    /// There is no name parameter at all, which is what keeps Task 037 free of
    /// a UCI local-name special case.
    #[test]
    fn uci_schema_version_is_the_supported_profile() {
        assert_eq!(
            string_profile(PrimitiveKind::String, &schema_version()),
            Ok(Some(StringProfile::UciSchemaVersion))
        );
    }

    /// An ordinary unconstrained `string` is not a Task 037 declaration at all,
    /// and must be reported distinctly from a *wrongly* constrained one so
    /// backends keep emitting their existing plain string representation.
    #[test]
    fn unconstrained_string_is_not_a_task_037_declaration() {
        assert_eq!(
            string_profile(PrimitiveKind::String, &ConstraintSet::default()),
            Ok(None)
        );
        assert!(!constrains_string(&ConstraintSet::default()));
    }

    /// Non-string primitives are outside this classifier entirely.
    #[test]
    fn non_string_primitives_are_not_classified() {
        for kind in [
            PrimitiveKind::Boolean,
            PrimitiveKind::SignedInteger,
            PrimitiveKind::UnsignedInteger,
            PrimitiveKind::Float32,
            PrimitiveKind::Float64,
            PrimitiveKind::DateTime,
            PrimitiveKind::Binary,
        ] {
            assert_eq!(
                string_profile(kind, &schema_version()),
                Ok(None),
                "{kind:?} must not be classified as a String profile"
            );
        }
    }

    /// The conventional-looking four-group spelling is a *different* pattern
    /// and is not the authoritative one. This is the specific mistake the
    /// evidence gate existed to prevent, so it is pinned by a test.
    #[test]
    fn the_four_group_version_pattern_is_not_the_supported_profile() {
        let mut constraints = schema_version();
        constraints.lexical.pattern_groups = vec![PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(
                r"[0-9]{3}\.[0-9]{3}\.[0-9]{3}\.[0-9]{3}",
            )],
        }];
        assert_eq!(
            string_profile(PrimitiveKind::String, &constraints),
            Err(StringProfileError::UnsupportedConstraints)
        );
    }

    /// Same pattern, different bounds: a neighbouring type whose `maxLength`
    /// the Task 037 carrier would not enforce. It must fail closed rather than
    /// silently losing the facet.
    #[test]
    fn the_same_pattern_with_different_bounds_is_unsupported() {
        for (min, max) in [(Some(7), Some(20)), (Some(1), Some(57)), (None, Some(57))] {
            let mut constraints = schema_version();
            constraints.min_length = min;
            constraints.max_length = max;
            assert_eq!(
                string_profile(PrimitiveKind::String, &constraints),
                Err(StringProfileError::UnsupportedConstraints),
                "minLength={min:?} maxLength={max:?} must fail closed"
            );
        }
    }

    /// The `length + pattern` family, which includes the UUID type, is the
    /// largest unimplemented String family and stays closed.
    #[test]
    fn a_length_facet_is_unsupported() {
        let mut constraints = schema_version();
        constraints.length = Some(36);
        assert_eq!(
            string_profile(PrimitiveKind::String, &constraints),
            Err(StringProfileError::UnsupportedConstraints)
        );
    }

    /// An explicit `whiteSpace` facet is a real narrowing for `string`, and the
    /// `WhitespaceVisibleString*` family depends on it. Not implemented.
    #[test]
    fn an_explicit_white_space_facet_is_unsupported() {
        for policy in [
            WhiteSpacePolicy::Preserve,
            WhiteSpacePolicy::Replace,
            WhiteSpacePolicy::Collapse,
        ] {
            let mut constraints = schema_version();
            constraints.lexical.white_space = Some(policy);
            assert_eq!(
                string_profile(PrimitiveKind::String, &constraints),
                Err(StringProfileError::UnsupportedConstraints),
                "{policy:?} must fail closed"
            );
        }
    }

    /// Two groups are AND-ed and two alternatives are OR-ed. The equivalence
    /// proof covers neither, so both fail closed.
    #[test]
    fn multiple_groups_or_alternatives_are_unsupported() {
        let mut two_groups = schema_version();
        two_groups.lexical.pattern_groups.push(PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(UCI_SCHEMA_VERSION_PATTERN)],
        });
        assert_eq!(
            string_profile(PrimitiveKind::String, &two_groups),
            Err(StringProfileError::UnsupportedConstraints)
        );

        let mut two_alternatives = schema_version();
        two_alternatives.lexical.pattern_groups[0]
            .alternatives
            .push(PatternExpression::xml_schema(UCI_SCHEMA_VERSION_PATTERN));
        assert_eq!(
            string_profile(PrimitiveKind::String, &two_alternatives),
            Err(StringProfileError::UnsupportedConstraints)
        );
    }

    /// A `string` carrying no pattern but real length facets -- the
    /// `minLength`/`maxLength`-only family -- is constrained yet unsupported.
    #[test]
    fn length_bounds_without_a_pattern_are_unsupported() {
        let mut constraints = schema_version();
        constraints.lexical.pattern_groups.clear();
        assert!(constrains_string(&constraints));
        assert_eq!(
            string_profile(PrimitiveKind::String, &constraints),
            Err(StringProfileError::UnsupportedConstraints)
        );
    }

    /// Numeric facets are not applicable to `string`; their presence means the
    /// IR is not a well-formed string restriction.
    #[test]
    fn numeric_facets_on_a_string_are_unsupported() {
        let mut constraints = schema_version();
        constraints.min_inclusive = Some(NumericValue::Integer(0));
        assert_eq!(
            string_profile(PrimitiveKind::String, &constraints),
            Err(StringProfileError::UnsupportedConstraints)
        );
    }

    /// Every accepted character is ASCII, which is what licenses the generated
    /// validators to count bytes for a facet XSD defines over characters.
    #[test]
    fn the_supported_alphabet_is_entirely_ascii() {
        // The union of every character class in the authoritative pattern.
        let alphabet: String = ('0'..='9')
            .chain('a'..='z')
            .chain('A'..='Z')
            .chain(['.', '_', '-'])
            .collect();
        assert!(
            alphabet.is_ascii(),
            "byte counting is only XSD-correct because the alphabet is ASCII"
        );
        assert_eq!(alphabet.len(), alphabet.chars().count());
    }

    /// The declared facets coincide exactly with the pattern's own bounds.
    /// They are still enforced; this pins the arithmetic behind that claim.
    #[test]
    fn the_declared_bounds_match_the_patterns_own_bounds() {
        let pattern_min: u64 = 3 + 1 + 1 + (1 + 1);
        let pattern_max: u64 = 3 + 1 + 2 + (1 + 2) + 2 + (1 + 45);
        assert_eq!(pattern_min, UCI_SCHEMA_VERSION_MIN_LENGTH);
        assert_eq!(pattern_max, UCI_SCHEMA_VERSION_MAX_LENGTH);
    }
}
