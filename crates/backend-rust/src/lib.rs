//! Minimal Rust type generation from normalized schema IR.

use ams_gra_oms_codegen_core::{
    AbstractValueProjection, Backend, BackendLanguage, CodegenError, EffectiveValueMember,
    FloatingDomain, GeneratedFile, GenerationWorld, InclusiveIntegralDomain, TemporalProfile,
    TypeEmission, abstract_value_projection_for_ref, backend_preflight,
    effective_choice_alternatives, effective_record_fields, field_storage_semantics,
    float32_literal, float64_literal, floating_domain, inclusive_integral_domain,
    is_temporal_primitive, plan_type_emissions, schema_emits_bounded_integer_support,
    schema_emits_unbounded_sequence_support, temporal_profile,
};
use ams_gra_oms_ir::{
    ConstraintSet, OccurrenceShape, PrimitiveKind, SchemaIr, TypeDecl, TypeKind, TypeRef,
    TypeRefTarget,
};
use std::fmt::Write as _;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Copy)]
pub struct RustBackend;

impl Backend for RustBackend {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn generate(
        &self,
        schema: &SchemaIr,
        world: GenerationWorld,
    ) -> Result<Vec<GeneratedFile>, CodegenError> {
        let contents = generate(schema, world)?;
        let namespace = schema
            .namespaces
            .first()
            .ok_or_else(|| error("Rust generation requires one namespace"))?;
        let stem = namespace
            .uri
            .split(|character: char| !character.is_ascii_alphanumeric())
            .rfind(|part| !part.is_empty())
            .ok_or_else(|| error("Rust generation requires a named namespace"))?;
        Ok(vec![GeneratedFile {
            relative_path: PathBuf::from(format!("{}.rs", snake_case(stem)?)),
            contents,
        }])
    }
}

/// Generate Rust declarations from normalized schema IR.
///
/// The `world` policy is required: there is no implicit world assumption at
/// any public generation entry point. Under
/// [`GenerationWorld::OpenExtensions`] any abstract structural value
/// reference fails closed, because external derived types have no generated
/// representation.
///
/// # Errors
///
/// Returns an error when the IR contains a construct this backend cannot
/// represent without losing semantics, including an abstract structural value
/// under the open-extensions world.
pub fn generate(schema: &SchemaIr, world: GenerationWorld) -> Result<String, CodegenError> {
    validate_schema(schema, world)?;
    let emissions = plan_type_emissions(schema, world)?;
    let mut output = String::from(concat!(
        "#[derive(Debug, Clone, PartialEq, Eq)]\n",
        "pub struct BoundedVec<T, const MIN: usize, const MAX: usize>(Vec<T>);\n\n",
        "impl<T, const MIN: usize, const MAX: usize> BoundedVec<T, MIN, MAX> {\n",
        "    pub fn new(values: Vec<T>) -> Option<Self> {\n",
        "        (MIN <= values.len() && values.len() <= MAX).then_some(Self(values))\n",
        "    }\n\n",
        "    pub fn as_slice(&self) -> &[T] {\n",
        "        &self.0\n",
        "    }\n",
        "}\n\n",
    ));
    if schema_emits_unbounded_sequence_support(schema) {
        output.push_str("use std::convert::TryFrom;\n\n");
        output.push_str(concat!(
            "#[derive(Debug, Clone, PartialEq, Eq)]\n",
            "pub struct UnboundedVec<T, const MIN: u64>(Vec<T>);\n\n",
            "impl<T, const MIN: u64> UnboundedVec<T, MIN> {\n",
            "    pub fn new(values: Vec<T>) -> Option<Self> {\n",
            "        usize::try_from(MIN).is_ok_and(|min| values.len() >= min).then_some(Self(values))\n",
            "    }\n\n",
            "    pub fn as_slice(&self) -> &[T] { &self.0 }\n",
            "}\n\n",
        ));
    }
    if schema_emits_bounded_integer_support(schema) {
        output.push_str(concat!(
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]\n",
            "pub struct BoundedI64<const MIN: i64, const MAX: i64>(i64);\n",
            "impl<const MIN: i64, const MAX: i64> BoundedI64<MIN, MAX> {\n",
            "    pub const fn new(value: i64) -> Option<Self> { if value >= MIN && value <= MAX { Some(Self(value)) } else { None } }\n",
            "    pub const fn get(self) -> i64 { self.0 }\n}\n\n",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]\n",
            "pub struct BoundedU64<const MIN: u64, const MAX: u64>(u64);\n",
            "impl<const MIN: u64, const MAX: u64> BoundedU64<MIN, MAX> {\n",
            "    pub const fn new(value: u64) -> Option<Self> { if value >= MIN && value <= MAX { Some(Self(value)) } else { None } }\n",
            "    pub const fn get(self) -> u64 { self.0 }\n}\n\n",
        ));
    }
    for emission in emissions {
        match emission {
            TypeEmission::Declaration(declaration) => {
                render_declaration(&mut output, schema, declaration, world)?
            }
            TypeEmission::AbstractValue(projection) => {
                render_abstract_value(&mut output, schema, &projection)?
            }
        }
    }
    Ok(output)
}

fn render_abstract_value(
    output: &mut String,
    schema: &SchemaIr,
    projection: &AbstractValueProjection<'_>,
) -> Result<(), CodegenError> {
    let name = upper_camel(&projection.declaration.name.local_name)?;
    // Task 036: `PartialEq` is omitted when any descendant transitively holds
    // a temporal carrier, which derives no equality of its own.
    let supports_partial_eq = projection
        .concrete_descendants
        .iter()
        .all(|descendant| declaration_supports_partial_eq(schema, descendant, &mut Vec::new()));
    let supports_eq = supports_partial_eq
        && projection
            .concrete_descendants
            .iter()
            .all(|descendant| declaration_supports_eq(schema, descendant, &mut Vec::new()));
    writeln!(
        output,
        "#[derive(Debug, Clone{})]\npub enum {name} {{",
        if supports_eq {
            ", PartialEq, Eq"
        } else if supports_partial_eq {
            ", PartialEq"
        } else {
            ""
        }
    )
    .expect("writing to String cannot fail");
    let mut variants = std::collections::BTreeSet::new();
    for descendant in &projection.concrete_descendants {
        let variant = upper_camel(&descendant.name.local_name)?;
        if !variants.insert(variant.clone()) {
            return unsupported(format!(
                "duplicate abstract value variant identifier {variant}"
            ));
        }
        writeln!(output, "    {variant}({variant}),").expect("writing to String cannot fail");
    }
    output.push_str("}\n\n");
    Ok(())
}

fn render_declaration(
    output: &mut String,
    schema: &SchemaIr,
    declaration: &TypeDecl,
    world: GenerationWorld,
) -> Result<(), CodegenError> {
    let name = upper_camel(&declaration.name.local_name)?;
    match &declaration.kind {
        TypeKind::Primitive(PrimitiveKind::SignedInteger) => {
            let Some(InclusiveIntegralDomain::Signed { min, max }) = integral_domain(
                PrimitiveKind::SignedInteger,
                &declaration.constraints,
                &name,
            )?
            else {
                return unsupported(format!("integer bounds on {name}"));
            };
            writeln!(
                output,
                concat!(
                    "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]\n",
                    "pub struct {name}(i64);\n\n",
                    "impl {name} {{\n",
                    "    pub const MIN: i64 = {min};\n",
                    "    pub const MAX: i64 = {max};\n\n",
                    "    pub const fn new(value: i64) -> Option<Self> {{\n",
                    "        if value >= Self::MIN && value <= Self::MAX {{\n",
                    "            Some(Self(value))\n",
                    "        }} else {{\n",
                    "            None\n",
                    "        }}\n",
                    "    }}\n\n",
                    "    pub const fn get(self) -> i64 {{\n",
                    "        self.0\n",
                    "    }}\n",
                    "}}\n",
                ),
                name = name,
                min = min,
                max = max,
            )
            .expect("writing to String cannot fail");
        }
        TypeKind::Primitive(PrimitiveKind::UnsignedInteger) => {
            let Some(InclusiveIntegralDomain::Unsigned { min, max }) = integral_domain(
                PrimitiveKind::UnsignedInteger,
                &declaration.constraints,
                &name,
            )?
            else {
                return unsupported(format!("integer bounds on {name}"));
            };
            writeln!(output, "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub struct {name}(u64);\n\nimpl {name} {{\n    pub const MIN: u64 = {min};\n    pub const MAX: u64 = {max};\n\n    pub const fn new(value: u64) -> Option<Self> {{\n        if value >= Self::MIN && value <= Self::MAX {{ Some(Self(value)) }} else {{ None }}\n    }}\n\n    pub const fn get(self) -> u64 {{ self.0 }}\n}}\n").expect("writing to String cannot fail");
        }
        TypeKind::Primitive(PrimitiveKind::Boolean) => {
            reject_any_constraints(&declaration.constraints, &name)?;
            writeln!(output, "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub struct {name}(bool);\n\nimpl {name} {{\n    pub const fn new(value: bool) -> Self {{ Self(value) }}\n    pub const fn get(self) -> bool {{ self.0 }}\n}}\n").expect("writing to String cannot fail");
        }
        TypeKind::Primitive(kind @ (PrimitiveKind::Float32 | PrimitiveKind::Float64)) => {
            render_floating_declaration(output, *kind, &declaration.constraints, &name)?;
        }
        TypeKind::Primitive(
            kind @ (PrimitiveKind::DateTime | PrimitiveKind::Time | PrimitiveKind::Duration),
        ) => {
            render_temporal_declaration(output, *kind, &declaration.constraints, &name)?;
        }
        TypeKind::Primitive(PrimitiveKind::Binary) => {
            reject_any_constraints(&declaration.constraints, &name)?;
            writeln!(output, "#[derive(Debug, Clone, PartialEq, Eq)]\npub struct {name}(Vec<u8>);\n\nimpl {name} {{\n    pub fn new(value: Vec<u8>) -> Self {{ Self(value) }}\n    pub fn as_slice(&self) -> &[u8] {{ &self.0 }}\n    pub fn into_vec(self) -> Vec<u8> {{ self.0 }}\n}}\n").expect("writing to String cannot fail");
        }
        TypeKind::Enumeration { variants } => {
            if variants.is_empty() {
                return unsupported(format!("empty enumeration {name}"));
            }
            writeln!(
                output,
                "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum {name} {{"
            )
            .expect("writing to String cannot fail");
            for variant in variants {
                writeln!(output, "    {},", upper_camel(&variant.wire_value)?)
                    .expect("writing to String cannot fail");
            }
            output.push_str("}\n\n");
        }
        TypeKind::Record { .. } => {
            if declaration.is_abstract {
                return Ok(());
            }
            writeln!(
                output,
                "#[derive(Debug, Clone{})]\npub struct {name} {{",
                structural_derives(schema, declaration)
            )
            .expect("writing to String cannot fail");
            for field in effective_record_fields(schema, &declaration.name).map_err(|_| {
                error(format!(
                    "unsupported Rust IR construct: non-Record inheritance {}",
                    declaration.name.local_name
                ))
            })? {
                if matches!(
                    field_storage_semantics(schema, field, world).map_err(|projection_error| {
                        error(format!(
                            "unsupported abstract structural value: {projection_error}"
                        ))
                    })?,
                    EffectiveValueMember::AbsentOnly(_)
                ) {
                    // Task 026: uninhabited abstract structural target with a
                    // 0..1 occurrence; absence is the only legal state, so no
                    // field is generated for it in this schema set.
                    continue;
                }
                let field_name = snake_case(&field.name)?;
                let base = rust_field_base(field)?;
                let field_type = match field.cardinality.shape() {
                    OccurrenceShape::RequiredOne => base,
                    OccurrenceShape::OptionalOne => format!("Option<{base}>"),
                    OccurrenceShape::Bounded { min, max } if max > 1 => {
                        format!("BoundedVec<{base}, {min}, {max}>")
                    }
                    OccurrenceShape::Unbounded { min } => format!("UnboundedVec<{base}, {min}>"),
                    _ => return unsupported(format!("cardinality on field {field_name}")),
                };
                writeln!(output, "    pub {field_name}: {field_type},")
                    .expect("writing to String cannot fail");
            }
            output.push_str("}\n");
        }
        TypeKind::Choice { .. } => {
            writeln!(
                output,
                "#[derive(Debug, Clone{})]\npub enum {name} {{",
                structural_derives(schema, declaration)
            )
            .expect("writing to String cannot fail");
            for alternative in effective_choice_alternatives(schema, &declaration.name).map_err(
                |projection_error| {
                    error(format!("unsupported Rust IR construct: {projection_error}"))
                },
            )? {
                let alternative_name = upper_camel(&alternative.name)?;
                writeln!(
                    output,
                    "    {alternative_name}({}),",
                    rust_field_type(alternative)?
                )
                .expect("writing to String cannot fail");
            }
            output.push_str("}\n\n");
        }
        other => return unsupported(format!("type {name}: {other:?}")),
    }
    Ok(())
}

fn validate_schema(schema: &SchemaIr, world: GenerationWorld) -> Result<(), CodegenError> {
    // Shared global preflight: the single-namespace boundary and generated
    // host-language name safety. Capability/readiness analysis consults the
    // same rules, so a READY verdict cannot disagree with what happens here.
    if let Err(preflight) = backend_preflight(schema, BackendLanguage::Rust, world) {
        return unsupported(preflight.to_string());
    }
    for declaration in &schema.types {
        if declaration.is_abstract
            && !matches!(
                declaration.kind,
                TypeKind::Record { .. } | TypeKind::Choice { .. }
            )
        {
            return unsupported(format!("abstract type {}", declaration.name.local_name));
        }
        // Task 033: a named floating declaration's effective constraints are
        // classified by the shared helper rather than rejected wholesale. The
        // bound-only subset is lowered; every other facet shape still fails
        // closed, here, before any output is produced.
        if let TypeKind::Primitive(kind @ (PrimitiveKind::Float32 | PrimitiveKind::Float64)) =
            declaration.kind
        {
            floating_domain(kind, &declaration.constraints).map_err(|reason| {
                error(format!(
                    "unsupported Rust IR construct: {reason} on {}",
                    declaration.name.local_name
                ))
            })?;
            // The generic `reject_extra_constraints` below understands only the
            // integral inclusive subset, so a legitimately exclusive floating
            // bound must not reach it.
            continue;
        }
        // Task 036: a named temporal declaration is classified by the shared
        // helper. Only the DateTime Zulu profile is lowered; `Time`,
        // `Duration`, an unconstrained `DateTime`, and any other facet shape
        // still fail closed, here, before any output is produced.
        if let TypeKind::Primitive(kind) = declaration.kind
            && is_temporal_primitive(kind)
        {
            match temporal_profile(kind, &declaration.constraints) {
                Ok(Some(TemporalProfile::DateTimeZulu)) => {}
                Ok(None) => unreachable!("is_temporal_primitive gates this branch"),
                Err(reason) => {
                    return unsupported(format!("{reason} on {}", declaration.name.local_name));
                }
            }
            // The supported profile's `.+Z` pattern is a genuine lexical
            // constraint that `reject_extra_constraints` would reject, so it
            // must not reach it: the classifier has already proven this exact
            // facet set is fully enforced by the generated validator.
            continue;
        }
        if matches!(declaration.kind, TypeKind::Primitive(PrimitiveKind::Binary))
            && declaration.constraints != ConstraintSet::default()
        {
            return unsupported(format!("constraints on {}", declaration.name.local_name));
        }
        reject_extra_constraints(&declaration.constraints, &declaration.name.local_name)?;
        if matches!(declaration.kind, TypeKind::Record { .. }) {
            let fields = effective_record_fields(schema, &declaration.name).map_err(|_| {
                error(format!(
                    "unsupported Rust IR construct: non-Record inheritance {}",
                    declaration.name.local_name
                ))
            })?;
            for field in fields {
                if matches!(
                    field_storage_semantics(schema, field, world).map_err(|projection_error| {
                        error(format!(
                            "unsupported abstract structural value: {projection_error}"
                        ))
                    })?,
                    EffectiveValueMember::AbsentOnly(_)
                ) {
                    // Task 026: this field's abstract structural target has zero
                    // concrete descendants in the current schema set, so absence
                    // is its only legal state; no storage is generated for it.
                    continue;
                }
                validate_abstract_value_reference(schema, &field.type_ref, world)?;
                if field.nillable {
                    return unsupported(format!("nillable field {}", field.name));
                }
            }
        }
        if matches!(declaration.kind, TypeKind::Choice { .. }) {
            let alternatives = effective_choice_alternatives(schema, &declaration.name).map_err(
                |projection_error| {
                    error(format!("unsupported Rust IR construct: {projection_error}"))
                },
            )?;
            validate_choice_alternatives(schema, alternatives, world)?;
        }
    }
    for message in &schema.messages {
        validate_abstract_value_reference(schema, &message.payload_type, world)?;
    }
    Ok(())
}

fn validate_abstract_value_reference(
    schema: &SchemaIr,
    type_ref: &TypeRef,
    world: GenerationWorld,
) -> Result<(), CodegenError> {
    abstract_value_projection_for_ref(schema, type_ref, world)
        .map(|_| ())
        .map_err(|projection_error| {
            error(format!(
                "unsupported abstract structural value: {projection_error}"
            ))
        })
}

fn validate_choice_alternatives(
    schema: &SchemaIr,
    alternatives: Vec<&ams_gra_oms_ir::FieldDecl>,
    world: GenerationWorld,
) -> Result<(), CodegenError> {
    // Alternative-name collisions are no longer checked here: the shared
    // backend name preflight owns that policy for every generated region, so
    // keeping a second Rust-local copy would let the two drift.
    for alternative in alternatives {
        if alternative.nillable {
            return unsupported(format!("nillable Choice alternative {}", alternative.name));
        }
        validate_abstract_value_reference(schema, &alternative.type_ref, world)?;
        rust_field_type(alternative)?;
    }
    Ok(())
}

fn rust_field_type(field: &ams_gra_oms_ir::FieldDecl) -> Result<String, CodegenError> {
    let base = rust_field_base(field)?;
    match field.cardinality.shape() {
        OccurrenceShape::RequiredOne => Ok(base),
        OccurrenceShape::OptionalOne => Ok(format!("Option<{base}>")),
        OccurrenceShape::Bounded { min, max } if max > 1 => {
            Ok(format!("BoundedVec<{base}, {min}, {max}>"))
        }
        OccurrenceShape::Unbounded { min } => Ok(format!("UnboundedVec<{base}, {min}>")),
        _ => unsupported(format!("cardinality on Choice alternative {}", field.name)),
    }
}

fn rust_type(type_ref: &TypeRef) -> Result<String, CodegenError> {
    match &type_ref.target {
        TypeRefTarget::Primitive(PrimitiveKind::SignedInteger) => Ok("i64".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::UnsignedInteger) => Ok("u64".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::Boolean) => Ok("bool".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::Float32) => Ok("f32".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::Float64) => Ok("f64".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::String) => Ok("String".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::Binary) => Ok("Vec<u8>".to_owned()),
        TypeRefTarget::Named(name) => upper_camel(&name.local_name),
        other => unsupported(format!("type reference {other:?}")),
    }
}

fn declaration_supports_eq(
    schema: &SchemaIr,
    declaration: &TypeDecl,
    visiting: &mut Vec<ams_gra_oms_ir::QualifiedName>,
) -> bool {
    if visiting.contains(&declaration.name) {
        return false;
    }
    visiting.push(declaration.name.clone());
    let result = match &declaration.kind {
        TypeKind::Primitive(PrimitiveKind::Float32 | PrimitiveKind::Float64) => false,
        // Task 036: the DateTime Zulu carrier deliberately derives no
        // `PartialEq`, because two distinct legal spellings can denote the
        // same XML Schema value. A record containing one therefore cannot
        // derive `Eq` either -- and, more importantly, cannot derive
        // `PartialEq`; see `declaration_supports_partial_eq`.
        TypeKind::Primitive(kind) if is_temporal_primitive(*kind) => false,
        TypeKind::Primitive(_) | TypeKind::Enumeration { .. } => true,
        TypeKind::Record { .. } => {
            effective_record_fields(schema, &declaration.name).is_ok_and(|fields| {
                fields
                    .iter()
                    .all(|field| type_ref_supports_eq(schema, &field.type_ref, visiting))
            })
        }
        TypeKind::Choice { .. } => effective_choice_alternatives(schema, &declaration.name)
            .is_ok_and(|fields| {
                fields
                    .iter()
                    .all(|field| type_ref_supports_eq(schema, &field.type_ref, visiting))
            }),
        TypeKind::Alias(_) | TypeKind::List { .. } => false,
    };
    visiting.pop();
    result
}

fn type_ref_supports_eq(
    schema: &SchemaIr,
    type_ref: &TypeRef,
    visiting: &mut Vec<ams_gra_oms_ir::QualifiedName>,
) -> bool {
    match &type_ref.target {
        TypeRefTarget::Primitive(PrimitiveKind::Float32 | PrimitiveKind::Float64) => false,
        TypeRefTarget::Primitive(_) => true,
        TypeRefTarget::Named(name) => schema
            .types
            .iter()
            .find(|candidate| candidate.name == *name)
            .is_some_and(|candidate| declaration_supports_eq(schema, candidate, visiting)),
    }
}

/// Whether a declaration may derive `PartialEq` at all.
///
/// Distinct from [`declaration_supports_eq`], which asks the stronger question
/// of *total* equality. Almost every generated type supports `PartialEq`:
/// floats do, even though they are not `Eq`.
///
/// The Task 036 DateTime Zulu carrier is the exception. It deliberately
/// derives **no** equality, because XML Schema dateTime value equality is not
/// string equality -- `2026-09-20T24:00:00Z` and `2026-09-21T00:00:00Z` denote
/// the same instant with different spellings, and trailing fractional zeros
/// are likewise value-preserving. Deriving `PartialEq` on the wrapper would
/// publish "same stored bytes" as though it were value-space equality, which
/// Task 036 has not implemented.
///
/// That absence propagates: a record or choice holding such a value cannot
/// derive `PartialEq` either, so this predicate exists to keep generated code
/// compiling rather than emitting a derive the field cannot satisfy.
fn declaration_supports_partial_eq(
    schema: &SchemaIr,
    declaration: &TypeDecl,
    visiting: &mut Vec<ams_gra_oms_ir::QualifiedName>,
) -> bool {
    if visiting.contains(&declaration.name) {
        return true;
    }
    visiting.push(declaration.name.clone());
    let result = match &declaration.kind {
        TypeKind::Primitive(kind) if is_temporal_primitive(*kind) => false,
        TypeKind::Primitive(_) | TypeKind::Enumeration { .. } => true,
        TypeKind::Record { .. } => {
            effective_record_fields(schema, &declaration.name).is_ok_and(|fields| {
                fields
                    .iter()
                    .all(|field| type_ref_supports_partial_eq(schema, &field.type_ref, visiting))
            })
        }
        TypeKind::Choice { .. } => effective_choice_alternatives(schema, &declaration.name)
            .is_ok_and(|fields| {
                fields
                    .iter()
                    .all(|field| type_ref_supports_partial_eq(schema, &field.type_ref, visiting))
            }),
        TypeKind::Alias(_) | TypeKind::List { .. } => true,
    };
    visiting.pop();
    result
}

fn type_ref_supports_partial_eq(
    schema: &SchemaIr,
    type_ref: &TypeRef,
    visiting: &mut Vec<ams_gra_oms_ir::QualifiedName>,
) -> bool {
    match &type_ref.target {
        // A *direct* primitive temporal field is unsupported in Task 036 and
        // never reaches rendering, so this arm is about named targets only.
        TypeRefTarget::Primitive(_) => true,
        TypeRefTarget::Named(name) => schema
            .types
            .iter()
            .find(|candidate| candidate.name == *name)
            .is_none_or(|candidate| declaration_supports_partial_eq(schema, candidate, visiting)),
    }
}

/// The `derive` list for a generated record or choice.
///
/// `PartialEq` is omitted entirely when some reachable member is a Task 036
/// temporal carrier, and `Eq` additionally requires total equality.
fn structural_derives(schema: &SchemaIr, declaration: &TypeDecl) -> &'static str {
    if !declaration_supports_partial_eq(schema, declaration, &mut Vec::new()) {
        ""
    } else if declaration_supports_eq(schema, declaration, &mut Vec::new()) {
        ", PartialEq, Eq"
    } else {
        ", PartialEq"
    }
}

/// Render a named Float32/Float64 declaration.
///
/// An unconstrained declaration keeps its exact Task 022 form -- an infallible
/// `new` returning `Self` -- because nothing about it can fail and churning
/// that API would break callers for no semantic gain. A Task 033 bound-only
/// declaration instead gets a checked `new` returning `Option<Self>`, since
/// construction genuinely can fail.
///
/// Neither form derives `Eq`, `Ord`, or `Hash`: `f32`/`f64` have no total order
/// and no reflexive equality, so those traits would be unsound to claim.
/// `PartialOrd` is derived because the underlying comparison is meaningful.
fn render_floating_declaration(
    output: &mut String,
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
    name: &str,
) -> Result<(), CodegenError> {
    let scalar = if kind == PrimitiveKind::Float32 {
        "f32"
    } else {
        "f64"
    };
    let Some(domain) = floating_domain(kind, constraints)
        .map_err(|reason| error(format!("unsupported Rust IR construct: {reason} on {name}")))?
    else {
        // Task 022 output, unchanged byte for byte.
        writeln!(output, "#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]\npub struct {name}({scalar});\n\nimpl {name} {{\n    pub const fn new(value: {scalar}) -> Self {{ Self(value) }}\n    pub const fn get(self) -> {scalar} {{ self.0 }}\n}}\n").expect("writing to String cannot fail");
        return Ok(());
    };

    // Bound comparisons are emitted verbatim, so NaN fails every clause, a
    // one-sided infinity keeps IEEE ordering, and both zero signs compare
    // equal -- all without a special case in the generated code.
    let mut clauses = Vec::new();
    let mut constants = String::new();
    match domain {
        FloatingDomain::Float32 { lower, upper } => {
            if let Some(bound) = lower {
                writeln!(
                    constants,
                    "    pub const MIN: f32 = {};",
                    float32_literal(bound.value.value())
                )
                .expect("writing to String cannot fail");
                clauses.push(format!("value {} Self::MIN", bound.kind.lower_operator()));
            }
            if let Some(bound) = upper {
                writeln!(
                    constants,
                    "    pub const MAX: f32 = {};",
                    float32_literal(bound.value.value())
                )
                .expect("writing to String cannot fail");
                clauses.push(format!("value {} Self::MAX", bound.kind.upper_operator()));
            }
        }
        FloatingDomain::Float64 { lower, upper } => {
            if let Some(bound) = lower {
                writeln!(
                    constants,
                    "    pub const MIN: f64 = {};",
                    float64_literal(bound.value.value())
                )
                .expect("writing to String cannot fail");
                clauses.push(format!("value {} Self::MIN", bound.kind.lower_operator()));
            }
            if let Some(bound) = upper {
                writeln!(
                    constants,
                    "    pub const MAX: f64 = {};",
                    float64_literal(bound.value.value())
                )
                .expect("writing to String cannot fail");
                clauses.push(format!("value {} Self::MAX", bound.kind.upper_operator()));
            }
        }
    }

    writeln!(
        output,
        concat!(
            "#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]\n",
            "pub struct {name}({scalar});\n\n",
            "impl {name} {{\n",
            "{constants}\n",
            "    pub const fn new(value: {scalar}) -> Option<Self> {{\n",
            "        if {condition} {{\n",
            "            Some(Self(value))\n",
            "        }} else {{\n",
            "            None\n",
            "        }}\n",
            "    }}\n\n",
            "    pub const fn get(self) -> {scalar} {{\n",
            "        self.0\n",
            "    }}\n",
            "}}\n",
        ),
        name = name,
        scalar = scalar,
        constants = constants.trim_end(),
        condition = clauses.join(" && "),
    )
    .expect("writing to String cannot fail");
    Ok(())
}

fn integral_domain(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
    name: &str,
) -> Result<Option<InclusiveIntegralDomain>, CodegenError> {
    inclusive_integral_domain(kind, constraints)
        .map_err(|reason| error(format!("unsupported Rust IR construct: {reason} on {name}")))
}

fn rust_field_base(field: &ams_gra_oms_ir::FieldDecl) -> Result<String, CodegenError> {
    match field.type_ref.target {
        TypeRefTarget::Primitive(
            kind @ (PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger),
        ) => match integral_domain(kind, &field.constraints, &field.name)? {
            Some(InclusiveIntegralDomain::Signed { min, max }) => {
                Ok(format!("BoundedI64<{min}, {max}>"))
            }
            Some(InclusiveIntegralDomain::Unsigned { min, max }) => {
                Ok(format!("BoundedU64<{min}, {max}>"))
            }
            None => rust_type(&field.type_ref),
        },
        _ => {
            reject_any_constraints(&field.constraints, &field.name)?;
            rust_type(&field.type_ref)
        }
    }
}

fn reject_any_constraints(constraints: &ConstraintSet, name: &str) -> Result<(), CodegenError> {
    if constraints != &ConstraintSet::default() {
        return unsupported(format!("field constraints on {name}"));
    }
    Ok(())
}

fn reject_extra_constraints(constraints: &ConstraintSet, name: &str) -> Result<(), CodegenError> {
    if constraints.min_exclusive.is_some()
        || constraints.max_exclusive.is_some()
        || constraints.length.is_some()
        || constraints.min_length.is_some()
        || constraints.max_length.is_some()
        || constraints.lexical != Default::default()
    {
        return unsupported(format!("constraints on {name}"));
    }
    Ok(())
}

fn words(value: &str) -> Result<Vec<&str>, CodegenError> {
    let words = value
        .split('_')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    if words.is_empty()
        || words
            .iter()
            .any(|word| !word.bytes().all(|c| c.is_ascii_alphanumeric()))
    {
        unsupported(format!("identifier {value:?}"))
    } else {
        Ok(words)
    }
}

fn upper_camel(value: &str) -> Result<String, CodegenError> {
    let mut result = String::new();
    for word in words(value)? {
        let mut characters = word.chars();
        let first = characters
            .next()
            .ok_or_else(|| error("empty identifier component"))?;
        result.push(first.to_ascii_uppercase());
        result.extend(characters);
    }
    Ok(result)
}

fn snake_case(value: &str) -> Result<String, CodegenError> {
    Ok(words(value)?.join("_").to_ascii_lowercase())
}

fn unsupported<T>(construct: String) -> Result<T, CodegenError> {
    Err(error(format!("unsupported Rust IR construct: {construct}")))
}

fn error(message: impl Into<String>) -> CodegenError {
    CodegenError {
        message: message.into(),
    }
}

/// Render a named temporal declaration.
///
/// Only the shared classifier's one supported profile is lowered. Task 036
/// deliberately implements `DateTime` + the UCI Zulu pattern and nothing else,
/// so `Time`, `Duration`, an unconstrained `DateTime`, and any other facet
/// shape fail closed here rather than being approximated.
///
/// # Representation
///
/// The generated type is a **validated lexical carrier**: it stores the
/// whitespace-normalized XML Schema `dateTime` spelling that passed
/// validation. It is deliberately not epoch seconds, a fixed-width timestamp,
/// or a third-party date/time crate -- each of those silently narrows XML
/// Schema's lexical and value space (unbounded year digits, arbitrary
/// fractional precision, the distinct `24:00:00` spelling) and would discard
/// wire-level information a future codec needs.
///
/// # Why no `PartialEq`/`Eq`/`Ord`
///
/// Two distinct legal spellings can denote the same XML Schema value, and the
/// `dateTime` order relation is only *partial* (section 3.2.7.4). Deriving
/// Rust equality here would silently publish "same stored string" as though it
/// were value-space equality. Task 036 implements no value comparison, so it
/// claims none.
fn render_temporal_declaration(
    output: &mut String,
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
    name: &str,
) -> Result<(), CodegenError> {
    match temporal_profile(kind, constraints) {
        Ok(Some(TemporalProfile::DateTimeZulu)) => {}
        Ok(None) => return unsupported(format!("non-temporal primitive on {name}")),
        Err(reason) => return unsupported(format!("{reason} on {name}")),
    }
    writeln!(
        output,
        "{}",
        RUST_DATE_TIME_ZULU_TEMPLATE.replace("{name}", name)
    )
    .expect("writing to String cannot fail");
    Ok(())
}

/// The generated Rust DateTime Zulu carrier, with `{name}` substituted.
///
/// # Validation order
///
/// 1. XML Schema `collapse` whitespace normalization (section 4.3.6), which is
///    *fixed* for `dateTime` and cannot be overridden by a schema author;
/// 2. the full `dateTime` lexical grammar and calendar rules (section 3.2.7.1);
/// 3. the UCI `.+Z` Zulu restriction.
///
/// Step 2 is not optional. A bare `ends_with('Z')` would accept `garbageZ` and
/// `2026-99-99T99:99:99Z`, so the base grammar is checked first and the Zulu
/// profile is the last gate rather than the only one.
///
/// # No fixed-width year
///
/// The year is validated and its leap-year properties computed **from decimal
/// digits**, never parsed into `i32`/`i64`. XML Schema admits a
/// "four-or-more digit" year with no upper bound, so a valid date must not
/// become invalid merely because it exceeds a host numeric type. Divisibility
/// by 4, 100, and 400 is decided from the last two or three digits, which is
/// exact for any digit count.
///
/// # Scope of the helpers
///
/// Every parser helper is a **private associated function**, so it lives in
/// this type's own scope and cannot collide with any schema-generated
/// top-level identifier. Task 036 therefore adds no new module-scope name to
/// the generated Rust module, and nothing new is owed to name preflight.
const RUST_DATE_TIME_ZULU_TEMPLATE: &str = r##"#[derive(Clone, Debug)]
pub struct {name} {
    lexical: String,
}

impl {name} {
    /// Validate `value` as an XML Schema dateTime restricted to Zulu.
    ///
    /// `value` is first normalized under the fixed `collapse` whiteSpace
    /// policy, then checked against the full dateTime lexical grammar and
    /// calendar rules, then against the `.+Z` Zulu restriction. Returns `None`
    /// if any step rejects. The stored text is the normalized form.
    pub fn new(value: &str) -> Option<Self> {
        let lexical = Self::collapse(value);
        if !Self::is_zulu_date_time(&lexical) {
            return None;
        }
        Some(Self { lexical })
    }

    /// The stored, normalized, validated lexical representation.
    ///
    /// This is an XML Schema dateTime spelling whose timezone is `Z`. It is
    /// not a point in time: no arithmetic, ordering, or conversion is offered.
    pub fn as_str(&self) -> &str {
        &self.lexical
    }

    /// XML Schema `collapse`: replace tab/LF/CR with space, squeeze runs of
    /// spaces, then trim leading and trailing spaces.
    fn collapse(value: &str) -> String {
        value
            .split(|c: char| c == ' ' || c == '\t' || c == '\n' || c == '\r')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The whole gate: valid lexical dateTime AND Zulu timezone.
    fn is_zulu_date_time(text: &str) -> bool {
        let bytes = text.as_bytes();
        // The Zulu restriction. Checked against the normalized form, which is
        // exactly where XML Schema applies a pattern facet.
        if bytes.len() < 2 || bytes[bytes.len() - 1] != b'Z' {
            return false;
        }
        // The base grammar must hold for everything before the timezone; this
        // is what stops `garbageZ` from being accepted.
        Self::is_date_time_body(&bytes[..bytes.len() - 1])
    }

    /// `'-'? yyyy '-' mm '-' dd 'T' hh ':' mm ':' ss ('.' s+)?`
    fn is_date_time_body(body: &[u8]) -> bool {
        let Some((year_end, leap)) = Self::scan_year(body) else {
            return false;
        };
        let rest = &body[year_end..];
        // '-' mm '-' dd, all fixed width.
        if rest.len() < 6 || rest[0] != b'-' || rest[3] != b'-' {
            return false;
        }
        let Some(month) = Self::two_digits(&rest[1..3]) else {
            return false;
        };
        let Some(day) = Self::two_digits(&rest[4..6]) else {
            return false;
        };
        if month < 1 || month > 12 || day < 1 || day > Self::days_in_month(month, leap) {
            return false;
        }
        Self::is_time_of_day(&rest[6..])
    }

    /// `'T' hh ':' mm ':' ss ('.' s+)?`
    fn is_time_of_day(rest: &[u8]) -> bool {
        if rest.len() < 9 || rest[0] != b'T' || rest[3] != b':' || rest[6] != b':' {
            return false;
        }
        let (Some(hour), Some(minute), Some(second)) = (
            Self::two_digits(&rest[1..3]),
            Self::two_digits(&rest[4..6]),
            Self::two_digits(&rest[7..9]),
        ) else {
            return false;
        };
        // XML Schema 1.0 Part 2, Appendix D: the two digits of `ss` "can have
        // values from 0 to 60". 60 is the LEAP SECOND and is lexically legal;
        // 61 never is, because the field itself stops at 60. Minutes remain
        // 00..59 -- a leap second lengthens the second field, not the minute.
        //
        // Appendix D adds that a 60 is "not sensible" away from March 31, June
        // 30, September 30, or December 31 UTC, but prescribes that such a
        // value "should [be] considered as added or subtracted from the
        // following minute" -- a VALUE mapping, not a lexical rejection. So no
        // calendar-position test is applied here, and no IERS leap-second
        // table is needed: Appendix E states outright that a definition
        // tracking real leap seconds "would need to be constantly updated".
        if hour > 24 || minute > 59 || second > 60 {
            return false;
        }
        let fraction = &rest[9..];
        // `'.' s+`: the dot requires at least one digit after it.
        let fraction_is_zero = if fraction.is_empty() {
            true
        } else {
            if fraction[0] != b'.' || fraction.len() < 2 {
                return false;
            }
            let digits = &fraction[1..];
            if !digits.iter().all(|byte| byte.is_ascii_digit()) {
                return false;
            }
            digits.iter().all(|byte| *byte == b'0')
        };
        // Hour 24 is legal only as the exact instant 24:00:00(.0*).
        if hour == 24 && (minute != 0 || second != 0 || !fraction_is_zero) {
            return false;
        }
        true
    }

    /// Validate `'-'? yyyy`, returning where it ends and whether it is a leap
    /// year. The digit count is unbounded, so nothing is parsed into an
    /// integer; leap-year divisibility is decided from trailing digits only.
    fn scan_year(body: &[u8]) -> Option<(usize, bool)> {
        let negative = body.first() == Some(&b'-');
        let start = usize::from(negative);
        let mut end = start;
        while end < body.len() && body[end].is_ascii_digit() {
            end += 1;
        }
        let digits = &body[start..end];
        // Four-or-more digits.
        if digits.len() < 4 {
            return None;
        }
        // If more than four digits, leading zeros are prohibited.
        if digits.len() > 4 && digits[0] == b'0' {
            return None;
        }
        // '0000' is not a valid lexical representation in XML Schema 1.0,
        // with or without a sign.
        if digits.iter().all(|byte| *byte == b'0') {
            return None;
        }
        Some((end, Self::is_leap_year(digits)))
    }

    /// Leap year from decimal digits: divisible by 400, or by 4 but not 100.
    ///
    /// Divisibility by 4 depends only on the last two digits and by 100/400
    /// only on the last three, so this is exact for an unbounded digit count
    /// and never narrows the year to a machine integer.
    fn is_leap_year(digits: &[u8]) -> bool {
        // Divisibility by 4 is decided by the last two digits alone, because
        // 100 is itself a multiple of 4.
        let last_two = digits[digits.len() - 2..]
            .iter()
            .fold(0_u32, |acc, byte| acc * 10 + u32::from(byte - b'0'));
        let divisible_by_4 = last_two % 4 == 0;
        let divisible_by_100 = last_two == 0;
        let divisible_by_400 = divisible_by_100 && Self::hundreds_multiple_of_four(digits);
        divisible_by_400 || (divisible_by_4 && !divisible_by_100)
    }

    /// Whether the year's hundreds-and-above part is a multiple of 4, i.e.
    /// whether a year ending in "00" is also divisible by 400.
    fn hundreds_multiple_of_four(digits: &[u8]) -> bool {
        // Strip the trailing "00" and test the remainder for divisibility by
        // 4, computed digit by digit so an arbitrarily long year is exact.
        let head = &digits[..digits.len() - 2];
        let mut remainder = 0_u32;
        for byte in head {
            remainder = (remainder * 10 + u32::from(byte - b'0')) % 4;
        }
        remainder == 0
    }

    /// Exactly two ASCII digits, as a number. Rejects signs and short input.
    fn two_digits(pair: &[u8]) -> Option<u32> {
        if pair.len() != 2 || !pair.iter().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        Some(u32::from(pair[0] - b'0') * 10 + u32::from(pair[1] - b'0'))
    }

    /// Maximum day for a month, following `maximumDayInMonthFor` in XML Schema
    /// 1.0 Part 2 Appendix E.
    fn days_in_month(month: u32, leap: bool) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            _ => 28,
        }
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::NumericValue;

    /// Existing pre-Task-028 regressions all asserted closed-world behaviour,
    /// so they keep asserting exactly that under the now-explicit policy.
    const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;
    /// Task 028 open-world regressions.
    #[allow(dead_code)]
    const OPEN: GenerationWorld = GenerationWorld::OpenExtensions;
    use ams_gra_oms_xsd_frontend::{
        load_schema_document, load_schema_set, load_schema_set_with_overlays,
    };
    use std::path::Path;

    fn track_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../xsd-frontend/tests/fixtures/track.xsd"),
        )
        .expect("track fixture should parse")
    }

    fn codegen_order_schema() -> SchemaIr {
        load_schema_set(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/codegen-order/root.xsd"),
        )
        .expect("codegen order fixture should parse")
    }

    fn floating_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/floating-primitives.xsd"),
        )
        .expect("floating fixture should parse")
    }

    fn inheritance_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-record-inheritance.xsd"),
        )
        .expect("inheritance fixture should parse")
    }

    fn constrained_floating_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-constrained-floating.xsd"),
        )
        .expect("constrained floating fixture should parse")
    }

    /// Task 033 sections 19--22: compile the generated constrained-float module
    /// and exercise every bound shape at runtime, including the IEEE special
    /// values. Asserting on generated *text* alone would not prove the emitted
    /// comparisons actually behave the way the schema requires.
    #[test]
    fn constrained_floats_enforce_bounds_at_runtime() {
        let source =
            generate(&constrained_floating_schema(), CLOSED).expect("fixture must generate");

        // Width correctness is structural, so assert it before running.
        assert!(source.contains("pub struct FloatUnitInterval(f32);"));
        assert!(source.contains("pub struct DoubleAltitude(f64);"));
        assert!(source.contains("pub const MIN: f64 = -6378237.0;"));
        // The named chain enforces its *effective* domain, both bounds.
        let derived = source
            .split("pub struct DerivedFloat(f64);")
            .nth(1)
            .expect("DerivedFloat must be generated");
        assert!(derived.contains("pub const MIN: f64 = 0.0;"));
        assert!(derived.contains("pub const MAX: f64 = 10.0;"));
        assert!(derived.contains("if value >= Self::MIN && value <= Self::MAX {"));

        use std::fs;
        use std::process::Command;
        let directory = std::env::temp_dir().join("ams-gra-oms-task033-rust-bounds");
        fs::create_dir_all(&directory).expect("create Rust probe directory");
        fs::write(directory.join("generated.rs"), source).expect("write generated Rust module");
        fs::write(directory.join("probe.rs"), RUST_BOUNDS_PROBE).expect("write Rust probe");

        let status = Command::new("rustc")
            .current_dir(&directory)
            .args(["--edition", "2021", "-o", "probe", "probe.rs"])
            .status()
            .expect("rustc should be available in a Rust workspace");
        assert!(status.success(), "generated Rust module must compile");
        let run = Command::new(directory.join("probe"))
            .status()
            .expect("compiled probe must run");
        fs::remove_dir_all(&directory).expect("remove Rust probe directory");
        assert!(run.success(), "generated bound checks must hold at runtime");
    }

    /// The runtime assertions compiled against the generated module above.
    const RUST_BOUNDS_PROBE: &str = r#"
include!("generated.rs");

fn main() {
    // Lower inclusive: the bound itself is admitted, just below is not.
    assert!(FloatLowerInclusive::new(0.0).is_some());
    assert!(FloatLowerInclusive::new(1.0).is_some());
    assert!(FloatLowerInclusive::new(-0.1).is_none());

    // Upper inclusive.
    assert!(FloatUpperInclusive::new(1.0).is_some());
    assert!(FloatUpperInclusive::new(1.000001).is_none());

    // Two-sided, and the accessor round-trips the stored value.
    assert!(FloatUnitInterval::new(0.5).is_some());
    assert_eq!(FloatUnitInterval::new(0.25).unwrap().get(), 0.25f32);
    assert!(FloatUnitInterval::new(-0.001).is_none());
    assert!(FloatUnitInterval::new(1.001).is_none());

    // Lower exclusive: the bound is rejected, anything above it accepted.
    assert!(DoublePositive::new(0.0).is_none());
    assert!(DoublePositive::new(f64::MIN_POSITIVE).is_some());

    // Upper exclusive.
    assert!(DoubleUpperExclusive::new(1.0).is_none());
    assert!(DoubleUpperExclusive::new(0.999).is_some());

    // Mixed two-sided: inclusive lower, exclusive upper.
    assert!(DoubleMixedRange::new(-3.5).is_some());
    assert!(DoubleMixedRange::new(3.5).is_none());
    assert!(DoubleMixedRange::new(3.4999).is_some());

    // The authoritative AltitudeType bound, at full binary64 precision.
    assert!(DoubleAltitude::new(-6378237.0).is_some());
    assert!(DoubleAltitude::new(-6378237.5).is_none());
    assert_eq!(DoubleAltitude::MIN, -6378237.0f64);

    // Named chain: the derived type enforces the effective inherited domain.
    assert!(DerivedFloat::new(0.0).is_some());
    assert!(DerivedFloat::new(10.0).is_some());
    assert!(DerivedFloat::new(-0.5).is_none());
    assert!(DerivedFloat::new(10.5).is_none());

    // NaN is an instance of no range-constrained type.
    assert!(FloatUnitInterval::new(f32::NAN).is_none());
    assert!(DoubleAltitude::new(f64::NAN).is_none());
    assert!(DoublePositive::new(f64::NAN).is_none());
    assert!(DoubleUpperExclusive::new(f64::NAN).is_none());

    // One-sided infinity keeps IEEE ordering rather than being rejected
    // merely because the type is constrained.
    assert!(DoubleAltitude::new(f64::INFINITY).is_some());
    assert!(DoubleAltitude::new(f64::NEG_INFINITY).is_none());
    assert!(DoubleUpperExclusive::new(f64::NEG_INFINITY).is_some());
    assert!(DoubleUpperExclusive::new(f64::INFINITY).is_none());
    // A two-sided finite range excludes both.
    assert!(DoubleMixedRange::new(f64::INFINITY).is_none());
    assert!(DoubleMixedRange::new(f64::NEG_INFINITY).is_none());

    // Signed zero compares equal, so inclusive admits both and exclusive
    // rejects both.
    assert!(FloatLowerInclusive::new(-0.0).is_some());
    assert!(FloatLowerInclusive::new(0.0).is_some());
    assert!(DoublePositive::new(-0.0).is_none());
    assert!(DoublePositive::new(0.0).is_none());
}
"#;

    /// Task 033 sections 21/68: an unconstrained named float keeps its exact
    /// Task 022 API and text. Adding constrained support must not churn it.
    #[test]
    fn unconstrained_float_output_is_unchanged() {
        for (kind, scalar) in [
            (PrimitiveKind::Float32, "f32"),
            (PrimitiveKind::Float64, "f64"),
        ] {
            let mut schema = track_schema();
            schema.types[0].kind = TypeKind::Primitive(kind);
            schema.types[0].constraints = ConstraintSet::default();
            let source = generate(&schema, CLOSED).expect("unconstrained float must generate");
            assert!(source.contains(&format!(
                "#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]\npub struct TrackId({scalar});\n\nimpl TrackId {{\n    pub const fn new(value: {scalar}) -> Self {{ Self(value) }}\n    pub const fn get(self) -> {scalar} {{ self.0 }}\n}}\n"
            )));
            // Infallible construction is retained for *this* declaration: no
            // Option, no bounds. Other declarations in the fixture legitimately
            // use checked constructors, so the assertion is scoped.
            let impl_block = source
                .split("impl TrackId {")
                .nth(1)
                .and_then(|rest| rest.split("\n}\n").next())
                .expect("TrackId impl must be generated");
            assert!(!impl_block.contains("-> Option<Self>"));
            assert!(!impl_block.contains("pub const MIN"));
            assert!(!impl_block.contains("pub const MAX"));
        }
    }

    /// Task 033 sections 33/69: a field whose TypeRef is a *direct* primitive
    /// float carrying its own numeric constraint has no checked wrapper to
    /// build, so it must still fail closed rather than drop the constraint.
    #[test]
    fn direct_float_field_constraints_remain_unsupported() {
        for kind in [PrimitiveKind::Float32, PrimitiveKind::Float64] {
            let mut schema = track_schema();
            let record = schema
                .types
                .iter_mut()
                .find(|declaration| matches!(declaration.kind, TypeKind::Record { .. }))
                .expect("track fixture should contain a record");
            let TypeKind::Record { fields } = &mut record.kind else {
                unreachable!("just matched a Record");
            };
            fields.truncate(1);
            fields[0].type_ref = TypeRef::primitive(kind);
            fields[0].constraints = ConstraintSet {
                min_inclusive: Some(if kind == PrimitiveKind::Float32 {
                    NumericValue::Float32(ams_gra_oms_ir::Float32Value::from_value(0.0))
                } else {
                    NumericValue::Float64(ams_gra_oms_ir::Float64Value::from_value(0.0))
                }),
                ..ConstraintSet::default()
            };
            let error = generate(&schema, CLOSED)
                .expect_err("field-local floating constraints remain unsupported");
            assert!(error.message.contains("field constraints"));
        }
    }

    /// Task 033 section 35: length facets are meaningless on a float and must
    /// not be quietly accepted.
    #[test]
    fn length_facets_on_floats_remain_unsupported() {
        for constraints in [
            ConstraintSet {
                length: Some(4),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                min_length: Some(1),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                max_length: Some(8),
                ..ConstraintSet::default()
            },
        ] {
            let mut schema = track_schema();
            schema.types[0].kind = TypeKind::Primitive(PrimitiveKind::Float64);
            schema.types[0].constraints = constraints;
            let error =
                generate(&schema, CLOSED).expect_err("length facets on a float must fail closed");
            assert!(error.message.contains("length constraints"));
        }
    }

    /// Task 033 section 71: a bound in the wrong width is a defect in
    /// externally constructed IR and is never converted.
    #[test]
    fn wrong_width_float_bounds_remain_unsupported() {
        let mut schema = track_schema();
        schema.types[0].kind = TypeKind::Primitive(PrimitiveKind::Float32);
        schema.types[0].constraints = ConstraintSet {
            min_inclusive: Some(NumericValue::Float64(
                ams_gra_oms_ir::Float64Value::from_value(0.0),
            )),
            ..ConstraintSet::default()
        };
        let error = generate(&schema, CLOSED).expect_err("wrong-width bound must fail closed");
        assert!(
            error
                .message
                .contains("Float32 declaration requires Float32 bounds")
        );

        let mut schema = track_schema();
        schema.types[0].kind = TypeKind::Primitive(PrimitiveKind::Float64);
        schema.types[0].constraints = ConstraintSet {
            max_inclusive: Some(NumericValue::Float32(
                ams_gra_oms_ir::Float32Value::from_value(1.0),
            )),
            ..ConstraintSet::default()
        };
        let error = generate(&schema, CLOSED).expect_err("wrong-width bound must fail closed");
        assert!(
            error
                .message
                .contains("Float64 declaration requires Float64 bounds")
        );
    }

    fn abstract_value_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-abstract-value-reference.xsd"),
        )
        .expect("abstract value fixture should parse")
    }

    fn uninhabited_optional_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-uninhabited-abstract-optional.xsd"),
        )
        .expect("uninhabited abstract optional fixture should parse")
    }

    fn uninhabited_required_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-uninhabited-abstract-required.xsd"),
        )
        .expect("uninhabited abstract required fixture should parse")
    }

    fn uninhabited_future_descendant_schema() -> SchemaIr {
        load_schema_document(&Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../xsd-frontend/tests/fixtures/backend-uninhabited-abstract-future-descendant.xsd",
        ))
        .expect("uninhabited abstract future-descendant fixture should parse")
    }

    fn uninhabited_inherited_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-uninhabited-abstract-inherited.xsd"),
        )
        .expect("uninhabited abstract inherited fixture should parse")
    }

    fn uninhabited_composes_closed_sum_schema() -> SchemaIr {
        load_schema_document(&Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../xsd-frontend/tests/fixtures/backend-uninhabited-abstract-composes-closed-sum.xsd",
        ))
        .expect("uninhabited abstract closed-sum composition fixture should parse")
    }

    fn choice_boundary_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-choice-boundary.xsd"),
        )
        .expect("choice boundary fixture should parse")
    }

    fn choice_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-choice-lowering.xsd"),
        )
        .expect("choice fixture should parse")
    }

    fn repeated_choice_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-choice-repeated.xsd"),
        )
        .expect("repeated Choice fixture should parse")
    }

    fn integral_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-integral-scalars.xsd"),
        )
        .expect("integral fixture should parse")
    }

    fn unbounded_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-unbounded-cardinality.xsd"),
        )
        .expect("unbounded fixture should parse")
    }

    #[test]
    fn lowers_unbounded_records_choices_and_constrained_elements() {
        let source =
            generate(&unbounded_schema(), CLOSED).expect("unbounded cardinality should generate");
        assert!(source.contains("pub struct UnboundedVec<T, const MIN: u64>(Vec<T>);"));
        assert!(source.contains("usize::try_from(MIN).is_ok_and(|min| values.len() >= min)"));
        assert!(!source.contains("u64::try_from(values.len())"));
        assert!(source.contains("UnboundedVec<Item, 0>"));
        assert!(source.contains("UnboundedVec<Item, 1>"));
        assert!(source.contains("UnboundedVec<Item, 2>"));
        assert!(source.contains("UnboundedVec<BoundedU64<0, 255>, 0>"));
        assert!(source.contains("ManyByte(UnboundedVec<BoundedU64<0, 255>, 1>)"));
    }

    #[test]
    fn lowers_integral_scalars_and_preserves_direct_ranges() {
        let source =
            generate(&integral_schema(), CLOSED).expect("integral scalars should generate");
        assert!(source.contains("pub struct BoundedI64<const MIN: i64, const MAX: i64>"));
        assert!(source.contains("pub enabled: bool"));
        assert!(source.contains("pub byte_value: BoundedI64<-128, 127>"));
        assert!(source.contains("pub unsigned_byte_value: BoundedU64<0, 255>"));
        assert!(source.contains("BoundedVec<BoundedI64<-128, 127>, 0, 8>"));
        assert!(source.contains("TrueCase(bool)"));
        assert!(source.contains("SignedCase(BoundedI64<-32768, 32767>)"));
        assert!(source.contains("UnsignedCase(BoundedU64<0, 255>)"));
        assert!(source.contains("pub struct UnsignedBounded(u64);"));
        assert!(source.contains("pub struct NamedBoolean(bool);"));
    }

    #[test]
    fn lowers_choice_as_named_enum_and_accepts_empty_record_ancestry() {
        let source = generate(&choice_schema(), CLOSED).expect("supported Choice should generate");
        assert!(source.contains("pub enum Selection {\n    First(Token),\n    Second(Token),"));
        assert!(source.contains("pub struct Holder {\n    pub selected: Selection,"));
        assert!(
            source.contains("pub enum DerivedSelection {\n    Left(Token),\n    Right(Token),")
        );
    }

    #[test]
    fn choice_validation_firewalls_and_finite_repetition_are_exercised_by_generation() {
        let mut collision = choice_schema();
        let TypeKind::Choice { alternatives } = &mut collision.types[1].kind else {
            panic!("Selection must be a Choice");
        };
        alternatives[0].name = "Foo".to_owned();
        alternatives[1].name = "foo".to_owned();
        // `Foo` and `foo` both upper-camel to `Foo`. This is now diagnosed by
        // the shared backend name preflight rather than by a Rust-local
        // duplicate check, so the assertion is on the semantic outcome -- the
        // collision is rejected and both spellings are named -- rather than on
        // the exact prose of the superseded local diagnostic.
        let message = generate(&collision, CLOSED)
            .expect_err("converging Rust variant names must be rejected")
            .message;
        assert!(
            message.contains("\"Foo\"") && message.contains("\"foo\""),
            "{message}"
        );

        let mut nillable = choice_schema();
        let TypeKind::Choice { alternatives } = &mut nillable.types[1].kind else {
            panic!("Selection must be a Choice");
        };
        alternatives[0].nillable = true;
        assert!(
            generate(&nillable, CLOSED)
                .unwrap_err()
                .message
                .contains("nillable Choice alternative First")
        );

        let mut constrained = choice_schema();
        let TypeKind::Choice { alternatives } = &mut constrained.types[1].kind else {
            panic!("Selection must be a Choice");
        };
        alternatives[0].constraints.length = Some(4);
        assert!(
            generate(&constrained, CLOSED)
                .unwrap_err()
                .message
                .contains("field constraints on First")
        );

        let mut abstract_target = abstract_value_schema();
        let holder = abstract_target
            .types
            .iter_mut()
            .find(|type_decl| type_decl.name.local_name == "Holder")
            .unwrap();
        let TypeKind::Record { fields } =
            std::mem::replace(&mut holder.kind, TypeKind::Record { fields: Vec::new() })
        else {
            panic!("Holder must be a Record");
        };
        holder.kind = TypeKind::Choice {
            alternatives: fields,
        };
        assert!(
            generate(&abstract_target, CLOSED)
                .expect("abstract Choice alternative should lower")
                .contains("Value(Base)")
        );

        let source = generate(&repeated_choice_schema(), CLOSED)
            .expect("finite repeated Choice must generate");
        assert!(source.contains("Items(BoundedVec<Token, 0, 3>)"));
    }

    #[test]
    fn lowers_effective_record_fields_and_omits_abstract_ancestor() {
        let source =
            generate(&inheritance_schema(), CLOSED).expect("pure Record inheritance is supported");
        assert!(!source.contains("pub struct Base"));
        let leaf = source.find("pub struct Leaf").unwrap();
        let fields = &source[leaf..];
        assert!(fields.find("base_optional").unwrap() < fields.find("base_values").unwrap());
        assert!(fields.find("base_values").unwrap() < fields.find("linked").unwrap());
        assert!(fields.find("linked").unwrap() < fields.find("local").unwrap());
    }

    #[test]
    fn lowers_abstract_value_references_and_retains_choice_boundary() {
        let source =
            generate(&abstract_value_schema(), CLOSED).expect("closed abstract value should lower");
        assert!(source.contains("pub enum Base {"));
        assert!(source.contains("    Derived(Derived),"));
        assert!(source.find("pub struct Derived").unwrap() < source.find("pub enum Base").unwrap());
        assert!(source.find("pub enum Base").unwrap() < source.find("pub struct Holder").unwrap());
        assert!(
            generate(&choice_boundary_schema(), CLOSED)
                .unwrap_err()
                .message
                .contains("contains Record segment Base while lowering Choice")
        );
    }

    #[test]
    fn binary_composes_with_abstract_closed_sum() {
        let mut schema = abstract_value_schema();
        let base = schema
            .types
            .iter_mut()
            .find(|declaration| declaration.name.local_name == "Base")
            .expect("abstract fixture should contain Base");
        let TypeKind::Record { fields } = &mut base.kind else {
            panic!("Base must be a Record");
        };
        fields[0].type_ref = TypeRef::primitive(PrimitiveKind::Binary);
        let source =
            generate(&schema, CLOSED).expect("abstract closed sum with binary field must lower");
        assert!(source.contains("pub enum Base {"));
        assert!(source.contains("pub struct Derived {"));
        assert!(source.contains("Vec<u8>"));
        assert!(source.contains("#[derive(Debug, Clone, PartialEq, Eq)]\npub struct Derived {"));
    }

    #[test]
    fn track_matches_golden_and_is_deterministic() {
        let schema = track_schema();
        let first = generate(&schema, CLOSED).expect("Rust generation should succeed");
        assert_eq!(
            first,
            generate(&schema, CLOSED).expect("generation should repeat")
        );
        assert_eq!(first, include_str!("../tests/expected/track.rs"));
    }

    #[test]
    fn schema_set_matches_dependency_order_golden() {
        let source =
            generate(&codegen_order_schema(), CLOSED).expect("Rust generation should succeed");
        assert_eq!(source, include_str!("../tests/expected/codegen_order.rs"));
        assert!(
            source.find("pub struct IncludedId").unwrap()
                < source.find("pub struct RecordFirstInSource").unwrap()
        );
        assert!(
            source.find("pub enum IncludedQuality").unwrap()
                < source.find("pub struct RecordFirstInSource").unwrap()
        );
    }

    #[test]
    fn preserves_order_naming_and_cardinality_semantics() {
        let source = generate(&track_schema(), CLOSED).expect("Rust generation should succeed");
        assert!(source.contains("pub struct TrackId(i64);"));
        assert!(source.contains("pub const MIN: i64 = 1;\n    pub const MAX: i64 = 65535;"));
        assert!(source.contains("Unknown,\n    Tentative,\n    Confirmed,"));
        assert!(source.contains("pub callsign: Option<String>"));
        assert!(source.contains("pub sensor_ids: BoundedVec<i64, 0, 8>"));
        assert!(source.contains("pub id: TrackId,\n    pub quality: TrackQuality,"));
    }

    #[test]
    fn unsupported_construct_fails_explicitly() {
        let mut schema = track_schema();
        let alternative = schema
            .types
            .iter()
            .find_map(|declaration| match &declaration.kind {
                TypeKind::Record { fields } => fields
                    .iter()
                    .find(|field| matches!(field.type_ref.target, TypeRefTarget::Primitive(_)))
                    .cloned(),
                _ => None,
            })
            .expect("track fixture should contain a field");
        schema.types[0].kind = TypeKind::Choice {
            alternatives: vec![alternative],
        };
        schema.types[0].base_type = None;
        let source = generate(&schema, CLOSED).expect("Choice must render");
        assert!(source.contains("pub enum TrackId"));
    }

    #[test]
    fn lowers_unconstrained_floating_and_rejects_constraints() {
        for kind in [PrimitiveKind::Float32, PrimitiveKind::Float64] {
            let mut schema = floating_schema();
            let TypeKind::Record { fields } = &mut schema.types[0].kind else {
                panic!("floating fixture should contain a record");
            };
            fields.truncate(1);
            fields[0].type_ref = TypeRef::primitive(kind);
            let source =
                generate(&schema, CLOSED).expect("unconstrained floating generation must succeed");
            assert!(source.contains(if kind == PrimitiveKind::Float32 {
                "f32"
            } else {
                "f64"
            }));
        }

        // Task 033 supersedes the Task 022 blanket rejection for the bound-only
        // subset: a named lower-inclusive Float64 is now lowered as a checked
        // newtype. Non-range facet shapes still fail closed, below.
        let mut schema = track_schema();
        schema.types[0].kind = TypeKind::Primitive(PrimitiveKind::Float64);
        schema.types[0].constraints = ConstraintSet {
            min_inclusive: Some(NumericValue::Float64(
                ams_gra_oms_ir::Float64Value::from_value(0.0),
            )),
            ..ConstraintSet::default()
        };
        let source = generate(&schema, CLOSED).expect("Task 033 bounded float must render");
        assert!(source.contains("pub struct TrackId(f64);"));
        assert!(source.contains("pub const MIN: f64 = 0.0;"));
        assert!(source.contains("pub const fn new(value: f64) -> Option<Self> {"));
        assert!(source.contains("if value >= Self::MIN {"));
        // No total-order or hashing traits may be claimed for a float.
        assert!(!source.contains("PartialOrd, Eq"));
        assert!(!source.contains("Hash)]\npub struct TrackId(f64)"));

        // A lexical facet alongside the numeric bound is not partially
        // enforced: the whole declaration is still rejected.
        schema.types[0].constraints.lexical = ams_gra_oms_ir::LexicalConstraintSet {
            pattern_groups: vec![ams_gra_oms_ir::PatternGroup {
                alternatives: vec![ams_gra_oms_ir::PatternExpression::xml_schema("[0-9]+")],
            }],
            white_space: None,
        };
        let error =
            generate(&schema, CLOSED).expect_err("lexical float constraints remain unsupported");
        assert!(error.message.contains("unsupported Rust IR construct"));
        assert!(error.message.contains("lexical constraints"));
    }

    #[test]
    fn lowers_unconstrained_binary_direct_field_and_named_declaration() {
        let mut schema = floating_schema();
        let TypeKind::Record { fields } = &mut schema.types[0].kind else {
            panic!("floating fixture should contain a record");
        };
        fields.truncate(1);
        fields[0].type_ref = TypeRef::primitive(PrimitiveKind::Binary);
        let source =
            generate(&schema, CLOSED).expect("unconstrained binary generation must succeed");
        assert!(source.contains("Vec<u8>"));

        let mut schema = track_schema();
        schema.types[0].kind = TypeKind::Primitive(PrimitiveKind::Binary);
        schema.types[0].constraints = ConstraintSet::default();
        let source = generate(&schema, CLOSED).expect("unconstrained named binary must generate");
        assert!(source.contains("pub struct TrackId(Vec<u8>);"));
        assert!(source.contains("pub fn as_slice(&self) -> &[u8] { &self.0 }"));
    }

    #[test]
    fn binary_declaration_with_constraints_remains_unsupported() {
        let mut schema = track_schema();
        schema.types[0].kind = TypeKind::Primitive(PrimitiveKind::Binary);
        schema.types[0].constraints = ConstraintSet {
            length: Some(4),
            ..ConstraintSet::default()
        };
        let error =
            generate(&schema, CLOSED).expect_err("constrained binary declaration must fail");
        assert!(
            error
                .message
                .contains("unsupported Rust IR construct: constraints on")
        );
    }

    #[test]
    fn binary_field_with_constraints_remains_unsupported() {
        let mut schema = floating_schema();
        let TypeKind::Record { fields } = &mut schema.types[0].kind else {
            panic!("floating fixture should contain a record");
        };
        fields.truncate(1);
        fields[0].type_ref = TypeRef::primitive(PrimitiveKind::Binary);
        fields[0].constraints = ConstraintSet {
            length: Some(4),
            ..ConstraintSet::default()
        };
        let error = generate(&schema, CLOSED).expect_err("constrained binary field must fail");
        assert!(
            error
                .message
                .contains("unsupported Rust IR construct: field constraints on")
        );
    }

    #[test]
    fn temporal_and_constrained_scalars_fail_explicitly() {
        for kind in [PrimitiveKind::Time, PrimitiveKind::Duration] {
            let mut schema = floating_schema();
            let TypeKind::Record { fields } = &mut schema.types[0].kind else {
                panic!("floating fixture should contain a record");
            };
            fields.truncate(1);
            fields[0].type_ref = TypeRef::primitive(kind);
            let error =
                generate(&schema, CLOSED).expect_err("temporal generation must remain unsupported");
            assert!(error.message.contains(&format!(
                "unsupported Rust IR construct: type reference Primitive({kind:?})"
            )));
        }

        for kind in [PrimitiveKind::String, PrimitiveKind::Binary] {
            let mut schema = track_schema();
            schema.types[0].kind = TypeKind::Primitive(kind);
            schema.types[0].constraints = ConstraintSet {
                length: Some(4),
                ..ConstraintSet::default()
            };
            let error = generate(&schema, CLOSED).expect_err("constraints must not be discarded");
            assert!(
                error
                    .message
                    .contains("unsupported Rust IR construct: constraints on")
            );
        }
    }

    /// Lexical constraints still fail before rendering everywhere Task 036 has
    /// not implemented a validator.
    ///
    /// `DateTime` + `.+Z` is deliberately absent from this list: that exact
    /// pair is the one Task 036 profile the generator now fully enforces, and
    /// the assertion below distinguishes it from `Time` carrying the *same*
    /// pattern text, which remains unsupported.
    #[test]
    fn lexical_constraints_fail_before_rendering() {
        for (kind, white_space) in [
            (PrimitiveKind::Time, false),
            (PrimitiveKind::SignedInteger, false),
            (PrimitiveKind::String, false),
            (PrimitiveKind::String, true),
        ] {
            let mut schema = track_schema();
            schema.types[0].kind = TypeKind::Primitive(kind);
            schema.types[0].constraints = ConstraintSet::default();
            if white_space {
                schema.types[0].constraints.lexical.white_space =
                    Some(ams_gra_oms_ir::WhiteSpacePolicy::Collapse);
            } else {
                schema.types[0].constraints.lexical.pattern_groups.push(
                    ams_gra_oms_ir::PatternGroup {
                        alternatives: vec![ams_gra_oms_ir::PatternExpression::xml_schema(".+Z")],
                    },
                );
            }
            let error =
                generate(&schema, CLOSED).expect_err("lexical constraints must be rejected");
            let message = &error.message;
            assert!(
                message.contains("unsupported Rust IR construct: constraints on")
                    || message.contains("unsupported temporal declaration: Time"),
                "unexpected diagnostic for {kind:?}: {message}"
            );
        }
        // The same pattern text on `DateTime` -- and only there -- is now the
        // supported profile, so it must generate rather than fail.
        let mut supported = track_schema();
        supported.types[0].kind = TypeKind::Primitive(PrimitiveKind::DateTime);
        supported.types[0].constraints = ConstraintSet::default();
        supported.types[0]
            .constraints
            .lexical
            .pattern_groups
            .push(ams_gra_oms_ir::PatternGroup {
                alternatives: vec![ams_gra_oms_ir::PatternExpression::xml_schema(".+Z")],
            });
        generate(&supported, CLOSED).expect("the supported DateTime Zulu profile must generate");
    }

    #[test]
    fn abstract_types_fail_explicitly() {
        let mut schema = track_schema();
        schema.types[0].is_abstract = true;
        let error = generate(&schema, CLOSED).expect_err("abstract type must be rejected");
        assert!(
            error
                .message
                .contains("unsupported Rust IR construct: abstract type")
        );
    }

    #[test]
    fn uninhabited_abstract_optional_field_is_elided_without_fake_payload() {
        let source = generate(&uninhabited_optional_schema(), CLOSED)
            .expect("absent-only occurrence should lower");
        assert!(!source.contains("SidecarPoint"));
        assert!(!source.contains("widget"));
        assert!(source.contains("pub struct Holder {\n    pub required: String,\n}"));
    }

    /// An unrelated semantic failure must not be re-attributed to a phantom
    /// generated-name collision.
    ///
    /// `BoundedVec` here is ancestry only, so nothing is emitted for it even
    /// though it projects successfully. If preflight promoted it to a Task
    /// 024 wrapper it would collide with this backend's own `BoundedVec`
    /// support type, and that invented naming error would be reported
    /// *instead of* the real `Uninhabited` failure -- pointing the user at a
    /// declaration that is not the cause.
    #[test]
    fn an_ancestry_only_abstract_does_not_mask_an_unrelated_semantic_failure() {
        let schema = load_schema_document(&Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../xsd-frontend/tests/fixtures/\
             backend-ancestry-only-abstract-with-unrelated-failure.xsd",
        ))
        .expect("ancestry-only/unrelated-failure fixture should parse");
        let error = generate(&schema, CLOSED)
            .expect_err("the demanded zero-descendant target must fail closed");
        assert!(
            error
                .message
                .contains("Uninhabited has no concrete structural descendants"),
            "the semantic failure must stay authoritative: {}",
            error.message
        );
        assert!(
            !error.message.contains("BoundedVec"),
            "an ancestry-only abstract must not be blamed for the failure: {}",
            error.message
        );
    }

    #[test]
    fn uninhabited_abstract_required_field_remains_unsupported() {
        let error = generate(&uninhabited_required_schema(), CLOSED)
            .expect_err("positive-minimum uninhabited value must fail closed");
        assert!(
            error
                .message
                .contains("SidecarPoint has no concrete structural descendants")
        );
    }

    #[test]
    fn future_descendant_reclassifies_uninhabited_target_as_a_closed_sum() {
        let source = generate(&uninhabited_future_descendant_schema(), CLOSED)
            .expect("schema set with a concrete descendant should lower as a closed sum");
        assert!(source.contains("pub enum SidecarPoint {"));
        assert!(source.contains("ConcreteSidecarPoint(ConcreteSidecarPoint)"));
        assert!(source.contains("pub widget: Option<SidecarPoint>"));
    }

    #[test]
    fn inherited_uninhabited_field_is_elided_on_the_concrete_descendant() {
        let source = generate(&uninhabited_inherited_schema(), CLOSED)
            .expect("inherited absent-only occurrence should lower");
        assert!(!source.contains("SidecarPoint"));
        assert!(source.contains("pub struct ConcreteHolder {\n    pub required: String,\n}"));
    }

    #[test]
    fn uninhabited_optional_field_composes_with_task_024_closed_sum() {
        let source = generate(&uninhabited_composes_closed_sum_schema(), CLOSED)
            .expect("Task 026 composition with Task 024 closed sum should lower");
        assert!(source.contains("pub enum Parent {"));
        assert!(source.contains("ConcreteChild(ConcreteChild)"));
        assert!(source.contains("pub struct ConcreteChild {\n}"));
        assert!(!source.contains("widget"));
    }

    // ---------------------------------------------------------------
    // Task 028 -- explicit generation world policy
    // ---------------------------------------------------------------

    fn base_only_ancestry_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-base-only-abstract-ancestry.xsd"),
        )
        .expect("base-only ancestry fixture should parse")
    }

    fn private_extension_overlay_schema() -> SchemaIr {
        load_schema_set(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/private-extension-overlay/root.xsd"),
        )
        .expect("private extension overlay fixture should parse")
    }

    fn public_only_repeated_extension_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/public-only-repeated-extension.xsd"),
        )
        .expect("public-only repeated extension fixture should parse")
    }

    /// Sections 13/16/40: the same Task 024 fixture generates a closed sum
    /// under `closed-schema` and fails closed under `open-extensions`, with a
    /// diagnostic that names the target and the policy rather than claiming
    /// the schema is invalid.
    #[test]
    fn task028_abstract_value_depends_on_world() {
        generate(&abstract_value_schema(), CLOSED)
            .expect("closed world must keep Task 024 closed-sum lowering");
        let error = generate(&abstract_value_schema(), OPEN)
            .expect_err("open world must reject an abstract value");
        let message = error.to_string();
        assert!(message.contains("Base"), "must name the target: {message}");
        assert!(
            message.contains("open-extensions"),
            "must name the policy: {message}"
        );
        assert!(
            message.contains("external derived types cannot be represented"),
            "must explain why: {message}"
        );
        assert!(
            !message.contains("invalid"),
            "must not claim the schema is invalid: {message}"
        );
        assert!(
            !message.contains("no concrete structural descendants"),
            "one-descendant target must use the open diagnostic, not the \
             zero-descendant diagnostic: {message}"
        );
    }

    /// Sections 41/42: the Task 026 zero-descendant optional field is elided
    /// under `closed-schema`, but must NOT be elided under `open-extensions`,
    /// because an external derived type may legally make it present.
    #[test]
    fn task028_zero_descendant_optional_depends_on_world() {
        generate(&uninhabited_optional_schema(), CLOSED)
            .expect("closed world must keep Task 026 absent-only elision");
        let message = generate(&uninhabited_optional_schema(), OPEN)
            .expect_err("open world must not elide a possibly-external payload")
            .to_string();
        assert!(
            message.contains("SidecarPoint") && message.contains("open-extensions"),
            "open diagnostic must name target and policy: {message}"
        );
    }

    /// Sections 12/43: an abstract base used only as ancestry is not a value
    /// position, so both worlds succeed and produce identical output.
    #[test]
    fn task028_base_only_ancestry_is_world_independent() {
        let closed = generate(&base_only_ancestry_schema(), CLOSED)
            .expect("base-only ancestry must generate in the closed world");
        let open = generate(&base_only_ancestry_schema(), OPEN)
            .expect("base-only ancestry must generate in the open world too");
        assert_eq!(
            closed, open,
            "ancestry-only abstraction must not be affected by world policy"
        );
    }

    /// Sections 45-47: supplying the private derived type as part of the
    /// generation schema set (ADR-0004 option 3) makes closed-schema lowering
    /// work, composing with repeated cardinality. Open-extensions stays
    /// conservative: including one private descendant does not prove that no
    /// OTHER external descendant exists.
    #[test]
    fn task028_private_extension_overlay() {
        let source = generate(&private_extension_overlay_schema(), CLOSED)
            .expect("supplying the private schema must close the extension point");
        assert!(
            source.contains("PrivateExtension") || source.contains("Private_Extension"),
            "closed sum must include the private descendant: {source}"
        );
        let message = generate(&private_extension_overlay_schema(), OPEN)
            .expect_err("open world stays conservative even with a private descendant")
            .to_string();
        assert!(
            message.contains("ExtensionBase") && message.contains("open-extensions"),
            "open diagnostic must name target and policy: {message}"
        );
    }

    /// Section 48: without the private overlay the repeated zero-descendant
    /// abstract value is unsupported in BOTH worlds. Task 028 deliberately
    /// does not extend Task 026 to always-empty repeated collections.
    #[test]
    fn task028_public_only_repeated_extension_is_unsupported_in_both_worlds() {
        let closed = generate(&public_only_repeated_extension_schema(), CLOSED)
            .expect_err("repeated absent-only elision was never adopted")
            .to_string();
        assert!(
            closed.contains("ExtensionBase"),
            "closed diagnostic must name the target: {closed}"
        );
        assert!(
            closed.contains("no concrete structural descendants"),
            "closed world reports the zero-descendant reason: {closed}"
        );
        let open = generate(&public_only_repeated_extension_schema(), OPEN)
            .expect_err("open world rejects the abstract value as well")
            .to_string();
        assert!(
            open.contains("open-extensions"),
            "open world reports the open-world reason instead: {open}"
        );
    }

    // ---------------------------------------------------------------
    // Task 029 -- additive same-namespace schema overlays
    // ---------------------------------------------------------------

    fn schema_overlay_fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/schema-overlay")
            .join(name)
    }

    fn overlay_composed_schema(overlays: &[&str]) -> SchemaIr {
        load_schema_set_with_overlays(
            &schema_overlay_fixture("public.xsd"),
            &overlays
                .iter()
                .map(|name| schema_overlay_fixture(name))
                .collect::<Vec<_>>(),
        )
        .expect("overlay fixture should compose")
    }

    /// Task 029 sections 29/30: supplying the private derived type through an
    /// explicit overlay -- with no edit to the public root -- closes the Task
    /// 024 sum for the repeated 0..* extension point. The same composed schema
    /// still fails closed under `open-extensions`, because knowing one private
    /// descendant never proves no OTHER external descendant exists.
    #[test]
    fn task029_private_overlay_closes_the_sum_but_not_the_world() {
        let source = generate(&overlay_composed_schema(&["private-a.xsd"]), CLOSED)
            .expect("an explicit overlay must close the extension point");
        assert!(source.contains("pub enum ExtensionBase {"));
        assert!(source.contains("PrivateA(PrivateA)"));
        assert!(
            source.contains("pub extensions: UnboundedVec<ExtensionBase, 0>"),
            "the repeated 0..* field must still be generated: {source}"
        );

        let message = generate(&overlay_composed_schema(&["private-a.xsd"]), OPEN)
            .expect_err("section 25: an overlay must not imply a closed world")
            .to_string();
        assert!(
            message.contains("ExtensionBase") && message.contains("open-extensions"),
            "open diagnostic must name target and policy: {message}"
        );
    }

    /// Task 029 section 31: closed-sum variant order follows the caller's
    /// overlay order, which is explicit deterministic input. Overlays are
    /// never sorted by filesystem path.
    #[test]
    fn task029_two_overlays_order_variants_by_caller_order() {
        let forward = generate(
            &overlay_composed_schema(&["private-a.xsd", "private-b.xsd"]),
            CLOSED,
        )
        .expect("two overlays should compose");
        let forward_a = forward
            .find("PrivateA(PrivateA)")
            .expect("PrivateA variant");
        let forward_b = forward
            .find("PrivateB(PrivateB)")
            .expect("PrivateB variant");
        assert!(forward_a < forward_b);

        let reversed = generate(
            &overlay_composed_schema(&["private-b.xsd", "private-a.xsd"]),
            CLOSED,
        )
        .expect("reversed overlays should compose");
        let reversed_a = reversed
            .find("PrivateA(PrivateA)")
            .expect("PrivateA variant");
        let reversed_b = reversed
            .find("PrivateB(PrivateB)")
            .expect("PrivateB variant");
        assert!(reversed_b < reversed_a);
    }

    // -----------------------------------------------------------------
    // Corrective cleanup -- shared backend generated-name preflight
    // -----------------------------------------------------------------

    fn preflight_fixture(name: &str) -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures")
                .join(name),
        )
        .expect("preflight fixture should parse")
    }

    /// Two distinct XSD type names that both upper-camel to `TrackReport`
    /// would declare the same Rust type twice.
    #[test]
    fn converging_declaration_names_are_rejected() {
        let schema = preflight_fixture("backend-name-preflight-declaration-collision.xsd");
        let message = generate(&schema, CLOSED)
            .expect_err("converging declaration names must be rejected")
            .message;
        assert!(message.contains("TrackReport"), "{message}");
    }

    /// An inherited field and a locally declared field that snake_case to the
    /// same member. Effective structural projection accepts them because the
    /// XSD names differ; only generated-name policy catches this.
    #[test]
    fn converging_inherited_field_names_are_rejected() {
        let schema = preflight_fixture("backend-name-preflight-inherited-collision.xsd");
        let message = generate(&schema, CLOSED)
            .expect_err("converging inherited field names must be rejected")
            .message;
        assert!(message.contains("track_id"), "{message}");
    }

    /// A field named `type` snake_cases onto a Rust keyword.
    #[test]
    fn reserved_word_field_is_rejected() {
        let schema = preflight_fixture("backend-name-preflight-reserved.xsd");
        let message = generate(&schema, CLOSED)
            .expect_err("a Rust keyword field must be rejected")
            .message;
        assert!(message.contains("reserved word"), "{message}");
    }

    /// The control must still render, and the generated module must actually
    /// compile: a preflight that rejected everything would pass the negative
    /// tests above while being useless.
    #[test]
    fn preflight_control_renders_and_compiles() {
        let source = generate(
            &preflight_fixture("backend-name-preflight-control.xsd"),
            CLOSED,
        )
        .expect("safe generated names must render");
        assert!(source.contains("pub struct TrackReport"), "{source}");
        // `TrackId` has no `_` word boundary, so it snake_cases to `trackid`.
        // Asserting the exact spelling pins that this module never renames.
        assert!(source.contains("pub trackid:"), "{source}");

        use std::fs;
        use std::process::Command;
        let directory = std::env::temp_dir().join("ams-gra-oms-preflight-rust-control");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create Rust probe directory");
        fs::write(directory.join("generated.rs"), &source).expect("write generated module");
        let status = Command::new("rustc")
            .current_dir(&directory)
            .args(["--edition", "2021", "--crate-type", "lib", "generated.rs"])
            .status()
            .expect("rustc should be available in a Rust workspace");
        let _ = fs::remove_dir_all(&directory);
        assert!(status.success(), "generated Rust module must compile");
    }

    /// Generated-name preflight tracks *emitted* entities, not raw Schema IR
    /// declarations. An abstract Record used only as ancestry is folded into
    /// its descendants and never emitted, so its local name does not occupy
    /// the generated top-level scope -- even when that name is `BoundedVec`,
    /// which Rust always emits as a support type.
    ///
    /// Preflight and actual generation must agree, and the result must
    /// compile: the module contains exactly one `BoundedVec`, the support
    /// type.
    #[test]
    fn ancestry_only_abstract_support_name_renders_and_compiles() {
        let schema = preflight_fixture("backend-ancestry-only-support-name.xsd");

        // Preflight verdict and generation verdict must agree.
        assert!(
            ams_gra_oms_codegen_core::validate_backend_names(
                &schema,
                BackendLanguage::Rust,
                CLOSED,
            )
            .is_ok(),
            "an abstract base Rust never emits must not reserve BoundedVec"
        );
        let source = generate(&schema, CLOSED).expect("ancestry-only base must render");

        // The only `BoundedVec` is the generic support type; no schema-owned
        // declaration of that name is emitted.
        assert!(source.contains("pub struct BoundedVec<T"), "{source}");
        assert!(!source.contains("pub struct BoundedVec {"), "{source}");
        // The inherited field really is folded into the emitted descendant.
        assert!(source.contains("pub struct Derived {"), "{source}");
        assert!(source.contains("pub inherited:"), "{source}");

        use std::fs;
        use std::process::Command;
        let directory = std::env::temp_dir().join("ams-gra-oms-ancestry-rust-control");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create Rust probe directory");
        fs::write(directory.join("generated.rs"), &source).expect("write generated module");
        let status = Command::new("rustc")
            .current_dir(&directory)
            .args(["--edition", "2021", "--crate-type", "lib", "generated.rs"])
            .status()
            .expect("rustc should be available in a Rust workspace");
        let _ = fs::remove_dir_all(&directory);
        assert!(status.success(), "generated Rust module must compile");
    }

    /// The Task 026 counterpart: a zero-descendant target used only in a
    /// supported absent-only slot emits nothing at all, so it reserves no
    /// name either.
    #[test]
    fn task026_elided_target_does_not_reserve_its_own_name() {
        let schema = preflight_fixture("backend-elided-target-support-name.xsd");
        assert!(
            ams_gra_oms_codegen_core::validate_backend_names(
                &schema,
                BackendLanguage::Rust,
                CLOSED,
            )
            .is_ok(),
            "a Task 026 elided target must not reserve its own name"
        );
        let source = generate(&schema, CLOSED).expect("elided target must render");
        // Nothing schema-owned is emitted for the elided target, and the
        // absent-only member is not stored.
        assert!(!source.contains("OptionalString"), "{source}");
        assert!(!source.contains("pub maybe"), "{source}");
        assert!(source.contains("pub struct Holder {"), "{source}");
    }

    /// Error ownership: when emission planning fails there is no generated
    /// surface, so name preflight must defer and the semantic diagnostic must
    /// be the one the backend reports.
    ///
    /// The fixture's abstract value target is spelled `BoundedVec`, exactly
    /// the Rust support type. The previous raw-schema fallback manufactured
    /// that collision and reported it *instead of* the real open-world
    /// failure, describing output that can never exist.
    #[test]
    fn open_world_abstract_value_failure_is_not_masked_by_a_name_collision() {
        let schema = preflight_fixture("backend-open-world-abstract-value-support-name.xsd");

        // Name preflight has no opinion: there is no emitted surface.
        assert!(
            ams_gra_oms_codegen_core::validate_backend_names(&schema, BackendLanguage::Rust, OPEN)
                .is_ok(),
            "a failed emission plan must not produce a name verdict"
        );

        // Generation reports the semantic abstract-value failure verbatim.
        let message = generate(&schema, OPEN)
            .expect_err("an open-world abstract value must fail")
            .message;
        assert!(
            message.contains("open-extensions")
                && message.contains("external derived types cannot be represented"),
            "the semantic open-world diagnostic must be authoritative: {message}"
        );
        assert!(
            !message.contains("generated top-level scope"),
            "a naming diagnostic must not stand in for the semantic failure: {message}"
        );

        // Under the closed world the wrapper genuinely is emitted and really
        // does take `BoundedVec`, so the collision there is real. This is what
        // keeps the deferral from becoming a blanket exemption.
        let closed = generate(&schema, CLOSED)
            .expect_err("the emitted wrapper really does collide with the support type")
            .message;
        assert!(closed.contains("BoundedVec"), "{closed}");
    }

    /// Section 44: a schema with no abstract value reference must produce
    /// byte-identical output under both worlds.
    #[test]
    fn task028_concrete_only_output_is_byte_identical_across_worlds() {
        for schema in [track_schema(), codegen_order_schema(), inheritance_schema()] {
            assert_eq!(
                generate(&schema, CLOSED).expect("closed generation must succeed"),
                generate(&schema, OPEN).expect("open generation must succeed"),
            );
        }
    }
}
