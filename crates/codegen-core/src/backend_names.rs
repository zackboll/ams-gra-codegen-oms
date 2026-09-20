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

use crate::abstract_value::{abstract_value_targets, is_structural, project_abstract_value};
use crate::coverage::BackendLanguage;
use crate::structure::{effective_choice_alternatives, effective_record_fields};
use ams_gra_oms_ir::{
    Cardinality, OccurrenceShape, PrimitiveKind, QualifiedName, SchemaIr, TypeDecl, TypeKind,
    TypeRefTarget,
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
    /// When true, a rejected name is recorded in `errors` and validation
    /// continues instead of returning early.
    ///
    /// Generation wants the first failure and nothing more. Coverage wants
    /// *every* failure, so it can attribute each one to the declarations
    /// responsible rather than condemning the whole schema for the first
    /// unsafe name found. Both behaviours run the same registration logic,
    /// which is what keeps the two from drifting.
    collecting: bool,
    errors: Vec<BackendNameError>,
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
    fn reject(&mut self, error: BackendNameError) -> Result<(), BackendNameError> {
        if self.collecting {
            self.errors.push(error);
            Ok(())
        } else {
            Err(error)
        }
    }

    /// Record one generated name, rejecting reserved words and collisions.
    fn insert(&mut self, ir_name: &str, generated: String) -> Result<(), BackendNameError> {
        if is_reserved(self.language, &generated) {
            return self.reject(BackendNameError::ReservedWord {
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
            Some(first) => {
                let error = BackendNameError::Collision {
                    language: self.language,
                    region: self.region.clone(),
                    generated,
                    first: first.clone(),
                    second: ir_name.to_owned(),
                };
                self.reject(error)
            }
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
        match generated {
            Some(generated) => self.insert(ir_name, generated),
            None => {
                let error = BackendNameError::InvalidIdentifier {
                    language: self.language,
                    region: self.region.clone(),
                    ir_name: ir_name.to_owned(),
                };
                self.reject(error)
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
    // identifier was responsible for the reservation.
    let attribution = format!("<generated {} support type>", language.name());
    let reserve = |top_level: &mut Region, generated: &str| -> Result<(), BackendNameError> {
        top_level.insert(&attribution, generated.to_owned())
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

/// The declarations for which Ada emits a `{Owner}_Kind` companion type.
///
/// Both sources are collected once, through the same abstract-value target
/// enumeration the emission planner uses, so preflight and generation agree on
/// which declarations produce a companion.
fn ada_kind_companion_owners(schema: &SchemaIr) -> Vec<&TypeDecl> {
    let abstract_value_names = abstract_value_targets(schema)
        .into_iter()
        .filter(|declaration| is_structural(declaration))
        .map(|declaration| &declaration.name)
        .collect::<std::collections::BTreeSet<_>>();
    schema
        .types
        .iter()
        .filter(|declaration| {
            matches!(declaration.kind, TypeKind::Choice { .. })
                || abstract_value_names.contains(&declaration.name)
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
    schema: &SchemaIr,
) -> Result<(), BackendNameError> {
    for declaration in ada_kind_companion_owners(schema) {
        // If the declaration's own identifier is unusable, that failure is
        // already reported for the declaration itself; do not re-report it
        // here as a companion problem.
        let Some(owner) = ada_identifier(&declaration.name.local_name) else {
            continue;
        };
        top_level.insert(
            &format!("{} companion", declaration.name.local_name),
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
) -> Result<(), BackendNameError> {
    let mut conflicts = Vec::new();
    collect_ada_literal_conflicts(top_level, schema, &mut conflicts);
    match conflicts.into_iter().next() {
        Some(error) => Err(error),
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
    conflicts: &mut Vec<BackendNameError>,
) {
    let check = |ir_name: &str, literal: &str, conflicts: &mut Vec<BackendNameError>| {
        let key = identity_key(BackendLanguage::Ada, literal);
        if let Some(first) = top_level.taken.get(&key) {
            conflicts.push(BackendNameError::Collision {
                language: BackendLanguage::Ada,
                region: NameRegion::TopLevel,
                generated: literal.to_owned(),
                first: first.clone(),
                second: ir_name.to_owned(),
            });
        }
    };
    for declaration in &schema.types {
        match &declaration.kind {
            // A plain enumeration's literals are the variant identifiers.
            TypeKind::Enumeration { variants } => {
                for variant in variants {
                    if let Some(literal) = ada_identifier(&variant.wire_value) {
                        check(&variant.wire_value, &literal, conflicts);
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
                        check(&alternative.name, &format!("{name}_Kind"), conflicts);
                    }
                }
            }
            _ => {}
        }
    }
    // A closed-sum wrapper's literals are `{Descendant}_Kind`, taken from the
    // same projection the Ada renderer walks.
    for declaration in ada_kind_companion_owners(schema) {
        let Ok(projection) = project_abstract_value(schema, &declaration.name) else {
            continue;
        };
        for descendant in &projection.concrete_descendants {
            if let Some(name) = ada_identifier(&descendant.name.local_name) {
                check(
                    &descendant.name.local_name,
                    &format!("{name}_Kind"),
                    conflicts,
                );
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
                region.insert_transformed(part, snake_case(part))?;
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
                region.insert_transformed(part, ada_title(part))?;
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
    // The enclosing unit's own identifier comes from the namespace URI rather
    // than from any declaration, so it is checked before the scope it encloses.
    validate_namespace_unit(schema, language)?;
    let mut top_level = Region::new(language, NameRegion::TopLevel);
    // Generated support types occupy the top-level scope before any user
    // declaration is placed in it, so a user declaration colliding with one is
    // attributed to the user declaration as the second, conflicting source.
    register_support_names(&mut top_level, schema, language)?;
    for declaration in &schema.types {
        top_level.insert_transformed(
            &declaration.name.local_name,
            declaration_name(language, &declaration.name.local_name),
        )?;
    }
    if language == BackendLanguage::Ada {
        register_ada_kind_companions(&mut top_level, schema)?;
    }
    for declaration in &schema.types {
        validate_declaration_members(schema, declaration, language, &mut top_level)?;
    }
    if language == BackendLanguage::Ada {
        // Literals are validated last, against the completed set of top-level
        // type names: a literal conflicts with a type name regardless of which
        // was declared first.
        validate_ada_enumeration_literals(&top_level, schema)?;
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
    register_declaration_members(schema, declaration, language, &mut members, top_level)
}

/// Register one declaration's member names into `members`, and any Ada
/// flat-package helper types it derives into `top_level`.
///
/// Shared verbatim by first-failure validation and by per-declaration
/// attribution; the two differ only in how their regions report a rejection.
fn register_declaration_members(
    schema: &SchemaIr,
    declaration: &TypeDecl,
    language: BackendLanguage,
    members: &mut Region,
    top_level: &mut Region,
) -> Result<(), BackendNameError> {
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
) -> BTreeSet<QualifiedName> {
    // IR local name -> declaration identity, for attributing a reported
    // failure back to its declaration in one lookup rather than a scan.
    let by_local_name = schema
        .types
        .iter()
        .map(|declaration| (declaration.name.local_name.as_str(), &declaration.name))
        .collect::<BTreeMap<_, _>>();
    let mut unsafe_names = BTreeSet::new();
    let attribute = |errors: &[BackendNameError], unsafe_names: &mut BTreeSet<QualifiedName>| {
        for error in errors {
            match error {
                BackendNameError::InvalidIdentifier {
                    region, ir_name, ..
                }
                | BackendNameError::ReservedWord {
                    region, ir_name, ..
                } => {
                    attribute_region(region, [ir_name.as_str()], &by_local_name, unsafe_names);
                }
                BackendNameError::Collision {
                    region,
                    first,
                    second,
                    ..
                } => {
                    attribute_region(
                        region,
                        [first.as_str(), second.as_str()],
                        &by_local_name,
                        unsafe_names,
                    );
                }
            }
        }
    };

    let mut top_level = Region::collecting(language, NameRegion::TopLevel);
    // Ignoring the `Result` is correct for a collecting region: it only ever
    // returns `Ok`, accumulating into `errors` instead.
    let _ = register_support_names(&mut top_level, schema, language);
    for declaration in &schema.types {
        let _ = top_level.insert_transformed(
            &declaration.name.local_name,
            declaration_name(language, &declaration.name.local_name),
        );
    }
    if language == BackendLanguage::Ada {
        let _ = register_ada_kind_companions(&mut top_level, schema);
    }
    for declaration in &schema.types {
        let mut members =
            Region::collecting(language, NameRegion::Members(declaration.name.clone()));
        let _ = register_declaration_members(
            schema,
            declaration,
            language,
            &mut members,
            &mut top_level,
        );
        // A member failure implicates exactly its owning declaration,
        // whatever the member was called.
        if !members.errors.is_empty() {
            unsafe_names.insert(declaration.name.clone());
        }
    }
    if language == BackendLanguage::Ada {
        let mut literals = Vec::new();
        collect_ada_literal_conflicts(&top_level, schema, &mut literals);
        attribute(&literals, &mut unsafe_names);
    }
    let top_level_errors = std::mem::take(&mut top_level.errors);
    attribute(&top_level_errors, &mut unsafe_names);
    unsafe_names
}

/// Map one reported top-level failure back to the declarations responsible.
///
/// Member-region failures are attributed by the caller, which already knows
/// the owning declaration.
fn attribute_region<'names>(
    region: &NameRegion,
    sources: impl IntoIterator<Item = &'names str>,
    by_local_name: &BTreeMap<&str, &QualifiedName>,
    unsafe_names: &mut BTreeSet<QualifiedName>,
) {
    match region {
        NameRegion::TopLevel => {
            for source in sources {
                // A generated support type or an Ada companion is attributed
                // through the schema identifier it was derived from; a
                // synthetic source that matches no declaration contributes
                // nothing, because no declaration is at fault for it alone.
                if let Some(name) = by_local_name.get(source) {
                    unsafe_names.insert((*name).clone());
                } else if let Some((owner, _)) = source.split_once(' ')
                    && let Some(name) = by_local_name.get(owner)
                {
                    // Ada helper/companion attributions of the form
                    // "Owner.Member helper" / "Owner companion".
                    unsafe_names.insert((*name).clone());
                }
            }
        }
        NameRegion::Members(owner) => {
            unsafe_names.insert(owner.clone());
        }
        // Not declaration-attributable; handled as a global precondition.
        NameRegion::NamespaceUnit => {}
    }
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
    use ams_gra_oms_ir::{
        ConstraintSet, EnumVariant, FieldDecl, NamespaceDecl, NumericValue, PrimitiveKind,
        QualifiedName, SourceRef, TypeRef,
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
        let error = validate_backend_names(schema, language)
            .expect_err("a generated name must not be silently shadowed");
        let BackendNameError::Collision { generated, .. } = &error else {
            panic!("expected a collision on {expected}, got {error:?}");
        };
        assert_eq!(generated, expected);
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
                validate_backend_names(&schema, language).is_ok(),
                "{language:?} must not reserve {spelling} for a schema that never emits it"
            );
        }
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
            assert!(validate_backend_names(&schema, language).is_ok());
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
            validate_backend_names(&schema, BackendLanguage::Ada).is_ok(),
            "Ada enumeration literals overload; this must not be a collision"
        );
    }

    // ---- Namespace / package identifiers -------------------------------

    /// A URI component that becomes the Ada package identifier `Record` makes
    /// the whole unit illegal, and Ada reserved words are case-insensitive.
    #[test]
    fn ada_package_component_may_not_be_a_reserved_word() {
        let error = validate_backend_names(&schema_in("urn:backend-record"), BackendLanguage::Ada)
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
        let error = validate_backend_names(&schema_in("urn:oms:class"), BackendLanguage::Cpp)
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
                validate_backend_names(&schema, language).is_ok(),
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
}
