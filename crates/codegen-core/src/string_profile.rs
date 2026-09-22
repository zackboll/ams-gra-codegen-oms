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
//! Three, each recognized by an exact effective facet shape:
//!
//! * [`StringProfile::UciSchemaVersion`] (Task 037) -- see below;
//! * [`StringProfile::UniversallyUniqueIdentifier`] (Task 038) -- see the
//!   dedicated section further down;
//! * [`StringProfile::VisibleAscii`] (Task 039) -- the parameterized
//!   visible-ASCII bounded-string family, see its own section last.
//!
//! None is recognized by declaration name, and a constrained `string` matching
//! none of the three shapes fails closed.
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
    Err(StringProfileError::UnsupportedConstraints)
}

/// Whether these effective facets are exactly a visible-ASCII family member.
///
/// The bounds are read from the facets *first*, and the expected pattern text
/// is then derived from them. The declaration matches only if its single
/// expression equals that derived text, so the quantifier and the facets are
/// checked against each other rather than independently. This is what keeps a
/// parameterized profile from degenerating into "any minLength + maxLength +
/// pattern": the parameters are constrained by the very pattern they explain.
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
    if constraints.min_inclusive.is_some()
        || constraints.max_inclusive.is_some()
        || constraints.min_exclusive.is_some()
        || constraints.max_exclusive.is_some()
    {
        return false;
    }
    if constraints.lexical.white_space.is_some() {
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
        for (min_length, max_length) in [
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
        ] {
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

        // QueryString4096Type: the class additionally admits LF and CR.
        let mut with_breaks = visible_ascii(0, 4096);
        with_breaks.lexical.pattern_groups = vec![PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(r"[ -~\n\r]{0,4096}")],
        }];

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
            ("a class admitting LF and CR", with_breaks),
            ("the NATO special-words profile", nato),
            ("inverted bounds", visible_ascii(9, 4)),
        ] {
            assert_eq!(
                string_profile(PrimitiveKind::String, &constraints),
                Err(StringProfileError::UnsupportedConstraints),
                "{label} must fail closed"
            );
        }
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
}
