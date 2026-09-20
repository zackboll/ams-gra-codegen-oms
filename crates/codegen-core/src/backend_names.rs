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

use crate::coverage::BackendLanguage;
use crate::structure::{effective_choice_alternatives, effective_record_fields};
use ams_gra_oms_ir::{Cardinality, OccurrenceShape, QualifiedName, SchemaIr, TypeDecl, TypeKind};
use std::collections::BTreeMap;
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
}

impl fmt::Display for NameRegion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TopLevel => formatter.write_str("generated top-level scope"),
            Self::Members(owner) => write!(formatter, "members of {}", owner.local_name),
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

fn upper_camel(value: &str) -> Option<String> {
    let mut result = String::new();
    for word in words(value)? {
        let mut characters = word.chars();
        let first = characters.next()?;
        result.push(first.to_ascii_uppercase());
        result.extend(characters);
    }
    Some(result)
}

fn snake_case(value: &str) -> Option<String> {
    Some(words(value)?.join("_").to_ascii_lowercase())
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
    taken: BTreeMap<String, String>,
}

impl Region {
    fn new(language: BackendLanguage, region: NameRegion) -> Self {
        Self {
            language,
            region,
            taken: BTreeMap::new(),
        }
    }

    /// Record one generated name, rejecting reserved words and collisions.
    fn insert(&mut self, ir_name: &str, generated: String) -> Result<(), BackendNameError> {
        if is_reserved(self.language, &generated) {
            return Err(BackendNameError::ReservedWord {
                language: self.language,
                region: self.region.clone(),
                ir_name: ir_name.to_owned(),
                generated,
            });
        }
        let key = identity_key(self.language, &generated);
        match self.taken.get(&key) {
            // An identical IR name reaching the same region twice is a schema
            // defect diagnosed elsewhere, not a naming defect; only *distinct*
            // sources converging is reported here.
            Some(first) if first == ir_name => Ok(()),
            Some(first) => Err(BackendNameError::Collision {
                language: self.language,
                region: self.region.clone(),
                generated,
                first: first.clone(),
                second: ir_name.to_owned(),
            }),
            None => {
                self.taken.insert(key, ir_name.to_owned());
                Ok(())
            }
        }
    }

    /// Transform then record, mapping a failed transformation to a typed
    /// invalid-identifier error rather than skipping the name.
    fn insert_transformed(
        &mut self,
        ir_name: &str,
        generated: Option<String>,
    ) -> Result<(), BackendNameError> {
        let generated = generated.ok_or_else(|| BackendNameError::InvalidIdentifier {
            language: self.language,
            region: self.region.clone(),
            ir_name: ir_name.to_owned(),
        })?;
        self.insert(ir_name, generated)
    }
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
) -> Result<(), BackendNameError> {
    let mut top_level = Region::new(language, NameRegion::TopLevel);
    for declaration in &schema.types {
        top_level.insert_transformed(
            &declaration.name.local_name,
            declaration_name(language, &declaration.name.local_name),
        )?;
    }
    for declaration in &schema.types {
        validate_declaration_members(schema, declaration, language, &mut top_level)?;
    }
    Ok(())
}

fn validate_declaration_members(
    schema: &SchemaIr,
    declaration: &TypeDecl,
    language: BackendLanguage,
    top_level: &mut Region,
) -> Result<(), BackendNameError> {
    let mut members = Region::new(language, NameRegion::Members(declaration.name.clone()));
    match &declaration.kind {
        TypeKind::Enumeration { variants } => {
            for variant in variants {
                members.insert_transformed(
                    &variant.wire_value,
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
                members.insert_transformed(&field.name, member_name(language, &field.name))?;
                if language == BackendLanguage::Ada && ada_emits_helper(field.cardinality) {
                    validate_ada_helpers(
                        top_level,
                        &declaration.name.local_name,
                        &field.name,
                        field.cardinality,
                    )?;
                }
            }
        }
        TypeKind::Choice { .. } => {
            let Ok(alternatives) = effective_choice_alternatives(schema, &declaration.name) else {
                return Ok(());
            };
            for alternative in alternatives {
                match language {
                    // Rust enum variants and C++ nested alternative structs
                    // are upper-camel.
                    BackendLanguage::Rust | BackendLanguage::Cpp => members.insert_transformed(
                        &alternative.name,
                        variant_name(language, &alternative.name),
                    )?,
                    // Ada renders alternatives as variant-part components.
                    BackendLanguage::Ada => {
                        members.insert_transformed(
                            &alternative.name,
                            member_name(language, &alternative.name),
                        )?;
                        if ada_emits_helper(alternative.cardinality) {
                            validate_ada_helpers(
                                top_level,
                                &declaration.name.local_name,
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
    owner: &str,
    member: &str,
    cardinality: Cardinality,
) -> Result<(), BackendNameError> {
    let Some(member_identifier) = ada_identifier(member) else {
        // The member identifier itself already failed in the member region.
        return Ok(());
    };
    let stem = format!("{owner}_{member_identifier}");
    // Exactly the suffixes `backend-ada` emits for each repeated shape.
    let suffixes: &[&str] = if matches!(cardinality.shape(), OccurrenceShape::Unbounded { .. }) {
        &["_Item", "_Vectors", "_Sequence"]
    } else {
        &["_Array", "_Sequence"]
    };
    for suffix in suffixes {
        // Attributed to the owning member so a collision names the schema
        // identifiers responsible, not an opaque generated string.
        top_level.insert(
            &format!("{owner}.{member} helper"),
            format!("{stem}{suffix}"),
        )?;
    }
    Ok(())
}

/// Whether every generated name one backend would emit for `schema` is safe.
///
/// This is the capability-analysis view of [`validate_backend_names`]; both
/// consult the same rules, so readiness cannot claim READY for output that
/// would not compile.
#[must_use]
pub fn backend_names_are_renderable(schema: &SchemaIr, language: BackendLanguage) -> bool {
    validate_backend_names(schema, language).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
