//! Shared generated-host-language-name policy for the Ada, Rust, and C++
//! backends.
//!
//! # Why this exists
//!
//! Schema IR identifiers are transformed differently per backend: Rust and C++
//! upper-camel declarations and snake_case members, Ada case-insensitive
//! identifiers in one flat package. Two distinct XSD names can therefore
//! normalize to one generated identifier (`foo_bar` and `fooBar` both become
//! `FooBar`), and a legal XSD name can normalize onto a host-language reserved
//! word. Either produces source that does not compile.
//!
//! Historically each renderer carried a little of this policy (Choice
//! alternatives were collision-checked; nothing else was) and coverage carried
//! none of it, so capability analysis could report a declaration READY while
//! generation emitted invalid source. This module is the single shared model
//! consumed by **both** backend generation validation and capability/readiness
//! analysis, so the two cannot drift.
//!
//! # Boundaries
//!
//! Language-specific naming policy lives here, in shared codegen
//! infrastructure -- never in Schema IR. Nothing here mutates IR, and no
//! generated name is stored back into IR.
//!
//! This module **rejects**; it never mangles, escapes, suffixes, or renames.
//! Inventing a disambiguation scheme would silently change the generated API
//! surface, so an unsafe generated name fails closed and deterministically
//! instead.

use crate::abstract_value::{
    EffectiveValueMember, abstract_value_projection_for_ref, field_storage_semantics,
};
use crate::coverage::BackendLanguage;
use crate::floating::floating_domain;
use crate::structure::{effective_choice_alternatives, effective_record_fields};
use crate::world::GenerationWorld;
use crate::{AbstractValueProjection, TypeEmission, name_preflight_plan};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, FieldDecl, OccurrenceShape, PrimitiveKind, QualifiedName, SchemaIr,
    TypeDecl, TypeKind, TypeRefTarget,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// The generated declarative region a name occupies.
///
/// Collisions are only meaningful *within* one region: two Record fields in
/// different structs may normalize identically, two fields in the same struct
/// may not. Ada's flat package makes `TopLevel` much busier than the Rust
/// module or C++ namespace, which is exactly the asymmetry this models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameRegion {
    /// The generated module (Rust), namespace (C++), or package (Ada).
    TopLevel,
    /// The members of one generated declaration: Record fields, Choice
    /// alternatives, or enumeration variants.
    Members(QualifiedName),
    /// The generated module/namespace/package identifier itself, derived from
    /// the schema namespace URI rather than from any declaration.
    ///
    /// This is not a declarative region the schema populates; it is the
    /// enclosing unit's own name. A URI component that normalizes onto a
    /// reserved word makes the whole unit fail to compile, so it must be
    /// rejected in preflight rather than in the renderer.
    NamespaceUnit,
}

impl fmt::Display for NameRegion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TopLevel => formatter.write_str("generated top-level scope"),
            Self::Members(owner) => write!(formatter, "members of {}", owner.local_name),
            Self::NamespaceUnit => {
                formatter.write_str("generated module/namespace/package identifier")
            }
        }
    }
}

/// What produced one registered generated name.
///
/// # Why attribution is structured rather than parsed
///
/// Coverage must mark **every** schema declaration responsible for emitting a
/// side of a collision. Earlier this was recovered by splitting the
/// human-readable diagnostic label (`"Owner.Member helper"`) back apart, which
/// silently failed for every generated name whose label was not a declaration
/// local name: Ada repeated helpers, `_Kind` companions, and enumeration /
/// Choice literals all attributed to nothing, leaving the generating
/// declaration counted renderable while generation rejected the schema.
///
/// Ownership is therefore carried explicitly from the point of registration.
/// Diagnostics still render the same human-readable labels via
/// [`NameSource::label`], but attribution never reads them.
///
/// This is deliberately private to the backend-name/preflight infrastructure:
/// it is generated-name bookkeeping, not Schema IR.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NameSource {
    /// A fixed or conditional support type the backend emits itself. Owned by
    /// no declaration, so it can never make one unsafe on its own.
    GeneratedSupport(BackendLanguage),
    /// The generated module/namespace/package identifier, derived from the
    /// schema namespace URI rather than any declaration.
    NamespaceUri(String),
    /// A top-level schema declaration's own identifier.
    Declaration(QualifiedName),
    /// One member (Record field, Choice alternative, enumeration variant) of
    /// `owner`, inside that declaration's member region.
    Member {
        owner: QualifiedName,
        member: String,
    },
    /// An Ada flat-package helper type derived from `owner`'s `member`.
    Helper {
        owner: QualifiedName,
        member: String,
    },
    /// The Ada `{Owner}_Kind` discriminant enumeration emitted beside a Choice
    /// or a closed-sum abstract-value wrapper.
    Companion { owner: QualifiedName },
    /// An Ada enumeration/Choice/closed-sum literal declared in the enclosing
    /// package by `owner`.
    EnumLiteral {
        owner: QualifiedName,
        literal: String,
    },
    /// A fixed identifier the backend synthesizes inside `owner`'s member
    /// region, taken from the lowering shape rather than from any schema
    /// member name.
    ///
    /// The Ada Choice discriminant `Kind` is the motivating case: it occupies
    /// the same record declarative region as the alternatives, so an
    /// alternative spelled `Kind` cannot be emitted beside it. Because the
    /// identifier is generated *by* the owning declaration, a collision makes
    /// that declaration unsafe.
    GeneratedMember {
        owner: QualifiedName,
        generated: &'static str,
    },
    /// An overloadable callable the backend synthesizes in the Ada package's
    /// top-level region on `owner`'s behalf.
    ///
    /// Unlike every other top-level name this one is *overloadable*: Ada
    /// permits many subprograms to share an identifier when their profiles
    /// differ, which is exactly what several constrained floats produce. It
    /// is therefore never inserted into the region; it is only checked
    /// against the non-overloadable names already there.
    GeneratedCallable {
        owner: QualifiedName,
        callable: &'static str,
    },
}

impl NameSource {
    /// The human-readable label used in diagnostics.
    ///
    /// Kept byte-identical to the strings the previous label-parsing scheme
    /// produced, so error text stays stable; nothing reads it semantically.
    fn label(&self) -> String {
        match self {
            Self::GeneratedSupport(language) => {
                format!("<generated {} support type>", language.name())
            }
            Self::NamespaceUri(uri) => uri.clone(),
            Self::Declaration(name) => name.local_name.clone(),
            Self::Member { member, .. } => member.clone(),
            Self::Helper { owner, member } => {
                format!("{}.{member} helper", owner.local_name)
            }
            Self::Companion { owner } => format!("{} companion", owner.local_name),
            Self::EnumLiteral { literal, .. } => literal.clone(),
            Self::GeneratedMember { owner, generated } => {
                format!("{} generated {generated}", owner.local_name)
            }
            Self::GeneratedCallable { owner, callable } => {
                format!("{} generated {callable} function", owner.local_name)
            }
        }
    }

    /// The schema declaration responsible for emitting this name, if any.
    ///
    /// Support types and the namespace unit belong to no declaration: a
    /// collision with one implicates only the *other* side.
    const fn owner(&self) -> Option<&QualifiedName> {
        match self {
            Self::GeneratedSupport(_) | Self::NamespaceUri(_) => None,
            Self::Declaration(name) => Some(name),
            Self::Member { owner, .. }
            | Self::Helper { owner, .. }
            | Self::Companion { owner }
            | Self::EnumLiteral { owner, .. }
            | Self::GeneratedMember { owner, .. }
            | Self::GeneratedCallable { owner, .. } => Some(owner),
        }
    }
}

/// One rejected name together with the declarations responsible for it.
///
/// Collected by the attribution pass. Both sides of a collision are recorded,
/// because neither can be emitted while the other exists.
#[derive(Debug, Clone)]
struct CollectedNameError {
    error: BackendNameError,
    owners: BTreeSet<QualifiedName>,
}

impl CollectedNameError {
    fn new(error: BackendNameError, sources: &[&NameSource]) -> Self {
        Self {
            error,
            owners: sources
                .iter()
                .filter_map(|source| source.owner())
                .cloned()
                .collect(),
        }
    }
}

/// A generated host-language name that cannot be emitted safely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendNameError {
    /// The IR name cannot be transformed into a legal identifier at all.
    InvalidIdentifier {
        language: BackendLanguage,
        region: NameRegion,
        ir_name: String,
    },
    /// The generated identifier is a reserved word of the target language.
    ReservedWord {
        language: BackendLanguage,
        region: NameRegion,
        ir_name: String,
        generated: String,
    },
    /// Two distinct IR names normalize to the same generated identifier in
    /// one declarative region.
    Collision {
        language: BackendLanguage,
        region: NameRegion,
        generated: String,
        first: String,
        second: String,
    },
}

impl BackendNameError {
    /// The declarative region the rejected name belongs to.
    ///
    /// Lets a caller distinguish a failure that condemns one declaration from
    /// one that makes the whole generated unit unusable, without re-deriving
    /// the classification.
    #[must_use]
    pub const fn region(&self) -> &NameRegion {
        match self {
            Self::InvalidIdentifier { region, .. }
            | Self::ReservedWord { region, .. }
            | Self::Collision { region, .. } => region,
        }
    }
}

impl fmt::Display for BackendNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier {
                language,
                region,
                ir_name,
            } => write!(
                formatter,
                "{} cannot form a legal identifier from {ir_name:?} in the {region}",
                language.name()
            ),
            Self::ReservedWord {
                language,
                region,
                ir_name,
                generated,
            } => write!(
                formatter,
                "{} name {ir_name:?} generates reserved word {generated:?} in the {region}",
                language.name()
            ),
            Self::Collision {
                language,
                region,
                generated,
                first,
                second,
            } => write!(
                formatter,
                "{} names {first:?} and {second:?} both generate {generated:?} in the {region}",
                language.name()
            ),
        }
    }
}

impl std::error::Error for BackendNameError {}

/// Rust keywords (2015/2018/2021 strict plus reserved) that would break the
/// emitted `pub struct` / `pub enum` / `pub` field surface.
const RUST_RESERVED: &[&str] = &[
    "Self", "abstract", "as", "async", "await", "become", "box", "break", "const", "continue",
    "crate", "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl",
    "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// C++17 keywords and alternative operator spellings.
const CPP_RESERVED: &[&str] = &[
    "alignas",
    "alignof",
    "and",
    "and_eq",
    "asm",
    "auto",
    "bitand",
    "bitor",
    "bool",
    "break",
    "case",
    "catch",
    "char",
    "char16_t",
    "char32_t",
    "class",
    "compl",
    "const",
    "const_cast",
    "constexpr",
    "continue",
    "decltype",
    "default",
    "delete",
    "do",
    "double",
    "dynamic_cast",
    "else",
    "enum",
    "explicit",
    "export",
    "extern",
    "false",
    "float",
    "for",
    "friend",
    "goto",
    "if",
    "inline",
    "int",
    "long",
    "mutable",
    "namespace",
    "new",
    "noexcept",
    "not",
    "not_eq",
    "nullptr",
    "operator",
    "or",
    "or_eq",
    "private",
    "protected",
    "public",
    "register",
    "reinterpret_cast",
    "return",
    "short",
    "signed",
    "sizeof",
    "static",
    "static_assert",
    "static_cast",
    "struct",
    "switch",
    "template",
    "this",
    "thread_local",
    "throw",
    "true",
    "try",
    "typedef",
    "typeid",
    "typename",
    "union",
    "unsigned",
    "using",
    "virtual",
    "void",
    "volatile",
    "wchar_t",
    "while",
    "xor",
    "xor_eq",
];

/// Ada 2012 reserved words. Ada identifiers are case-insensitive, so these are
/// compared against the case-folded generated name.
const ADA_RESERVED: &[&str] = &[
    "abort",
    "abs",
    "abstract",
    "accept",
    "access",
    "aliased",
    "all",
    "and",
    "array",
    "at",
    "begin",
    "body",
    "case",
    "constant",
    "declare",
    "delay",
    "delta",
    "digits",
    "do",
    "else",
    "elsif",
    "end",
    "entry",
    "exception",
    "exit",
    "for",
    "function",
    "generic",
    "goto",
    "if",
    "in",
    "interface",
    "is",
    "limited",
    "loop",
    "mod",
    "new",
    "not",
    "null",
    "of",
    "or",
    "others",
    "out",
    "overriding",
    "package",
    "pragma",
    "private",
    "procedure",
    "protected",
    "raise",
    "range",
    "record",
    "rem",
    "renames",
    "requeue",
    "return",
    "reverse",
    "select",
    "separate",
    "some",
    "subtype",
    "synchronized",
    "tagged",
    "task",
    "terminate",
    "then",
    "type",
    "until",
    "use",
    "when",
    "while",
    "with",
    "xor",
];

/// Split an IR identifier into `_`-separated ASCII alphanumeric words.
///
/// This mirrors the transformation the Rust and C++ renderers already apply,
/// and is the reason `foo_bar` and `fooBar` can converge: word boundaries come
/// only from `_`, so inner camel humps are preserved verbatim rather than
/// re-split.
fn words(value: &str) -> Option<Vec<&str>> {
    let words = value
        .split('_')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    if words.is_empty()
        || words
            .iter()
            .any(|word| !word.bytes().all(|byte| byte.is_ascii_alphanumeric()))
    {
        None
    } else {
        Some(words)
    }
}

/// Whether a generated Rust/C++ identifier is syntactically legal.
///
/// # Why the start character is checked separately
///
/// [`words`] only guarantees the characters are ASCII alphanumeric or `_`,
/// which is not sufficient: `1Foo` survives upper-camel as `1Foo` and
/// `field_1` is fine while a member spelled `1` normalizes to `1`. Neither is
/// a legal Rust or C++ identifier, yet both previously passed preflight and
/// reached the renderer, producing source no compiler accepts.
///
/// The policy is deliberately the narrow ASCII subset the generators already
/// operate on -- not full Unicode XID -- because that is exactly what the
/// transformations above can produce. The **generated** identifier is checked,
/// not the source spelling, since the source is reshaped before emission.
fn is_ascii_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    match bytes.next() {
        Some(first) if first.is_ascii_alphabetic() || first == b'_' => {}
        _ => return false,
    }
    bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn upper_camel(value: &str) -> Option<String> {
    let mut result = String::new();
    for word in words(value)? {
        let mut characters = word.chars();
        let first = characters.next()?;
        result.push(first.to_ascii_uppercase());
        result.extend(characters);
    }
    is_ascii_identifier(&result).then_some(result)
}

fn snake_case(value: &str) -> Option<String> {
    let result = words(value)?.join("_").to_ascii_lowercase();
    is_ascii_identifier(&result).then_some(result)
}

/// Ada identifiers are accepted verbatim, exactly as `backend-ada` emits them.
fn ada_identifier(value: &str) -> Option<String> {
    (!value.is_empty()
        && value.as_bytes()[0].is_ascii_alphabetic()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !value.contains("__")
        && !value.ends_with('_'))
    .then(|| value.to_owned())
}

/// The generated declaration identifier for one IR local name.
fn declaration_name(language: BackendLanguage, local_name: &str) -> Option<String> {
    match language {
        BackendLanguage::Rust | BackendLanguage::Cpp => upper_camel(local_name),
        BackendLanguage::Ada => ada_identifier(local_name),
    }
}

/// The generated Record-field / Ada Choice-component member identifier.
fn member_name(language: BackendLanguage, name: &str) -> Option<String> {
    match language {
        BackendLanguage::Rust | BackendLanguage::Cpp => snake_case(name),
        BackendLanguage::Ada => ada_identifier(name),
    }
}

/// The generated enumeration-variant / Choice-variant identifier.
fn variant_name(language: BackendLanguage, name: &str) -> Option<String> {
    match language {
        BackendLanguage::Rust | BackendLanguage::Cpp => upper_camel(name),
        BackendLanguage::Ada => ada_identifier(name),
    }
}

/// The comparison key for collision detection.
///
/// Ada identifiers are case-insensitive, so `Track_Id` and `track_id` are the
/// same identifier and must collide. Rust and C++ are case-sensitive and
/// compare verbatim.
fn identity_key(language: BackendLanguage, generated: &str) -> String {
    match language {
        BackendLanguage::Ada => generated.to_ascii_lowercase(),
        BackendLanguage::Rust | BackendLanguage::Cpp => generated.to_owned(),
    }
}

/// Whether the generated identifier is a reserved word of the target language.
fn is_reserved(language: BackendLanguage, generated: &str) -> bool {
    match language {
        // Case-insensitive: `Type`, `TYPE`, and `type` are one Ada word.
        BackendLanguage::Ada => ADA_RESERVED
            .binary_search(&generated.to_ascii_lowercase().as_str())
            .is_ok(),
        BackendLanguage::Rust => RUST_RESERVED.binary_search(&generated).is_ok(),
        BackendLanguage::Cpp => CPP_RESERVED.binary_search(&generated).is_ok(),
    }
}

/// One declarative region's accumulated generated names.
struct Region {
    language: BackendLanguage,
    region: NameRegion,
    /// Generated-identity key -> the structured source that claimed it.
    taken: BTreeMap<String, NameSource>,
    /// When true, a rejected name is recorded in `errors` and validation
    /// continues instead of returning early.
    ///
    /// Generation wants the first failure and nothing more. Coverage wants
    /// *every* failure, so it can attribute each one to the declarations
    /// responsible rather than condemning the whole schema for the first
    /// unsafe name found. Both behaviours run the same registration logic,
    /// which is what keeps the two from drifting.
    collecting: bool,
    errors: Vec<CollectedNameError>,
}

impl Region {
    fn new(language: BackendLanguage, region: NameRegion) -> Self {
        Self {
            language,
            region,
            taken: BTreeMap::new(),
            collecting: false,
            errors: Vec::new(),
        }
    }

    fn collecting(language: BackendLanguage, region: NameRegion) -> Self {
        Self {
            collecting: true,
            ..Self::new(language, region)
        }
    }

    /// Report one rejected name, honouring the region's failure mode.
    ///
    /// `sources` are the structured owners implicated by this rejection: one
    /// for a reserved/invalid name, both sides for a collision.
    fn reject(
        &mut self,
        error: BackendNameError,
        sources: &[&NameSource],
    ) -> Result<(), BackendNameError> {
        if self.collecting {
            self.errors.push(CollectedNameError::new(error, sources));
            Ok(())
        } else {
            Err(error)
        }
    }

    /// Record one generated name, rejecting reserved words and collisions.
    fn insert(&mut self, source: NameSource, generated: String) -> Result<(), BackendNameError> {
        let ir_name = source.label();
        if is_reserved(self.language, &generated) {
            let error = BackendNameError::ReservedWord {
                language: self.language,
                region: self.region.clone(),
                ir_name,
                generated,
            };
            return self.reject(error, &[&source]);
        }
        let key = identity_key(self.language, &generated);
        match self.taken.get(&key) {
            // An identical source reaching the same region twice is a schema
            // defect diagnosed elsewhere, not a naming defect; only *distinct*
            // sources converging is reported here.
            Some(first) if *first == source => Ok(()),
            Some(first) => {
                let first = first.clone();
                let error = BackendNameError::Collision {
                    language: self.language,
                    region: self.region.clone(),
                    generated,
                    first: first.label(),
                    second: ir_name,
                };
                // Both sides are implicated: neither can be emitted while the
                // other exists, so coverage must exclude both owners.
                self.reject(error, &[&first, &source])
            }
            None => {
                self.taken.insert(key, source);
                Ok(())
            }
        }
    }

    /// Transform then record, mapping a failed transformation to a typed
    /// invalid-identifier error rather than skipping the name.
    fn insert_transformed(
        &mut self,
        source: NameSource,
        generated: Option<String>,
    ) -> Result<(), BackendNameError> {
        match generated {
            Some(generated) => self.insert(source, generated),
            None => {
                let error = BackendNameError::InvalidIdentifier {
                    language: self.language,
                    region: self.region.clone(),
                    ir_name: source.label(),
                };
                self.reject(error, &[&source])
            }
        }
    }
}

/// The members of one declaration, for the shape-independent scans the
/// support-name predicates need.
///
/// Record fields and Choice alternatives are both `FieldDecl` lists and every
/// renderer treats them identically when deciding whether to emit a support
/// type, so they are folded here rather than at each call site.
fn declared_members(declaration: &TypeDecl) -> &[ams_gra_oms_ir::FieldDecl] {
    match &declaration.kind {
        TypeKind::Record { fields } => fields,
        TypeKind::Choice { alternatives } => alternatives,
        _ => &[],
    }
}

/// Whether any declaration has an unbounded repeated member.
///
/// This is the exact predicate `backend-rust`, `backend-cpp`, and `backend-ada`
/// each use to decide whether to emit their unbounded-sequence support type
/// (`UnboundedVec` / `UnboundedVector` / `Ada.Containers.Vectors`). It is
/// defined once here so preflight and the renderers cannot drift: reserving a
/// support name the renderer will not emit would block a legal user
/// declaration, and failing to reserve one it does emit is the defect this
/// corrective fixes.
#[must_use]
pub fn schema_emits_unbounded_sequence_support(schema: &SchemaIr) -> bool {
    schema.types.iter().any(|declaration| {
        declared_members(declaration).iter().any(|member| {
            matches!(
                member.cardinality.shape(),
                OccurrenceShape::Unbounded { .. }
            )
        })
    })
}

/// Whether any declaration has a directly constrained integral member.
///
/// The exact predicate behind Rust's `BoundedI64`/`BoundedU64` pair and C++'s
/// `BoundedInteger` template. Both renderers emit their bounded-integer
/// support gated on this one condition.
#[must_use]
pub fn schema_emits_bounded_integer_support(schema: &SchemaIr) -> bool {
    schema.types.iter().any(|declaration| {
        declared_members(declaration).iter().any(|member| {
            matches!(
                member.type_ref.target,
                TypeRefTarget::Primitive(
                    PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger
                )
            ) && member.constraints != ams_gra_oms_ir::ConstraintSet::default()
        })
    })
}

/// The fixed discriminant identifier Ada gives every generated variant record.
///
/// Emitted by `backend-ada` for ordinary Choice lowering and for Task 024
/// closed-sum abstract-value wrappers alike.
const ADA_CHOICE_DISCRIMINANT: &str = "Kind";

/// The overloadable subprograms Ada emits for each *constrained* named
/// floating declaration.
///
/// `backend-ada::render_floating_declaration` emits, in the package's visible
/// part, `function Create (Value : <base>) return T` and `function Value
/// (Item : T) return <base>` -- but only when the declaration has a supported
/// bound-only domain. An unconstrained float emits a plain derived type with
/// no subprograms, so these names stay available to user declarations.
const ADA_FLOAT_CALLABLES: &[&str] = &["Create", "Value"];

/// The declarations for which Ada emits constrained-float `Create` / `Value`
/// subprograms.
///
/// Mirrors `backend-ada`'s `schema_has_constrained_floating` predicate exactly,
/// but per declaration rather than schema-wide, so attribution can name the
/// float responsible for a collision.
fn ada_constrained_float_owners(schema: &SchemaIr) -> Vec<&TypeDecl> {
    schema
        .types
        .iter()
        .filter(|declaration| {
            matches!(
                declaration.kind,
                TypeKind::Primitive(kind @ (PrimitiveKind::Float32 | PrimitiveKind::Float64))
                    if floating_domain(kind, &declaration.constraints)
                        .is_ok_and(|domain| domain.is_some())
            )
        })
        .collect()
}

/// Collect every conflict between a generated constrained-float subprogram and
/// a non-overloadable top-level declaration, in schema order.
///
/// # Why callables are checked rather than inserted
///
/// Ada allows subprograms to overload one another, so several constrained
/// floats may each emit a `Create` and a `Value` without conflict. Verified
/// against GNAT 14.2:
///
/// ```text
/// function Create (Value : Interfaces.IEEE_Float_64) return Burn_Rate;
/// function Create (Value : Interfaces.IEEE_Float_64) return Altitude;
/// -- accepted: the profiles differ in result type
///
/// type Create is new Integer;
/// function Create (Value : Interfaces.IEEE_Float_64) return Burn_Rate;
/// -- error: "Create" conflicts with declaration
/// ```
///
/// Inserting `Create` into the top-level region with the ordinary uniqueness
/// rule would therefore reject the *legal* multi-float case. The callable is
/// instead tested against the accumulated non-overloadable names without being
/// inserted -- the same asymmetry [`collect_ada_literal_conflicts`] uses, and
/// for the same reason.
///
/// Both sides are implicated: the float that generates the subprogram and the
/// declaration that occupies the identifier.
fn collect_ada_float_callable_conflicts(
    top_level: &Region,
    schema: &SchemaIr,
    conflicts: &mut Vec<CollectedNameError>,
) {
    for declaration in ada_constrained_float_owners(schema) {
        for callable in ADA_FLOAT_CALLABLES {
            let key = identity_key(BackendLanguage::Ada, callable);
            let Some(first) = top_level.taken.get(&key) else {
                continue;
            };
            let source = NameSource::GeneratedCallable {
                owner: declaration.name.clone(),
                callable,
            };
            let error = BackendNameError::Collision {
                language: BackendLanguage::Ada,
                region: NameRegion::TopLevel,
                generated: (*callable).to_owned(),
                first: first.label(),
                second: source.label(),
            };
            conflicts.push(CollectedNameError::new(error, &[first, &source]));
        }
    }
}

/// Whether Ada instantiates its `Binary_Vectors` octet-vector package.
///
/// Mirrors `backend-ada`'s `schema_needs_binary`: a Binary primitive
/// declaration, or any member whose type is the Binary primitive.
#[must_use]
pub fn schema_emits_ada_binary_vectors(schema: &SchemaIr) -> bool {
    schema.types.iter().any(|declaration| {
        matches!(declaration.kind, TypeKind::Primitive(PrimitiveKind::Binary))
            || declared_members(declaration).iter().any(|member| {
                matches!(
                    member.type_ref.target,
                    TypeRefTarget::Primitive(PrimitiveKind::Binary)
                )
            })
    })
}

/// Register the fixed and conditional support type names one backend emits
/// into the generated top-level scope.
///
/// # Why this is part of preflight
///
/// These identifiers are not derived from any schema name, so no
/// declaration-driven scan can see them -- yet they occupy exactly the same
/// declarative region as every generated declaration. A schema declaring
/// `BoundedVec` previously passed preflight and then emitted two items with
/// that name. Registering them here, in the shared model, means a collision is
/// reported with the same typed error and attribution as any other.
///
/// Conditional support types are registered **only when the renderer's own
/// predicate says they will be emitted**, so a schema that never triggers one
/// keeps that spelling available to user declarations.
fn register_support_names(
    top_level: &mut Region,
    schema: &SchemaIr,
    language: BackendLanguage,
) -> Result<(), BackendNameError> {
    // Attribution names the generator rather than pretending some schema
    // identifier was responsible for the reservation. Because a support type
    // has no owning declaration, a collision with one implicates only the
    // user declaration on the other side.
    let reserve = |top_level: &mut Region, generated: &str| -> Result<(), BackendNameError> {
        top_level.insert(NameSource::GeneratedSupport(language), generated.to_owned())
    };
    match language {
        BackendLanguage::Rust => {
            // Emitted unconditionally by `backend-rust::generate`.
            reserve(top_level, "BoundedVec")?;
            if schema_emits_unbounded_sequence_support(schema) {
                reserve(top_level, "UnboundedVec")?;
            }
            if schema_emits_bounded_integer_support(schema) {
                reserve(top_level, "BoundedI64")?;
                reserve(top_level, "BoundedU64")?;
            }
        }
        BackendLanguage::Cpp => {
            // Emitted unconditionally inside the generated namespace.
            reserve(top_level, "BoundedVector")?;
            if schema_emits_unbounded_sequence_support(schema) {
                reserve(top_level, "UnboundedVector")?;
            }
            if schema_emits_bounded_integer_support(schema) {
                reserve(top_level, "BoundedInteger")?;
            }
        }
        BackendLanguage::Ada => {
            // Emitted unconditionally into the generated package spec.
            reserve(top_level, "Optional_String")?;
            if schema_emits_ada_binary_vectors(schema) {
                reserve(top_level, "Binary_Vectors")?;
            }
        }
    }
    Ok(())
}

/// The declarations for which Ada actually emits a `{Owner}_Kind` companion
/// type, under the requested [`GenerationWorld`].
///
/// # Why this is world-sensitive
///
/// Being an abstract *value target* does not prove a closed-sum wrapper is
/// emitted, and the name model must never invent a name from output the
/// requested world cannot produce. Two cases make that concrete:
///
/// * **Task 026 elision.** A zero-descendant abstract target used only as
///   supported absent-only optional storage emits no wrapper at all, so there
///   is no `{Owner}_Kind`. Reserving it anyway falsely rejected otherwise
///   generable schemas.
/// * **Open extensions.** A Task 024 wrapper exists only under
///   [`GenerationWorld::ClosedSchemaSet`]; under `OpenExtensions` the abstract
///   value fails closed *before* any wrapper exists. The semantic
///   abstract-value blocker is authoritative there, and manufacturing a
///   closed-world companion name to produce a name blocker instead would
///   report a cause that cannot occur.
///
/// Ordinary Choice lowering is unconditional: `backend-ada` emits
/// `{Choice}_Kind` for every Choice in either world, so those owners are
/// always included.
///
/// The owners are now read straight off the planned emissions rather than
/// re-derived: a `TypeEmission::AbstractValue` *is* the Task 024 wrapper, and
/// a `TypeEmission::Declaration` of Choice kind *is* an emitted Choice. There
/// is no second predicate that can drift from the planner, and nothing that
/// was never planned can contribute a companion name.
fn ada_kind_companion_owners<'a>(emissions: &[TypeEmission<'a>]) -> Vec<&'a TypeDecl> {
    emissions
        .iter()
        .filter_map(|emission| match emission {
            TypeEmission::AbstractValue(projection) => Some(projection.declaration),
            TypeEmission::Declaration(declaration)
                if matches!(declaration.kind, TypeKind::Choice { .. }) =>
            {
                Some(*declaration)
            }
            TypeEmission::Declaration(_) => None,
        })
        .collect()
}

/// Register the Ada `_Kind` companion types generated beside Choice and
/// abstract closed-sum declarations.
///
/// # Why only Ada, and why only the type name
///
/// Ada renders a Choice (and a Task 024 abstract-value wrapper) as a
/// discriminated record whose discriminant type is a separate top-level
/// enumeration named `{Owner}_Kind`. Rust and C++ need no companion: their
/// variants are nested inside the `enum` / `std::variant` itself.
///
/// The companion *type* shares the flat package with every declared type, so
/// `Foo_Kind` can collide with a user declaration, an Ada repeated helper, or
/// another companion. It is registered in the real top-level region.
fn register_ada_kind_companions(
    top_level: &mut Region,
    emissions: &[TypeEmission<'_>],
) -> Result<(), BackendNameError> {
    for declaration in ada_kind_companion_owners(emissions) {
        // If the declaration's own identifier is unusable, that failure is
        // already reported for the declaration itself; do not re-report it
        // here as a companion problem.
        let Some(owner) = ada_identifier(&declaration.name.local_name) else {
            continue;
        };
        top_level.insert(
            NameSource::Companion {
                owner: declaration.name.clone(),
            },
            format!("{owner}_Kind"),
        )?;
    }
    Ok(())
}

/// Validate every generated Ada enumeration literal against the top-level
/// *type* names.
///
/// Ada enumeration literals are declared directly in the enclosing package's
/// declarative region. Verified against GNAT 14.2, a literal may overload
/// another literal but conflicts with a type name:
///
/// ```text
/// type Color is (Red, Green);
/// type Red is range 1 .. 5;   -- error: "Red" conflicts with declaration
///
/// type X_Kind is (A_Kind, B_Kind);
/// type Y_Kind is (A_Kind, C_Kind);  -- accepted: literals overload
/// ```
///
/// The check is therefore asymmetric on purpose: literals are tested against
/// the accumulated type names *without being inserted*, which rejects the real
/// conflict without inventing a literal-versus-literal collision Ada allows.
fn validate_ada_enumeration_literals(
    top_level: &Region,
    schema: &SchemaIr,
    emissions: &[TypeEmission<'_>],
) -> Result<(), BackendNameError> {
    let mut conflicts = Vec::new();
    collect_ada_literal_conflicts(top_level, schema, emissions, &mut conflicts);
    match conflicts.into_iter().next() {
        Some(conflict) => Err(conflict.error),
        None => Ok(()),
    }
}

/// Collect every Ada enumeration-literal/type-name conflict, in schema order.
///
/// Shared by first-failure validation, which takes the first, and by
/// per-declaration attribution, which needs them all.
fn collect_ada_literal_conflicts(
    top_level: &Region,
    schema: &SchemaIr,
    emissions: &[TypeEmission<'_>],
    conflicts: &mut Vec<CollectedNameError>,
) {
    // The literal is owned by the declaration that *declares the enumeration*,
    // not by whatever declaration happens to share the spelling. Both that
    // owner and the owner of the conflicting top-level type are implicated:
    // generation rejects the package because one generated the literal and the
    // other generated the type, so coverage must exclude both.
    let check = |source: NameSource, literal: &str, conflicts: &mut Vec<CollectedNameError>| {
        let key = identity_key(BackendLanguage::Ada, literal);
        if let Some(first) = top_level.taken.get(&key) {
            let error = BackendNameError::Collision {
                language: BackendLanguage::Ada,
                region: NameRegion::TopLevel,
                generated: literal.to_owned(),
                first: first.label(),
                second: source.label(),
            };
            conflicts.push(CollectedNameError::new(error, &[first, &source]));
        }
    };
    // Only *emitted* declarations contribute literals. A non-emitted abstract
    // Record has no enumeration and no Choice lowering, and a Task 024 wrapper
    // contributes its descendant literals below rather than the original
    // declaration's member surface.
    for declaration in emissions.iter().filter_map(|emission| match emission {
        TypeEmission::Declaration(declaration) => Some(*declaration),
        TypeEmission::AbstractValue(_) => None,
    }) {
        match &declaration.kind {
            // A plain enumeration's literals are the variant identifiers.
            TypeKind::Enumeration { variants } => {
                for variant in variants {
                    if let Some(literal) = ada_identifier(&variant.wire_value) {
                        let source = NameSource::EnumLiteral {
                            owner: declaration.name.clone(),
                            literal: variant.wire_value.clone(),
                        };
                        check(source, &literal, conflicts);
                    }
                }
            }
            // A Choice companion's literals are `{Alternative}_Kind`.
            TypeKind::Choice { .. } => {
                let Ok(alternatives) = effective_choice_alternatives(schema, &declaration.name)
                else {
                    continue;
                };
                for alternative in alternatives {
                    if let Some(name) = ada_identifier(&alternative.name) {
                        let source = NameSource::EnumLiteral {
                            owner: declaration.name.clone(),
                            literal: alternative.name.clone(),
                        };
                        check(source, &format!("{name}_Kind"), conflicts);
                    }
                }
            }
            _ => {}
        }
    }
    // A closed-sum wrapper's literals are `{Descendant}_Kind`, taken from the
    // very projection the Ada renderer walks. Only planned wrappers exist, so
    // only they contribute literals.
    for projection in emissions.iter().filter_map(|emission| match emission {
        TypeEmission::AbstractValue(projection) => Some(projection),
        TypeEmission::Declaration(_) => None,
    }) {
        for descendant in &projection.concrete_descendants {
            if let Some(name) = ada_identifier(&descendant.name.local_name) {
                let source = NameSource::EnumLiteral {
                    owner: projection.declaration.name.clone(),
                    literal: descendant.name.local_name.clone(),
                };
                check(source, &format!("{name}_Kind"), conflicts);
            }
        }
    }
}

/// Validate the generated module/namespace/package identifier derived from
/// the schema's namespace URI.
///
/// # Why preflight owns this
///
/// All three backends derive their enclosing unit name from the namespace URI,
/// not from any declaration, so the declaration scans cannot see it. A URI
/// whose trailing component normalizes to `class` (C++) or `Record` (Ada)
/// produces a unit that cannot compile. Rejecting it here makes the failure a
/// typed preflight error consistent with every other generated name, instead
/// of a late renderer error or a raw compiler diagnostic.
///
/// Namespace *syntax* stays out of Schema IR: this reads the URI the IR
/// already carries and applies backend naming policy, which is where that
/// policy belongs.
///
/// Only the components the renderers actually emit are checked. `backend-rust`
/// emits no `mod` identifier -- its single generated file is named from the
/// URI stem -- so Rust is checked for a usable file stem rather than for
/// reserved-word safety it never exercises.
fn validate_namespace_unit(
    schema: &SchemaIr,
    language: BackendLanguage,
) -> Result<(), BackendNameError> {
    let Some(namespace) = schema.namespaces.first() else {
        // A schema with no declared namespace is diagnosed by the renderer's
        // own "generation requires one namespace" path, not renamed here.
        return Ok(());
    };
    let uri = &namespace.uri;
    let parts = uri
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let invalid = || BackendNameError::InvalidIdentifier {
        language,
        region: NameRegion::NamespaceUnit,
        ir_name: uri.clone(),
    };
    match language {
        BackendLanguage::Rust => {
            let stem = parts.last().ok_or_else(invalid)?;
            snake_case(stem).ok_or_else(invalid)?;
            Ok(())
        }
        // `namespace_name` emits the last two components as `outer::inner`.
        BackendLanguage::Cpp => {
            if parts.len() < 2 {
                return Err(invalid());
            }
            let mut region = Region::new(language, NameRegion::NamespaceUnit);
            for part in &parts[parts.len() - 2..] {
                region.insert_transformed(
                    NameSource::NamespaceUri((*part).to_owned()),
                    snake_case(part),
                )?;
            }
            Ok(())
        }
        // `package_name` emits `Outer.Inner`; Ada reserved words are
        // case-insensitive, so a URI component `record` becomes the illegal
        // package identifier `Record`.
        BackendLanguage::Ada => {
            if parts.len() < 2 {
                return Err(invalid());
            }
            let mut region = Region::new(language, NameRegion::NamespaceUnit);
            for part in &parts[parts.len() - 2..] {
                region.insert_transformed(
                    NameSource::NamespaceUri((*part).to_owned()),
                    ada_title(part),
                )?;
            }
            Ok(())
        }
    }
}

/// The Ada package-component spelling: leading capital, rest verbatim.
///
/// Mirrors `backend-ada::ada_title` exactly.
fn ada_title(value: &str) -> Option<String> {
    let mut characters = value.chars();
    let first = characters.next()?;
    ada_identifier(&format!(
        "{}{}",
        first.to_ascii_uppercase(),
        characters.as_str()
    ))
}

/// Whether the field's cardinality makes Ada emit user-name-derived helper
/// types beside the owning declaration.
///
/// Only repeated occurrences do. Ada's package is flat, so those helper names
/// share the top-level region with every declared type, which is precisely the
/// collision this exists to catch.
fn ada_emits_helper(cardinality: Cardinality) -> bool {
    match cardinality.shape() {
        OccurrenceShape::Bounded { max, .. } => max > 1,
        OccurrenceShape::Unbounded { .. } => true,
        OccurrenceShape::RequiredOne | OccurrenceShape::OptionalOne => false,
    }
}

/// Whether Ada emits a Task 034 per-field optional wrapper for this member.
///
/// Mirrors `backend-ada::ada_emits_optional_wrapper` exactly. The wrapper is a
/// generated Ada **top-level** identifier in the flat package, so it has to be
/// registered beside declared types -- and only when it is really emitted,
/// otherwise a name nothing writes would be reserved and a legitimate user
/// declaration falsely rejected.
fn ada_emits_optional_wrapper(field: &FieldDecl) -> bool {
    field.cardinality == Cardinality::OPTIONAL_ONE
        && !field.nillable
        && matches!(field.type_ref.target, TypeRefTarget::Named(_))
        && field.constraints == ConstraintSet::default()
}

/// Record the flat-package name of the Task 034 optional wrapper Ada derives
/// from one emitted member: `{Owner}_{Member}_Optional`.
///
/// `owner` is the **emitted** declaration, exactly as for repeated helpers, so
/// an inherited optional field registers under the concrete descendant that
/// actually renders it and never under a non-emitted abstract ancestor.
fn validate_ada_optional_helper(
    top_level: &mut Region,
    owner: &QualifiedName,
    member: &str,
) -> Result<(), BackendNameError> {
    let Some(member_identifier) = ada_identifier(member) else {
        // The member identifier itself already failed in the member region.
        return Ok(());
    };
    top_level.insert(
        NameSource::Helper {
            owner: owner.clone(),
            member: member.to_owned(),
        },
        format!("{}_{member_identifier}_Optional", owner.local_name),
    )
}

/// Register the top-level name of every entity the plan actually emits.
///
/// # Why this is not simply every Schema IR declaration
///
/// A Schema IR declaration is not the same thing as a generated host-language
/// declaration. Registering all of them reserved names that never appear in
/// the output, which falsely rejected schemas whose only "collision" was
/// against a type the backend does not emit:
///
/// * **Ancestry-only abstract Records.** Every backend folds an abstract
///   Record's fields into its concrete descendants and writes no type for the
///   base, so an abstract Record named `BoundedVec` must not take that
///   identifier away from the generated Rust support type.
/// * **Task 026 elision.** A zero-descendant target used only as supported
///   absent-only optional storage emits no wrapper, no declaration, and no
///   `_Kind`, so it reserves nothing.
///
/// Names that *are* emitted keep their reservation, including abstract
/// Choices -- which no backend skips -- and Task 024 closed-sum wrappers,
/// which own the base's name in the generated scope.
///
/// The plan is formed **once per schema/world** by the caller and walked here,
/// so no whole-schema planning work is repeated per declaration.
fn register_emitted_declaration_names(
    top_level: &mut Region,
    emissions: &[TypeEmission<'_>],
    language: BackendLanguage,
) -> Result<(), BackendNameError> {
    for emission in emissions {
        if !emission.emits_own_top_level_name() {
            continue;
        }
        let name = emission.name();
        top_level.insert_transformed(
            NameSource::Declaration(name.clone()),
            declaration_name(language, &name.local_name),
        )?;
    }
    Ok(())
}

/// Validate every generated host-language name one backend would emit for
/// `schema`, in deterministic schema declaration order.
///
/// This is the shared entry point for backend generation validation and for
/// capability/readiness analysis. It is intentionally whole-schema: a
/// collision is a relationship between two declarations, so it cannot be
/// decided from one declaration alone.
///
/// Structural projection failures are **not** reported here. A declaration
/// whose effective members cannot be projected is already rejected by the
/// structural capability rules, and re-diagnosing it as a naming problem would
/// duplicate policy and obscure the real cause.
///
/// # Errors
///
/// Returns the first [`BackendNameError`] in schema order.
pub fn validate_backend_names(
    schema: &SchemaIr,
    language: BackendLanguage,
    world: GenerationWorld,
) -> Result<(), BackendNameError> {
    // The enclosing unit's own identifier comes from the namespace URI rather
    // than from any declaration, so it is checked before the scope it encloses.
    // It is genuinely independent of the emission plan -- derived from the
    // namespace URI alone -- so it is checked even when no plan exists.
    validate_namespace_unit(schema, language)?;
    // One plan per schema/world, shared by every name surface below.
    //
    // A declaration whose own emitted shape cannot be formed contributes no
    // surface and therefore no names: its semantic failure is the
    // authoritative diagnostic, owned by the backend, and fabricating a naming
    // verdict from raw Schema IR would describe output that can never exist.
    // Surfaces that *are* well-formed are still checked, because a global
    // planning abort says nothing about them.
    let plan = name_preflight_plan(schema, world);
    let emissions = plan.surfaces();
    let mut top_level = Region::new(language, NameRegion::TopLevel);
    // Generated support types occupy the top-level scope before any user
    // declaration is placed in it, so a user declaration colliding with one is
    // attributed to the user declaration as the second, conflicting source.
    register_support_names(&mut top_level, schema, language)?;
    register_emitted_declaration_names(&mut top_level, emissions, language)?;
    if language == BackendLanguage::Ada {
        register_ada_kind_companions(&mut top_level, emissions)?;
    }
    for emission in emissions {
        let mut members = Region::new(language, NameRegion::Members(emission.name().clone()));
        register_emission_names(
            schema,
            emission,
            language,
            world,
            &mut members,
            &mut top_level,
        )?;
    }
    if language == BackendLanguage::Ada {
        // Literals and generated callables are validated last, against the
        // completed set of top-level type names: either conflicts with a type
        // name regardless of which was declared first.
        validate_ada_enumeration_literals(&top_level, schema, emissions)?;
        let mut callables = Vec::new();
        collect_ada_float_callable_conflicts(&top_level, schema, &mut callables);
        if let Some(conflict) = callables.into_iter().next() {
            return Err(conflict.error);
        }
    }
    Ok(())
}

/// Register the member-region and Ada helper names one **planned emission**
/// generates, switching on the emitted shape rather than on raw Schema IR.
///
/// # Why the switch matters
///
/// The two `TypeEmission` arms have genuinely different generated member
/// surfaces, and mixing them fabricates names:
///
/// * A `Declaration` renders its own member region -- Record components,
///   Choice variant parts, enumeration literals -- from its *effective*
///   members, and Ada derives its repeated helpers under **that**
///   declaration's identifier. Because only emitted declarations reach here,
///   an ancestry-only abstract Record contributes nothing: it has no member
///   region and no helpers. Its inherited fields are still validated, in the
///   place they are actually emitted -- each concrete descendant's
///   `effective_record_fields`, under the descendant's helper stem.
/// * An `AbstractValue` renders the Task 024 closed-sum wrapper, whose member
///   surface is `Kind` plus one `{Descendant}_Value` component. The original
///   abstract Record's own fields are *not* rendered there, so feeding it
///   through Record validation would check an imaginary scope.
///
/// `world` is threaded down because an emitted Record's *stored* members are a
/// world-dependent question: Task 026 elides absent-only fields entirely, so
/// name analysis must classify each effective field through the same
/// [`field_storage_semantics`] the renderers use rather than reserving names
/// from raw Schema IR.
fn register_emission_names(
    schema: &SchemaIr,
    emission: &TypeEmission<'_>,
    language: BackendLanguage,
    world: GenerationWorld,
    members: &mut Region,
    top_level: &mut Region,
) -> Result<(), BackendNameError> {
    match emission {
        // A planned `Declaration` is not automatically a *rendered* one. With
        // no abstract value target in the schema the planner hands back every
        // declaration, including ancestry-only abstract Records that every
        // renderer returns early for. The same predicate that keeps such a
        // base from reserving its top-level name keeps it from claiming a
        // member region or an Ada helper stem, so one rule governs both.
        TypeEmission::Declaration(_) if !emission.emits_own_top_level_name() => Ok(()),
        TypeEmission::Declaration(declaration) => {
            register_declaration_members(schema, declaration, language, world, members, top_level)
        }
        TypeEmission::AbstractValue(projection) => {
            register_abstract_value_members(projection, language, members)
        }
    }
}

/// Register the member names a Task 024 closed-sum wrapper actually emits.
///
/// Only Ada puts user-derived identifiers in the wrapper's member region:
/// `backend-ada::render_abstract_value` writes a discriminated record with a
/// fixed `Kind` discriminant and one `{Descendant}_Value` component per
/// concrete descendant, all sharing one declarative region.
///
/// Rust and C++ derive no colliding member identifiers inside the wrapper:
/// `backend-rust` emits `Variant(Variant)` enum variants whose identifiers are
/// the descendants' own declaration names -- already registered and validated
/// as top-level declarations, with duplicates rejected by the renderer itself
/// -- and `backend-cpp` emits a single fixed `value` member holding a
/// `std::variant<...>`. Registering those again here would invent a second,
/// unrelated collision domain.
fn register_abstract_value_members(
    projection: &AbstractValueProjection<'_>,
    language: BackendLanguage,
    members: &mut Region,
) -> Result<(), BackendNameError> {
    if language != BackendLanguage::Ada {
        return Ok(());
    }
    let owner = &projection.declaration.name;
    // The discriminant occupies the record's declarative region first, exactly
    // as it does for an ordinary Ada Choice.
    members.insert(
        NameSource::GeneratedMember {
            owner: owner.clone(),
            generated: ADA_CHOICE_DISCRIMINANT,
        },
        ADA_CHOICE_DISCRIMINANT.to_owned(),
    )?;
    for descendant in &projection.concrete_descendants {
        let Some(name) = ada_identifier(&descendant.name.local_name) else {
            // The descendant's own identifier is unusable; that failure is
            // already reported for the descendant declaration itself.
            continue;
        };
        members.insert(
            NameSource::Member {
                owner: owner.clone(),
                member: descendant.name.local_name.clone(),
            },
            format!("{name}_Value"),
        )?;
    }
    Ok(())
}

/// Register one emitted declaration's member names into `members`, and any Ada
/// flat-package helper types it derives into `top_level`.
///
/// Shared verbatim by first-failure validation and by per-declaration
/// attribution; the two differ only in how their regions report a rejection.
fn register_declaration_members(
    schema: &SchemaIr,
    declaration: &TypeDecl,
    language: BackendLanguage,
    world: GenerationWorld,
    members: &mut Region,
    top_level: &mut Region,
) -> Result<(), BackendNameError> {
    match &declaration.kind {
        TypeKind::Enumeration { variants } => {
            for variant in variants {
                members.insert_transformed(
                    NameSource::Member {
                        owner: declaration.name.clone(),
                        member: variant.wire_value.clone(),
                    },
                    variant_name(language, &variant.wire_value),
                )?;
            }
        }
        TypeKind::Record { .. } => {
            // Effective fields, so an inherited field and a locally declared
            // field that normalize identically are caught. A projection
            // failure is another capability's diagnosis, so it is left to that
            // capability rather than re-reported here as a naming problem.
            let Ok(fields) = effective_record_fields(schema, &declaration.name) else {
                return Ok(());
            };
            for field in fields {
                // Task 026 / Task 034 boundary: classify storage with the very
                // same shared decision point the renderers use, before any name
                // is reserved. A field the backend stores nowhere emits no
                // component, no repeated helper and no `_Optional` wrapper, so
                // reserving names from the raw effective field would describe
                // output that cannot exist and could falsely reject a
                // legitimate user declaration spelled the same way.
                //
                // A projection *error* is likewise not a naming problem: the
                // semantic layer already owns diagnostics such as
                // "abstract value X is not closed under open-extensions", and
                // inventing a name for the failing field could mask that cause
                // behind a phantom collision. Only this field defers; every
                // other field and declaration is still fully checked.
                let Ok(EffectiveValueMember::Stored(field)) =
                    field_storage_semantics(schema, field, world)
                else {
                    continue;
                };
                // A stored field is still only emitted if its own value
                // reference can be represented at all. This is the identical
                // shared call `backend-ada::validate_abstract_value_reference`
                // makes before rendering the component, so an open-world
                // abstract value -- whose authoritative diagnostic is
                // "external derived types cannot be represented" -- cannot be
                // re-reported here as a phantom `_Optional` collision.
                if abstract_value_projection_for_ref(schema, &field.type_ref, world).is_err() {
                    continue;
                }
                members.insert_transformed(
                    NameSource::Member {
                        owner: declaration.name.clone(),
                        member: field.name.clone(),
                    },
                    member_name(language, &field.name),
                )?;
                if language == BackendLanguage::Ada {
                    if ada_emits_helper(field.cardinality) {
                        validate_ada_helpers(
                            top_level,
                            &declaration.name,
                            &field.name,
                            field.cardinality,
                        )?;
                    } else if ada_emits_optional_wrapper(field) {
                        // Task 034. Record fields only: a Choice alternative's
                        // exclusivity is already carried by the generated
                        // discriminant, so no optional wrapper is emitted --
                        // or reserved -- there.
                        validate_ada_optional_helper(top_level, &declaration.name, &field.name)?;
                    }
                }
            }
        }
        TypeKind::Choice { .. } => {
            let Ok(alternatives) = effective_choice_alternatives(schema, &declaration.name) else {
                return Ok(());
            };
            // Ada lowers a Choice to `type C (Kind : C_Kind := ...) is record
            // case Kind is ...`. The discriminant `Kind` is a component of
            // that record's declarative region, so it must occupy the member
            // region *before* the alternatives: an alternative spelled `Kind`
            // is rejected by GNAT with `"Kind" conflicts with declaration`.
            //
            // Rust and C++ need no equivalent. Their variants carry no
            // generated discriminant component, so this stays an Ada-only
            // rule rather than a shared one they would inherit wrongly.
            if language == BackendLanguage::Ada {
                members.insert(
                    NameSource::GeneratedMember {
                        owner: declaration.name.clone(),
                        generated: ADA_CHOICE_DISCRIMINANT,
                    },
                    ADA_CHOICE_DISCRIMINANT.to_owned(),
                )?;
            }
            for alternative in alternatives {
                let source = NameSource::Member {
                    owner: declaration.name.clone(),
                    member: alternative.name.clone(),
                };
                match language {
                    // Rust enum variants and C++ nested alternative structs
                    // are upper-camel.
                    BackendLanguage::Rust | BackendLanguage::Cpp => members
                        .insert_transformed(source, variant_name(language, &alternative.name))?,
                    // Ada renders alternatives as variant-part components.
                    BackendLanguage::Ada => {
                        members
                            .insert_transformed(source, member_name(language, &alternative.name))?;
                        if ada_emits_helper(alternative.cardinality) {
                            validate_ada_helpers(
                                top_level,
                                &declaration.name,
                                &alternative.name,
                                alternative.cardinality,
                            )?;
                        }
                    }
                }
            }
        }
        TypeKind::Primitive(_) | TypeKind::Alias(_) | TypeKind::List { .. } => {}
    }
    Ok(())
}

/// Record the flat-package helper type names Ada derives from a member name.
///
/// These are user-controlled: `{Owner}_{Member}_Sequence` and friends are
/// built from schema identifiers, so they can collide with a declared type or
/// with another member's helpers. Because the Ada package is flat, they are
/// registered in the **top-level** region alongside declared types rather than
/// in the owning declaration's member region.
fn validate_ada_helpers(
    top_level: &mut Region,
    owner: &QualifiedName,
    member: &str,
    cardinality: Cardinality,
) -> Result<(), BackendNameError> {
    let Some(member_identifier) = ada_identifier(member) else {
        // The member identifier itself already failed in the member region.
        return Ok(());
    };
    let stem = format!("{}_{member_identifier}", owner.local_name);
    // Exactly the suffixes `backend-ada` emits for each repeated shape.
    //
    // The unbounded case splits on the minimum: `write_unbounded_helper`
    // emits a plain vector alias when `min == 0`, but a required-prefix array
    // plus a separate vector package when `min > 0`. Reserving only the
    // `min == 0` spelling left `{stem}_Required_Array` and
    // `{stem}_Additional_Vectors` emitted but unregistered, so a user
    // declaration could silently collide with one.
    let suffixes: &[&str] = match cardinality.shape() {
        OccurrenceShape::Unbounded { min } if min > 0 => &[
            "_Item",
            "_Required_Array",
            "_Additional_Vectors",
            "_Sequence",
        ],
        OccurrenceShape::Unbounded { .. } => &["_Item", "_Vectors", "_Sequence"],
        _ => &["_Array", "_Sequence"],
    };
    for suffix in suffixes {
        // Attributed structurally to the owning declaration, so a collision
        // marks the declaration that actually generated the helper rather than
        // relying on the diagnostic label's shape.
        top_level.insert(
            NameSource::Helper {
                owner: owner.clone(),
                member: member.to_owned(),
            },
            format!("{stem}{suffix}"),
        )?;
    }
    Ok(())
}

/// Which declarations a backend cannot name safely, attributed individually.
///
/// # Why this exists beside [`validate_backend_names`]
///
/// Generation needs one deterministic first failure and stops there.
/// Capability analysis needs the opposite: the *set* of declarations
/// responsible, so it can report the rest of the schema honestly instead of
/// condemning all of it for one unsafe name. Real UCI, for example, contains
/// a handful of members whose generated identifiers are reserved words in one
/// backend or another; those declarations are genuinely not renderable, but
/// the thousands around them are.
///
/// Both views run the *same* registration logic over the same regions, so
/// there is still exactly one naming policy. This one collects rather than
/// returning early, then maps each failure back to the declaration it belongs
/// to.
///
/// A failure in the top-level region is attributed to whichever declarations
/// the generated name came from. A collision names two sources, and both are
/// implicated: neither can be emitted while the other exists.
///
/// Namespace/package failures are deliberately **not** included here. They
/// belong to no declaration and make the whole unit unusable, so they remain
/// a global precondition reported by [`crate::backend_preflight`].
#[must_use]
pub fn unsafe_named_declarations(
    schema: &SchemaIr,
    language: BackendLanguage,
    world: GenerationWorld,
) -> BTreeSet<QualifiedName> {
    let mut unsafe_names = BTreeSet::new();

    // Same single shared plan as validation, so capability attribution cannot
    // condemn a declaration for a name the backend never emits while
    // validation accepts it -- nor manufacture a naming failure for an entity
    // with no emitted surface at all, which the existing semantic coverage
    // rules already handle on their own terms.
    let plan = name_preflight_plan(schema, world);
    let emissions = plan.surfaces();
    let mut top_level = Region::collecting(language, NameRegion::TopLevel);
    // Ignoring the `Result` is correct for a collecting region: it only ever
    // returns `Ok`, accumulating into `errors` instead.
    let _ = register_support_names(&mut top_level, schema, language);
    let _ = register_emitted_declaration_names(&mut top_level, emissions, language);
    if language == BackendLanguage::Ada {
        let _ = register_ada_kind_companions(&mut top_level, emissions);
    }
    for emission in emissions {
        let owner = emission.name().clone();
        let mut members = Region::collecting(language, NameRegion::Members(owner.clone()));
        let _ = register_emission_names(
            schema,
            emission,
            language,
            world,
            &mut members,
            &mut top_level,
        );
        // A member failure implicates exactly the emitted entity that owns the
        // generated member region, which for a Task 024 wrapper is the wrapper
        // and for an ordinary declaration is that declaration.
        if !members.errors.is_empty() {
            unsafe_names.insert(owner);
        }
    }
    if language == BackendLanguage::Ada {
        let mut literals = Vec::new();
        collect_ada_literal_conflicts(&top_level, schema, emissions, &mut literals);
        collect_ada_float_callable_conflicts(&top_level, schema, &mut literals);
        for conflict in literals {
            unsafe_names.extend(conflict.owners);
        }
    }
    // Every declaration responsible for emitting a side of a top-level failure
    // is marked, taken from the structured owner recorded at registration
    // time. Names owned by no declaration -- generated support types, the
    // namespace unit -- contribute nothing on their own, because no
    // declaration is at fault for them alone.
    for conflict in std::mem::take(&mut top_level.errors) {
        unsafe_names.extend(conflict.owners);
    }
    unsafe_names
}

/// Whether every generated name one backend would emit for `schema` is safe.
///
/// This is the capability-analysis view of [`validate_backend_names`]; both
/// consult the same rules, so readiness cannot claim READY for output that
/// would not compile.
#[must_use]
pub fn backend_names_are_renderable(
    schema: &SchemaIr,
    language: BackendLanguage,
    world: GenerationWorld,
) -> bool {
    validate_backend_names(schema, language, world).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::{
        ConstraintSet, EnumVariant, FieldDecl, Float64Value, NamespaceDecl, NumericValue,
        PrimitiveKind, QualifiedName, SourceRef, TypeRef,
    };

    const NS: &str = "urn:example:oms";

    const UNBOUNDED: Cardinality = Cardinality {
        min_occurs: 0,
        max_occurs: None,
    };

    fn source() -> SourceRef {
        SourceRef {
            document: "test.ir".to_owned(),
            line: Some(1),
        }
    }

    fn primitive(name: &str) -> TypeDecl {
        TypeDecl {
            name: QualifiedName::new(NS, name),
            is_abstract: false,
            base_type: None,
            kind: TypeKind::Primitive(PrimitiveKind::String),
            constraints: ConstraintSet::default(),
            documentation: None,
            source: source(),
        }
    }

    fn field(name: &str, target: TypeRefTarget, cardinality: Cardinality) -> FieldDecl {
        FieldDecl {
            name: name.to_owned(),
            type_ref: TypeRef { target },
            cardinality,
            nillable: false,
            constraints: ConstraintSet::default(),
            documentation: None,
            source: source(),
        }
    }

    fn record(name: &str, fields: Vec<FieldDecl>) -> TypeDecl {
        TypeDecl {
            kind: TypeKind::Record { fields },
            ..primitive(name)
        }
    }

    fn choice(name: &str, alternatives: &[&str]) -> TypeDecl {
        TypeDecl {
            kind: TypeKind::Choice {
                alternatives: alternatives
                    .iter()
                    .map(|alternative| {
                        field(
                            alternative,
                            TypeRefTarget::Primitive(PrimitiveKind::String),
                            Cardinality::REQUIRED_ONE,
                        )
                    })
                    .collect(),
            },
            ..primitive(name)
        }
    }

    fn schema_with(types: Vec<TypeDecl>) -> SchemaIr {
        SchemaIr {
            schema_version: None,
            namespaces: vec![NamespaceDecl {
                uri: NS.to_owned(),
                preferred_prefix: None,
            }],
            types,
            messages: Vec::new(),
        }
    }

    /// A schema whose only repeated member is unbounded, so every backend's
    /// unbounded-sequence support type is emitted.
    fn unbounded_schema(extra: Vec<TypeDecl>) -> SchemaIr {
        let mut types = vec![
            primitive("Item"),
            record(
                "Holder",
                vec![field(
                    "Items",
                    TypeRefTarget::Named(QualifiedName::new(NS, "Item")),
                    UNBOUNDED,
                )],
            ),
        ];
        types.extend(extra);
        schema_with(types)
    }

    /// A schema with a directly constrained integral member, so the
    /// bounded-integer support types are emitted.
    fn bounded_integer_schema(extra: Vec<TypeDecl>) -> SchemaIr {
        let mut constrained = field(
            "Count",
            TypeRefTarget::Primitive(PrimitiveKind::UnsignedInteger),
            Cardinality::REQUIRED_ONE,
        );
        constrained.constraints = ConstraintSet {
            min_inclusive: Some(NumericValue::Integer(0)),
            max_inclusive: Some(NumericValue::Integer(255)),
            ..ConstraintSet::default()
        };
        let mut types = vec![record("Holder", vec![constrained])];
        types.extend(extra);
        schema_with(types)
    }

    fn assert_collides(schema: &SchemaIr, language: BackendLanguage, expected: &str) {
        let error = validate_backend_names(schema, language, GenerationWorld::ClosedSchemaSet)
            .expect_err("a generated name must not be silently shadowed");
        let BackendNameError::Collision { generated, .. } = &error else {
            panic!("expected a collision on {expected}, got {error:?}");
        };
        assert_eq!(generated, expected);
    }

    /// A named floating declaration with a supported bound-only domain, so
    /// Ada emits `Create` / `Value` for it.
    fn constrained_float(name: &str) -> TypeDecl {
        TypeDecl {
            kind: TypeKind::Primitive(PrimitiveKind::Float64),
            constraints: ConstraintSet {
                min_inclusive: Some(NumericValue::Float64(Float64Value::from_value(0.0))),
                max_inclusive: Some(NumericValue::Float64(Float64Value::from_value(1.0))),
                ..ConstraintSet::default()
            },
            ..primitive(name)
        }
    }

    /// Ada's Choice discriminant is the fixed identifier `Kind`, declared in
    /// the same record region as the alternatives. GNAT 14.2 rejects an
    /// alternative that reuses it with `"Kind" conflicts with declaration`,
    /// so preflight must too.
    #[test]
    fn an_ada_choice_alternative_named_kind_collides_with_the_discriminant() {
        let schema = schema_with(vec![choice("Selection", &["Kind", "Other"])]);
        assert_collides(&schema, BackendLanguage::Ada, "Kind");
    }

    /// The owning Choice generates the discriminant, so the Choice itself is
    /// what coverage must exclude.
    #[test]
    fn an_ada_kind_alternative_marks_the_owning_choice_unsafe() {
        let schema = schema_with(vec![choice("Selection", &["Kind", "Other"])]);
        assert_eq!(
            unsafe_named_declarations(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            ),
            BTreeSet::from([QualifiedName::new(NS, "Selection")])
        );
    }

    /// The discriminant rule is Ada's, derived from Ada's lowering. Rust
    /// enums and C++ `std::variant` alternatives carry no generated
    /// discriminant component, so they must not inherit it.
    #[test]
    fn rust_and_cpp_do_not_inherit_the_ada_kind_discriminant_rule() {
        let schema = schema_with(vec![choice("Selection", &["Kind", "Other"])]);
        for language in [BackendLanguage::Rust, BackendLanguage::Cpp] {
            assert!(
                validate_backend_names(&schema, language, GenerationWorld::ClosedSchemaSet).is_ok(),
                "{language:?} has no generated Kind discriminant to collide with"
            );
        }
    }

    /// Only the exact identifier is reserved; a merely similar alternative
    /// stays legal.
    #[test]
    fn an_ada_choice_alternative_named_kind_value_remains_safe() {
        let schema = schema_with(vec![choice("Selection", &["KindValue", "Other"])]);
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_ok()
        );
    }

    /// Closed-sum wrappers emit the same `Kind` discriminant, but their
    /// components are `{Descendant}_Value` -- derived from *declaration*
    /// names, never from a user-supplied member name. A descendant would have
    /// to be named `Kin` for `Kin_Value` to approach it, and no descendant
    /// spelling can produce the bare identifier `Kind`. No rule is therefore
    /// registered for wrapper members; this test records that determination
    /// so it is not mistaken for an oversight.
    #[test]
    fn closed_sum_wrapper_components_cannot_collide_with_their_kind_discriminant() {
        let mut base = primitive("Base");
        base.is_abstract = true;
        base.kind = TypeKind::Record { fields: vec![] };
        let mut descendant = record("Kind", vec![]);
        descendant.base_type = Some(TypeRef {
            target: TypeRefTarget::Named(QualifiedName::new(NS, "Base")),
        });
        let holder = record(
            "Holder",
            vec![field(
                "Payload",
                TypeRefTarget::Named(QualifiedName::new(NS, "Base")),
                Cardinality::REQUIRED_ONE,
            )],
        );
        let schema = schema_with(vec![base, descendant, holder]);
        // The descendant contributes the component `Kind_Value` and the
        // literal `Kind_Kind`, not `Kind`, so the discriminant is untouched.
        // Any rejection here must come from another rule, never from the
        // wrapper's own discriminant.
        let unsafe_names = unsafe_named_declarations(
            &schema,
            BackendLanguage::Ada,
            GenerationWorld::ClosedSchemaSet,
        );
        assert!(
            !unsafe_names.contains(&QualifiedName::new(NS, "Holder")),
            "no wrapper component can normalize onto the bare discriminant"
        );
    }

    /// A constrained float emits `function Create ...` into the package's
    /// visible part, where a type of the same name cannot coexist. Confirmed
    /// against GNAT 14.2.
    #[test]
    fn an_ada_type_named_create_collides_with_the_generated_float_constructor() {
        let schema = schema_with(vec![constrained_float("BurnRate"), primitive("Create")]);
        assert_collides(&schema, BackendLanguage::Ada, "Create");
    }

    #[test]
    fn an_ada_type_named_value_collides_with_the_generated_float_accessor() {
        let schema = schema_with(vec![constrained_float("BurnRate"), primitive("Value")]);
        assert_collides(&schema, BackendLanguage::Ada, "Value");
    }

    /// Both sides are implicated: the float that generates the subprogram and
    /// the declaration occupying the identifier.
    #[test]
    fn a_generated_float_callable_collision_marks_both_declarations() {
        let schema = schema_with(vec![constrained_float("BurnRate"), primitive("Create")]);
        assert_eq!(
            unsafe_named_declarations(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            ),
            BTreeSet::from([
                QualifiedName::new(NS, "BurnRate"),
                QualifiedName::new(NS, "Create"),
            ])
        );
    }

    /// Ada subprograms overload. Several constrained floats each emit a
    /// `Create` and a `Value`, and GNAT 14.2 accepts the result, so sharing
    /// the identifier must not be reported as a collision.
    #[test]
    fn several_constrained_floats_may_share_overloaded_create_and_value() {
        let schema = schema_with(vec![
            constrained_float("BurnRate"),
            constrained_float("AltitudeMeters"),
            constrained_float("UnitInterval"),
        ]);
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_ok(),
            "generated float subprograms overload rather than collide"
        );
        assert!(
            unsafe_named_declarations(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_empty()
        );
    }

    /// No constrained float means no generated subprogram, so the spelling
    /// stays available. Reserving an unemitted name would block a legal
    /// schema.
    #[test]
    fn create_and_value_are_free_when_no_constrained_float_is_declared() {
        let schema = schema_with(vec![primitive("Create"), primitive("Value")]);
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_ok()
        );
    }

    /// An *unconstrained* float emits a plain derived type and no
    /// subprograms, so it must not reserve the names either.
    #[test]
    fn an_unconstrained_float_reserves_no_callable_names() {
        let unconstrained = TypeDecl {
            kind: TypeKind::Primitive(PrimitiveKind::Float64),
            ..primitive("Bearing")
        };
        let schema = schema_with(vec![unconstrained, primitive("Create")]);
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_ok()
        );
    }

    /// The generated float subprograms are an Ada package-scope concern.
    /// Rust and C++ emit no such free functions.
    #[test]
    fn rust_and_cpp_do_not_reserve_create_or_value() {
        let schema = schema_with(vec![constrained_float("BurnRate"), primitive("Create")]);
        for language in [BackendLanguage::Rust, BackendLanguage::Cpp] {
            assert!(
                validate_backend_names(&schema, language, GenerationWorld::ClosedSchemaSet).is_ok(),
                "{language:?} emits no package-level Create"
            );
        }
    }

    fn schema_in(uri: &str) -> SchemaIr {
        SchemaIr {
            schema_version: None,
            namespaces: vec![NamespaceDecl {
                uri: uri.to_owned(),
                preferred_prefix: None,
            }],
            types: vec![TypeDecl {
                name: QualifiedName::new(uri, "Track"),
                ..primitive("Track")
            }],
            messages: Vec::new(),
        }
    }

    // ---- Unconditional generated support names -------------------------

    /// The defect this corrective fixes: `BoundedVec` is emitted by every
    /// Rust generation, so a declaration of that name produces two top-level
    /// items with one identifier.
    #[test]
    fn unconditional_support_names_are_reserved() {
        assert_collides(
            &schema_with(vec![primitive("BoundedVec")]),
            BackendLanguage::Rust,
            "BoundedVec",
        );
        assert_collides(
            &schema_with(vec![primitive("BoundedVector")]),
            BackendLanguage::Cpp,
            "BoundedVector",
        );
        assert_collides(
            &schema_with(vec![primitive("Optional_String")]),
            BackendLanguage::Ada,
            "Optional_String",
        );
    }

    // ---- Conditional support names, actually emitted -------------------

    #[test]
    fn conditional_support_names_collide_when_the_helper_is_emitted() {
        assert_collides(
            &unbounded_schema(vec![primitive("UnboundedVec")]),
            BackendLanguage::Rust,
            "UnboundedVec",
        );
        assert_collides(
            &unbounded_schema(vec![primitive("UnboundedVector")]),
            BackendLanguage::Cpp,
            "UnboundedVector",
        );
        assert_collides(
            &bounded_integer_schema(vec![primitive("BoundedI64")]),
            BackendLanguage::Rust,
            "BoundedI64",
        );
        assert_collides(
            &bounded_integer_schema(vec![primitive("BoundedU64")]),
            BackendLanguage::Rust,
            "BoundedU64",
        );
        assert_collides(
            &bounded_integer_schema(vec![primitive("BoundedInteger")]),
            BackendLanguage::Cpp,
            "BoundedInteger",
        );
    }

    /// Ada's `Binary_Vectors` package is only instantiated for a schema that
    /// actually uses the Binary primitive.
    #[test]
    fn ada_binary_vectors_collides_only_when_instantiated() {
        let binary = TypeDecl {
            kind: TypeKind::Primitive(PrimitiveKind::Binary),
            ..primitive("Payload")
        };
        assert_collides(
            &schema_with(vec![binary, primitive("Binary_Vectors")]),
            BackendLanguage::Ada,
            "Binary_Vectors",
        );
    }

    // ---- Control: not emitted, so not reserved -------------------------

    /// The control this corrective requires: reserving a conditional support
    /// name unconditionally would block a legal declaration. When the helper
    /// is not emitted, the spelling stays available.
    #[test]
    fn conditional_support_names_are_free_when_the_helper_is_not_emitted() {
        for (language, spelling) in [
            (BackendLanguage::Rust, "UnboundedVec"),
            (BackendLanguage::Rust, "BoundedI64"),
            (BackendLanguage::Rust, "BoundedU64"),
            (BackendLanguage::Cpp, "UnboundedVector"),
            (BackendLanguage::Cpp, "BoundedInteger"),
            (BackendLanguage::Ada, "Binary_Vectors"),
        ] {
            let schema = schema_with(vec![primitive(spelling)]);
            assert!(
                validate_backend_names(&schema, language, GenerationWorld::ClosedSchemaSet).is_ok(),
                "{language:?} must not reserve {spelling} for a schema that never emits it"
            );
        }
    }

    // ---- Emitted entities, not raw IR declarations ---------------------

    /// An abstract Record used only as ancestry, plus a concrete descendant
    /// and a value position that refers to the *descendant*. No backend emits
    /// the base, so it is a naming no-op.
    fn ancestry_only_schema(base_name: &str) -> SchemaIr {
        let mut base = record(
            base_name,
            vec![field(
                "Tag",
                TypeRefTarget::Primitive(PrimitiveKind::String),
                Cardinality::REQUIRED_ONE,
            )],
        );
        base.is_abstract = true;
        let mut derived = record("Derived", Vec::new());
        derived.base_type = Some(TypeRef {
            target: TypeRefTarget::Named(QualifiedName::new(NS, base_name)),
        });
        let holder = record(
            "Holder",
            vec![field(
                "Value",
                TypeRefTarget::Named(QualifiedName::new(NS, "Derived")),
                Cardinality::REQUIRED_ONE,
            )],
        );
        schema_with(vec![base, derived, holder])
    }

    /// The defect this corrective fixes. Every backend folds an abstract
    /// Record into its descendants and emits no type for the base, so the
    /// base's identifier is not taken in the generated scope. Reserving it
    /// rejected schemas whose only conflict was against a declaration that is
    /// never generated.
    #[test]
    fn an_ancestry_only_abstract_record_does_not_reserve_a_support_name() {
        for (language, spelling) in [
            (BackendLanguage::Rust, "BoundedVec"),
            (BackendLanguage::Cpp, "BoundedVector"),
            (BackendLanguage::Ada, "Optional_String"),
        ] {
            let schema = ancestry_only_schema(spelling);
            assert!(
                validate_backend_names(&schema, language, GenerationWorld::ClosedSchemaSet).is_ok(),
                "{language:?} must not reserve {spelling} for an abstract base it never emits"
            );
            assert!(
                !unsafe_named_declarations(&schema, language, GenerationWorld::ClosedSchemaSet)
                    .contains(&QualifiedName::new(NS, spelling)),
                "{language:?} must not condemn {spelling} for a name it never emits"
            );
        }
    }

    /// The other side of the same rule: a *concrete* declaration really is
    /// emitted, so it still owns its identifier and still collides.
    #[test]
    fn a_concrete_declaration_still_reserves_its_support_name() {
        assert_collides(
            &schema_with(vec![record("BoundedVec", Vec::new())]),
            BackendLanguage::Rust,
            "BoundedVec",
        );
        assert_collides(
            &schema_with(vec![record("Optional_String", Vec::new())]),
            BackendLanguage::Ada,
            "Optional_String",
        );
    }

    /// An abstract **Choice** is not skipped by any backend, so unlike an
    /// abstract Record it genuinely appears in the output and must keep its
    /// reservation. This guards the fix against over-reaching into a
    /// blanket "abstract is never emitted" rule.
    #[test]
    fn an_ancestry_only_abstract_choice_still_reserves_its_name() {
        let mut base = choice("BoundedVec", &["Alpha", "Beta"]);
        base.is_abstract = true;
        assert_collides(
            &schema_with(vec![base]),
            BackendLanguage::Rust,
            "BoundedVec",
        );
    }

    /// A real Task 024 wrapper *is* emitted under the closed world, so the
    /// abstract base's own name stays reserved.
    #[test]
    fn a_real_abstract_value_wrapper_still_reserves_its_name() {
        let mut base = record("BoundedVec", Vec::new());
        base.is_abstract = true;
        let mut concrete = record("Concrete", Vec::new());
        concrete.base_type = Some(TypeRef {
            target: TypeRefTarget::Named(QualifiedName::new(NS, "BoundedVec")),
        });
        let holder = record(
            "Holder",
            vec![field(
                "Value",
                TypeRefTarget::Named(QualifiedName::new(NS, "BoundedVec")),
                Cardinality::REQUIRED_ONE,
            )],
        );
        assert_collides(
            &schema_with(vec![base, concrete, holder]),
            BackendLanguage::Rust,
            "BoundedVec",
        );
    }

    /// Task 026: a zero-descendant target used only as supported absent-only
    /// optional storage emits no wrapper and no declaration, so it reserves
    /// neither its own name nor an Ada `_Kind` companion.
    #[test]
    fn a_task_026_elided_target_reserves_no_declaration_name() {
        for (language, spelling) in [
            (BackendLanguage::Rust, "BoundedVec"),
            (BackendLanguage::Ada, "Optional_String"),
        ] {
            let mut target = record(spelling, Vec::new());
            target.is_abstract = true;
            let holder = record(
                "Holder",
                vec![
                    field(
                        "Required",
                        TypeRefTarget::Primitive(PrimitiveKind::String),
                        Cardinality::REQUIRED_ONE,
                    ),
                    field(
                        "Maybe",
                        TypeRefTarget::Named(QualifiedName::new(NS, spelling)),
                        Cardinality {
                            min_occurs: 0,
                            max_occurs: Some(1),
                        },
                    ),
                ],
            );
            let schema = schema_with(vec![target, holder]);
            assert!(
                validate_backend_names(&schema, language, GenerationWorld::ClosedSchemaSet).is_ok(),
                "{language:?} must not reserve {spelling} for a Task 026 elided target"
            );
        }
    }

    /// Inherited members still belong to the emitted descendant's scope. The
    /// base is not emitted, but its fields are folded into `Derived`, so an
    /// unsafe effective member must still condemn the descendant.
    #[test]
    fn inherited_members_of_a_non_emitted_base_still_condemn_the_descendant() {
        let mut base = record(
            "BoundedVec",
            vec![field(
                // A Rust reserved word, reached only through inheritance.
                "match",
                TypeRefTarget::Primitive(PrimitiveKind::String),
                Cardinality::REQUIRED_ONE,
            )],
        );
        base.is_abstract = true;
        let mut derived = record("Derived", Vec::new());
        derived.base_type = Some(TypeRef {
            target: TypeRefTarget::Named(QualifiedName::new(NS, "BoundedVec")),
        });
        let schema = schema_with(vec![base, derived]);
        let unsafe_names = unsafe_named_declarations(
            &schema,
            BackendLanguage::Rust,
            GenerationWorld::ClosedSchemaSet,
        );
        assert!(
            unsafe_names.contains(&QualifiedName::new(NS, "Derived")),
            "an inherited reserved member must still condemn the emitted descendant"
        );
    }

    /// A non-emitted abstract Record's repeated field generates **no** Ada
    /// helper under the base's name. `backend-ada` returns early for an
    /// abstract Record, so the only helpers it writes for the inherited field
    /// are `Derived_Items_*`; reserving `Base_Items_Array` would falsely
    /// reject a user declaration that really can be emitted.
    #[test]
    fn a_non_emitted_abstract_record_reserves_no_ada_helper_names() {
        let schema = abstract_helper_schema("Base_Items_Array");
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_ok(),
            "no helper is emitted under a non-emitted abstract Record owner"
        );
        assert!(
            unsafe_named_declarations(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_empty(),
            "coverage must not condemn anything for a phantom helper surface"
        );
    }

    /// The counterpart, proving the fix is not simply "skip inherited
    /// members": the helper Ada really emits for the inherited field is named
    /// after the **emitted** descendant, so colliding with that spelling is a
    /// genuine failure.
    #[test]
    fn an_inherited_helper_under_the_emitted_descendant_still_collides() {
        let schema = abstract_helper_schema("Derived_Items_Array");
        assert_collides(&schema, BackendLanguage::Ada, "Derived_Items_Array");
        assert!(
            unsafe_named_declarations(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .contains(&QualifiedName::new(NS, "Derived")),
            "the emitted helper owner must be condemned"
        );
    }

    /// Task 034: the same emitted-owner rule, for the new optional wrapper.
    ///
    /// `backend-ada` returns early for an abstract Record, so the only wrapper
    /// it writes for the inherited optional field is `Derived_Maybe_Optional`.
    /// Reserving `Base_Maybe_Optional` would falsely reject a user declaration
    /// that really can be emitted -- exactly the class of defect PR #34 fixed
    /// for repeated helpers.
    #[test]
    fn a_non_emitted_abstract_record_reserves_no_ada_optional_wrapper_name() {
        let schema = abstract_optional_schema("Base_Maybe_Optional");
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_ok(),
            "no optional wrapper is emitted under a non-emitted abstract Record owner"
        );
        assert!(
            unsafe_named_declarations(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_empty(),
            "coverage must not condemn anything for a phantom wrapper surface"
        );
    }

    /// The counterpart: the wrapper Ada really emits for the inherited
    /// optional field is named after the **emitted** descendant, so colliding
    /// with that spelling is a genuine failure that must mark both
    /// responsible declarations.
    #[test]
    fn an_inherited_optional_wrapper_under_the_emitted_descendant_still_collides() {
        let schema = abstract_optional_schema("Derived_Maybe_Optional");
        assert_collides(&schema, BackendLanguage::Ada, "Derived_Maybe_Optional");
        let condemned = unsafe_named_declarations(
            &schema,
            BackendLanguage::Ada,
            GenerationWorld::ClosedSchemaSet,
        );
        assert!(
            condemned.contains(&QualifiedName::new(NS, "Derived")),
            "the emitted wrapper owner must be condemned"
        );
        assert!(
            condemned.contains(&QualifiedName::new(NS, "Derived_Maybe_Optional")),
            "the colliding user declaration must be condemned too"
        );
    }

    /// Rust and C++ derive no top-level name from an optional field at all, so
    /// the Task 034 registration must stay Ada-only.
    #[test]
    fn an_optional_wrapper_spelling_is_free_in_rust_and_cpp() {
        let schema = abstract_optional_schema("Derived_Maybe_Optional");
        for language in [BackendLanguage::Rust, BackendLanguage::Cpp] {
            assert!(
                validate_backend_names(&schema, language, GenerationWorld::ClosedSchemaSet).is_ok(),
                "{language:?} generates no optional wrapper name"
            );
        }
    }

    /// `abstract Base { Maybe : Item [0..1] }`, `Derived extends Base`, plus a
    /// user declaration spelled `user_type`. Only that spelling differs
    /// between the two tests above, which is exactly the scope distinction.
    fn abstract_optional_schema(user_type: &str) -> SchemaIr {
        let mut base = record(
            "Base",
            vec![field(
                "Maybe",
                TypeRefTarget::Named(QualifiedName::new(NS, "Item")),
                Cardinality::OPTIONAL_ONE,
            )],
        );
        base.is_abstract = true;
        let mut derived = record("Derived", Vec::new());
        derived.base_type = Some(TypeRef {
            target: TypeRefTarget::Named(QualifiedName::new(NS, "Base")),
        });
        schema_with(vec![
            primitive("Item"),
            base,
            derived,
            record(
                "Holder",
                vec![field(
                    "Item",
                    TypeRefTarget::Named(QualifiedName::new(NS, "Derived")),
                    Cardinality::REQUIRED_ONE,
                )],
            ),
            primitive(user_type),
        ])
    }

    const BOUNDED_FOUR: Cardinality = Cardinality {
        min_occurs: 0,
        max_occurs: Some(4),
    };

    /// `abstract Base { Items : Item [0..4] }`, `Derived extends Base`, plus a
    /// user declaration spelled `user_type`. Only the helper stem differs
    /// between the two tests above, which is exactly the scope distinction.
    fn abstract_helper_schema(user_type: &str) -> SchemaIr {
        let mut base = record(
            "Base",
            vec![field(
                "Items",
                TypeRefTarget::Named(QualifiedName::new(NS, "Item")),
                BOUNDED_FOUR,
            )],
        );
        base.is_abstract = true;
        let mut derived = record("Derived", Vec::new());
        derived.base_type = Some(TypeRef {
            target: TypeRefTarget::Named(QualifiedName::new(NS, "Base")),
        });
        schema_with(vec![
            primitive("Item"),
            base,
            derived,
            record(
                "Holder",
                vec![field(
                    "Item",
                    TypeRefTarget::Named(QualifiedName::new(NS, "Derived")),
                    Cardinality::REQUIRED_ONE,
                )],
            ),
            primitive(user_type),
        ])
    }

    /// A reserved-word member on an ancestry-only abstract Record is a member
    /// of a record that is never written. Ada must not diagnose `Base` for it,
    /// but must diagnose `Derived`, whose effective emitted record really does
    /// contain the component.
    #[test]
    fn a_reserved_member_on_a_non_emitted_base_is_attributed_to_the_descendant() {
        let mut base = record(
            "Base",
            vec![field(
                // `Range` is an Ada reserved word.
                "Range",
                TypeRefTarget::Primitive(PrimitiveKind::String),
                Cardinality::REQUIRED_ONE,
            )],
        );
        base.is_abstract = true;
        let mut derived = record("Derived", Vec::new());
        derived.base_type = Some(TypeRef {
            target: TypeRefTarget::Named(QualifiedName::new(NS, "Base")),
        });
        let schema = schema_with(vec![base, derived]);
        let unsafe_names = unsafe_named_declarations(
            &schema,
            BackendLanguage::Ada,
            GenerationWorld::ClosedSchemaSet,
        );
        assert!(
            unsafe_names.contains(&QualifiedName::new(NS, "Derived")),
            "the emitted descendant's real component must be diagnosed"
        );
        assert!(
            !unsafe_names.contains(&QualifiedName::new(NS, "Base")),
            "a non-emitted abstract Record has no member region to diagnose"
        );
    }

    /// A Task 024 wrapper's generated member surface is `Kind` plus one
    /// `{Descendant}_Value` component -- never the original abstract Record's
    /// fields. Every base field necessarily reappears in the concrete
    /// descendant, so member *spelling* cannot distinguish the two scopes;
    /// the Ada helper **owner** can, and does: the real helper is
    /// `Concrete_Items_Array`, never `Base_Items_Array`.
    #[test]
    fn a_task_024_wrapper_does_not_validate_the_original_record_surface() {
        let mut base = record(
            "Base",
            vec![field(
                "Items",
                TypeRefTarget::Named(QualifiedName::new(NS, "Item")),
                BOUNDED_FOUR,
            )],
        );
        base.is_abstract = true;
        let mut concrete = record("Concrete", Vec::new());
        concrete.base_type = Some(TypeRef {
            target: TypeRefTarget::Named(QualifiedName::new(NS, "Base")),
        });
        let types = vec![
            primitive("Item"),
            base,
            concrete,
            record(
                "Holder",
                vec![field(
                    "Value",
                    TypeRefTarget::Named(QualifiedName::new(NS, "Base")),
                    Cardinality::REQUIRED_ONE,
                )],
            ),
        ];
        // The wrapper really is emitted here, so `Base` and `Base_Kind` stay
        // reserved -- but `Base_Items_Array` is not a name anything emits.
        let mut with_phantom = types.clone();
        with_phantom.push(primitive("Base_Items_Array"));
        assert!(
            validate_backend_names(
                &schema_with(with_phantom),
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_ok(),
            "the wrapper's member surface is not the abstract Record's fields"
        );
        let mut with_real = types;
        with_real.push(primitive("Concrete_Items_Array"));
        assert_collides(
            &schema_with(with_real),
            BackendLanguage::Ada,
            "Concrete_Items_Array",
        );
    }

    /// When the emission plan cannot be formed there is no emitted surface, so
    /// naming must stay silent and let the semantic failure be authoritative.
    ///
    /// `BoundedVec` is deliberately also the Rust support type's spelling: the
    /// previous raw-schema fallback manufactured exactly that collision, which
    /// described output the open world can never produce.
    #[test]
    fn a_failed_emission_plan_yields_no_name_verdict() {
        let mut base = record("BoundedVec", Vec::new());
        base.is_abstract = true;
        let mut concrete = record("Concrete", Vec::new());
        concrete.base_type = Some(TypeRef {
            target: TypeRefTarget::Named(QualifiedName::new(NS, "BoundedVec")),
        });
        let schema = schema_with(vec![
            base,
            concrete,
            record(
                "Holder",
                vec![field(
                    "Value",
                    TypeRefTarget::Named(QualifiedName::new(NS, "BoundedVec")),
                    Cardinality::REQUIRED_ONE,
                )],
            ),
        ]);
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Rust,
                GenerationWorld::OpenExtensions
            )
            .is_ok(),
            "an open-world abstract value is a semantic failure, not a naming one"
        );
        assert!(
            unsafe_named_declarations(
                &schema,
                BackendLanguage::Rust,
                GenerationWorld::OpenExtensions
            )
            .is_empty(),
            "no emitted surface means nothing to attribute"
        );
        // The very same schema under the closed world does plan, and there the
        // wrapper genuinely takes `BoundedVec`, so the collision is real.
        assert_collides(&schema, BackendLanguage::Rust, "BoundedVec");
    }

    /// Deferral must be **scoped to the entity that cannot be emitted**, not
    /// applied to the whole schema.
    ///
    /// `plan_type_emissions` fails closed globally: one unrepresentable target
    /// aborts the entire plan. Treating that as "no surface exists" silently
    /// stopped checking every other declaration's names, which restored the
    /// previously rejected coverage overclaims on authoritative UCI. The
    /// unrelated collision here must still be caught even though a different
    /// declaration makes whole-schema planning fail.
    #[test]
    fn a_global_planning_abort_still_checks_unrelated_emitted_names() {
        // A zero-descendant abstract value target demanded by value: the
        // planner cannot represent it, so the whole plan fails.
        let mut uninhabited = record("Uninhabited", Vec::new());
        uninhabited.is_abstract = true;
        let schema = schema_with(vec![
            uninhabited,
            record(
                "Demand",
                vec![field(
                    "Value",
                    TypeRefTarget::Named(QualifiedName::new(NS, "Uninhabited")),
                    Cardinality::REQUIRED_ONE,
                )],
            ),
            // Entirely unrelated to the failing target, and genuinely emitted.
            primitive("foo_bar"),
            primitive("fooBar"),
        ]);
        assert!(
            crate::plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).is_err(),
            "the fixture must actually make whole-schema planning fail"
        );
        assert_collides(&schema, BackendLanguage::Rust, "FooBar");
        let unsafe_names = unsafe_named_declarations(
            &schema,
            BackendLanguage::Rust,
            GenerationWorld::ClosedSchemaSet,
        );
        assert!(
            unsafe_names.contains(&QualifiedName::new(NS, "foo_bar"))
                && unsafe_names.contains(&QualifiedName::new(NS, "fooBar")),
            "both converging emitted declarations must still be attributed"
        );
        // The unrepresentable target still contributes no surface of its own.
        assert!(
            !unsafe_names.contains(&QualifiedName::new(NS, "Uninhabited")),
            "an entity with no emitted surface must not gain a naming verdict"
        );
    }

    /// Fallback must classify abstract declarations with the **same** notion
    /// of "abstract value target" the normal planner uses.
    ///
    /// `project_abstract_value` answers "can this abstract declaration be
    /// represented as a closed sum?". It does *not* answer "is this
    /// declaration actually demanded as a generated abstract value?" -- only
    /// `abstract_value_targets` answers that. Treating a successful
    /// projection as evidence of demand let the fallback promote an
    /// ancestry-only abstract Record into a phantom Task 024 wrapper whenever
    /// an *unrelated* declaration aborted whole-schema planning.
    ///
    /// Here `BoundedVec` is reached only through `base_type`, so nothing is
    /// rendered for it, yet it projects successfully (because `Derived`
    /// exists) and it is spelled exactly like Rust's own emitted support
    /// type. The phantom wrapper therefore manufactured a `BoundedVec`
    /// collision that masked the real `Uninhabited` semantic failure.
    #[test]
    fn a_global_planning_abort_does_not_promote_ancestry_only_abstracts_to_wrappers() {
        // Area A: ancestry only. No value reference to `BoundedVec` exists.
        let mut bounded_vec = record("BoundedVec", Vec::new());
        bounded_vec.is_abstract = true;
        let mut derived = record("Derived", Vec::new());
        derived.base_type = Some(TypeRef {
            target: TypeRefTarget::Named(QualifiedName::new(NS, "BoundedVec")),
        });
        // Area B: a real semantic failure, entirely unrelated to area A.
        let mut uninhabited = record("Uninhabited", Vec::new());
        uninhabited.is_abstract = true;
        let schema = schema_with(vec![
            bounded_vec,
            derived,
            record(
                "Holder",
                vec![field(
                    "Item",
                    TypeRefTarget::Named(QualifiedName::new(NS, "Derived")),
                    Cardinality::REQUIRED_ONE,
                )],
            ),
            uninhabited,
            record(
                "Demand",
                vec![field(
                    "Value",
                    TypeRefTarget::Named(QualifiedName::new(NS, "Uninhabited")),
                    Cardinality::REQUIRED_ONE,
                )],
            ),
        ]);
        let bounded_vec_name = QualifiedName::new(NS, "BoundedVec");

        // The fixture must actually distinguish the two concepts.
        assert!(
            !crate::abstract_value_targets(&schema)
                .iter()
                .any(|declaration| declaration.name == bounded_vec_name),
            "ancestry alone must not make `BoundedVec` an abstract value target"
        );
        assert!(
            crate::project_abstract_value(&schema, &bounded_vec_name).is_ok(),
            "the projection must succeed, or this test would not separate \
             `projectable` from `demanded`"
        );
        assert!(
            crate::plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).is_err(),
            "the fixture must actually make whole-schema planning fail"
        );

        // The phantom wrapper must not appear, so no `BoundedVec` collision
        // with Rust's generated support type is reported.
        assert_eq!(
            validate_backend_names(
                &schema,
                BackendLanguage::Rust,
                GenerationWorld::ClosedSchemaSet
            ),
            Ok(()),
            "an ancestry-only abstract owns no generated name, so it cannot \
             collide with the `BoundedVec` support type"
        );
        assert!(
            !unsafe_named_declarations(
                &schema,
                BackendLanguage::Rust,
                GenerationWorld::ClosedSchemaSet
            )
            .contains(&bounded_vec_name),
            "an ancestry-only abstract must not be attributed a phantom wrapper"
        );
    }

    /// The predicates preflight uses are the renderers' own predicates, so
    /// they must track the schema shape rather than be always-true.
    #[test]
    fn support_predicates_track_the_schema_shape() {
        let plain = schema_with(vec![primitive("Item")]);
        assert!(!schema_emits_unbounded_sequence_support(&plain));
        assert!(schema_emits_unbounded_sequence_support(&unbounded_schema(
            Vec::new()
        )));
        assert!(!schema_emits_bounded_integer_support(&plain));
        assert!(schema_emits_bounded_integer_support(
            &bounded_integer_schema(Vec::new())
        ));
        assert!(!schema_emits_ada_binary_vectors(&plain));
    }

    // ---- Ada `_Kind` companions ---------------------------------------

    /// `Selection` as a Choice generates the companion `Selection_Kind`,
    /// which shares Ada's flat package with a user declaration.
    #[test]
    fn ada_kind_companion_collides_with_a_user_declaration() {
        let schema = schema_with(vec![
            choice("Selection", &["First", "Second"]),
            primitive("Selection_Kind"),
        ]);
        assert_collides(&schema, BackendLanguage::Ada, "Selection_Kind");
    }

    /// Rust and C++ nest their variants, so the same schema is safe there.
    /// This keeps the companion rule Ada-specific instead of copied to every
    /// backend.
    #[test]
    fn kind_companion_is_an_ada_only_rule() {
        let schema = schema_with(vec![
            choice("Selection", &["First", "Second"]),
            primitive("Selection_Kind"),
        ]);
        for language in [BackendLanguage::Rust, BackendLanguage::Cpp] {
            assert!(
                validate_backend_names(&schema, language, GenerationWorld::ClosedSchemaSet).is_ok()
            );
        }
    }

    /// An Ada enumeration literal conflicts with a *type* name in the same
    /// package. Verified against GNAT 14.2.
    #[test]
    fn ada_enumeration_literal_conflicts_with_a_type_name() {
        let enumeration = TypeDecl {
            kind: TypeKind::Enumeration {
                variants: ["Red", "Green"]
                    .into_iter()
                    .map(|value| EnumVariant {
                        wire_value: value.to_owned(),
                        documentation: None,
                    })
                    .collect(),
            },
            ..primitive("Color")
        };
        assert_collides(
            &schema_with(vec![enumeration, primitive("Red")]),
            BackendLanguage::Ada,
            "Red",
        );
    }

    /// Two Ada enumerations may share a literal spelling: literals overload
    /// rather than conflict. Verified against GNAT 14.2, so modelling this as
    /// a collision would reject valid Ada.
    #[test]
    fn ada_enumeration_literals_may_overload_each_other() {
        let schema = schema_with(vec![
            choice("Alpha", &["Shared", "OnlyAlpha"]),
            choice("Beta", &["Shared", "OnlyBeta"]),
        ]);
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_ok(),
            "Ada enumeration literals overload; this must not be a collision"
        );
    }

    // ---- Namespace / package identifiers -------------------------------

    /// A URI component that becomes the Ada package identifier `Record` makes
    /// the whole unit illegal, and Ada reserved words are case-insensitive.
    #[test]
    fn ada_package_component_may_not_be_a_reserved_word() {
        let error = validate_backend_names(
            &schema_in("urn:backend-record"),
            BackendLanguage::Ada,
            GenerationWorld::ClosedSchemaSet,
        )
        .expect_err("Ada package component `Record` is a reserved word");
        assert!(
            matches!(
                &error,
                BackendNameError::ReservedWord {
                    region: NameRegion::NamespaceUnit,
                    generated,
                    ..
                } if generated == "Record"
            ),
            "got {error:?}"
        );
    }

    /// The C++ equivalent: a component normalizing to `class`.
    #[test]
    fn cpp_namespace_component_may_not_be_a_reserved_word() {
        let error = validate_backend_names(
            &schema_in("urn:oms:class"),
            BackendLanguage::Cpp,
            GenerationWorld::ClosedSchemaSet,
        )
        .expect_err("C++ namespace component `class` is a reserved word");
        assert!(
            matches!(
                &error,
                BackendNameError::ReservedWord {
                    region: NameRegion::NamespaceUnit,
                    generated,
                    ..
                } if generated == "class"
            ),
            "got {error:?}"
        );
    }

    /// A safe namespace must keep generating for every backend: this module
    /// only rejects, and must not start rejecting ordinary input.
    #[test]
    fn a_safe_namespace_passes_every_backend() {
        let schema = schema_in("urn:example:oms:track");
        for language in BackendLanguage::ALL {
            assert!(
                validate_backend_names(&schema, language, GenerationWorld::ClosedSchemaSet).is_ok(),
                "{language:?} must accept a safe namespace"
            );
        }
    }

    /// `binary_search` is only correct on a sorted slice, so the reserved-word
    /// tables must stay sorted. A silently unsorted table would make some
    /// reserved words undetectable.
    #[test]
    fn reserved_word_tables_are_sorted_for_binary_search() {
        for (name, table) in [
            ("Rust", RUST_RESERVED),
            ("C++", CPP_RESERVED),
            ("Ada", ADA_RESERVED),
        ] {
            let mut sorted = table.to_vec();
            sorted.sort_unstable();
            assert_eq!(table, sorted.as_slice(), "{name} reserved words must sort");
        }
    }

    /// The defect this module exists to catch: two distinct IR spellings
    /// normalizing onto one generated identifier.
    #[test]
    fn distinct_ir_names_can_converge_on_one_generated_name() {
        assert_eq!(upper_camel("foo_bar"), upper_camel("fooBar"));
        assert_eq!(upper_camel("foo_bar").as_deref(), Some("FooBar"));
    }

    /// Existing accepted UCI-shaped identifiers must keep their exact
    /// generated spelling; this module only rejects, it never renames.
    #[test]
    fn accepted_identifier_spellings_are_unchanged() {
        assert_eq!(upper_camel("SystemID").as_deref(), Some("SystemID"));
        assert_eq!(snake_case("Track_Id").as_deref(), Some("track_id"));
        assert_eq!(ada_identifier("Track_Id").as_deref(), Some("Track_Id"));
        assert_eq!(ada_identifier("Trailing_").as_deref(), None);
        assert_eq!(ada_identifier("Double__Underscore").as_deref(), None);
    }

    #[test]
    fn reserved_word_detection_is_case_insensitive_only_for_ada() {
        assert!(is_reserved(BackendLanguage::Ada, "Range"));
        assert!(is_reserved(BackendLanguage::Ada, "RANGE"));
        assert!(is_reserved(BackendLanguage::Ada, "range"));
        assert!(is_reserved(BackendLanguage::Rust, "type"));
        assert!(is_reserved(BackendLanguage::Rust, "Self"));
        assert!(
            !is_reserved(BackendLanguage::Rust, "Type"),
            "Rust is case-sensitive, so `Type` is a legal identifier"
        );
        assert!(is_reserved(BackendLanguage::Cpp, "class"));
        assert!(!is_reserved(BackendLanguage::Cpp, "Class"));
    }

    /// Ada's case-insensitive identity must make case-only differences
    /// collide, while Rust and C++ treat them as distinct.
    #[test]
    fn ada_identity_key_folds_case_and_others_do_not() {
        assert_eq!(
            identity_key(BackendLanguage::Ada, "Track_Id"),
            identity_key(BackendLanguage::Ada, "TRACK_ID")
        );
        assert_ne!(
            identity_key(BackendLanguage::Rust, "TrackId"),
            identity_key(BackendLanguage::Rust, "Trackid")
        );
    }

    /// Only repeated occurrences make Ada emit user-derived helper types.
    #[test]
    fn ada_helper_emission_tracks_repeated_cardinality_only() {
        assert!(!ada_emits_helper(Cardinality::REQUIRED_ONE));
        assert!(!ada_emits_helper(Cardinality::OPTIONAL_ONE));
        assert!(ada_emits_helper(Cardinality {
            min_occurs: 0,
            max_occurs: Some(4),
        }));
        assert!(ada_emits_helper(Cardinality {
            min_occurs: 0,
            max_occurs: None,
        }));
    }

    // ---- Generated-name ownership attribution --------------------------
    //
    // Generation already rejected every schema below. The defect these cover
    // is the *coverage* side: attribution used to recover the owner by
    // splitting the human-readable diagnostic label, which matched nothing for
    // a helper, companion, or literal, so the declaration that generated the
    // conflicting identifier stayed counted renderable while generation
    // refused the package. Each asserts **both** responsible declarations,
    // because neither side can be emitted while the other exists.

    fn unsafe_local_names(schema: &SchemaIr, language: BackendLanguage) -> BTreeSet<String> {
        unsafe_named_declarations(schema, language, GenerationWorld::ClosedSchemaSet)
            .into_iter()
            .map(|name| name.local_name)
            .collect()
    }

    fn unsafe_ada(schema: &SchemaIr) -> BTreeSet<String> {
        unsafe_local_names(schema, BackendLanguage::Ada)
    }

    /// `Owner.Items` at unbounded cardinality emits `Owner_Items_Sequence`,
    /// which a user declaration of that name cannot coexist with. Both the
    /// helper's owner and the colliding declaration are unsafe.
    #[test]
    fn ada_helper_collision_marks_the_generating_owner_and_the_declaration() {
        let schema = schema_with(vec![
            primitive("Item"),
            record(
                "Owner",
                vec![field(
                    "Items",
                    TypeRefTarget::Named(QualifiedName::new(NS, "Item")),
                    UNBOUNDED,
                )],
            ),
            primitive("Owner_Items_Sequence"),
        ]);
        // Generation rejects it, so coverage must not call either side
        // renderable.
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_err()
        );
        let unsafe_names = unsafe_ada(&schema);
        assert!(
            unsafe_names.contains("Owner"),
            "the declaration that generated the helper must be unsafe: {unsafe_names:?}"
        );
        assert!(
            unsafe_names.contains("Owner_Items_Sequence"),
            "the colliding user declaration must be unsafe: {unsafe_names:?}"
        );
    }

    /// Two owners whose repeated members derive the same helper stem. Neither
    /// label is a declaration local name, so label-based attribution marked
    /// nothing at all; structured ownership marks both owners.
    #[test]
    fn ada_helper_versus_helper_marks_both_owners() {
        // `OwnerA` + member `B_Items` and `OwnerA_B` + member `Items` both
        // derive the stem `OwnerA_B_Items`.
        let repeated = |name: &str| {
            field(
                name,
                TypeRefTarget::Named(QualifiedName::new(NS, "Item")),
                UNBOUNDED,
            )
        };
        let schema = schema_with(vec![
            primitive("Item"),
            record("OwnerA", vec![repeated("B_Items")]),
            record("OwnerA_B", vec![repeated("Items")]),
        ]);
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_err()
        );
        let unsafe_names = unsafe_ada(&schema);
        assert!(
            unsafe_names.contains("OwnerA") && unsafe_names.contains("OwnerA_B"),
            "both helper-generating owners must be unsafe: {unsafe_names:?}"
        );
    }

    /// `type Color is (Red, Green);` beside `type Red` is rejected by GNAT.
    /// `Color` generated the literal and `Red` generated the type, so marking
    /// only `Red` would leave `Color` counted renderable.
    #[test]
    fn ada_enumeration_literal_collision_marks_the_enumeration_and_the_type() {
        let enumeration = TypeDecl {
            kind: TypeKind::Enumeration {
                variants: ["Red", "Green"]
                    .into_iter()
                    .map(|value| EnumVariant {
                        wire_value: value.to_owned(),
                        documentation: None,
                    })
                    .collect(),
            },
            ..primitive("Color")
        };
        let unsafe_names = unsafe_ada(&schema_with(vec![enumeration, primitive("Red")]));
        assert!(
            unsafe_names.contains("Color"),
            "the enumeration that generated the literal must be unsafe: {unsafe_names:?}"
        );
        assert!(
            unsafe_names.contains("Red"),
            "the colliding type declaration must be unsafe: {unsafe_names:?}"
        );
    }

    /// A Choice alternative's literal is `{Alternative}_Kind`, and the owning
    /// Choice is what generated it.
    #[test]
    fn ada_choice_literal_collision_marks_the_choice_and_the_type() {
        let unsafe_names = unsafe_ada(&schema_with(vec![
            choice("Selection", &["First", "Second"]),
            primitive("First_Kind"),
        ]));
        assert!(
            unsafe_names.contains("Selection"),
            "the Choice that generated the literal must be unsafe: {unsafe_names:?}"
        );
        assert!(
            unsafe_names.contains("First_Kind"),
            "the colliding type declaration must be unsafe: {unsafe_names:?}"
        );
    }

    /// Control: an ordinary declaration-versus-declaration collision keeps its
    /// existing behaviour, marking exactly the two declarations involved.
    #[test]
    fn ordinary_declaration_collision_attribution_is_unchanged() {
        let schema = schema_with(vec![primitive("foo_bar"), primitive("fooBar")]);
        let unsafe_names = unsafe_local_names(&schema, BackendLanguage::Rust);
        assert!(unsafe_names.contains("foo_bar") && unsafe_names.contains("fooBar"));
        assert_eq!(unsafe_names.len(), 2);
    }

    /// A user declaration colliding with a generated support type implicates
    /// only that declaration: the support type belongs to no declaration, so
    /// there is no second owner to blame.
    #[test]
    fn a_support_type_collision_marks_only_the_user_declaration() {
        let schema = unbounded_schema(vec![primitive("UnboundedVec")]);
        assert_eq!(
            unsafe_local_names(&schema, BackendLanguage::Rust),
            ["UnboundedVec".to_owned()].into_iter().collect()
        );
    }

    // ---- World-aware `_Kind` companion registration --------------------
    //
    // Being an abstract value target does not prove a wrapper is emitted. The
    // companion may only be reserved where the requested world really
    // produces a `TypeEmission::AbstractValue`.

    /// `Base` abstract with a concrete descendant, referenced by value: a real
    /// closed sum under `ClosedSchemaSet`.
    fn closed_sum_schema(extra: Vec<TypeDecl>) -> SchemaIr {
        let mut base = record("Base", Vec::new());
        base.is_abstract = true;
        let mut concrete = record("Concrete", Vec::new());
        concrete.base_type = Some(TypeRef {
            target: TypeRefTarget::Named(QualifiedName::new(NS, "Base")),
        });
        let holder = record(
            "Holder",
            vec![field(
                "Value",
                TypeRefTarget::Named(QualifiedName::new(NS, "Base")),
                Cardinality::REQUIRED_ONE,
            )],
        );
        let mut types = vec![base, concrete, holder];
        types.extend(extra);
        schema_with(types)
    }

    #[test]
    fn a_real_closed_sum_reserves_its_kind_companion() {
        let schema = closed_sum_schema(vec![primitive("Base_Kind")]);
        assert_collides(&schema, BackendLanguage::Ada, "Base_Kind");
        let unsafe_names = unsafe_ada(&schema);
        assert!(
            unsafe_names.contains("Base") && unsafe_names.contains("Base_Kind"),
            "both the wrapper owner and the colliding declaration are unsafe: {unsafe_names:?}"
        );
    }

    /// Task 026 elision control. `EmptyBase` is an abstract value target with
    /// zero concrete descendants, used only as absent-only optional storage,
    /// so no wrapper and no `EmptyBase_Kind` are emitted. Reserving the
    /// companion anyway falsely rejected an otherwise generable schema.
    #[test]
    fn a_zero_descendant_elided_target_reserves_no_kind_companion() {
        let mut empty_base = record("EmptyBase", Vec::new());
        empty_base.is_abstract = true;
        let holder = record(
            "Holder",
            vec![field(
                "Optional",
                TypeRefTarget::Named(QualifiedName::new(NS, "EmptyBase")),
                Cardinality::OPTIONAL_ONE,
            )],
        );
        let schema = schema_with(vec![empty_base, holder, primitive("EmptyBase_Kind")]);
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_ok(),
            "no wrapper is emitted, so `EmptyBase_Kind` must stay available"
        );
        assert!(unsafe_ada(&schema).is_empty());
    }

    // ---- Storage-classified Record member registration -----------------
    //
    // Ada rendering removes Task 026 `AbsentOnly` fields *before* emitting any
    // component, repeated helper or Task 034 `_Optional` wrapper. Name
    // preflight used to walk the raw effective fields instead, so it could
    // reserve identifiers for a field the backend emits nowhere. That both
    // manufactured false collisions and could mask the authoritative semantic
    // diagnostic for a field whose storage classification *fails*.

    /// `abstract EmptyBase` with zero concrete descendants, held only as an
    /// absent-only optional field, so Task 026 elides the field entirely.
    /// `Holder_Maybe_Optional` is therefore a spelling nothing generates, and
    /// the user declaration of that name is legal.
    fn elided_optional_schema(field_name: &str, extra: Vec<TypeDecl>) -> SchemaIr {
        let mut empty_base = record("EmptyBase", Vec::new());
        empty_base.is_abstract = true;
        let holder = record(
            "Holder",
            vec![field(
                field_name,
                TypeRefTarget::Named(QualifiedName::new(NS, "EmptyBase")),
                Cardinality::OPTIONAL_ONE,
            )],
        );
        let mut types = vec![empty_base, holder];
        types.extend(extra);
        schema_with(types)
    }

    /// `abstract Base` with a concrete descendant, held as an optional named
    /// field. Closed world stores it, so the Task 034 wrapper is emitted;
    /// open world cannot represent `Base` at all.
    fn closed_sum_optional_schema(extra: Vec<TypeDecl>) -> SchemaIr {
        let mut base = record("Base", Vec::new());
        base.is_abstract = true;
        let mut concrete = record("Concrete", Vec::new());
        concrete.base_type = Some(TypeRef {
            target: TypeRefTarget::Named(QualifiedName::new(NS, "Base")),
        });
        let holder = record(
            "Holder",
            vec![field(
                "Maybe",
                TypeRefTarget::Named(QualifiedName::new(NS, "Base")),
                Cardinality::OPTIONAL_ONE,
            )],
        );
        let mut types = vec![base, concrete, holder];
        types.extend(extra);
        schema_with(types)
    }

    /// Regression 1: a Task 026 elided optional field reserves no Task 034
    /// `_Optional` wrapper name.
    #[test]
    fn an_elided_optional_field_reserves_no_ada_optional_wrapper_name() {
        let schema = elided_optional_schema("Maybe", vec![primitive("Holder_Maybe_Optional")]);
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_ok(),
            "no wrapper is emitted for an absent-only field, so the spelling stays available"
        );
        let condemned = unsafe_ada(&schema);
        assert!(
            !condemned.contains("Holder") && !condemned.contains("Holder_Maybe_Optional"),
            "nothing may be condemned for a wrapper that is never generated: {condemned:?}"
        );
    }

    /// Regression 3: the correction covers the **member** surface, not merely
    /// the `_Optional` suffix. An elided field emits no Record component at
    /// all, so its identifier is never checked -- even when it is a reserved
    /// word that would otherwise make GNAT reject the component.
    #[test]
    fn an_elided_reserved_word_member_is_not_checked() {
        let schema = elided_optional_schema("Range", Vec::new());
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_ok(),
            "no component named `Range` is emitted, so no reserved word is used"
        );
        assert!(unsafe_ada(&schema).is_empty());
    }

    /// Regression 4 (control): an inhabited optional named field is genuinely
    /// stored, so it keeps reserving `{Owner}_{Member}_Optional` and a user
    /// declaration of that spelling is still a real collision implicating both
    /// declarations.
    #[test]
    fn a_stored_optional_named_field_still_reserves_its_optional_wrapper() {
        let schema = schema_with(vec![
            record("Item", Vec::new()),
            record(
                "Holder",
                vec![field(
                    "Maybe",
                    TypeRefTarget::Named(QualifiedName::new(NS, "Item")),
                    Cardinality::OPTIONAL_ONE,
                )],
            ),
            primitive("Holder_Maybe_Optional"),
        ]);
        assert_collides(&schema, BackendLanguage::Ada, "Holder_Maybe_Optional");
        let condemned = unsafe_ada(&schema);
        assert!(
            condemned.contains("Holder") && condemned.contains("Holder_Maybe_Optional"),
            "both the emitting owner and the colliding declaration are unsafe: {condemned:?}"
        );
    }

    /// Regression 5: under open-extensions the field's storage classification
    /// *fails* -- an abstract value with possible external descendants cannot
    /// be represented -- so no `_Optional` wrapper can exist either. Name
    /// analysis must defer rather than manufacture a collision that would mask
    /// the authoritative semantic diagnostic.
    #[test]
    fn an_open_world_unrepresentable_optional_field_reserves_no_wrapper() {
        let schema = closed_sum_optional_schema(vec![primitive("Holder_Maybe_Optional")]);
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::OpenExtensions
            )
            .is_ok(),
            "the open-world semantic failure owns this field, not a phantom name collision"
        );
        assert!(
            unsafe_named_declarations(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::OpenExtensions
            )
            .is_empty(),
            "no declaration may be condemned for a wrapper the failing field never emits"
        );
        // Closed world is the control: there the field really is stored, so
        // the very same spelling collides.
        assert_collides(&schema, BackendLanguage::Ada, "Holder_Maybe_Optional");
    }

    /// Regression 6: field-level deferral is scoped. A failing/elided abstract
    /// field silences only itself; unrelated declarations that genuinely
    /// collide are still detected.
    #[test]
    fn deferral_for_one_field_does_not_disable_name_checking() {
        let schema = elided_optional_schema(
            "Maybe",
            vec![
                primitive("Holder_Maybe_Optional"),
                record(
                    "Other",
                    vec![field(
                        "Items",
                        TypeRefTarget::Primitive(PrimitiveKind::String),
                        UNBOUNDED,
                    )],
                ),
                primitive("Other_Items_Sequence"),
            ],
        );
        assert_collides(&schema, BackendLanguage::Ada, "Other_Items_Sequence");
        let condemned = unsafe_ada(&schema);
        assert!(
            condemned.contains("Other") && condemned.contains("Other_Items_Sequence"),
            "the unrelated genuine collision is still attributed: {condemned:?}"
        );
        assert!(
            !condemned.contains("Holder") && !condemned.contains("Holder_Maybe_Optional"),
            "the elided field still contributes nothing: {condemned:?}"
        );
    }

    /// Open world: the wrapper does not exist there, so no closed-world
    /// companion name may be invented. The abstract-value capability failure
    /// remains the authoritative blocker, reported by the semantic rules
    /// rather than as a manufactured name collision.
    #[test]
    fn the_open_world_does_not_reserve_closed_sum_companions() {
        let schema = closed_sum_schema(vec![primitive("Base_Kind")]);
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::OpenExtensions
            )
            .is_ok(),
            "an unemitted open-world wrapper must not reserve `Base_Kind`"
        );
        assert!(
            unsafe_named_declarations(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::OpenExtensions
            )
            .is_empty()
        );
    }

    /// Ordinary Choice lowering emits `{Choice}_Kind` in **either** world, so
    /// that registration is not gated on abstract-value world policy.
    #[test]
    fn choice_kind_companions_are_registered_in_every_world() {
        let schema = schema_with(vec![
            choice("Selection", &["First", "Second"]),
            primitive("Selection_Kind"),
        ]);
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            assert!(
                validate_backend_names(&schema, BackendLanguage::Ada, world).is_err(),
                "a Choice companion is emitted in {world}"
            );
        }
    }

    // ---- Rust/C++ identifier-start syntax ------------------------------
    //
    // `words` only guaranteed the characters were ASCII alphanumeric or `_`,
    // so `1Foo` survived upper-camel unchanged, passed preflight, and reached
    // the renderer as an identifier no compiler accepts. Ada already required
    // an alphabetic first character; this is the Rust/C++ equivalent. The
    // *generated* identifier is what is validated.

    fn assert_invalid_identifier(schema: &SchemaIr, language: BackendLanguage) {
        let error = validate_backend_names(schema, language, GenerationWorld::ClosedSchemaSet)
            .expect_err("an identifier starting with a digit is not legal syntax");
        assert!(
            matches!(error, BackendNameError::InvalidIdentifier { .. }),
            "expected InvalidIdentifier, got {error:?}"
        );
    }

    #[test]
    fn a_leading_digit_top_level_declaration_is_rejected() {
        let schema = schema_with(vec![primitive("1Foo")]);
        assert_invalid_identifier(&schema, BackendLanguage::Rust);
        assert_invalid_identifier(&schema, BackendLanguage::Cpp);
    }

    #[test]
    fn a_leading_digit_member_is_rejected() {
        let schema = schema_with(vec![record(
            "Holder",
            vec![field(
                "1Field",
                TypeRefTarget::Primitive(PrimitiveKind::String),
                Cardinality::REQUIRED_ONE,
            )],
        )]);
        for language in [BackendLanguage::Rust, BackendLanguage::Cpp] {
            assert_invalid_identifier(&schema, language);
            // Attributed to the owning declaration, as any member failure is.
            assert!(
                unsafe_named_declarations(&schema, language, GenerationWorld::ClosedSchemaSet)
                    .contains(&QualifiedName::new(NS, "Holder"))
            );
        }
    }

    #[test]
    fn a_leading_digit_variant_is_rejected() {
        let schema = schema_with(vec![choice("Selection", &["1First", "Second"])]);
        assert_invalid_identifier(&schema, BackendLanguage::Rust);
        assert_invalid_identifier(&schema, BackendLanguage::Cpp);
    }

    /// Each emitted C++ namespace component is a real identifier too: a URI
    /// whose emitted component begins with a digit cannot compile.
    #[test]
    fn a_leading_digit_cpp_namespace_component_is_rejected() {
        let error = validate_backend_names(
            &schema_in("urn:oms:2example"),
            BackendLanguage::Cpp,
            GenerationWorld::ClosedSchemaSet,
        )
        .expect_err("a namespace component cannot begin with a digit");
        assert!(matches!(
            error,
            BackendNameError::InvalidIdentifier {
                region: NameRegion::NamespaceUnit,
                ..
            }
        ));
    }

    /// A required-minimum unbounded member makes `backend-ada` emit a
    /// different helper set -- `_Required_Array` and `_Additional_Vectors`
    /// instead of `_Vectors` -- and those were emitted but never reserved, so
    /// a user declaration could collide with one undetected.
    #[test]
    fn ada_required_minimum_unbounded_helpers_are_reserved() {
        let required_unbounded = Cardinality {
            min_occurs: 2,
            max_occurs: None,
        };
        for suffix in [
            "_Required_Array",
            "_Additional_Vectors",
            "_Item",
            "_Sequence",
        ] {
            let schema = schema_with(vec![
                primitive("Item"),
                record(
                    "Owner",
                    vec![field(
                        "Items",
                        TypeRefTarget::Named(QualifiedName::new(NS, "Item")),
                        required_unbounded,
                    )],
                ),
                primitive(&format!("Owner_Items{suffix}")),
            ]);
            assert_collides(
                &schema,
                BackendLanguage::Ada,
                &format!("Owner_Items{suffix}"),
            );
            let unsafe_names = unsafe_ada(&schema);
            assert!(
                unsafe_names.contains("Owner"),
                "the helper-generating owner must be unsafe for {suffix}: {unsafe_names:?}"
            );
        }
        // The `min == 0` spelling stays available at a required minimum,
        // because that shape does not emit it.
        let schema = schema_with(vec![
            primitive("Item"),
            record(
                "Owner",
                vec![field(
                    "Items",
                    TypeRefTarget::Named(QualifiedName::new(NS, "Item")),
                    required_unbounded,
                )],
            ),
            primitive("Owner_Items_Vectors"),
        ]);
        assert!(
            validate_backend_names(
                &schema,
                BackendLanguage::Ada,
                GenerationWorld::ClosedSchemaSet
            )
            .is_ok()
        );
    }

    /// Control: digits elsewhere in an identifier are perfectly legal and must
    /// keep passing, in every region and every backend.
    #[test]
    fn digits_after_the_first_character_remain_accepted() {
        let schema = schema_with(vec![
            primitive("Foo1"),
            record(
                "X2",
                vec![field(
                    "field_1",
                    TypeRefTarget::Primitive(PrimitiveKind::String),
                    Cardinality::REQUIRED_ONE,
                )],
            ),
        ]);
        for language in BackendLanguage::ALL {
            assert!(
                validate_backend_names(&schema, language, GenerationWorld::ClosedSchemaSet).is_ok(),
                "{language:?} must accept Foo1 / X2 / field_1"
            );
        }
    }

    /// The transformation itself, independent of any schema: the generated
    /// spelling is what decides legality, not the source spelling.
    #[test]
    fn identifier_start_policy_is_applied_to_the_generated_spelling() {
        assert_eq!(upper_camel("foo_bar"), Some("FooBar".to_owned()));
        assert_eq!(snake_case("fooBar"), Some("foobar".to_owned()));
        assert_eq!(upper_camel("1foo"), None);
        assert_eq!(snake_case("1foo"), None);
        assert_eq!(upper_camel("9"), None);
        assert!(is_ascii_identifier("Foo1"));
        assert!(is_ascii_identifier("field_1"));
        assert!(is_ascii_identifier("_private"));
        assert!(!is_ascii_identifier("1Foo"));
        assert!(!is_ascii_identifier(""));
    }
}
