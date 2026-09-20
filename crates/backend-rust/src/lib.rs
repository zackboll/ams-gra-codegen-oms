//! Minimal Rust type generation from normalized schema IR.

use ams_gra_oms_codegen_core::{
    AbstractValueProjection, Backend, CodegenError, EffectiveValueMember, FloatingDomain,
    GeneratedFile, GenerationWorld, InclusiveIntegralDomain, TypeEmission,
    abstract_value_projection_for_ref, effective_choice_alternatives, effective_record_fields,
    field_storage_semantics, float32_literal, float64_literal, floating_domain,
    inclusive_integral_domain, plan_type_emissions,
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
    if schema.types.iter().any(has_unbounded_occurrence) {
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
    if schema.types.iter().any(has_direct_integral_range) {
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
    let supports_eq = projection
        .concrete_descendants
        .iter()
        .all(|descendant| declaration_supports_eq(schema, descendant, &mut Vec::new()));
    writeln!(
        output,
        "#[derive(Debug, Clone, PartialEq{})]\npub enum {name} {{",
        if supports_eq { ", Eq" } else { "" }
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
                "#[derive(Debug, Clone, PartialEq{})]\npub struct {name} {{",
                if declaration_supports_eq(schema, declaration, &mut Vec::new()) {
                    ", Eq"
                } else {
                    ""
                }
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
                "#[derive(Debug, Clone, PartialEq{})]\npub enum {name} {{",
                if declaration_supports_eq(schema, declaration, &mut Vec::new()) {
                    ", Eq"
                } else {
                    ""
                }
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
    let namespace = schema
        .namespaces
        .first()
        .ok_or_else(|| error("Rust generation requires one namespace"))?;
    if schema.namespaces.len() != 1
        || schema
            .types
            .iter()
            .any(|declaration| declaration.name.namespace_uri != namespace.uri)
    {
        return unsupported("multiple namespaces".to_owned());
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
    let mut names = std::collections::BTreeSet::new();
    for alternative in alternatives {
        let name = upper_camel(&alternative.name)?;
        if !names.insert(name.clone()) {
            return unsupported(format!("duplicate Choice alternative identifier {name}"));
        }
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

fn has_unbounded_occurrence(declaration: &TypeDecl) -> bool {
    match &declaration.kind {
        TypeKind::Record { fields } => fields,
        TypeKind::Choice { alternatives } => alternatives,
        _ => return false,
    }
    .iter()
    .any(|field| matches!(field.cardinality.shape(), OccurrenceShape::Unbounded { .. }))
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

fn has_direct_integral_range(declaration: &TypeDecl) -> bool {
    let fields = match &declaration.kind {
        TypeKind::Record { fields } => fields,
        TypeKind::Choice { alternatives } => alternatives,
        _ => return false,
    };
    fields.iter().any(|field| {
        matches!(
            field.type_ref.target,
            TypeRefTarget::Primitive(PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger)
        ) && field.constraints != ConstraintSet::default()
    })
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
        assert!(
            generate(&collision, CLOSED)
                .unwrap_err()
                .message
                .contains("duplicate Choice alternative identifier Foo")
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

    #[test]
    fn lexical_constraints_fail_before_rendering() {
        for (kind, white_space) in [
            (PrimitiveKind::DateTime, false),
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
            assert!(
                error
                    .message
                    .contains("unsupported Rust IR construct: constraints on")
            );
        }
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
