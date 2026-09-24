//! Shared semantics for named constrained XML Schema `string` declarations.
//!
//! This module answers exactly one question, once, for `backend-ada`,
//! `backend-rust`, `backend-cpp`, and `CoverageAnalysis`:
//!
//! > Is this named declaration a constrained `string` declaration that the
//! > generator fully implements?
//!
//! It is the String-side analogue of Task 036's [`crate::temporal`]
//! classifier, and exists for the same reason: four independent readings of
//! the same facet set is precisely how three backends drift into disagreeing
//! about which declarations are renderable while coverage measures a fourth
//! opinion. Every consumer calls [`string_profile`].
//!
//! # The supported profiles
//!
//! Four, each recognized by an exact effective facet shape:
//!
//! * [`StringProfile::UciSchemaVersion`] (Task 037) -- see below;
//! * [`StringProfile::UniversallyUniqueIdentifier`] (Task 038) -- see the
//!   dedicated section further down;
//! * [`StringProfile::VisibleAscii`] (Task 039) -- the parameterized
//!   visible-ASCII bounded-string family, see its own section last;
//! * [`StringProfile::WhitespaceVisible`] (Task 041) -- the whitespace-visible
//!   bounded-string family, whose class adds LF and CR and whose effective
//!   `whiteSpace` policy is part of the profile.
//!
//! None is recognized by declaration name, and a constrained `string` matching
//! none of the four shapes fails closed.
//!
//! # The two pinned releases are not equivalent here
//!
//! Task 041's family is the first place where the same UCI declaration name
//! carries a materially different *value space* in the two tracked releases.
//! UCI 2.5's `WhitespaceVisibleString1024Type` restricts with
//! `whiteSpace = collapse` and `minLength = 0`; UCI 2.6's carries no
//! `whiteSpace` facet at all and `minLength = 1`. Both are supported, as
//! separate profile triples, and the collapse half genuinely normalizes the
//! constructor's input before storing it. Nothing infers one release's shape
//! from the other's, and nothing infers the `minLength` facet from the
//! pattern's own quantifier -- both are read, and their agreement is asserted.
//!
//! `QueryString4096Type` falls into this family in both releases purely because
//! its facets match, which is the intended behaviour of a name-free classifier.
//!
//! # The Task 037 schema-version profile
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
//! # The Task 038 UUID profile
//!
//! [`StringProfile::UniversallyUniqueIdentifier`]: a [`PrimitiveKind::String`]
//! declaration whose *effective* [`ConstraintSet`] carries exactly
//! `length = 36`, no `minLength`, no `maxLength`, no explicit `whiteSpace`, and
//! exactly one pattern group holding exactly one XML-Schema-dialect alternative
//! whose expression is [`UCI_UUID_PATTERN`] verbatim.
//!
//! Read from the pinned release bytes, which are byte-identical for this
//! declaration after end-of-line normalization:
//!
//! ```xml
//! <xs:simpleType name="UniversallyUniqueIdentifierType" uci:version="000.001.000.000">
//!   <xs:annotation>
//!     <xs:documentation>A UUID is a 128-bit number (32 hexadecimal digits, 16
//!     bytes) that is conformant to any version of variant 1 or nil UUID, as
//!     described in IETF RFC 4122.</xs:documentation>
//!   </xs:annotation>
//!   <xs:restriction base="xs:string">
//!     <xs:length value="36"/>
//!     <xs:pattern value="(0{8}(-0{4}){3}-0{12})|([a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[1-5][a-fA-F0-9]{3}-[89abAB][a-fA-F0-9]{3}-[a-fA-F0-9]{12})"/>
//!   </xs:restriction>
//! </xs:simpleType>
//! ```
//!
//! * UCI 2.5 `093610b7753944059360d3236770ab446d039556`:
//!   `UCI_MessageDefinitions_v2_5_0.xsd:145719`
//! * UCI 2.6 `78eb61b6112c8bffa40820c33124b57787fc5bd9`:
//!   `UCI_MessageDefinitions_v2_6_0.xsd:145967`
//!
//! Restriction-chain depth is 1: the immediate and only base is the primitive
//! `xs:string`.
//!
//! ## One facet, one expression, internal alternation
//!
//! The schema contains **one** `<xs:pattern>` element, so the normalized IR is
//! one pattern group holding **one** pattern expression. The `|` lives *inside*
//! that single expression.
//!
//! This matters because Task 016's IR represents multiple `xs:pattern` facets
//! at one restriction level as several alternatives in one group. That
//! machinery is **not** involved here, and a classifier demanding two
//! alternatives would reject the authoritative declaration outright. An earlier
//! inventory abbreviated this profile behind an ellipsis, which suggested a
//! two-alternative shape; that reading is corrected here against the pinned
//! bytes.
//!
//! ## The two internal branches
//!
//! ```text
//! A   0{8}(-0{4}){3}-0{12}
//!     the nil UUID, and nothing else
//!
//! B   [a-fA-F0-9]{8} - [a-fA-F0-9]{4} - [1-5][a-fA-F0-9]{3}
//!                    - [89abAB][a-fA-F0-9]{3} - [a-fA-F0-9]{12}
//! ```
//!
//! Branch A denotes exactly one string, `00000000-0000-0000-0000-000000000000`,
//! because every atom is the literal `0` under a fixed quantifier.
//!
//! ## Branch A is *not* redundant
//!
//! It would be tempting to drop branch A on the grounds that `0` is a
//! hexadecimal digit, making the nil UUID an instance of branch B. It is not.
//! Branch B constrains two positions beyond plain hexadecimal:
//!
//! * the version nibble must be `[1-5]`, and nil's is `0`;
//! * the variant nibble must be `[89abAB]`, and nil's is `0`.
//!
//! So branch B *rejects* the nil UUID, and branch A contributes exactly one
//! value that is otherwise unreachable. The union must be implemented
//! explicitly; collapsing it to branch B alone would reject the nil UUID, and
//! collapsing it to a general 8-4-4-4-12 hexadecimal shape would accept values
//! the schema forbids.
//!
//! ## Version and variant are schema constraints, not imported RFC policy
//!
//! `[1-5]` and `[89abAB]` are present in the authoritative pattern text, which
//! matches the declaration's own documentation ("conformant to any version of
//! variant 1 or nil UUID"). Enforcing them is implementing the schema, not
//! importing RFC rules. Nothing *beyond* them is imposed: no RFC 9562 v6/v7/v8
//! policy, no canonical-lowercase rule, no URN or brace syntax, no nil
//! prohibition, and no conversion to a 128-bit integer.
//!
//! ## Why no regular-expression engine
//!
//! Both branches are fixed-width concatenations of character classes and
//! literal hyphens, with no optional piece, no unbounded repetition, and no
//! backtracking: every position's class is determined by its index alone.
//! Deciding the expression is therefore a bounded positional test, implemented
//! by the generated validators as:
//!
//! ```text
//! length must be 36
//! the exact nil literal is accepted outright                     (branch A)
//! otherwise:                                                     (branch B)
//!   indexes 8, 13, 18, 23 must be '-'
//!   every other index must be [0-9a-fA-F]
//!   index 14 must be [1-5]
//!   index 19 must be [89abAB]
//! ```
//!
//! The group widths `8-4-4-4-12` place the third group at indexes 14..17 and
//! the fourth at 19..22, so index 14 is the version nibble and index 19 the
//! variant nibble. XML Schema patterns are anchored (§4.3.4.3), which the fixed
//! length requirement enforces directly.
//!
//! ## `length` is enforced independently
//!
//! Both branches are exactly 36 characters wide, so the facet is formally
//! redundant for *this* expression. It is nevertheless required exactly by the
//! classifier and checked explicitly by the validators, for the same reason the
//! schema-version bounds are: recognizing a profile by pattern alone would
//! silently accept a neighbouring declaration carrying the same pattern with a
//! different `length`, and then generate a carrier that ignores that facet.
//!
//! ## Character counting
//!
//! The accepted alphabet is exactly `0-9`, `a-f`, `A-F`, and `-`, all ASCII.
//! Every accepted value is therefore pure ASCII and its UTF-8 byte count equals
//! its XSD character count, which is what licenses the generated Rust and C++
//! validators to use byte length for a facet XSD defines over characters. Ada's
//! `String'Length` is a character count already. This proof is specific to this
//! profile and is re-derived, not assumed, for any future one.
//!
//! ## Case is preserved
//!
//! `[a-fA-F0-9]` admits both letter cases, and `[89abAB]` admits `a`/`b` and
//! `A`/`B`. Accepted values are stored exactly as supplied: this remains an
//! `xs:string` carrier, whose value equality is equality of the stored text, so
//! two otherwise-valid UUIDs differing only in letter case are *distinct*
//! values. No case-insensitive comparison is introduced.
//!
//! ## `whiteSpace`
//!
//! Like the schema-version profile, this declaration adds no `whiteSpace`
//! facet, so `xs:string`'s intrinsic `preserve` applies and normalization is
//! the identity. No whitespace character appears in either branch, so a value
//! carrying leading, trailing, or interior whitespace is rejected rather than
//! repaired.
//!
//! # The Task 039 visible-ASCII family
//!
//! [`StringProfile::VisibleAscii`] is the first *parameterized* profile, and
//! the parameterization was forced by evidence rather than chosen for
//! generality. The pinned bytes of both tracked releases contain thirteen
//! `string` declarations whose effective constraints are
//! `minLength = M`, `maxLength = N`, no `length`, no explicit `whiteSpace`,
//! and one XML-Schema pattern `[ -~]{M,N}` with the *same* `M` and `N`:
//!
//! ```text
//! AttributedURI_Type      1..256     VisibleString256Type    1..256
//! MIME_Type               1..256     VisibleString2_4Type    2..4
//! MissionCategoryType     1..32      VisibleString32Type     1..32
//! VisibleString20Type     1..20      VisibleString480Type    1..480
//! VisibleString64Type     1..64      VisibleString512Type    1..512
//! VisibleString81Type     1..81      VisibleString1024Type   1..1024
//! VisibleString128Type    1..128
//! ```
//!
//! Two of those names contain no "VisibleString" at all, and `MissionCategoryType`
//! reaches the shape only through a depth-2 restriction of `VisibleString32Type`
//! that adds no facets. Membership is decided from effective facets alone, so
//! all thirteen are recognized and none of them is recognized by name.
//!
//! ## The parameterization is bounded by evidence, not open-ended
//!
//! Those thirteen declarations carry only **ten unique effective bound pairs**,
//! because several share a shape:
//!
//! ```text
//! 1..20   1..32   1..64   1..81   1..128
//! 1..256  1..480  1..512  1..1024  2..4
//! ```
//!
//! Those ten pairs, listed in `UCI_VISIBLE_ASCII_BOUNDS`, are the *entire*
//! accepted parameter domain. This profile is deliberately **not** a general
//! visible-ASCII datatype facility: an arbitrary `[ -~]{M,N}` whose `minLength`
//! and `maxLength` facets agree with its quantifier is still **unsupported**
//! unless `(M, N)` is one of the ten. A synthetic `3..17`, and a synthetic
//! `1..18446744073709551615`, both fail closed with
//! [`StringProfileError::UnsupportedConstraints`].
//!
//! The restriction is not conservatism for its own sake. Three reasons:
//!
//! * **no authoritative evidence.** Every other profile here is pinned to the
//!   bytes of the tracked releases; a bound pair no release contains has no
//!   conformance corpus and no measured declaration behind it.
//! * **baseline support must be compiler-backed.** Claiming a declaration is
//!   baseline-renderable is a claim that the *generated* code compiles in every
//!   claimed backend. The ten accepted pairs are all small and are exercised by
//!   GNAT-, rustc-, and g++-backed tests.
//! * **unrestricted `u64` bounds would exceed backend literal and host-size
//!   assumptions.** The backends emit the bounds as length constants --
//!   C++ `static constexpr std::size_t kMaxLength`, Rust `const MAX_LENGTH: usize`,
//!   Ada `Max_Length : constant` -- and a `u64::MAX` literal is not something
//!   this task established as portable under `-std=c++17 -Wall -Wextra
//!   -pedantic-errors`. Widening the domain is a future task with its own
//!   evidence and its own compile-backed tests, not a side effect of this one.
//!
//! Membership in that table is purely semantic. A differently named declaration
//! carrying one of the ten exact profiles classifies; a UCI-looking local name
//! carrying unobserved bounds does not.
//!
//! The selected blocker, `VisibleString256Type`, is byte-identical in both
//! releases after end-of-line normalization:
//!
//! ```xml
//! <xs:simpleType name="VisibleString256Type" uci:version="000.001.000.000">
//!   <xs:annotation>
//!     <xs:documentation>A string representing up to 256 characters in length, restricted to visible characters (0x20-0x7E).</xs:documentation>
//!   </xs:annotation>
//!   <xs:restriction base="xs:string">
//!     <xs:minLength value="1"/>
//!     <xs:maxLength value="256"/>
//!     <xs:pattern value="[&#x20;-&#x7E;]{1,256}"/>
//!   </xs:restriction>
//! </xs:simpleType>
//! ```
//!
//! * UCI 2.5 `093610b7753944059360d3236770ab446d039556`:
//!   `UCI_MessageDefinitions_v2_5_0.xsd:146232`
//! * UCI 2.6 `78eb61b6112c8bffa40820c33124b57787fc5bd9`:
//!   `UCI_MessageDefinitions_v2_6_0.xsd:146583`
//!
//! ## The pattern source text uses character references
//!
//! The XSD spells the class `[&#x20;-&#x7E;]`. Those are XML character
//! references, expanded by the parser before any schema processing, so the
//! normalized IR expression is the five characters `[ -~]` with a literal
//! SPACE and a literal TILDE. Reading the raw file and reading the IR
//! therefore disagree textually while agreeing semantically, which is why the
//! expected text here is built from [`visible_ascii_pattern`] and confirmed
//! against the frontend's own output over the real pinned roots.
//!
//! ## The character range
//!
//! `[ -~]` is a single XML Schema character range from U+0020 SPACE to U+007E
//! TILDE inclusive. It admits SPACE, every ASCII punctuation mark, the digits,
//! both letter cases, and `{ | } ~`. It excludes TAB (U+0009), LF (U+000A), CR
//! (U+000D), every other C0 control, DEL (U+007F), and every non-ASCII
//! character. The generated validators test that ordinal interval explicitly
//! and never call a locale-sensitive "printable" classifier, which would
//! disagree on both DEL and the high half of Latin-1 under some locales.
//!
//! ## SPACE is valid, and is never trimmed
//!
//! The base is `xs:string`, whose intrinsic `whiteSpace` is `preserve`, and
//! this restriction adds no `whiteSpace` facet, so normalization is the
//! identity. SPACE is itself a member of the pattern's class. Leading,
//! trailing, interior, and all-space values are therefore **valid** whenever
//! their lengths fit, and are stored exactly as supplied. This is the opposite
//! of the Task 037 and 038 profiles, whose alphabets excluded whitespace
//! entirely, so their corpus expectations must not be copied here. TAB, LF, and
//! CR are different characters and still fail.
//!
//! ## Character counting
//!
//! Every accepted character has a code point at most U+007E, so every accepted
//! value is pure ASCII and its UTF-8 byte count equals its XSD character
//! count. That licenses the generated Rust and C++ validators to use byte
//! length for a facet XML Schema defines over characters; Ada's `String'Length`
//! is a character count already. The argument is re-derived per profile from
//! its own alphabet and is not general.
//!
//! ## Both the bounds and the pattern are enforced
//!
//! For every member the quantifier and the facets are semantically redundant.
//! Both are still required by the classifier and checked by the validators. A
//! declaration carrying this pattern under different bounds, or these bounds
//! under a different pattern, is a different type and fails closed.
//!
//! ## Neighbours that are deliberately not members
//!
//! * the `[ -~]{N}` fixed-`length` declarations, such as
//!   `VisibleStringLength10Type` and the `NITF_*` family: a different facet
//!   shape and a different quantifier form;
//! * `QueryString4096Type`, whose class `[ -~\n\r]` additionally admits LF and
//!   CR;
//! * the `WhitespaceVisibleString*` family, which carries an explicit
//!   `whiteSpace = collapse` requiring a separate normalization analysis;
//! * `NATO_SpecialWordsType`, a distinct `NATO:[a-zA-Z\-_]{1,256}` lexical
//!   profile that happens to be ASCII-only.
//!
//! # What this classifier does not decide
//!
//! It says nothing about *direct* `TypeRefTarget::Primitive(String)` fields, or
//! about field-local anonymous restrictions. These are **named declaration**
//! slices: the carrier is emitted for a named declaration, and an ordinary
//! unconstrained `string` keeps its existing plain representation in every
//! backend.

use ams_gra_oms_ir::{ConstraintSet, PatternDialect, PrimitiveKind, WhiteSpacePolicy};

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

/// The authoritative UCI UUID lexical restriction, verbatim.
///
/// Reconfirmed from the pinned release bytes at
/// `UCI_MessageDefinitions_v2_5_0.xsd:145719` (UCI 2.5) and
/// `UCI_MessageDefinitions_v2_6_0.xsd:145967` (UCI 2.6), which are
/// byte-identical for this declaration after end-of-line normalization.
///
/// This is the text of the declaration's **single** `<xs:pattern>` facet, and
/// therefore of the single `PatternExpression` in the normalized IR. The `|` is
/// internal to this one expression; it is not two IR alternatives. The string
/// is compared for exact equality and is never parsed as a regular expression.
pub const UCI_UUID_PATTERN: &str = "(0{8}(-0{4}){3}-0{12})|([a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[1-5][a-fA-F0-9]{3}-[89abAB][a-fA-F0-9]{3}-[a-fA-F0-9]{12})";

/// The authoritative `length` facet: exactly 36 characters.
///
/// Both internal branches are 36 characters wide, so this is formally implied
/// by the pattern. It is required exactly by the classifier and enforced
/// explicitly by the generated validators regardless, so a neighbouring
/// declaration carrying the same pattern under a different `length` cannot be
/// mistaken for this profile.
pub const UCI_UUID_LENGTH: u64 = 36;

/// The one value accepted by the pattern's nil branch, and by nothing else.
///
/// The general branch rejects it, because its version nibble is `0` (outside
/// `[1-5]`) and its variant nibble is `0` (outside `[89abAB]`). The nil branch
/// is therefore load-bearing rather than redundant.
pub const UCI_UUID_NIL: &str = "00000000-0000-0000-0000-000000000000";

/// The whitespace-visible family's character-class source text, verbatim.
///
/// # Three distinct layers of escaping
///
/// The pinned XSD spells this facet
/// `<xs:pattern value="[&#x20;-&#x7E;\n\r]{0,1024}"/>`, and exactly one of the
/// three escaping layers involved is the XML parser's:
///
/// 1. **XML character references.** `&#x20;` and `&#x7E;` are expanded *by the
///    XML parser*, before any schema component exists. The attribute value the
///    frontend receives already contains a literal SPACE and a literal TILDE.
/// 2. **XSD regex backslash escapes.** `\n` and `\r` are **not** XML escapes.
///    The XML parser passes them through as the two characters BACKSLASH, `n`
///    and BACKSLASH, `r`; they are escapes of the *XML Schema regular
///    expression* dialect (§G.1.1 `SingleCharEsc`), which denote LF and CR to a
///    regex reader. The normalized IR expression therefore holds them
///    literally, which is why this Rust raw string spells them the same way.
/// 3. **Decoded constructor input.** The generated `Create` / `new` / `create`
///    receives already-decoded text: an actual U+000A, not a backslash and an
///    `n`. The validators test code points, never escape spellings.
///
/// So this constant is nine characters -- `[`, SPACE, `-`, `~`, `\`, `n`, `\`,
/// `r`, `]` -- and is compared for exact equality. It is never parsed as a
/// regular expression, and it is never searched for as a substring.
const WHITESPACE_VISIBLE_CLASS: &str = r"[ -~\n\r]";

/// The whitespace-visible class's printable interval: U+0020 SPACE.
///
/// Identical to the visible-ASCII lower bound. The whitespace-visible class is
/// that same interval *plus* the two explicitly escaped characters below.
pub const WHITESPACE_VISIBLE_MIN_CODE_POINT: u8 = 0x20;

/// The whitespace-visible class's printable interval: U+007E TILDE.
pub const WHITESPACE_VISIBLE_MAX_CODE_POINT: u8 = 0x7E;

/// U+000A LINE FEED: admitted by the class through the regex escape `\n`.
pub const WHITESPACE_VISIBLE_LINE_FEED: u8 = 0x0A;

/// U+000D CARRIAGE RETURN: admitted by the class through the regex escape `\r`.
pub const WHITESPACE_VISIBLE_CARRIAGE_RETURN: u8 = 0x0D;

/// The exact XML Schema expression a whitespace-visible declaration must carry
/// for the given bounds.
///
/// As with [`visible_ascii_pattern`], the expected text is *derived from the
/// facets* so that a declaration whose `maxLength` disagrees with its own
/// quantifier cannot match. Disagreement is unrepresentable rather than merely
/// unchecked.
#[must_use]
pub fn whitespace_visible_pattern(min_length: u64, max_length: u64) -> String {
    format!("{WHITESPACE_VISIBLE_CLASS}{{{min_length},{max_length}}}")
}

/// How a whitespace-visible declaration's effective `whiteSpace` policy treats
/// the decoded constructor input.
///
/// This is the axis on which the two pinned releases genuinely differ, and it
/// changes the *stored value*, not merely which inputs are accepted. It is
/// therefore part of the profile rather than a rendering detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhitespaceVisiblePolicy {
    /// An explicit `<xs:whiteSpace value="collapse"/>` facet, as UCI 2.5
    /// carries on its whitespace-visible pair.
    ///
    /// Generated code applies the §4.3.6 `collapse` algorithm to the decoded
    /// input *before* the pattern and length facets are applied, and stores the
    /// normalized result.
    Collapse,

    /// No explicit `whiteSpace` facet, so `xs:string`'s intrinsic `preserve`
    /// applies -- as all of UCI 2.6, and UCI 2.5's `QueryString4096Type`, carry.
    ///
    /// Generated code validates and stores the caller's decoded text
    /// unchanged. LF and CR are ordinary members of the class and remain
    /// significant; TAB is outside the class and is rejected.
    Preserve,
}

impl WhitespaceVisiblePolicy {
    /// The IR facet value this policy corresponds to, for exact matching.
    ///
    /// `Preserve` maps to `None`, because the authoritative preserve-side
    /// declarations carry *no* `whiteSpace` facet at all. An explicit
    /// `<xs:whiteSpace value="preserve"/>` is a different declaration shape and
    /// is deliberately not admitted: it has not been observed, and admitting it
    /// would mean the classifier no longer distinguishes "absent" from
    /// "explicitly restated".
    #[must_use]
    const fn expected_facet(self) -> Option<WhiteSpacePolicy> {
        match self {
            Self::Collapse => Some(WhiteSpacePolicy::Collapse),
            Self::Preserve => None,
        }
    }

    /// Whether this policy normalizes its input before validating it.
    #[must_use]
    pub const fn normalizes(self) -> bool {
        matches!(self, Self::Collapse)
    }

    /// A short, stable word for generated documentation.
    ///
    /// Deliberately not the phrase "exactly as supplied", which is true only of
    /// the preserve half of this family.
    #[must_use]
    pub const fn stored_value_word(self) -> &'static str {
        match self {
            Self::Collapse => "normalized",
            Self::Preserve => "preserved",
        }
    }
}

/// Every `(policy, minLength, maxLength)` triple the authoritative
/// whitespace-visible family is observed to carry, across both pinned releases.
///
/// # Why whole triples rather than three independent axes
///
/// The two releases are **not** equivalent here, and the observed values do not
/// form a product. UCI 2.5's whitespace-visible pair combines `collapse` with a
/// zero minimum; UCI 2.6 drops the facet entirely and raises the minimum to
/// one. Listing the axes separately -- policies `{collapse, preserve}` x minima
/// `{0, 1}` x maxima `{1024, 4096}` -- would admit eight profiles, three of
/// which no pinned release contains. Each entry here is one whole observed
/// declaration shape.
///
/// # The inventory, from the pinned bytes
///
/// | Release | Declaration | File:line | `whiteSpace` | min | max | quantifier |
/// | --- | --- | --- | --- | ---: | ---: | --- |
/// | 2.5 | `WhitespaceVisibleString1024Type` | `UCI_SecurityMarkings_v2_5_0.xsd:8565` | `collapse` | 0 | 1024 | `{0,1024}` |
/// | 2.5 | `WhitespaceVisibleString4096Type` | `UCI_SecurityMarkings_v2_5_0.xsd:8576` | `collapse` | 0 | 4096 | `{0,4096}` |
/// | 2.5 | `QueryString4096Type` | `UCI_MessageDefinitions_v2_5_0.xsd:138224` | absent | 0 | 4096 | `{0,4096}` |
/// | 2.6 | `WhitespaceVisibleString1024Type` | `UCI_SecurityMarkings_v2_6_0.xsd:8578` | absent | 1 | 1024 | `{1,1024}` |
/// | 2.6 | `WhitespaceVisibleString4096Type` | `UCI_SecurityMarkings_v2_6_0.xsd:8588` | absent | 1 | 4096 | `{1,4096}` |
/// | 2.6 | `QueryString4096Type` | `UCI_MessageDefinitions_v2_6_0.xsd:138589` | absent | 1 | 4096 | `{1,4096}` |
///
/// Six declarations collapse onto **five** distinct triples, because UCI 2.6's
/// `QueryString4096Type` and `WhitespaceVisibleString4096Type` are the same
/// shape. `QueryString4096Type` is a genuine semantic neighbour, admitted
/// because its *facets* match and not because of its name -- and in UCI 2.5 it
/// is a *different* shape from its own 2.6 self, carrying no `whiteSpace` facet
/// where the 2.5 whitespace-visible pair carries `collapse`.
///
/// The `minLength` facet and the quantifier's lower bound are read
/// **independently** and then required to agree; neither is inferred from the
/// other. In both releases they happen to agree, and that agreement is an
/// asserted observation rather than an assumption.
///
/// Every entry is one release's observed evidence. `(Collapse, 1, 1024)`,
/// `(Preserve, 0, 1024)`, and an enormous agreeing pair such as
/// `(Preserve, 1, u64::MAX)` are all absent, and all fail closed.
///
/// Kept in deterministic order so the set is diffable and the enumerating test
/// reads as an inventory.
const UCI_WHITESPACE_VISIBLE_PROFILES: &[(WhitespaceVisiblePolicy, u64, u64)] = &[
    // UCI 2.5 whitespace-visible: explicit collapse, zero minimum.
    (WhitespaceVisiblePolicy::Collapse, 0, 1024),
    (WhitespaceVisiblePolicy::Collapse, 0, 4096),
    // UCI 2.5 `QueryString4096Type`: no whiteSpace facet, zero minimum.
    (WhitespaceVisiblePolicy::Preserve, 0, 4096),
    // UCI 2.6, all three declarations: no whiteSpace facet, minimum of one.
    (WhitespaceVisiblePolicy::Preserve, 1, 1024),
    (WhitespaceVisiblePolicy::Preserve, 1, 4096),
];

/// Whether this whole triple is one the authoritative family actually shows.
///
/// Semantic membership only: compared against observed evidence, never against
/// a declaration name, a file name, a schema version, or a restriction depth.
#[must_use]
fn is_authoritative_whitespace_visible_profile(
    white_space: WhitespaceVisiblePolicy,
    min_length: u64,
    max_length: u64,
) -> bool {
    UCI_WHITESPACE_VISIBLE_PROFILES.contains(&(white_space, min_length, max_length))
}

/// The visible-ASCII family's character-class source text, verbatim.
///
/// The pinned XSD spells this with character references, `[&#x20;-&#x7E;]`,
/// which the XML parser expands before the IR ever sees them; the normalized
/// `PatternExpression` therefore carries the two literal characters SPACE and
/// TILDE, as reconfirmed by running the frontend over both pinned roots.
///
/// Used only to *build* the expected expression for a given bound pair in
/// [`visible_ascii_pattern`]. It is never searched for as a substring: a
/// declaration is a family member because its whole expression equals the
/// expression its own facets imply, not because it happens to mention `[ -~]`.
const VISIBLE_ASCII_CLASS: &str = "[ -~]";

/// The lowest code point the visible-ASCII class admits: U+0020 SPACE.
///
/// SPACE is an ordinary member of this class, not a delimiter. See the
/// module-level discussion of `whiteSpace = preserve`.
pub const VISIBLE_ASCII_MIN_CODE_POINT: u8 = 0x20;

/// The highest code point the visible-ASCII class admits: U+007E TILDE.
///
/// U+007F DELETE is excluded, which is what makes this interval exactly
/// `[ -~]` rather than an informal "printable ASCII" notion.
pub const VISIBLE_ASCII_MAX_CODE_POINT: u8 = 0x7E;

/// The exact XML Schema expression a visible-ASCII declaration must carry for
/// the given bounds.
///
/// The family's defining property is that the pattern quantifier and the
/// length facets *agree*. Deriving the expected text from the facets, instead
/// of comparing them independently, makes disagreement unrepresentable: a
/// declaration whose `maxLength` is 128 while its pattern says `{1,256}` does
/// not match the expression this builds for `1,128`, and fails closed.
#[must_use]
pub fn visible_ascii_pattern(min_length: u64, max_length: u64) -> String {
    format!("{VISIBLE_ASCII_CLASS}{{{min_length},{max_length}}}")
}

/// Every `(minLength, maxLength)` pair the authoritative visible-ASCII family
/// is actually observed to carry, in both pinned releases.
///
/// These are the **unique effective bound pairs**, not one entry per
/// declaration: the thirteen family members of UCI 2.5 and 2.6 collapse onto
/// these ten profiles because several declarations share a shape. `1..256` is
/// carried by `AttributedURI_Type`, `MIME_Type`, and `VisibleString256Type`
/// alike, and `1..32` by both `VisibleString32Type` and the depth-2
/// `MissionCategoryType`. Every pair occurs in *both* tracked releases, so no
/// entry here is release-specific.
///
/// Nothing in this table is derived from a declaration's name. A member is
/// matched entirely on its effective facets, so a differently named
/// declaration carrying one of these exact profiles classifies, and a
/// UCI-looking name carrying unobserved bounds does not.
///
/// The table exists because "baseline-renderable" is a claim about *generated*
/// code. A bound pair only belongs here once the pinned evidence shows it, and
/// each accepted pair is small enough that the Ada, Rust, and C++ length
/// constants are representable and compiler-backed. Admitting arbitrary `u64`
/// bounds would let coverage claim support for declarations whose emitted
/// length literals are not safely representable in every claimed backend.
///
/// Kept in deterministic ascending order so the set is diffable and so the
/// enumerating test reads as an inventory.
const UCI_VISIBLE_ASCII_BOUNDS: &[(u64, u64)] = &[
    (1, 20),
    (1, 32),
    (1, 64),
    (1, 81),
    (1, 128),
    (1, 256),
    (1, 480),
    (1, 512),
    (1, 1024),
    (2, 4),
];

/// Whether this bound pair is one the authoritative UCI family actually shows.
///
/// Semantic membership only: the pair is compared against observed evidence,
/// never against a declaration name or a restriction depth.
#[must_use]
fn is_authoritative_visible_ascii_bounds(min_length: u64, max_length: u64) -> bool {
    UCI_VISIBLE_ASCII_BOUNDS.contains(&(min_length, max_length))
}

/// A named constrained-`string` shape that the generator fully implements.
///
/// Each variant arrives with its own evidence, its own validator, and its own
/// conformance corpus; this is not a place to accumulate near-misses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringProfile {
    /// The UCI schema-version profile: `minLength 7`, `maxLength 57`, and the
    /// single authoritative dotted-numeric pattern, over `whiteSpace=preserve`.
    ///
    /// Generated code stores the caller's string unchanged once it has been
    /// accepted by both the pattern and the length facets.
    UciSchemaVersion,

    /// The UCI UUID profile: `length 36` plus the single authoritative pattern
    /// whose internal `|` separates the exact nil UUID from a
    /// version-`[1-5]` / variant-`[89abAB]` hexadecimal form, over
    /// `whiteSpace=preserve`.
    ///
    /// Generated code stores the caller's string unchanged, preserving
    /// hexadecimal letter case, once it has been accepted by the pattern and
    /// the `length` facet.
    UniversallyUniqueIdentifier,

    /// The UCI visible-ASCII bounded-string family: `minLength = min_length`,
    /// `maxLength = max_length`, and the single pattern `[ -~]{min,max}` whose
    /// quantifier agrees with those facets, over `whiteSpace = preserve`.
    ///
    /// Unlike the two profiles above this one is **parameterized**, because
    /// the pinned bytes of both tracked releases contain thirteen declarations
    /// that differ from one another *only* in the bound pair. Admitting the
    /// family costs one pair of integers; refusing it would have meant
    /// thirteen hand-written near-duplicate variants.
    ///
    /// Membership is still exact. The bounds are not free parameters over
    /// which any pattern is tolerated: the expression must equal
    /// [`visible_ascii_pattern`] for *these* bounds, so the facets and the
    /// quantifier can never disagree.
    ///
    /// Generated code stores the caller's string unchanged once it has been
    /// accepted by both the character range and the length facets. Ordinary
    /// SPACE is inside the class, so leading, trailing, and all-space values
    /// are valid when their lengths fit.
    VisibleAscii {
        /// The `minLength` facet, which is also the quantifier's minimum.
        min_length: u64,
        /// The `maxLength` facet, which is also the quantifier's maximum.
        max_length: u64,
    },

    /// The UCI whitespace-visible bounded-string family (Task 041):
    /// `minLength = min_length`, `maxLength = max_length`, the single pattern
    /// `[ -~\n\r]{min,max}` whose quantifier agrees with those facets, and the
    /// effective `whiteSpace` policy named by `white_space`.
    ///
    /// # A separate variant, not a flag on `VisibleAscii`
    ///
    /// Its class is strictly larger -- `[ -~]` plus LF and CR -- and, for the
    /// collapse half, its constructor *changes the stored value*. Folding a
    /// policy flag into `VisibleAscii` would have made every existing
    /// visible-ASCII match site responsible for a normalization decision that
    /// cannot arise there, and would have let a `collapse` facet reach the
    /// visible-ASCII validators, which reject it today and must keep doing so.
    ///
    /// # Membership is a whole observed triple
    ///
    /// Like `VisibleAscii` this is parameterized, but the parameters are **not**
    /// independent. Only the triples in `UCI_WHITESPACE_VISIBLE_PROFILES` are
    /// admitted, so the Cartesian product of separately observed policies,
    /// minima, and maxima is *not* accepted. The expression must additionally
    /// equal [`whitespace_visible_pattern`] for these bounds, so the facets and
    /// the quantifier can never disagree.
    ///
    /// # Generated value semantics
    ///
    /// * [`WhitespaceVisiblePolicy::Collapse`]: the decoded input is collapsed
    ///   per §4.3.6 and the *normalized* result is validated and stored, so a
    ///   raw input longer than `max_length` is accepted when its normalized form
    ///   fits.
    /// * [`WhitespaceVisiblePolicy::Preserve`]: the decoded input is validated
    ///   and stored unchanged, so LF, CR, and every interior space stay
    ///   significant.
    WhitespaceVisible {
        /// The effective `whiteSpace` policy, which decides whether the stored
        /// value is normalized or preserved.
        white_space: WhitespaceVisiblePolicy,
        /// The `minLength` facet, which is also the quantifier's minimum.
        min_length: u64,
        /// The `maxLength` facet, which is also the quantifier's maximum.
        max_length: u64,
    },
}

/// Why a constrained `string` declaration falls outside the implemented set.
///
/// Every variant describes a declaration that *is* a constrained `string`. An
/// unconstrained `string`, or a non-`string` declaration, is reported as
/// `Ok(None)` instead, never as an error -- those keep their existing
/// representation and this classifier has no opinion about them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringProfileError {
    /// A constrained `string` whose effective facet shape is not the one
    /// supported profile.
    ///
    /// Covers a different pattern text, multiple alternatives in one group,
    /// multiple restriction-level groups, a non-XML-Schema dialect, a `length`
    /// facet, different `minLength`/`maxLength` values, an explicit
    /// `whiteSpace` facet, or any numeric facet. The classifier enforces each
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
/// through the validated-carrier machinery.
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

/// Classify a named `string` declaration against the implemented subset.
///
/// * `Ok(None)` -- not a constrained `string`; this module has no opinion, and the
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
/// `string` whose effective facet shape is not an implemented profile. No
/// facet is ever silently ignored: a constraint this module cannot enforce is a
/// rejection, not a warning.
pub fn string_profile(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
) -> Result<Option<StringProfile>, StringProfileError> {
    if kind != PrimitiveKind::String {
        return Ok(None);
    }
    // An ordinary `string` is not a concern of this module. This is what keeps
    // existing Rust `String` / C++ `std::string` / Ada `Unbounded_String`
    // output completely unchanged.
    if !constrains_string(constraints) {
        return Ok(None);
    }
    // Each implemented profile is offered the constraint set in turn, and each
    // decides independently. Failing to be the schema-version profile must not
    // become an error before the UUID profile has had its chance, which is what
    // the single-profile shape this replaced would have done.
    if matches_schema_version_profile(constraints) {
        return Ok(Some(StringProfile::UciSchemaVersion));
    }
    if matches_uuid_profile(constraints) {
        return Ok(Some(StringProfile::UniversallyUniqueIdentifier));
    }
    if let Some(profile) = match_visible_ascii_profile(constraints) {
        return Ok(Some(profile));
    }
    if let Some(profile) = match_whitespace_visible_profile(constraints) {
        return Ok(Some(profile));
    }
    Err(StringProfileError::UnsupportedConstraints)
}

/// Whether these effective facets are exactly a whitespace-visible family
/// member.
///
/// # Order of reasoning
///
/// 1. reject a `length` facet, which this family never carries;
/// 2. read both bound facets; a half-bounded restriction is not a member;
/// 3. read the effective `whiteSpace` policy from the facet, mapping *absent*
///    to `Preserve` and an explicit `collapse` to `Collapse`. An explicit
///    `preserve` or a `replace` facet is an unobserved shape and is rejected;
/// 4. require the single expression to equal [`whitespace_visible_pattern`] for
///    the bounds read in step 2, which is what makes facet/quantifier
///    disagreement unrepresentable;
/// 5. only then ask whether the whole triple is evidence-backed.
///
/// Step 5 is what keeps this from degenerating into "any bounds plus any
/// policy". Shape agreement is necessary but not sufficient: a synthetic
/// `[ -~\n\r]{1,18446744073709551615}` with perfectly agreeing facets fails
/// closed, because no pinned release contains it.
///
/// Nothing here reads a declaration name, a file name, a schema version, or a
/// restriction depth.
fn match_whitespace_visible_profile(constraints: &ConstraintSet) -> Option<StringProfile> {
    // `length` is mutually exclusive with min/maxLength in this family.
    if constraints.length.is_some() {
        return None;
    }
    // The minLength facet and the quantifier's lower bound are separate pieces
    // of evidence. Both are read; neither is inferred from the other.
    let min_length = constraints.min_length?;
    let max_length = constraints.max_length?;
    if min_length > max_length {
        return None;
    }
    // The whiteSpace facet is the release-distinguishing axis, so it is mapped
    // to a policy explicitly rather than being tolerated. `Replace`, and an
    // explicit `Preserve`, are unobserved shapes and fall through to `None`.
    let white_space = match constraints.lexical.white_space {
        None => WhitespaceVisiblePolicy::Preserve,
        Some(WhiteSpacePolicy::Collapse) => WhitespaceVisiblePolicy::Collapse,
        Some(WhiteSpacePolicy::Preserve | WhiteSpacePolicy::Replace) => return None,
    };
    if !has_only_pattern_under_white_space(
        constraints,
        &whitespace_visible_pattern(min_length, max_length),
        white_space,
    ) {
        return None;
    }
    if !is_authoritative_whitespace_visible_profile(white_space, min_length, max_length) {
        return None;
    }
    Some(StringProfile::WhitespaceVisible {
        white_space,
        min_length,
        max_length,
    })
}

/// Whether these effective facets are exactly a visible-ASCII family member.
///
/// The bounds are read from the facets *first*, and the expected pattern text
/// is then derived from them. The declaration matches only if its single
/// expression equals that derived text, so the quantifier and the facets are
/// checked against each other rather than independently. This is what keeps a
/// parameterized profile from degenerating into "any minLength + maxLength +
/// pattern": the parameters are constrained by the very pattern they explain.
///
/// Agreement alone is necessary but not sufficient. The bounds must in
/// addition be one of the pairs the pinned releases are observed to carry, per
/// [`UCI_VISIBLE_ASCII_BOUNDS`]. The parameterization is over authoritative
/// evidence, not over the whole `u64` range, so a synthetic
/// `[ -~]{1,18446744073709551615}` with matching facets fails closed instead of
/// being claimed as baseline-renderable.
fn match_visible_ascii_profile(constraints: &ConstraintSet) -> Option<StringProfile> {
    // `length` is mutually exclusive with min/maxLength in this family. The
    // `[ -~]{N}` fixed-`length` declarations UCI also defines are a *different*
    // shape and are deliberately not matched here.
    if constraints.length.is_some() {
        return None;
    }
    // Both bounds must be present: a half-bounded visible-ASCII restriction is
    // not a member, and its pattern could not agree with absent facets anyway.
    let min_length = constraints.min_length?;
    let max_length = constraints.max_length?;
    // A member's own quantifier is `{min,max}`, which is unsatisfiable unless
    // the bounds are ordered. Rejecting here keeps the generated validators
    // from having to reason about an empty accepted language.
    if min_length > max_length {
        return None;
    }
    if !has_only_pattern(constraints, &visible_ascii_pattern(min_length, max_length)) {
        return None;
    }
    // The semantic shape is established above; only now is the declaration
    // asked whether its bounds are evidence-backed. Agreeing facets and
    // quantifier make a declaration *shaped* like the family, but this profile
    // is parameterized over the pairs the pinned releases actually contain,
    // not over every `u64` pair whose pattern happens to agree. An unobserved
    // pair fails closed rather than being admitted on resemblance alone: see
    // `UCI_VISIBLE_ASCII_BOUNDS`.
    if !is_authoritative_visible_ascii_bounds(min_length, max_length) {
        return None;
    }
    Some(StringProfile::VisibleAscii {
        min_length,
        max_length,
    })
}

/// Whether these effective facets are exactly the schema-version profile.
///
/// Matched structurally against the normalized Task 016 IR -- facet values,
/// group count, alternative count, dialect, expression text -- never against a
/// rendered debug string, which would silently accept a formatting change, and
/// never against the declaration's name.
fn matches_schema_version_profile(constraints: &ConstraintSet) -> bool {
    // `length` is mutually exclusive with min/maxLength in this profile.
    if constraints.length.is_some() {
        return false;
    }
    // The exact authoritative bounds. A same-pattern declaration with
    // different bounds is a different type and fails closed.
    if constraints.min_length != Some(UCI_SCHEMA_VERSION_MIN_LENGTH)
        || constraints.max_length != Some(UCI_SCHEMA_VERSION_MAX_LENGTH)
    {
        return false;
    }
    has_only_pattern(constraints, UCI_SCHEMA_VERSION_PATTERN)
}

/// Whether these effective facets are exactly the UUID profile.
///
/// Requires the single `length` facet and, critically, exactly **one** pattern
/// group holding exactly **one** expression: the authoritative declaration has
/// one `<xs:pattern>` element whose alternation is internal to its text.
fn matches_uuid_profile(constraints: &ConstraintSet) -> bool {
    // `length` exactly, and no bounds facets: a same-pattern declaration using
    // minLength/maxLength, or a different length, is a different type.
    if constraints.length != Some(UCI_UUID_LENGTH)
        || constraints.min_length.is_some()
        || constraints.max_length.is_some()
    {
        return false;
    }
    has_only_pattern(constraints, UCI_UUID_PATTERN)
}

/// The facet requirements both implemented profiles share.
///
/// Numeric facets are not applicable to `string` at all; their presence means
/// this IR did not come from a well-formed `string` restriction. An explicit
/// `whiteSpace` facet is a real, enforceable narrowing for `xs:string` (whose
/// intrinsic policy is `preserve` and is *not* fixed) -- exactly the
/// `WhitespaceVisibleString*` family -- so it fails closed rather than being
/// assumed harmless.
///
/// Exactly one restriction level carrying exactly one expression is required.
/// Multiple groups are AND-ed and multiple alternatives are OR-ed (§4.3.4.3);
/// neither combination is implemented, so both fail closed rather than being
/// approximated by a validator that would ignore part of the constraint.
fn has_only_pattern(constraints: &ConstraintSet, expected: &str) -> bool {
    // Task 041 note: this guard is DELIBERATELY not relaxed. The three profiles
    // that call it -- schema-version, UUID, visible-ASCII -- are each observed
    // with no `whiteSpace` facet, and each of their validators stores the
    // caller's text unchanged. A `collapse` facet reaching one of them would be
    // a constraint the emitted code does not implement, so it must stay a
    // rejection here. The whitespace-visible family instead calls
    // `has_only_pattern_under_white_space` with the policy it has proven it
    // implements, which is why that family needed a new entry point rather than
    // a globally weakened one.
    has_only_pattern_under_white_space(constraints, expected, WhitespaceVisiblePolicy::Preserve)
}

/// [`has_only_pattern`], but for a profile whose `whiteSpace` policy is part of
/// its evidence and whose generated validator implements that policy.
///
/// `white_space` is the policy the *caller* has committed to implementing. The
/// declaration's facet must equal exactly what that policy corresponds to, per
/// [`WhitespaceVisiblePolicy::expected_facet`], so the distinction between
/// `collapse` and an absent facet is preserved rather than erased. A profile
/// passing `Preserve` here therefore keeps the original strict behaviour: any
/// explicit `whiteSpace` facet at all fails closed.
fn has_only_pattern_under_white_space(
    constraints: &ConstraintSet,
    expected: &str,
    white_space: WhitespaceVisiblePolicy,
) -> bool {
    if constraints.lexical.white_space != white_space.expected_facet() {
        return false;
    }
    if constraints.min_inclusive.is_some()
        || constraints.max_inclusive.is_some()
        || constraints.min_exclusive.is_some()
        || constraints.max_exclusive.is_some()
    {
        return false;
    }
    let [group] = constraints.lexical.pattern_groups.as_slice() else {
        return false;
    };
    let [alternative] = group.alternatives.as_slice() else {
        return false;
    };
    alternative.dialect == PatternDialect::XmlSchema && alternative.expression == expected
}

/// Whether a schema emits at least one String-profile value carrier.
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

    /// The authoritative UUID profile, built from the normalized IR shape
    /// observed in both pinned releases: ONE group holding ONE expression.
    fn uuid() -> ConstraintSet {
        ConstraintSet {
            length: Some(UCI_UUID_LENGTH),
            lexical: LexicalConstraintSet {
                pattern_groups: vec![PatternGroup {
                    alternatives: vec![PatternExpression::xml_schema(UCI_UUID_PATTERN)],
                }],
                white_space: None,
            },
            ..ConstraintSet::default()
        }
    }

    /// The UUID profile is recognized from kind + effective facets alone.
    #[test]
    fn uci_uuid_is_a_supported_profile() {
        assert_eq!(
            string_profile(PrimitiveKind::String, &uuid()),
            Ok(Some(StringProfile::UniversallyUniqueIdentifier))
        );
    }

    /// The authoritative declaration has exactly ONE `<xs:pattern>` element, so
    /// the IR carries one group with one expression and the `|` is internal to
    /// that expression's text.
    ///
    /// This is the correction that the pinned bytes forced: an abbreviated
    /// inventory had suggested two IR alternatives. A classifier demanding two
    /// would reject the real UCI declaration, so the single-expression shape is
    /// pinned here explicitly.
    #[test]
    fn the_uuid_profile_is_one_group_holding_one_expression() {
        let constraints = uuid();
        assert_eq!(constraints.lexical.pattern_groups.len(), 1);
        assert_eq!(constraints.lexical.pattern_groups[0].alternatives.len(), 1);
        assert!(
            UCI_UUID_PATTERN.contains('|'),
            "the alternation is internal to the single expression"
        );

        // Splitting the one expression into two IR alternatives is a DIFFERENT
        // declaration shape and is not this profile.
        let mut split = uuid();
        split.lexical.pattern_groups = vec![PatternGroup {
            alternatives: vec![
                PatternExpression::xml_schema("(0{8}(-0{4}){3}-0{12})"),
                PatternExpression::xml_schema(
                    "([a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[1-5][a-fA-F0-9]{3}-[89abAB][a-fA-F0-9]{3}-[a-fA-F0-9]{12})",
                ),
            ],
        }];
        assert_eq!(
            string_profile(PrimitiveKind::String, &split),
            Err(StringProfileError::UnsupportedConstraints)
        );
    }

    /// The exact expression text, reproduced from the pinned release bytes.
    ///
    /// Pins the two constraints an abbreviating summary loses: the version
    /// class `[1-5]` and the variant class `[89abAB]`.
    #[test]
    fn the_uuid_pattern_is_the_exact_authoritative_text() {
        assert_eq!(
            UCI_UUID_PATTERN,
            "(0{8}(-0{4}){3}-0{12})|([a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[1-5][a-fA-F0-9]{3}-[89abAB][a-fA-F0-9]{3}-[a-fA-F0-9]{12})"
        );
        assert!(UCI_UUID_PATTERN.contains("[1-5]"), "version class");
        assert!(UCI_UUID_PATTERN.contains("[89abAB]"), "variant class");
        assert_eq!(UCI_UUID_NIL.len(), 36);
        assert_eq!(UCI_UUID_LENGTH, 36);
    }

    /// Every near-miss of the UUID profile fails closed.
    #[test]
    fn uuid_near_misses_are_unsupported() {
        let mut cases: Vec<(&str, ConstraintSet)> = Vec::new();

        let mut no_length = uuid();
        no_length.length = None;
        cases.push(("length absent", no_length));

        let mut wrong_length = uuid();
        wrong_length.length = Some(37);
        cases.push(("length 37", wrong_length));

        let mut short_length = uuid();
        short_length.length = Some(35);
        cases.push(("length 35", short_length));

        // The same pattern expressed with bounds instead of `length`.
        let mut bounds = uuid();
        bounds.length = None;
        bounds.min_length = Some(36);
        bounds.max_length = Some(36);
        cases.push(("minLength/maxLength instead of length", bounds));

        let mut with_bounds = uuid();
        with_bounds.min_length = Some(36);
        cases.push(("length plus minLength", with_bounds));

        let mut second_group = uuid();
        second_group.lexical.pattern_groups.push(PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(UCI_UUID_PATTERN)],
        });
        cases.push(("second pattern group", second_group));

        let mut extra_alternative = uuid();
        extra_alternative.lexical.pattern_groups[0]
            .alternatives
            .push(PatternExpression::xml_schema(UCI_UUID_PATTERN));
        cases.push(("extra alternative", extra_alternative));

        let mut no_pattern = uuid();
        no_pattern.lexical.pattern_groups.clear();
        cases.push(("length only, no pattern", no_pattern));

        // The abbreviated general-hexadecimal shape, WITHOUT the authoritative
        // version/variant classes. It is a strictly larger lexical space and
        // must not be mistaken for this profile.
        let mut unconstrained_hex = uuid();
        unconstrained_hex.lexical.pattern_groups = vec![PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(
                "[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{12}",
            )],
        }];
        cases.push((
            "general hexadecimal without version/variant",
            unconstrained_hex,
        ));

        let mut explicit_ws = uuid();
        explicit_ws.lexical.white_space = Some(WhiteSpacePolicy::Collapse);
        cases.push(("explicit whiteSpace", explicit_ws));

        let mut numeric = uuid();
        numeric.min_inclusive = Some(NumericValue::Integer(0));
        cases.push(("numeric facet", numeric));

        for (label, constraints) in cases {
            assert_eq!(
                string_profile(PrimitiveKind::String, &constraints),
                Err(StringProfileError::UnsupportedConstraints),
                "{label} must fail closed"
            );
        }
    }

    /// Adding the UUID profile must not disturb the schema-version profile, and
    /// the two must not be confusable with each other.
    ///
    /// The crossed cases are the regression that matters for the refactor: a
    /// dispatch that errored as soon as the schema-version shape failed would
    /// never have reached the UUID matcher at all.
    #[test]
    fn the_two_profiles_stay_distinct() {
        assert_eq!(
            string_profile(PrimitiveKind::String, &schema_version()),
            Ok(Some(StringProfile::UciSchemaVersion))
        );
        assert_ne!(UCI_SCHEMA_VERSION_PATTERN, UCI_UUID_PATTERN);

        // Each profile's pattern under the other's facets.
        let mut crossed = uuid();
        crossed.lexical.pattern_groups[0].alternatives[0] =
            PatternExpression::xml_schema(UCI_SCHEMA_VERSION_PATTERN);
        assert_eq!(
            string_profile(PrimitiveKind::String, &crossed),
            Err(StringProfileError::UnsupportedConstraints)
        );

        let mut swapped = schema_version();
        swapped.lexical.pattern_groups[0].alternatives[0] =
            PatternExpression::xml_schema(UCI_UUID_PATTERN);
        assert_eq!(
            string_profile(PrimitiveKind::String, &swapped),
            Err(StringProfileError::UnsupportedConstraints)
        );
    }

    /// The UUID alphabet is ASCII-only, which is what licenses the generated
    /// Rust/C++ validators to count bytes for a character-defined facet.
    #[test]
    fn the_uuid_alphabet_is_entirely_ascii() {
        let alphabet: String = ('0'..='9')
            .chain('a'..='f')
            .chain('A'..='F')
            .chain(['-'])
            .collect();
        assert!(alphabet.is_ascii());
        assert_eq!(alphabet.len(), alphabet.chars().count());
        assert!(UCI_UUID_NIL.is_ascii());
    }

    /// The single supported profile, recognized from kind + effective facets.
    /// There is no name parameter at all, which is what keeps the classifier free of
    /// a UCI local-name special case.
    #[test]
    fn uci_schema_version_is_the_supported_profile() {
        assert_eq!(
            string_profile(PrimitiveKind::String, &schema_version()),
            Ok(Some(StringProfile::UciSchemaVersion))
        );
    }

    /// An ordinary unconstrained `string` is not a profiled declaration at all,
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
    /// the generated carrier would not enforce. It must fail closed rather than
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

    // ---------------------------------------------------------------------
    // Task 039: the visible-ASCII family.
    // ---------------------------------------------------------------------

    /// A visible-ASCII family member with the given bounds, built from the
    /// normalized IR shape observed in both pinned releases.
    fn visible_ascii(min_length: u64, max_length: u64) -> ConstraintSet {
        ConstraintSet {
            min_length: Some(min_length),
            max_length: Some(max_length),
            lexical: LexicalConstraintSet {
                pattern_groups: vec![PatternGroup {
                    alternatives: vec![PatternExpression::xml_schema(visible_ascii_pattern(
                        min_length, max_length,
                    ))],
                }],
                white_space: None,
            },
            ..ConstraintSet::default()
        }
    }

    /// The selected blocker's profile is recognized from kind + facets alone.
    #[test]
    fn the_visible_ascii_256_profile_is_supported() {
        assert_eq!(
            string_profile(PrimitiveKind::String, &visible_ascii(1, 256)),
            Ok(Some(StringProfile::VisibleAscii {
                min_length: 1,
                max_length: 256
            }))
        );
    }

    /// The IR expression contains the EXPANDED characters, not the `&#x20;`
    /// character references the XSD source spells them with.
    #[test]
    fn the_expected_expression_uses_expanded_characters() {
        let pattern = visible_ascii_pattern(1, 256);
        assert_eq!(pattern, "[ -~]{1,256}");
        assert!(!pattern.contains("&#x"));
        // The two class endpoints really are U+0020 and U+007E.
        let bytes = pattern.as_bytes();
        assert_eq!(bytes[1], VISIBLE_ASCII_MIN_CODE_POINT);
        assert_eq!(bytes[3], VISIBLE_ASCII_MAX_CODE_POINT);
        assert_eq!(VISIBLE_ASCII_MIN_CODE_POINT, b' ');
        assert_eq!(VISIBLE_ASCII_MAX_CODE_POINT, b'~');
    }

    /// Every bound pair the pinned releases actually contain is supported.
    ///
    /// This is the family inventory, asserted rather than described. Two of
    /// those QNames contain no "VisibleString" at all, and one is reached only
    /// through a restriction chain, which is exactly why membership is decided
    /// from effective facets.
    #[test]
    fn every_authoritative_family_member_is_supported() {
        // Spelled out literally rather than read from
        // `UCI_VISIBLE_ASCII_BOUNDS`, so that narrowing the table away from the
        // thirteen measured declarations fails this test instead of silently
        // agreeing with itself.
        let observed = [
            (1, 20),
            (1, 32),
            (1, 64),
            (1, 81),
            (1, 128),
            (1, 256),
            (1, 480),
            (1, 512),
            (1, 1024),
            (2, 4),
        ];
        assert_eq!(
            UCI_VISIBLE_ASCII_BOUNDS,
            observed.as_slice(),
            "the authoritative bound table must be exactly the observed inventory"
        );
        for (min_length, max_length) in observed {
            assert_eq!(
                string_profile(
                    PrimitiveKind::String,
                    &visible_ascii(min_length, max_length)
                ),
                Ok(Some(StringProfile::VisibleAscii {
                    min_length,
                    max_length
                })),
                "the {min_length}..{max_length} member must be supported"
            );
        }
    }

    /// A huge but internally consistent synthetic profile is NOT supported.
    ///
    /// This is the load-bearing regression for the bound table. `1..u64::MAX`
    /// satisfies every structural requirement -- both facets present, ordered,
    /// no `length`, no `whiteSpace`, one group, one expression, and a pattern
    /// `[ -~]{1,18446744073709551615}` that agrees with its own facets -- so
    /// the shape checks alone would admit it. Admitting it would let coverage
    /// claim baseline support for a declaration whose emitted C++ length
    /// constant (`static constexpr std::size_t kMaxLength = 18446744073709551615;`)
    /// is not guaranteed to compile under the project's strict flags. The
    /// classifier must reject before generation can claim support.
    #[test]
    fn an_unobserved_huge_visible_ascii_profile_is_not_supported() {
        let constraints = visible_ascii(1, u64::MAX);

        assert_eq!(
            string_profile(PrimitiveKind::String, &constraints),
            Err(StringProfileError::UnsupportedConstraints)
        );
    }

    /// An ordinary, perfectly representable, but UNOBSERVED pair is rejected.
    ///
    /// `3..17` is small, would render fine in every backend, and carries the
    /// exactly agreeing pattern `[ -~]{3,17}`. It is still not a member,
    /// because the profile is bounded by authoritative evidence rather than by
    /// machine range. This is what separates "semantically similar" from
    /// "authoritatively supported".
    #[test]
    fn an_unobserved_ordinary_visible_ascii_pair_is_not_supported() {
        let constraints = visible_ascii(3, 17);
        assert_eq!(
            constraints.lexical.pattern_groups[0].alternatives[0].expression,
            "[ -~]{3,17}"
        );

        assert_eq!(
            string_profile(PrimitiveKind::String, &constraints),
            Err(StringProfileError::UnsupportedConstraints)
        );
    }

    /// The bounds and the quantifier must AGREE. A declaration carrying one
    /// member's pattern under another member's facets is not a member.
    #[test]
    fn bounds_disagreeing_with_the_quantifier_fail_closed() {
        // Same pattern text, different maxLength facet.
        let mut constraints = visible_ascii(1, 256);
        constraints.max_length = Some(128);
        assert_eq!(
            string_profile(PrimitiveKind::String, &constraints),
            Err(StringProfileError::UnsupportedConstraints)
        );

        // Same facets, a different member's pattern text.
        let mut constraints = visible_ascii(1, 256);
        constraints.lexical.pattern_groups = vec![PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(visible_ascii_pattern(1, 32))],
        }];
        assert_eq!(
            string_profile(PrimitiveKind::String, &constraints),
            Err(StringProfileError::UnsupportedConstraints)
        );
    }

    /// Every near-miss of the visible-ASCII profile fails closed.
    #[test]
    fn visible_ascii_near_misses_are_unsupported() {
        // An explicit whiteSpace facet: the WhitespaceVisibleString* shape,
        // which needs a separate normalization analysis and is NOT implemented.
        let mut collapse = visible_ascii(1, 256);
        collapse.lexical.white_space = Some(WhiteSpacePolicy::Collapse);

        // `length` instead of min/maxLength: the VisibleStringLength*/NITF_*
        // fixed-width shape, whose quantifier is `{N}` rather than `{M,N}`.
        let mut fixed_length = ConstraintSet {
            length: Some(10),
            ..ConstraintSet::default()
        };
        fixed_length.lexical.pattern_groups = vec![PatternGroup {
            alternatives: vec![PatternExpression::xml_schema("[ -~]{10}")],
        }];

        // A half-bounded restriction: no maxLength at all.
        let mut half_bounded = visible_ascii(1, 256);
        half_bounded.max_length = None;

        // A second pattern GROUP: groups are AND-ed and this is not implemented.
        let mut two_groups = visible_ascii(1, 256);
        two_groups.lexical.pattern_groups.push(PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(visible_ascii_pattern(1, 256))],
        });

        // A second EXPRESSION in the one group: alternatives are OR-ed.
        let mut two_alternatives = visible_ascii(1, 256);
        two_alternatives.lexical.pattern_groups[0]
            .alternatives
            .push(PatternExpression::xml_schema(visible_ascii_pattern(1, 256)));

        // A numeric facet, which is not applicable to `string` at all.
        let mut numeric = visible_ascii(1, 256);
        numeric.min_inclusive = Some(NumericValue::Integer(0));

        // NATO_SpecialWordsType: ASCII-only, but a distinct lexical profile.
        let mut nato = visible_ascii(1, 256);
        nato.lexical.pattern_groups = vec![PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(r"NATO:[a-zA-Z\-_]{1,256}")],
        }];

        for (label, constraints) in [
            ("explicit whiteSpace=collapse", collapse),
            ("length instead of min/max", fixed_length),
            ("half-bounded", half_bounded),
            ("a second pattern group", two_groups),
            ("a second expression", two_alternatives),
            ("a numeric facet", numeric),
            ("the NATO special-words profile", nato),
            ("inverted bounds", visible_ascii(9, 4)),
            ("unobserved bounds 3..17", visible_ascii(3, 17)),
            ("unobserved u64::MAX bounds", visible_ascii(1, u64::MAX)),
        ] {
            assert_eq!(
                string_profile(PrimitiveKind::String, &constraints),
                Err(StringProfileError::UnsupportedConstraints),
                "{label} must fail closed"
            );
        }
    }

    /// A class admitting LF and CR is never the visible-ASCII profile.
    ///
    /// # Why this replaces a Task 039 near-miss case
    ///
    /// Task 039 listed `[ -~\n\r]{0,4096}` -- UCI 2.5's `QueryString4096Type` --
    /// among the shapes that must fail closed, because at that time NO profile
    /// implemented a class containing LF and CR. Task 041 implements exactly that
    /// family, so "unsupported" is no longer the correct expectation and
    /// asserting it would now be asserting a bug.
    ///
    /// The property that actually mattered is preserved and strengthened here:
    /// the two classes are DIFFERENT, so a declaration admitting LF and CR must
    /// never be handed to the visible-ASCII validator, whose emitted code
    /// rejects both characters. This is a redirection of support, not a
    /// weakening: the case is still asserted, and now asserts the stronger claim
    /// that it lands in the *correct* profile.
    #[test]
    fn a_class_admitting_line_breaks_is_never_the_visible_ascii_profile() {
        let mut with_breaks = visible_ascii(0, 4096);
        with_breaks.lexical.pattern_groups = vec![PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(r"[ -~\n\r]{0,4096}")],
        }];
        let classified = string_profile(PrimitiveKind::String, &with_breaks);
        // Never the visible-ASCII profile, whose validator rejects LF and CR.
        assert!(
            !matches!(classified, Ok(Some(StringProfile::VisibleAscii { .. }))),
            "a class admitting LF and CR must not be the visible-ASCII profile"
        );
        // It is the Task 041 family instead, with the 2.5 query shape's policy.
        assert_eq!(
            classified,
            Ok(Some(StringProfile::WhitespaceVisible {
                white_space: WhitespaceVisiblePolicy::Preserve,
                min_length: 0,
                max_length: 4096
            }))
        );
        // And 0..4096 is NOT a visible-ASCII bound pair, so even the
        // visible-ASCII pattern at those bounds still fails closed.
        assert!(!is_authoritative_visible_ascii_bounds(0, 4096));
        assert_eq!(
            string_profile(PrimitiveKind::String, &visible_ascii(0, 4096)),
            Err(StringProfileError::UnsupportedConstraints)
        );
    }

    /// The dialect is required to be `XmlSchema`, not merely assumed.
    ///
    /// `PatternDialect` currently has exactly one variant, so a foreign-dialect
    /// constraint set is not constructible and cannot be asserted against
    /// behaviourally. The requirement is still written down in the classifier,
    /// which is what keeps it correct when a second dialect is added; this test
    /// pins the assumption that makes the omission safe today.
    #[test]
    fn the_only_pattern_dialect_today_is_xml_schema() {
        let constraints = visible_ascii(1, 256);
        assert_eq!(
            constraints.lexical.pattern_groups[0].alternatives[0].dialect,
            PatternDialect::XmlSchema
        );
    }

    /// Recognition is not a substring search for `[ -~]`.
    ///
    /// An expression that merely *contains* the class, under facets that would
    /// otherwise look plausible, is not a family member.
    #[test]
    fn merely_containing_the_class_is_not_membership() {
        let mut constraints = visible_ascii(1, 256);
        constraints.lexical.pattern_groups = vec![PatternGroup {
            alternatives: vec![PatternExpression::xml_schema("x[ -~]{1,256}")],
        }];
        assert_eq!(
            string_profile(PrimitiveKind::String, &constraints),
            Err(StringProfileError::UnsupportedConstraints)
        );
    }

    /// The accepted alphabet is entirely ASCII, which is what licenses the
    /// generated validators to count bytes for a character-defined facet.
    #[test]
    fn the_visible_ascii_alphabet_is_entirely_single_byte() {
        for code_point in VISIBLE_ASCII_MIN_CODE_POINT..=VISIBLE_ASCII_MAX_CODE_POINT {
            let character = char::from(code_point);
            assert!(character.is_ascii());
            assert_eq!(character.len_utf8(), 1);
        }
        // The class has exactly 95 members, and DEL is not one of them: the
        // upper bound is TILDE, one below U+007F.
        let width = VISIBLE_ASCII_MAX_CODE_POINT - VISIBLE_ASCII_MIN_CODE_POINT + 1;
        assert_eq!(width, 95);
        assert_eq!(VISIBLE_ASCII_MAX_CODE_POINT + 1, 0x7F);
    }

    /// SPACE is INSIDE the class, and the other whitespace characters are not.
    ///
    /// This is the trap the profile turns on: `xs:string` carries
    /// `whiteSpace = preserve` and the class contains U+0020, so a leading or
    /// trailing space is part of the value rather than noise to be trimmed.
    #[test]
    fn space_is_a_member_but_the_other_whitespace_characters_are_not() {
        let member = |character: char| {
            (character as u32) >= u32::from(VISIBLE_ASCII_MIN_CODE_POINT)
                && (character as u32) <= u32::from(VISIBLE_ASCII_MAX_CODE_POINT)
        };
        assert!(member(' '), "U+0020 SPACE is inside [ -~]");
        assert!(member('~'));
        assert!(member('!'));
        for outside in ['\t', '\n', '\r', '\u{0}', '\u{1f}', '\u{7f}', 'é', '😀'] {
            assert!(!member(outside), "{outside:?} is outside [ -~]");
        }
        // The profile declares no whiteSpace facet, so `preserve` is inherited
        // and normalization is the identity.
        assert!(visible_ascii(1, 256).lexical.white_space.is_none());
    }

    /// The three implemented profiles stay distinct: none shadows another.
    #[test]
    fn the_implemented_profiles_remain_distinct() {
        assert_eq!(
            string_profile(PrimitiveKind::String, &schema_version()),
            Ok(Some(StringProfile::UciSchemaVersion))
        );
        assert_eq!(
            string_profile(PrimitiveKind::String, &uuid()),
            Ok(Some(StringProfile::UniversallyUniqueIdentifier))
        );
        assert_eq!(
            string_profile(PrimitiveKind::String, &visible_ascii(1, 256)),
            Ok(Some(StringProfile::VisibleAscii {
                min_length: 1,
                max_length: 256
            }))
        );
        // A non-String declaration is still nobody's business.
        assert_eq!(
            string_profile(PrimitiveKind::Boolean, &visible_ascii(1, 256)),
            Ok(None)
        );
    }

    // ---------------------------------------------------------------------
    // Task 041: the whitespace-visible family.
    // ---------------------------------------------------------------------

    /// A whitespace-visible member, built from the normalized IR shape observed
    /// in the pinned releases.
    fn whitespace_visible(
        white_space: WhitespaceVisiblePolicy,
        min_length: u64,
        max_length: u64,
    ) -> ConstraintSet {
        ConstraintSet {
            min_length: Some(min_length),
            max_length: Some(max_length),
            lexical: LexicalConstraintSet {
                pattern_groups: vec![PatternGroup {
                    alternatives: vec![PatternExpression::xml_schema(whitespace_visible_pattern(
                        min_length, max_length,
                    ))],
                }],
                white_space: white_space.expected_facet(),
            },
            ..ConstraintSet::default()
        }
    }

    /// EVERY evidenced triple is supported, and the table is the inventory.
    ///
    /// Enumerating the table itself means narrowing it away from the pinned
    /// evidence breaks this test rather than silently shrinking support.
    #[test]
    fn every_authoritative_whitespace_visible_profile_is_supported() {
        for &(white_space, min_length, max_length) in UCI_WHITESPACE_VISIBLE_PROFILES {
            assert_eq!(
                string_profile(
                    PrimitiveKind::String,
                    &whitespace_visible(white_space, min_length, max_length)
                ),
                Ok(Some(StringProfile::WhitespaceVisible {
                    white_space,
                    min_length,
                    max_length
                })),
                "{white_space:?} {min_length}..{max_length} must be supported"
            );
        }
        // The exact inventory read from the pinned bytes: five triples for six
        // declarations, because UCI 2.6's QueryString4096Type and
        // WhitespaceVisibleString4096Type are the same shape.
        assert_eq!(
            UCI_WHITESPACE_VISIBLE_PROFILES,
            &[
                (WhitespaceVisiblePolicy::Collapse, 0, 1024),
                (WhitespaceVisiblePolicy::Collapse, 0, 4096),
                (WhitespaceVisiblePolicy::Preserve, 0, 4096),
                (WhitespaceVisiblePolicy::Preserve, 1, 1024),
                (WhitespaceVisiblePolicy::Preserve, 1, 4096),
            ]
        );
    }

    /// The pattern text is the exact authoritative expression.
    ///
    /// Pins all three escaping layers: the character references are EXPANDED,
    /// the XSD-regex backslash escapes stay LITERAL, and no XML markup survives.
    #[test]
    fn the_whitespace_visible_expression_is_the_exact_authoritative_text() {
        assert_eq!(whitespace_visible_pattern(0, 1024), r"[ -~\n\r]{0,1024}");
        assert_eq!(whitespace_visible_pattern(1, 4096), r"[ -~\n\r]{1,4096}");
        let pattern = whitespace_visible_pattern(0, 1024);
        // The character references are gone: a literal SPACE and TILDE remain.
        assert!(!pattern.contains("&#x"));
        assert!(pattern.contains("[ -~"));
        // The regex escapes are the LITERAL two-character sequences, NOT the
        // control characters they denote. This is the distinction the evidence
        // gate requires be kept explicit.
        assert!(pattern.contains(r"\n"));
        assert!(pattern.contains(r"\r"));
        assert!(!pattern.contains('\n'), "no actual LF in the expression");
        assert!(!pattern.contains('\r'), "no actual CR in the expression");
        // The class is a strict superset of the visible-ASCII one.
        assert_ne!(
            whitespace_visible_pattern(1, 1024),
            visible_ascii_pattern(1, 1024)
        );
        // The code points the class admits.
        assert_eq!(WHITESPACE_VISIBLE_MIN_CODE_POINT, 0x20);
        assert_eq!(WHITESPACE_VISIBLE_MAX_CODE_POINT, 0x7E);
        assert_eq!(WHITESPACE_VISIBLE_LINE_FEED, 0x0A);
        assert_eq!(WHITESPACE_VISIBLE_CARRIAGE_RETURN, 0x0D);
    }

    /// A differently NAMED declaration with the same facets gets the same
    /// treatment, and the policy is read from the facet rather than guessed.
    #[test]
    fn the_whitespace_visible_profile_is_decided_by_facets_not_names() {
        // UCI 2.5's QueryString4096Type: preserve policy, ZERO minimum. It is a
        // member because its facets match, not because of its name -- and it is
        // a DIFFERENT shape from its own UCI 2.6 self.
        let query_25 = whitespace_visible(WhitespaceVisiblePolicy::Preserve, 0, 4096);
        let query_26 = whitespace_visible(WhitespaceVisiblePolicy::Preserve, 1, 4096);
        assert_ne!(query_25, query_26);
        assert_eq!(
            string_profile(PrimitiveKind::String, &query_25),
            Ok(Some(StringProfile::WhitespaceVisible {
                white_space: WhitespaceVisiblePolicy::Preserve,
                min_length: 0,
                max_length: 4096
            }))
        );
        // The collapse and preserve halves are DISTINCT profiles for the same
        // bounds, so a changed whiteSpace facet is never ignored.
        assert_eq!(
            string_profile(
                PrimitiveKind::String,
                &whitespace_visible(WhitespaceVisiblePolicy::Collapse, 0, 4096)
            ),
            Ok(Some(StringProfile::WhitespaceVisible {
                white_space: WhitespaceVisiblePolicy::Collapse,
                min_length: 0,
                max_length: 4096
            }))
        );
    }

    /// The observed axes are NOT a Cartesian product.
    ///
    /// Policies `{collapse, preserve}` x minima `{0, 1}` x maxima
    /// `{1024, 4096}` is eight combinations; only five are evidenced. The three
    /// unobserved ones must fail closed even though each of their individual
    /// components appears somewhere in the pinned bytes.
    #[test]
    fn the_whitespace_visible_axes_are_not_a_cartesian_product() {
        let mut unobserved = 0;
        for white_space in [
            WhitespaceVisiblePolicy::Collapse,
            WhitespaceVisiblePolicy::Preserve,
        ] {
            for min_length in [0_u64, 1] {
                for max_length in [1024_u64, 4096] {
                    let constraints = whitespace_visible(white_space, min_length, max_length);
                    let classified = string_profile(PrimitiveKind::String, &constraints);
                    if UCI_WHITESPACE_VISIBLE_PROFILES.contains(&(
                        white_space,
                        min_length,
                        max_length,
                    )) {
                        assert!(
                            classified.is_ok(),
                            "{white_space:?} {min_length}..{max_length}"
                        );
                    } else {
                        unobserved += 1;
                        assert_eq!(
                            classified,
                            Err(StringProfileError::UnsupportedConstraints),
                            "unobserved {white_space:?} {min_length}..{max_length} must fail closed"
                        );
                    }
                }
            }
        }
        // (Collapse, 1, 1024), (Collapse, 1, 4096), (Preserve, 0, 1024).
        assert_eq!(unobserved, 3, "exactly three combinations are unobserved");
    }

    /// Unobserved bounds and enormous agreeing bounds fail closed.
    #[test]
    fn whitespace_visible_unobserved_bounds_are_unsupported() {
        let mut cases: Vec<(&str, ConstraintSet)> = Vec::new();
        // Ordinary-but-unobserved bounds, with a perfectly agreeing quantifier.
        for (min_length, max_length) in [(0_u64, 512_u64), (1, 2048), (0, 1023), (1, 4097)] {
            cases.push((
                "unobserved bounds, preserve",
                whitespace_visible(WhitespaceVisiblePolicy::Preserve, min_length, max_length),
            ));
            cases.push((
                "unobserved bounds, collapse",
                whitespace_visible(WhitespaceVisiblePolicy::Collapse, min_length, max_length),
            ));
        }
        // ENORMOUS agreeing pairs: shape agreement is not evidence, and an
        // emitted length literal must stay representable in every backend.
        cases.push((
            "u64::MAX maximum",
            whitespace_visible(WhitespaceVisiblePolicy::Preserve, 1, u64::MAX),
        ));
        cases.push((
            "huge agreeing pair",
            whitespace_visible(WhitespaceVisiblePolicy::Collapse, 0, 1 << 40),
        ));
        for (label, constraints) in cases {
            assert_eq!(
                string_profile(PrimitiveKind::String, &constraints),
                Err(StringProfileError::UnsupportedConstraints),
                "{label} must fail closed"
            );
        }
    }

    /// Every structural near-miss of the whitespace-visible family fails closed.
    #[test]
    fn whitespace_visible_near_misses_are_unsupported() {
        let mut cases: Vec<(&str, ConstraintSet)> = Vec::new();

        // Mismatched facets and quantifier: the facets say 1024, the pattern
        // says 4096. Deriving the expected text from the facets makes this
        // unrepresentable rather than merely unchecked.
        let mut mismatched = whitespace_visible(WhitespaceVisiblePolicy::Collapse, 0, 1024);
        mismatched.lexical.pattern_groups = vec![PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(whitespace_visible_pattern(
                0, 4096,
            ))],
        }];
        cases.push(("facets disagree with the quantifier", mismatched));

        // Mismatched MINIMA specifically: minLength 0 with a `{1,N}` quantifier.
        // The evidence gate forbids inferring one from the other, so a
        // declaration whose two minima disagree must not be approximated.
        let mut minima = whitespace_visible(WhitespaceVisiblePolicy::Collapse, 0, 1024);
        minima.lexical.pattern_groups = vec![PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(whitespace_visible_pattern(
                1, 1024,
            ))],
        }];
        cases.push(("minLength disagrees with the quantifier", minima));

        // An explicit `whiteSpace = preserve` is an UNOBSERVED shape: the
        // authoritative preserve-side declarations carry NO facet at all, and
        // "absent" must stay distinguishable from "explicitly restated".
        let mut explicit_preserve = whitespace_visible(WhitespaceVisiblePolicy::Preserve, 1, 1024);
        explicit_preserve.lexical.white_space = Some(WhiteSpacePolicy::Preserve);
        cases.push(("explicit whiteSpace = preserve", explicit_preserve));

        // `whiteSpace = replace` is a real, different narrowing.
        let mut replace = whitespace_visible(WhitespaceVisiblePolicy::Preserve, 1, 1024);
        replace.lexical.white_space = Some(WhiteSpacePolicy::Replace);
        cases.push(("whiteSpace = replace", replace));

        // A MISSING collapse facet where the profile needs one: 2.6's absence
        // wearing 2.5's bounds, which no pinned release contains.
        let mut missing_collapse = whitespace_visible(WhitespaceVisiblePolicy::Collapse, 0, 1024);
        missing_collapse.lexical.white_space = None;
        cases.push(("collapse facet removed", missing_collapse));

        // A `length` facet: this family never carries one.
        let mut with_length = whitespace_visible(WhitespaceVisiblePolicy::Collapse, 0, 1024);
        with_length.length = Some(1024);
        cases.push(("length facet present", with_length));

        // Half-bounded restrictions.
        let mut no_min = whitespace_visible(WhitespaceVisiblePolicy::Preserve, 1, 1024);
        no_min.min_length = None;
        cases.push(("minLength absent", no_min));
        let mut no_max = whitespace_visible(WhitespaceVisiblePolicy::Preserve, 1, 1024);
        no_max.max_length = None;
        cases.push(("maxLength absent", no_max));

        // A second pattern group is an AND of constraints; a second alternative
        // is an OR. Neither is implemented, so both fail closed.
        let mut second_group = whitespace_visible(WhitespaceVisiblePolicy::Collapse, 0, 1024);
        second_group.lexical.pattern_groups.push(PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(whitespace_visible_pattern(
                0, 1024,
            ))],
        });
        cases.push(("second pattern group", second_group));

        let mut second_alternative = whitespace_visible(WhitespaceVisiblePolicy::Collapse, 0, 1024);
        second_alternative.lexical.pattern_groups[0]
            .alternatives
            .push(PatternExpression::xml_schema(whitespace_visible_pattern(
                0, 1024,
            )));
        cases.push(("second alternative", second_alternative));

        // A numeric facet means this IR did not come from a `string`.
        let mut numeric = whitespace_visible(WhitespaceVisiblePolicy::Collapse, 0, 1024);
        numeric.min_inclusive = Some(NumericValue::Integer(0));
        cases.push(("minInclusive present", numeric));

        // TAB added to the class is a DIFFERENT lexical space.
        let mut with_tab = whitespace_visible(WhitespaceVisiblePolicy::Preserve, 1, 1024);
        with_tab.lexical.pattern_groups = vec![PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(r"[ -~\n\r\t]{1,1024}")],
        }];
        cases.push(("TAB added to the class", with_tab));

        // The class spelled with ACTUAL control characters rather than the XSD
        // regex escapes: a different expression, and not what the IR carries.
        let mut decoded = whitespace_visible(WhitespaceVisiblePolicy::Preserve, 1, 1024);
        decoded.lexical.pattern_groups = vec![PatternGroup {
            alternatives: vec![PatternExpression::xml_schema("[ -~\n\r]{1,1024}")],
        }];
        cases.push(("class spelled with decoded controls", decoded));

        for (label, constraints) in cases {
            assert_eq!(
                string_profile(PrimitiveKind::String, &constraints),
                Err(StringProfileError::UnsupportedConstraints),
                "{label} must fail closed"
            );
        }
    }

    /// The OLDER profiles keep their exact accepted and rejected sets.
    ///
    /// Task 041 added a whitespace-aware entry point. This asserts the original
    /// guard was not relaxed globally: an explicit `whiteSpace` facet on any of
    /// the three older profiles is STILL a rejection, because their validators
    /// store the caller's text unchanged and do not implement it.
    #[test]
    fn the_older_profiles_still_reject_every_explicit_whitespace_facet() {
        for policy in [
            WhiteSpacePolicy::Collapse,
            WhiteSpacePolicy::Preserve,
            WhiteSpacePolicy::Replace,
        ] {
            for (label, mut constraints) in [
                ("schema version", schema_version()),
                ("uuid", uuid()),
                ("visible ASCII", visible_ascii(1, 256)),
            ] {
                // Sanity: each is supported with NO whiteSpace facet.
                assert!(
                    string_profile(PrimitiveKind::String, &constraints).is_ok(),
                    "{label} must be supported without a whiteSpace facet"
                );
                constraints.lexical.white_space = Some(policy);
                assert_eq!(
                    string_profile(PrimitiveKind::String, &constraints),
                    Err(StringProfileError::UnsupportedConstraints),
                    "{label} with whiteSpace = {policy:?} must still fail closed"
                );
            }
        }
    }

    /// The two bounded-string families never collide.
    ///
    /// Their classes differ by exactly LF and CR, so a member of one must not be
    /// classified as the other even at a bound pair both tables contain.
    #[test]
    fn the_two_bounded_string_families_stay_distinct() {
        // 1..1024 is an observed pair in BOTH tables.
        assert!(is_authoritative_visible_ascii_bounds(1, 1024));
        assert!(is_authoritative_whitespace_visible_profile(
            WhitespaceVisiblePolicy::Preserve,
            1,
            1024
        ));
        // The patterns still differ, so the declarations classify differently.
        assert_eq!(
            string_profile(PrimitiveKind::String, &visible_ascii(1, 1024)),
            Ok(Some(StringProfile::VisibleAscii {
                min_length: 1,
                max_length: 1024
            }))
        );
        assert_eq!(
            string_profile(
                PrimitiveKind::String,
                &whitespace_visible(WhitespaceVisiblePolicy::Preserve, 1, 1024)
            ),
            Ok(Some(StringProfile::WhitespaceVisible {
                white_space: WhitespaceVisiblePolicy::Preserve,
                min_length: 1,
                max_length: 1024
            }))
        );
    }

    /// The documentation word is policy-specific, never one phrase for both.
    #[test]
    fn the_stored_value_word_distinguishes_the_two_policies() {
        assert_eq!(
            WhitespaceVisiblePolicy::Collapse.stored_value_word(),
            "normalized"
        );
        assert_eq!(
            WhitespaceVisiblePolicy::Preserve.stored_value_word(),
            "preserved"
        );
        assert!(WhitespaceVisiblePolicy::Collapse.normalizes());
        assert!(!WhitespaceVisiblePolicy::Preserve.normalizes());
        // The facet mapping keeps "absent" and "collapse" apart.
        assert_eq!(WhitespaceVisiblePolicy::Preserve.expected_facet(), None);
        assert_eq!(
            WhitespaceVisiblePolicy::Collapse.expected_facet(),
            Some(WhiteSpacePolicy::Collapse)
        );
    }
}
