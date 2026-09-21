//! Minimal C++17 type generation from normalized schema IR.

use ams_gra_oms_codegen_core::{
    AbstractValueProjection, Backend, BackendLanguage, CodegenError, EffectiveValueMember,
    FloatingDomain, GeneratedFile, GenerationWorld, InclusiveIntegralDomain, TemporalProfile,
    TypeEmission, abstract_value_projection_for_ref, backend_preflight,
    effective_choice_alternatives, effective_record_fields, field_storage_semantics,
    float32_literal, float64_literal, floating_domain, inclusive_integral_domain,
    is_temporal_primitive, plan_type_emissions, schema_emits_bounded_integer_support,
    schema_emits_temporal_carrier, schema_emits_unbounded_sequence_support, temporal_profile,
};
use ams_gra_oms_ir::{
    ConstraintSet, OccurrenceShape, PrimitiveKind, SchemaIr, TypeDecl, TypeKind, TypeRef,
    TypeRefTarget,
};
use std::fmt::Write as _;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Copy)]
pub struct CppBackend;

impl Backend for CppBackend {
    fn name(&self) -> &'static str {
        "cpp"
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
            .ok_or_else(|| error("C++ generation requires one namespace"))?;
        let stem = namespace
            .uri
            .split(|character: char| !character.is_ascii_alphanumeric())
            .rfind(|part| !part.is_empty())
            .ok_or_else(|| error("C++ generation requires a named namespace"))?;
        Ok(vec![GeneratedFile {
            relative_path: PathBuf::from(format!("{}.hpp", snake_case(stem)?)),
            contents,
        }])
    }
}

/// Generate a C++17 header from normalized schema IR.
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
    let namespace = namespace_name(schema)?;
    let variant_header = if emissions.iter().any(|emission| {
        matches!(
            emission,
            TypeEmission::AbstractValue(_)
                | TypeEmission::Declaration(TypeDecl {
                    kind: TypeKind::Choice { .. },
                    ..
                })
        )
    }) {
        "#include <variant>\n"
    } else {
        ""
    };
    let has_floating = schema_has_floating(schema);
    let limits_header = if schema_needs_limits(schema)
        || schema_emits_unbounded_sequence_support(schema)
        || has_floating
    {
        "#include <limits>\n"
    } else {
        ""
    };
    let climits_header = if has_floating {
        "#include <climits>\n"
    } else {
        ""
    };
    // Task 036: the DateTime Zulu carrier's factory takes a `std::string_view`.
    // The header is added only when a supported temporal declaration is
    // actually emitted, so every other schema's generated output is unchanged
    // byte for byte.
    let string_view_header = if schema_emits_temporal_carrier(schema) {
        "#include <string_view>\n"
    } else {
        ""
    };
    let mut output = String::from(
        "#pragma once\n\n\
         #include <cstddef>\n\
         #include <cstdint>\n\
         #include <optional>\n\
         #include <string>\n\
         #include <utility>\n\
         #include <vector>\n",
    );
    output.insert_str(
        "#pragma once\n\n".len() + "#include <cstddef>\n#include <cstdint>\n".len(),
        &format!("{limits_header}{climits_header}{string_view_header}"),
    );
    output.push_str(variant_header);
    output.push('\n');
    if has_floating {
        output.push_str(concat!(
            "static_assert(sizeof(float) * CHAR_BIT == 32 && std::numeric_limits<float>::radix == 2 && std::numeric_limits<float>::digits == 24 && std::numeric_limits<float>::min_exponent == -125 && std::numeric_limits<float>::max_exponent == 128 && std::numeric_limits<float>::is_iec559, \"OMS Float32 requires IEEE binary32 float\");\n",
            "static_assert(sizeof(double) * CHAR_BIT == 64 && std::numeric_limits<double>::radix == 2 && std::numeric_limits<double>::digits == 53 && std::numeric_limits<double>::min_exponent == -1021 && std::numeric_limits<double>::max_exponent == 1024 && std::numeric_limits<double>::is_iec559, \"OMS Float64 requires IEEE binary64 double\");\n\n",
        ));
    }
    writeln!(output, "namespace {namespace} {{\n").expect("writing to String cannot fail");
    output.push_str(concat!(
        "template <typename T, std::size_t Min, std::size_t Max>\n",
        "class BoundedVector {\n",
        "public:\n",
        "    static std::optional<BoundedVector> create(std::vector<T> values) {\n",
        "        if (values.size() < Min || values.size() > Max) {\n",
        "            return std::nullopt;\n",
        "        }\n",
        "        return BoundedVector(std::move(values));\n",
        "    }\n\n",
        "    const std::vector<T>& values() const noexcept { return values_; }\n\n",
        "private:\n",
        "    explicit BoundedVector(std::vector<T> values) : values_(std::move(values)) {}\n",
        "    std::vector<T> values_;\n",
        "};\n\n",
    ));
    if schema_emits_unbounded_sequence_support(schema) {
        output.push_str(concat!(
            "template <typename T, std::uint64_t Min>\n",
            "class UnboundedVector {\n",
            "public:\n",
            "    static std::optional<UnboundedVector> create(std::vector<T> values) {\n",
            "        if (values.size() > std::numeric_limits<std::uint64_t>::max() || static_cast<std::uint64_t>(values.size()) < Min) return std::nullopt;\n",
            "        return UnboundedVector(std::move(values));\n",
            "    }\n\n",
            "    const std::vector<T>& values() const noexcept { return values_; }\n\n",
            "private:\n",
            "    explicit UnboundedVector(std::vector<T> values) : values_(std::move(values)) {}\n",
            "    std::vector<T> values_;\n",
            "};\n\n",
        ));
    }
    if schema_emits_bounded_integer_support(schema) {
        output.push_str(concat!(
            "template <typename T, T Min, T Max>\nclass BoundedInteger {\npublic:\n",
            "    static std::optional<BoundedInteger> create(T value) noexcept {\n",
            "        if (value < Min || value > Max) return std::nullopt;\n",
            "        return BoundedInteger(value);\n    }\n",
            "    T value() const noexcept { return value_; }\nprivate:\n",
            "    explicit BoundedInteger(T value) noexcept : value_(value) {}\n    T value_;\n};\n\n",
        ));
    }
    for emission in emissions {
        match emission {
            TypeEmission::Declaration(declaration) => {
                render_declaration(&mut output, schema, declaration, world)?
            }
            TypeEmission::AbstractValue(projection) => {
                render_abstract_value(&mut output, &projection)?
            }
        }
    }
    writeln!(output, "}}  // namespace {namespace}").expect("writing to String cannot fail");
    Ok(output)
}

fn render_abstract_value(
    output: &mut String,
    projection: &AbstractValueProjection<'_>,
) -> Result<(), CodegenError> {
    let name = upper_camel(&projection.declaration.name.local_name)?;
    writeln!(output, "struct {name} {{\n    std::variant<").expect("writing to String cannot fail");
    for (index, descendant) in projection.concrete_descendants.iter().enumerate() {
        let variant = upper_camel(&descendant.name.local_name)?;
        let suffix = if index + 1 == projection.concrete_descendants.len() {
            ""
        } else {
            ","
        };
        writeln!(output, "        {variant}{suffix}").expect("writing to String cannot fail");
    }
    output.push_str("    > value;\n};\n\n");
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
            let min = cpp_signed_bound(min);
            let max = cpp_signed_bound(max);
            writeln!(
                output,
                concat!(
                    "class {name} {{\n",
                    "public:\n",
                    "    static constexpr std::int64_t min_value = {min};\n",
                    "    static constexpr std::int64_t max_value = {max};\n\n",
                    "    static std::optional<{name}> create(std::int64_t value) noexcept {{\n",
                    "        if (value < min_value || value > max_value) {{\n",
                    "            return std::nullopt;\n",
                    "        }}\n",
                    "        return {name}(value);\n",
                    "    }}\n\n",
                    "    std::int64_t value() const noexcept {{ return value_; }}\n\n",
                    "private:\n",
                    "    explicit {name}(std::int64_t value) noexcept : value_(value) {{}}\n",
                    "    std::int64_t value_;\n",
                    "}};\n",
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
            let min = cpp_unsigned_bound(min);
            let max = cpp_unsigned_bound(max);
            writeln!(output, "class {name} {{\npublic:\n    static constexpr std::uint64_t min_value = {min};\n    static constexpr std::uint64_t max_value = {max};\n\n    static std::optional<{name}> create(std::uint64_t value) noexcept {{\n        if (value < min_value || value > max_value) return std::nullopt;\n        return {name}(value);\n    }}\n\n    std::uint64_t value() const noexcept {{ return value_; }}\nprivate:\n    explicit {name}(std::uint64_t value) noexcept : value_(value) {{}}\n    std::uint64_t value_;\n}};\n").expect("writing to String cannot fail");
        }
        TypeKind::Primitive(PrimitiveKind::Boolean) => {
            reject_any_constraints(&declaration.constraints, &name)?;
            writeln!(output, "class {name} {{\npublic:\n    explicit constexpr {name}(bool value) noexcept : value_(value) {{}}\n    constexpr bool value() const noexcept {{ return value_; }}\nprivate:\n    bool value_;\n}};\n").expect("writing to String cannot fail");
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
            writeln!(output, "class {name} {{\npublic:\n    explicit {name}(std::vector<std::uint8_t> value) : value_(std::move(value)) {{}}\n    const std::vector<std::uint8_t>& value() const noexcept {{ return value_; }}\nprivate:\n    std::vector<std::uint8_t> value_;\n}};\n").expect("writing to String cannot fail");
        }
        TypeKind::Enumeration { variants } => {
            if variants.is_empty() {
                return unsupported(format!("empty enumeration {name}"));
            }
            writeln!(output, "enum class {name} {{").expect("writing to String cannot fail");
            for variant in variants {
                writeln!(output, "    {},", upper_camel(&variant.wire_value)?)
                    .expect("writing to String cannot fail");
            }
            output.push_str("};\n\n");
        }
        TypeKind::Record { .. } => {
            if declaration.is_abstract {
                return Ok(());
            }
            writeln!(output, "struct {name} {{").expect("writing to String cannot fail");
            for field in effective_record_fields(schema, &declaration.name).map_err(|_| {
                error(format!(
                    "unsupported C++ IR construct: non-Record inheritance {}",
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
                    // member is generated for it in this schema set.
                    continue;
                }
                let field_name = snake_case(&field.name)?;
                let base = cpp_field_base(field)?;
                let field_type = match field.cardinality.shape() {
                    OccurrenceShape::RequiredOne => base,
                    OccurrenceShape::OptionalOne => format!("std::optional<{base}>"),
                    OccurrenceShape::Bounded { min, max } if max > 1 => {
                        format!("BoundedVector<{base}, {min}, {max}>")
                    }
                    OccurrenceShape::Unbounded { min } => {
                        format!("UnboundedVector<{base}, {}>", cpp_unsigned_bound(min))
                    }
                    _ => return unsupported(format!("cardinality on field {field_name}")),
                };
                writeln!(output, "    {field_type} {field_name};")
                    .expect("writing to String cannot fail");
            }
            output.push_str("};\n\n");
        }
        TypeKind::Choice { .. } => {
            writeln!(output, "struct {name} {{").expect("writing to String cannot fail");
            let alternatives = effective_choice_alternatives(schema, &declaration.name).map_err(
                |projection_error| {
                    error(format!("unsupported C++ IR construct: {projection_error}"))
                },
            )?;
            for alternative in &alternatives {
                let alternative_name = upper_camel(&alternative.name)?;
                writeln!(
                    output,
                    "    struct {alternative_name} {{ {} value; }};",
                    cpp_field_type(alternative)?
                )
                .expect("writing to String cannot fail");
            }
            writeln!(
                output,
                "\n    std::variant<{}> value;\n}};\n",
                alternatives
                    .iter()
                    .map(|alternative| upper_camel(&alternative.name))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            )
            .expect("writing to String cannot fail");
        }
        other => return unsupported(format!("type {name}: {other:?}")),
    }
    Ok(())
}

fn validate_schema(schema: &SchemaIr, world: GenerationWorld) -> Result<(), CodegenError> {
    // Shared global preflight: the single-namespace boundary and generated
    // host-language name safety. Capability/readiness analysis consults the
    // same rules, so a READY verdict cannot disagree with what happens here.
    if let Err(preflight) = backend_preflight(schema, BackendLanguage::Cpp, world) {
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
        // Task 033: the shared helper classifies a named floating
        // declaration's effective constraints. The bound-only subset is
        // lowered; every other facet shape still fails closed here.
        if let TypeKind::Primitive(kind @ (PrimitiveKind::Float32 | PrimitiveKind::Float64)) =
            declaration.kind
        {
            floating_domain(kind, &declaration.constraints).map_err(|reason| {
                error(format!(
                    "unsupported C++ IR construct: {reason} on {}",
                    declaration.name.local_name
                ))
            })?;
            // `reject_extra_constraints` understands only the integral
            // inclusive subset, so a legitimate exclusive floating bound must
            // not reach it.
            continue;
        }
        // Task 036: a named temporal declaration is classified by the shared
        // helper. Only the DateTime Zulu profile is lowered; every other
        // temporal shape still fails closed here, before any output exists.
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
            // The supported profile's `.+Z` pattern is a real lexical
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
                    "unsupported C++ IR construct: non-Record inheritance {}",
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
                    error(format!("unsupported C++ IR construct: {projection_error}"))
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
    // keeping a second C++-local copy would let the two drift.
    for alternative in alternatives {
        if alternative.nillable {
            return unsupported(format!("nillable Choice alternative {}", alternative.name));
        }
        validate_abstract_value_reference(schema, &alternative.type_ref, world)?;
        cpp_field_type(alternative)?;
    }
    Ok(())
}

fn cpp_field_type(field: &ams_gra_oms_ir::FieldDecl) -> Result<String, CodegenError> {
    let base = cpp_field_base(field)?;
    match field.cardinality.shape() {
        OccurrenceShape::RequiredOne => Ok(base),
        OccurrenceShape::OptionalOne => Ok(format!("std::optional<{base}>")),
        OccurrenceShape::Bounded { min, max } if max > 1 => {
            Ok(format!("BoundedVector<{base}, {min}, {max}>"))
        }
        OccurrenceShape::Unbounded { min } => Ok(format!(
            "UnboundedVector<{base}, {}>",
            cpp_unsigned_bound(min)
        )),
        _ => unsupported(format!("cardinality on Choice alternative {}", field.name)),
    }
}

fn namespace_name(schema: &SchemaIr) -> Result<String, CodegenError> {
    let uri = &schema
        .namespaces
        .first()
        .ok_or_else(|| error("C++ generation requires one namespace"))?
        .uri;
    let parts = uri
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return unsupported(format!("namespace URI {uri}"));
    }
    Ok(format!(
        "{}::{}",
        snake_case(parts[parts.len() - 2])?,
        snake_case(parts[parts.len() - 1])?
    ))
}

fn cpp_type(type_ref: &TypeRef) -> Result<String, CodegenError> {
    match &type_ref.target {
        TypeRefTarget::Primitive(PrimitiveKind::SignedInteger) => Ok("std::int64_t".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::UnsignedInteger) => Ok("std::uint64_t".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::Boolean) => Ok("bool".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::Float32) => Ok("float".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::Float64) => Ok("double".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::String) => Ok("std::string".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::Binary) => {
            Ok("std::vector<std::uint8_t>".to_owned())
        }
        TypeRefTarget::Named(name) => upper_camel(&name.local_name),
        other => unsupported(format!("type reference {other:?}")),
    }
}

fn schema_has_floating(schema: &SchemaIr) -> bool {
    schema.types.iter().any(|declaration| {
        matches!(
            declaration.kind,
            TypeKind::Primitive(PrimitiveKind::Float32 | PrimitiveKind::Float64)
        )
    }) || schema
        .types
        .iter()
        .any(|declaration| match &declaration.kind {
            TypeKind::Record { fields }
            | TypeKind::Choice {
                alternatives: fields,
            } => fields.iter().any(|field| {
                matches!(
                    field.type_ref.target,
                    TypeRefTarget::Primitive(PrimitiveKind::Float32 | PrimitiveKind::Float64)
                )
            }),
            _ => false,
        })
}

/// Render a named Float32/Float64 declaration as an owning value wrapper.
///
/// An unconstrained declaration keeps its exact Task 022 form: a public
/// explicit constructor, because every `float`/`double` is a legal instance and
/// construction cannot fail. A Task 033 bound-only declaration instead exposes
/// only a checked `create` returning `std::optional`, with the constructor made
/// private -- an unchecked public constructor would let a caller build a value
/// outside the schema's domain, which is exactly what the constraint forbids.
///
/// Bound comparisons are emitted verbatim, so NaN fails every clause, a
/// one-sided infinity keeps IEEE ordering, and both zero signs compare equal.
/// Nothing beyond `<optional>` is required, which the header already includes.
fn render_floating_declaration(
    output: &mut String,
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
    name: &str,
) -> Result<(), CodegenError> {
    let scalar = if kind == PrimitiveKind::Float32 {
        "float"
    } else {
        "double"
    };
    let Some(domain) = floating_domain(kind, constraints)
        .map_err(|reason| error(format!("unsupported C++ IR construct: {reason} on {name}")))?
    else {
        // Task 022 output, unchanged byte for byte.
        writeln!(output, "class {name} {{\npublic:\n    explicit {name}({scalar} value) noexcept : value_(value) {{}}\n    {scalar} value() const noexcept {{ return value_; }}\nprivate:\n    {scalar} value_;\n}};\n").expect("writing to String cannot fail");
        return Ok(());
    };

    let mut clauses = Vec::new();
    let mut constants = String::new();
    match domain {
        FloatingDomain::Float32 { lower, upper } => {
            if let Some(bound) = lower {
                writeln!(
                    constants,
                    "    static constexpr float min_value = {}f;",
                    float32_literal(bound.value.value())
                )
                .expect("writing to String cannot fail");
                clauses.push(format!("value {} min_value", bound.kind.lower_operator()));
            }
            if let Some(bound) = upper {
                writeln!(
                    constants,
                    "    static constexpr float max_value = {}f;",
                    float32_literal(bound.value.value())
                )
                .expect("writing to String cannot fail");
                clauses.push(format!("value {} max_value", bound.kind.upper_operator()));
            }
        }
        FloatingDomain::Float64 { lower, upper } => {
            if let Some(bound) = lower {
                writeln!(
                    constants,
                    "    static constexpr double min_value = {};",
                    float64_literal(bound.value.value())
                )
                .expect("writing to String cannot fail");
                clauses.push(format!("value {} min_value", bound.kind.lower_operator()));
            }
            if let Some(bound) = upper {
                writeln!(
                    constants,
                    "    static constexpr double max_value = {};",
                    float64_literal(bound.value.value())
                )
                .expect("writing to String cannot fail");
                clauses.push(format!("value {} max_value", bound.kind.upper_operator()));
            }
        }
    }

    writeln!(
        output,
        concat!(
            "class {name} {{\n",
            "public:\n",
            "{constants}\n",
            "    static std::optional<{name}> create({scalar} value) noexcept {{\n",
            "        if ({condition}) {{\n",
            "            return {name}(value);\n",
            "        }}\n",
            "        return std::nullopt;\n",
            "    }}\n\n",
            "    {scalar} value() const noexcept {{ return value_; }}\n\n",
            "private:\n",
            "    explicit {name}({scalar} value) noexcept : value_(value) {{}}\n",
            "    {scalar} value_;\n",
            "}};\n",
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
        .map_err(|reason| error(format!("unsupported C++ IR construct: {reason} on {name}")))
}

fn cpp_field_base(field: &ams_gra_oms_ir::FieldDecl) -> Result<String, CodegenError> {
    match field.type_ref.target {
        TypeRefTarget::Primitive(
            kind @ (PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger),
        ) => match integral_domain(kind, &field.constraints, &field.name)? {
            Some(InclusiveIntegralDomain::Signed { min, max }) => Ok(format!(
                "BoundedInteger<std::int64_t, {}, {}>",
                cpp_signed_bound(min),
                cpp_signed_bound(max)
            )),
            Some(InclusiveIntegralDomain::Unsigned { min, max }) => Ok(format!(
                "BoundedInteger<std::uint64_t, {}, {}>",
                cpp_unsigned_bound(min),
                cpp_unsigned_bound(max)
            )),
            None => cpp_type(&field.type_ref),
        },
        _ => {
            reject_any_constraints(&field.constraints, &field.name)?;
            cpp_type(&field.type_ref)
        }
    }
}

fn cpp_signed_bound(value: i64) -> String {
    if value == i64::MIN {
        "std::numeric_limits<std::int64_t>::min()".to_owned()
    } else {
        value.to_string()
    }
}

fn cpp_unsigned_bound(value: u64) -> String {
    if value == u64::MAX {
        "std::numeric_limits<std::uint64_t>::max()".to_owned()
    } else {
        value.to_string()
    }
}

fn schema_needs_limits(schema: &SchemaIr) -> bool {
    schema.types.iter().any(|declaration| {
        let declaration_needs_limits = match declaration.kind {
            TypeKind::Primitive(
                kind @ (PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger),
            ) => inclusive_integral_domain(kind, &declaration.constraints)
                .ok()
                .flatten()
                .is_some_and(domain_needs_limits),
            _ => false,
        };
        declaration_needs_limits
            || match &declaration.kind {
                TypeKind::Record { fields } => fields,
                TypeKind::Choice { alternatives } => alternatives,
                _ => return false,
            }
            .iter()
            .any(|field| integral_domain_for_limits(field).is_some_and(domain_needs_limits))
    })
}

fn domain_needs_limits(domain: InclusiveIntegralDomain) -> bool {
    matches!(
        domain,
        InclusiveIntegralDomain::Signed { min: i64::MIN, .. }
            | InclusiveIntegralDomain::Unsigned { max: u64::MAX, .. }
    )
}

fn integral_domain_for_limits(
    field: &ams_gra_oms_ir::FieldDecl,
) -> Option<InclusiveIntegralDomain> {
    match field.type_ref.target {
        TypeRefTarget::Primitive(
            kind @ (PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger),
        ) => inclusive_integral_domain(kind, &field.constraints)
            .ok()
            .flatten(),
        _ => None,
    }
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

fn reject_any_constraints(constraints: &ConstraintSet, name: &str) -> Result<(), CodegenError> {
    if constraints != &ConstraintSet::default() {
        return unsupported(format!("field constraints on {name}"));
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
    Err(error(format!("unsupported C++ IR construct: {construct}")))
}

fn error(message: impl Into<String>) -> CodegenError {
    CodegenError {
        message: message.into(),
    }
}

/// Render a named temporal declaration.
///
/// Only the shared classifier's one supported profile is lowered; `Time`,
/// `Duration`, an unconstrained `DateTime`, and every other facet shape fail
/// closed rather than being approximated.
///
/// # Representation
///
/// A **validated lexical carrier** holding the whitespace-normalized XML
/// Schema `dateTime` spelling. Deliberately not `std::chrono`, `time_t`, or
/// epoch seconds: each narrows XML Schema's lexical and value space (unbounded
/// year digits, arbitrary fractional precision, the distinct `24:00:00`
/// spelling) and discards wire-level information a codec will need.
///
/// # No comparison operators
///
/// No `operator==`, `operator<`, `operator<=>`, or ordering helper is emitted.
/// XML Schema `dateTime` equality is value-space equality over spellings, and
/// its order relation is only partial (section 3.2.7.4). Task 036 implements
/// neither, so it declares neither.
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
        CPP_DATE_TIME_ZULU_TEMPLATE.replace("{name}", name)
    )
    .expect("writing to String cannot fail");
    Ok(())
}

/// The generated C++17 DateTime Zulu carrier, with `{name}` substituted.
///
/// # Validation order
///
/// 1. XML Schema `collapse` normalization (section 4.3.6), fixed for
///    `dateTime`;
/// 2. the full `dateTime` lexical grammar and calendar rules (3.2.7.1);
/// 3. the UCI `.+Z` Zulu restriction.
///
/// A bare `ends_with('Z')` would accept `garbageZ`, so the base grammar is
/// checked first and Zulu is the last gate, not the only one.
///
/// # No fixed-width year
///
/// The year is validated and its leap-year properties computed from decimal
/// digits, never parsed into `int`/`long`/`time_t`. XML Schema admits a
/// four-or-more digit year with no upper bound.
///
/// # Scope of the helpers
///
/// Every parser helper is a **private static member function**, so it lives in
/// this class's own scope and cannot collide with a schema-generated
/// namespace-scope identifier. Task 036 adds no new namespace-scope name.
///
/// # Strictness
///
/// Compiles clean under `-std=c++17 -Wall -Wextra -pedantic-errors` and uses
/// only `<string>`, `<string_view>`, and `<optional>`, which the generated
/// header already includes. No external temporal or regex library.
const CPP_DATE_TIME_ZULU_TEMPLATE: &str = r##"class {name} {
public:
    // Validate `value` as an XML Schema dateTime restricted to Zulu.
    //
    // The input is normalized under the fixed `collapse` whiteSpace policy,
    // then checked against the full dateTime lexical grammar and calendar
    // rules, then against the `.+Z` restriction. Returns std::nullopt if any
    // step rejects. The stored text is the normalized form.
    static std::optional<{name}> create(std::string_view value) {
        std::string lexical = collapse(value);
        if (!is_zulu_date_time(lexical)) {
            return std::nullopt;
        }
        return {name}(std::move(lexical));
    }

    // The stored, normalized, validated lexical representation. This is a
    // dateTime spelling whose timezone is 'Z'; it is not a point in time.
    const std::string& value() const noexcept { return value_; }

private:
    explicit {name}(std::string normalized) : value_(std::move(normalized)) {}

    // XML Schema `collapse`: tab/LF/CR become spaces, runs of spaces are
    // squeezed to one, and leading/trailing spaces are removed.
    static std::string collapse(std::string_view value) {
        std::string out;
        bool pending_space = false;
        for (char character : value) {
            if (character == ' ' || character == '\t' || character == '\n' || character == '\r') {
                pending_space = !out.empty();
                continue;
            }
            if (pending_space) {
                out.push_back(' ');
                pending_space = false;
            }
            out.push_back(character);
        }
        return out;
    }

    static bool is_digit(char character) noexcept {
        return character >= '0' && character <= '9';
    }

    // The whole gate: valid lexical dateTime AND Zulu timezone.
    static bool is_zulu_date_time(const std::string& text) {
        // The Zulu restriction, applied to the normalized form -- which is
        // exactly where XML Schema applies a pattern facet.
        if (text.size() < 2 || text.back() != 'Z') {
            return false;
        }
        // The base grammar must hold for everything before the timezone; this
        // is what stops "garbageZ" from being accepted.
        return is_date_time_body(std::string_view(text).substr(0, text.size() - 1));
    }

    // '-'? yyyy '-' mm '-' dd 'T' hh ':' mm ':' ss ('.' s+)?
    static bool is_date_time_body(std::string_view body) {
        std::size_t year_end = 0;
        bool leap = false;
        if (!scan_year(body, year_end, leap)) {
            return false;
        }
        std::string_view rest = body.substr(year_end);
        if (rest.size() < 6 || rest[0] != '-' || rest[3] != '-') {
            return false;
        }
        unsigned month = 0;
        unsigned day = 0;
        if (!two_digits(rest.substr(1, 2), month) || !two_digits(rest.substr(4, 2), day)) {
            return false;
        }
        if (month < 1 || month > 12 || day < 1 || day > days_in_month(month, leap)) {
            return false;
        }
        return is_time_of_day(rest.substr(6));
    }

    // 'T' hh ':' mm ':' ss ('.' s+)?
    static bool is_time_of_day(std::string_view rest) {
        if (rest.size() < 9 || rest[0] != 'T' || rest[3] != ':' || rest[6] != ':') {
            return false;
        }
        unsigned hour = 0;
        unsigned minute = 0;
        unsigned second = 0;
        if (!two_digits(rest.substr(1, 2), hour) || !two_digits(rest.substr(4, 2), minute)
            || !two_digits(rest.substr(7, 2), second)) {
            return false;
        }
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
        if (hour > 24 || minute > 59 || second > 60) {
            return false;
        }
        std::string_view fraction = rest.substr(9);
        bool fraction_is_zero = true;
        if (!fraction.empty()) {
            // '.' s+ : the dot requires at least one digit after it.
            if (fraction[0] != '.' || fraction.size() < 2) {
                return false;
            }
            for (std::size_t index = 1; index < fraction.size(); ++index) {
                if (!is_digit(fraction[index])) {
                    return false;
                }
                if (fraction[index] != '0') {
                    fraction_is_zero = false;
                }
            }
        }
        // Hour 24 is legal only as the exact instant 24:00:00(.0*).
        if (hour == 24 && (minute != 0 || second != 0 || !fraction_is_zero)) {
            return false;
        }
        return true;
    }

    // Validate '-'? yyyy, reporting where it ends and whether it is a leap
    // year. The digit count is unbounded, so nothing is parsed into an
    // integer; divisibility is decided from trailing digits only.
    static bool scan_year(std::string_view body, std::size_t& year_end, bool& leap) {
        std::size_t start = (!body.empty() && body[0] == '-') ? 1u : 0u;
        std::size_t end = start;
        while (end < body.size() && is_digit(body[end])) {
            ++end;
        }
        std::string_view digits = body.substr(start, end - start);
        // Four-or-more digits.
        if (digits.size() < 4) {
            return false;
        }
        // If more than four digits, leading zeros are prohibited.
        if (digits.size() > 4 && digits[0] == '0') {
            return false;
        }
        // '0000' is not a valid lexical representation in XML Schema 1.0,
        // with or without a sign.
        bool all_zero = true;
        for (char digit : digits) {
            if (digit != '0') {
                all_zero = false;
                break;
            }
        }
        if (all_zero) {
            return false;
        }
        year_end = end;
        leap = is_leap_year(digits);
        return true;
    }

    // Leap year from decimal digits: divisible by 400, or by 4 but not 100.
    // Divisibility by 4 depends only on the last two digits (100 is itself a
    // multiple of 4), so this is exact for any digit count.
    static bool is_leap_year(std::string_view digits) {
        unsigned last_two = static_cast<unsigned>(digits[digits.size() - 2] - '0') * 10u
            + static_cast<unsigned>(digits[digits.size() - 1] - '0');
        bool divisible_by_4 = (last_two % 4u) == 0u;
        bool divisible_by_100 = last_two == 0u;
        bool divisible_by_400 = divisible_by_100 && hundreds_multiple_of_four(digits);
        return divisible_by_400 || (divisible_by_4 && !divisible_by_100);
    }

    // Whether a year ending in "00" is also divisible by 400: strip the
    // trailing "00" and test the remainder for divisibility by 4, digit by
    // digit so an arbitrarily long year stays exact.
    static bool hundreds_multiple_of_four(std::string_view digits) {
        unsigned remainder = 0;
        for (std::size_t index = 0; index + 2 < digits.size(); ++index) {
            remainder = (remainder * 10u + static_cast<unsigned>(digits[index] - '0')) % 4u;
        }
        return remainder == 0u;
    }

    // Exactly two ASCII digits, as a number.
    static bool two_digits(std::string_view pair, unsigned& out) {
        if (pair.size() != 2 || !is_digit(pair[0]) || !is_digit(pair[1])) {
            return false;
        }
        out = static_cast<unsigned>(pair[0] - '0') * 10u + static_cast<unsigned>(pair[1] - '0');
        return true;
    }

    // maximumDayInMonthFor, XML Schema 1.0 Part 2 Appendix E.
    static unsigned days_in_month(unsigned month, bool leap) {
        switch (month) {
        case 1: case 3: case 5: case 7: case 8: case 10: case 12:
            return 31;
        case 4: case 6: case 9: case 11:
            return 30;
        case 2:
            return leap ? 29u : 28u;
        default:
            return 0;
        }
    }

    std::string value_;
};
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
    use std::fs;
    use std::path::Path;
    use std::process::Command;

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

    fn float_only_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-float-only.xsd"),
        )
        .expect("float-only fixture should parse")
    }

    fn binary_only_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-binary-only.xsd"),
        )
        .expect("binary-only fixture should parse")
    }

    fn inheritance_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-record-inheritance.xsd"),
        )
        .expect("inheritance fixture should parse")
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

    fn constrained_floating_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-constrained-floating.xsd"),
        )
        .expect("constrained floating fixture should parse")
    }

    /// Task 033 sections 23--26: compile the generated header under strict
    /// C++17 and exercise every bound shape, including the IEEE special
    /// values, at runtime.
    #[test]
    fn constrained_floats_enforce_bounds_under_strict_cpp17() {
        let source =
            generate(&constrained_floating_schema(), CLOSED).expect("fixture must generate");

        // Width correctness, and the Task 022 host guards, are structural.
        assert!(source.contains("static constexpr float min_value = 0.0f;"));
        assert!(source.contains("static constexpr double min_value = -6378237.0;"));
        assert!(source.contains("OMS Float32 requires IEEE binary32 float"));
        assert!(source.contains("OMS Float64 requires IEEE binary64 double"));
        // The named chain enforces its effective domain.
        let derived = source
            .split("class DerivedFloat {")
            .nth(1)
            .expect("DerivedFloat must be generated");
        assert!(derived.contains("static constexpr double min_value = 0.0;"));
        assert!(derived.contains("static constexpr double max_value = 10.0;"));

        use std::fs;
        use std::process::Command;
        let directory = std::env::temp_dir().join("ams-gra-oms-task033-cpp-bounds");
        fs::create_dir_all(&directory).expect("create C++ probe directory");
        fs::write(directory.join("generated.hpp"), source).expect("write generated header");
        fs::write(directory.join("probe.cpp"), CPP_BOUNDS_PROBE).expect("write C++ probe");

        let status = Command::new("c++")
            .current_dir(&directory)
            .args([
                "-std=c++17",
                "-Wall",
                "-Wextra",
                "-pedantic-errors",
                "-o",
                "probe",
                "probe.cpp",
            ])
            .status()
            .expect("C++ compiler must be available");
        assert!(status.success(), "strict C++17 compile must succeed");
        let run = Command::new(directory.join("probe"))
            .status()
            .expect("compiled probe must run");
        fs::remove_dir_all(&directory).expect("remove C++ probe directory");
        assert!(run.success(), "generated bound checks must hold at runtime");
    }

    /// The runtime assertions compiled against the generated header above.
    ///
    /// Infinity and NaN are produced here, in the *test*, via
    /// `std::numeric_limits`; no such dependency is added to generated code.
    const CPP_BOUNDS_PROBE: &str = r#"
#include "generated.hpp"

#include <cassert>
#include <limits>

using namespace constrained::floating;

int main() {
    const double inf = std::numeric_limits<double>::infinity();
    const double nan = std::numeric_limits<double>::quiet_NaN();
    const float finf = std::numeric_limits<float>::infinity();
    const float fnan = std::numeric_limits<float>::quiet_NaN();

    // Lower inclusive: the bound itself is admitted.
    assert(FloatLowerInclusive::create(0.0f).has_value());
    assert(!FloatLowerInclusive::create(-0.1f).has_value());

    // Upper inclusive.
    assert(FloatUpperInclusive::create(1.0f).has_value());
    assert(!FloatUpperInclusive::create(1.000001f).has_value());

    // Two-sided, and the accessor round-trips the stored value.
    assert(FloatUnitInterval::create(0.5f).has_value());
    assert(FloatUnitInterval::create(0.25f)->value() == 0.25f);
    assert(!FloatUnitInterval::create(-0.001f).has_value());
    assert(!FloatUnitInterval::create(1.001f).has_value());

    // Lower exclusive rejects its own bound.
    assert(!DoublePositive::create(0.0).has_value());
    assert(DoublePositive::create(std::numeric_limits<double>::min()).has_value());

    // Upper exclusive.
    assert(!DoubleUpperExclusive::create(1.0).has_value());
    assert(DoubleUpperExclusive::create(0.999).has_value());

    // Mixed two-sided: inclusive lower, exclusive upper.
    assert(DoubleMixedRange::create(-3.5).has_value());
    assert(!DoubleMixedRange::create(3.5).has_value());

    // The authoritative AltitudeType bound at full binary64 precision.
    assert(DoubleAltitude::create(-6378237.0).has_value());
    assert(!DoubleAltitude::create(-6378237.5).has_value());

    // Named chain: the effective inherited domain is enforced.
    assert(DerivedFloat::create(0.0).has_value());
    assert(DerivedFloat::create(10.0).has_value());
    assert(!DerivedFloat::create(-0.5).has_value());
    assert(!DerivedFloat::create(10.5).has_value());

    // NaN is an instance of no range-constrained type.
    assert(!FloatUnitInterval::create(fnan).has_value());
    assert(!DoubleAltitude::create(nan).has_value());
    assert(!DoublePositive::create(nan).has_value());
    assert(!DoubleUpperExclusive::create(nan).has_value());

    // One-sided infinity keeps IEEE ordering.
    assert(DoubleAltitude::create(inf).has_value());
    assert(!DoubleAltitude::create(-inf).has_value());
    assert(DoubleUpperExclusive::create(-inf).has_value());
    assert(!DoubleUpperExclusive::create(inf).has_value());
    assert(FloatLowerInclusive::create(finf).has_value());
    // A two-sided finite range excludes both.
    assert(!DoubleMixedRange::create(inf).has_value());
    assert(!DoubleMixedRange::create(-inf).has_value());

    // Signed zero compares equal.
    assert(FloatLowerInclusive::create(-0.0f).has_value());
    assert(!DoublePositive::create(-0.0).has_value());
    assert(!DoublePositive::create(0.0).has_value());
    return 0;
}
"#;

    #[test]
    fn lowers_unbounded_records_choices_and_constrained_elements() {
        let source =
            generate(&unbounded_schema(), CLOSED).expect("unbounded cardinality should generate");
        assert!(source.contains("class UnboundedVector"));
        assert!(source.contains("UnboundedVector<Item, 0>"));
        assert!(source.contains("UnboundedVector<Item, 1>"));
        assert!(source.contains("UnboundedVector<Item, 2>"));
        assert!(source.contains("UnboundedVector<BoundedInteger<std::uint64_t, 0, 255>, 0>"));
        assert!(source.contains(
            "struct ManyByte { UnboundedVector<BoundedInteger<std::uint64_t, 0, 255>, 1> value; };"
        ));
    }

    #[test]
    fn renders_and_strictly_compiles_full_unbounded_minimum() {
        let mut schema = unbounded_schema();
        for declaration in &mut schema.types {
            match &mut declaration.kind {
                TypeKind::Record { fields } if declaration.name.local_name == "Record" => {
                    fields[1].cardinality.min_occurs = u64::MAX;
                }
                TypeKind::Choice { alternatives } if declaration.name.local_name == "Selection" => {
                    alternatives[1].cardinality.min_occurs = u64::MAX;
                }
                _ => {}
            }
        }
        let source = generate(&schema, CLOSED).expect("full unbounded minimum should generate");
        assert!(
            source.contains("UnboundedVector<Item, std::numeric_limits<std::uint64_t>::max()>")
        );
        assert!(!source.contains("UnboundedVector<Item, 18446744073709551615>"));
        assert!(source.contains("#include <limits>"));

        use std::fs;
        use std::process::Command;
        let directory = std::env::temp_dir().join("ams-gra-oms-full-unbounded-minimum");
        let header = directory.join("full_unbounded.hpp");
        let unit = directory.join("full_unbounded.cpp");
        fs::create_dir_all(&directory).expect("create C++ probe directory");
        fs::write(&header, source).expect("write generated C++ header");
        fs::write(&unit, "#include \"full_unbounded.hpp\"\n").expect("write C++ unit");
        let status = Command::new("c++")
            .args([
                "-std=c++17",
                "-Wall",
                "-Wextra",
                "-pedantic-errors",
                "-fsyntax-only",
            ])
            .arg(&unit)
            .status()
            .expect("C++ compiler must be available");
        fs::remove_dir_all(&directory).expect("remove C++ probe directory");
        assert!(
            status.success(),
            "strict C++17 full-minimum compile must succeed"
        );
    }

    fn full_integral_boundary_schema() -> SchemaIr {
        let mut schema = integral_schema();
        for declaration in &mut schema.types {
            if declaration.name.local_name == "Signed_Bounded" {
                declaration.constraints =
                    integer_bounds(i128::from(i64::MIN), i128::from(i64::MAX));
            }
            if declaration.name.local_name == "Unsigned_Bounded" {
                declaration.constraints = integer_bounds(0, i128::from(u64::MAX));
            }
            if let TypeKind::Record { fields } = &mut declaration.kind {
                for field in fields {
                    if field.name == "Byte_Value" {
                        field.constraints =
                            integer_bounds(i128::from(i64::MIN), i128::from(i64::MAX));
                    }
                    if field.name == "Unsigned_Byte_Value" {
                        field.constraints = integer_bounds(0, i128::from(u64::MAX));
                    }
                }
            }
        }
        schema
    }

    fn integer_bounds(min: i128, max: i128) -> ConstraintSet {
        ConstraintSet {
            min_inclusive: Some(NumericValue::Integer(min)),
            max_inclusive: Some(NumericValue::Integer(max)),
            ..ConstraintSet::default()
        }
    }

    #[test]
    fn lowers_integral_scalars_and_preserves_direct_ranges() {
        let source =
            generate(&integral_schema(), CLOSED).expect("integral scalars should generate");
        assert!(source.contains("class BoundedInteger"));
        assert!(source.contains("bool enabled;"));
        assert!(source.contains("BoundedInteger<std::int64_t, -128, 127> byte_value;"));
        assert!(source.contains("BoundedInteger<std::uint64_t, 0, 255> unsigned_byte_value;"));
        assert!(source.contains("BoundedVector<BoundedInteger<std::int64_t, -128, 127>, 0, 8>"));
        assert!(source.contains("struct TrueCase { bool value; };"));
        assert!(source.contains("class UnsignedBounded"));
        assert!(source.contains("class NamedBoolean"));
    }

    #[test]
    fn renders_and_strictly_compiles_full_integral_boundaries() {
        let source = generate(&full_integral_boundary_schema(), CLOSED)
            .expect("full i64/u64 domains should generate");
        assert!(source.contains("#include <limits>"));
        assert!(source.contains(
            "static constexpr std::int64_t min_value = std::numeric_limits<std::int64_t>::min();"
        ));
        assert!(source.contains("static constexpr std::int64_t max_value = 9223372036854775807;"));
        assert!(source.contains("static constexpr std::uint64_t min_value = 0;"));
        assert!(source.contains(
            "static constexpr std::uint64_t max_value = std::numeric_limits<std::uint64_t>::max();"
        ));
        assert!(source.contains("BoundedInteger<std::int64_t, std::numeric_limits<std::int64_t>::min(), 9223372036854775807> byte_value;"));
        assert!(source.contains("BoundedInteger<std::uint64_t, 0, std::numeric_limits<std::uint64_t>::max()> unsigned_byte_value;"));
        assert!(!source.contains("-9223372036854775808"));
        assert!(!source.contains("18446744073709551615"));

        let header_path = std::env::temp_dir().join(format!(
            "ams-gra-oms-full-integral-boundaries-{}.hpp",
            std::process::id()
        ));
        let source_path = header_path.with_extension("cpp");
        fs::write(&header_path, source).expect("write generated C++ header");
        fs::write(
            &source_path,
            format!("#include \"{}\"\n", header_path.display()),
        )
        .expect("write C++ translation unit");
        let status = Command::new("c++")
            .args([
                "-std=c++17",
                "-Wall",
                "-Wextra",
                "-pedantic-errors",
                "-fsyntax-only",
            ])
            .arg(&source_path)
            .status()
            .expect("C++ compiler must be available");
        fs::remove_file(&header_path).expect("remove generated C++ header");
        fs::remove_file(&source_path).expect("remove C++ translation unit");
        assert!(
            status.success(),
            "strict C++17 full-bound compile must succeed"
        );
    }

    #[test]
    fn float_only_schema_includes_and_compiles_host_guards() {
        let source =
            generate(&float_only_schema(), CLOSED).expect("float-only schema should generate");
        assert!(source.contains("#include <limits>"));
        assert!(source.contains("#include <climits>"));
        assert!(source.contains("sizeof(float) * CHAR_BIT == 32"));
        assert!(source.contains("std::numeric_limits<float>::min_exponent == -125"));
        assert!(source.contains("sizeof(double) * CHAR_BIT == 64"));
        assert!(source.contains("std::numeric_limits<double>::max_exponent == 1024"));

        let header_path =
            std::env::temp_dir().join(format!("ams-gra-oms-float-only-{}.hpp", std::process::id()));
        let source_path = header_path.with_extension("cpp");
        fs::write(&header_path, source).expect("write generated C++ header");
        fs::write(
            &source_path,
            format!("#include \"{}\"\n", header_path.display()),
        )
        .expect("write C++ translation unit");
        let status = Command::new("c++")
            .args([
                "-std=c++17",
                "-Wall",
                "-Wextra",
                "-pedantic-errors",
                "-fsyntax-only",
            ])
            .arg(&source_path)
            .status()
            .expect("C++ compiler must be available");
        fs::remove_file(&header_path).expect("remove generated C++ header");
        fs::remove_file(&source_path).expect("remove C++ translation unit");
        assert!(
            status.success(),
            "strict C++17 float-only compile must succeed"
        );
    }

    /// Generated-name preflight tracks *emitted* entities, not raw Schema IR
    /// declarations. An ancestry-only abstract Record is folded into its
    /// descendants and never emitted, so its local name does not occupy the
    /// generated scope even when it is `BoundedVector`, which backend-cpp
    /// always emits as a support type.
    #[test]
    fn ancestry_only_abstract_support_name_renders_and_strictly_compiles() {
        let schema = load_schema_document(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-ancestry-only-support-name.xsd"),
        )
        .expect("ancestry-only fixture should parse");
        assert!(
            ams_gra_oms_codegen_core::validate_backend_names(&schema, BackendLanguage::Cpp, CLOSED)
                .is_ok(),
            "an abstract base C++ never emits must not reserve BoundedVector"
        );
        let source = generate(&schema, CLOSED).expect("ancestry-only base must render");
        // The support template is present as a `class`; no schema-owned
        // `struct` takes the name.
        assert!(source.contains("class BoundedVector {"), "{source}");
        assert!(!source.contains("struct BoundedVector"), "{source}");
        assert!(source.contains("struct Derived {"), "{source}");

        let header_path = std::env::temp_dir().join(format!(
            "ams-gra-oms-ancestry-cpp-{}.hpp",
            std::process::id()
        ));
        let source_path = header_path.with_extension("cpp");
        fs::write(&header_path, source).expect("write generated C++ header");
        fs::write(
            &source_path,
            format!("#include \"{}\"\n", header_path.display()),
        )
        .expect("write C++ translation unit");
        let status = Command::new("c++")
            .args([
                "-std=c++17",
                "-Wall",
                "-Wextra",
                "-pedantic-errors",
                "-fsyntax-only",
            ])
            .arg(&source_path)
            .status()
            .expect("C++ compiler must be available");
        fs::remove_file(&header_path).expect("remove generated C++ header");
        fs::remove_file(&source_path).expect("remove C++ translation unit");
        assert!(
            status.success(),
            "strict C++17 ancestry-only compile must succeed"
        );
    }

    #[test]
    fn binary_only_schema_includes_and_strictly_compiles() {
        let source =
            generate(&binary_only_schema(), CLOSED).expect("binary-only schema should generate");
        assert!(source.contains("#include <cstdint>"));
        assert!(source.contains("#include <vector>"));
        assert!(source.contains("std::vector<std::uint8_t> payload;"));
        assert!(!source.contains("#include <climits>"));

        let header_path = std::env::temp_dir().join(format!(
            "ams-gra-oms-binary-only-{}.hpp",
            std::process::id()
        ));
        let source_path = header_path.with_extension("cpp");
        fs::write(&header_path, source).expect("write generated C++ header");
        fs::write(
            &source_path,
            format!("#include \"{}\"\n", header_path.display()),
        )
        .expect("write C++ translation unit");
        let status = Command::new("c++")
            .args([
                "-std=c++17",
                "-Wall",
                "-Wextra",
                "-pedantic-errors",
                "-fsyntax-only",
            ])
            .arg(&source_path)
            .status()
            .expect("C++ compiler must be available");
        fs::remove_file(&header_path).expect("remove generated C++ header");
        fs::remove_file(&source_path).expect("remove C++ translation unit");
        assert!(
            status.success(),
            "strict C++17 binary-only compile must succeed"
        );
    }

    #[test]
    fn non_floating_schema_does_not_emit_floating_guards() {
        let source = generate(&track_schema(), CLOSED).expect("track schema should generate");
        assert!(!source.contains("OMS Float32 requires"));
        assert!(!source.contains("OMS Float64 requires"));
        assert!(!source.contains("#include <climits>"));
    }

    #[test]
    fn rejects_unsupported_integral_semantics() {
        let cases = [
            (
                "exclusive",
                ConstraintSet {
                    min_exclusive: Some(NumericValue::Integer(0)),
                    ..ConstraintSet::default()
                },
                "exclusive, length, or lexical constraints",
            ),
            (
                "lexical",
                ConstraintSet {
                    lexical: ams_gra_oms_ir::LexicalConstraintSet {
                        pattern_groups: vec![ams_gra_oms_ir::PatternGroup {
                            alternatives: vec![ams_gra_oms_ir::PatternExpression::xml_schema(
                                "[0-9]+",
                            )],
                        }],
                        white_space: None,
                    },
                    ..ConstraintSet::default()
                },
                "exclusive, length, or lexical constraints",
            ),
            (
                "negative unsigned",
                integer_bounds(-1, 1),
                "negative or oversized unsigned range",
            ),
            (
                "signed overflow",
                integer_bounds(0, i128::from(i64::MAX) + 1),
                "signed range outside i64",
            ),
        ];
        for (label, constraints, diagnostic) in cases {
            let mut schema = integral_schema();
            let TypeKind::Record { fields } = &mut schema
                .types
                .iter_mut()
                .find(|declaration| declaration.name.local_name == "Scalars")
                .expect("integral fixture must contain Scalars")
                .kind
            else {
                panic!("Scalars must remain a Record");
            };
            let field = fields
                .iter_mut()
                .find(|field| field.name == "Byte_Value")
                .expect("integral fixture must contain Byte_Value");
            field.constraints = constraints;
            if label == "negative unsigned" {
                field.type_ref = TypeRef::primitive(PrimitiveKind::UnsignedInteger);
            }
            let error =
                generate(&schema, CLOSED).expect_err("unsupported integral semantics must fail");
            assert!(
                error.message.contains(diagnostic),
                "{label}: {}",
                error.message
            );
        }
    }

    #[test]
    fn lowers_choice_as_named_variant_and_accepts_empty_record_ancestry() {
        let source = generate(&choice_schema(), CLOSED).expect("supported Choice should generate");
        assert!(source.contains("#include <variant>"));
        assert!(source.contains("struct First { Token value; };\n    struct Second { Token value; };\n\n    std::variant<First, Second> value;"));
        assert!(source.contains("Selection selected;"));
        assert!(source.contains("std::variant<Left, Right> value;"));
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
        // the shared backend name preflight rather than by a C++-local
        // duplicate check, so the assertion is on the semantic outcome -- the
        // collision is rejected and both spellings are named -- rather than on
        // the exact prose of the superseded local diagnostic.
        let message = generate(&collision, CLOSED)
            .expect_err("converging C++ alternative names must be rejected")
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
                .contains("struct Value { Base value; };")
        );

        let source = generate(&repeated_choice_schema(), CLOSED)
            .expect("finite repeated Choice must generate");
        assert!(source.contains("struct Items { BoundedVector<Token, 0, 3> value; };"));
    }

    #[test]
    fn lowers_effective_record_fields_and_omits_abstract_ancestor() {
        let source =
            generate(&inheritance_schema(), CLOSED).expect("pure Record inheritance is supported");
        assert!(!source.contains("struct Base"));
        let leaf = source.find("struct Leaf").unwrap();
        let fields = &source[leaf..];
        assert!(fields.find("base_optional").unwrap() < fields.find("base_values").unwrap());
        assert!(fields.find("base_values").unwrap() < fields.find("linked").unwrap());
        assert!(fields.find("linked").unwrap() < fields.find("local").unwrap());
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
        assert!(source.contains("struct Base {\n    std::variant<"));
        assert!(source.contains("struct Derived {"));
        assert!(source.contains("std::vector<std::uint8_t>"));
    }

    #[test]
    fn lowers_abstract_value_references_and_retains_choice_boundary() {
        let source =
            generate(&abstract_value_schema(), CLOSED).expect("closed abstract value should lower");
        assert!(source.contains("#include <variant>"));
        assert!(source.contains("struct Base {\n    std::variant<"));
        assert!(source.find("struct Derived").unwrap() < source.find("struct Base").unwrap());
        assert!(source.find("struct Base").unwrap() < source.find("struct Holder").unwrap());
        assert!(
            generate(&choice_boundary_schema(), CLOSED)
                .unwrap_err()
                .message
                .contains("contains Record segment Base while lowering Choice")
        );
    }

    #[test]
    fn track_matches_golden_and_is_deterministic() {
        let schema = track_schema();
        let first = generate(&schema, CLOSED).expect("C++ generation should succeed");
        assert_eq!(
            first,
            generate(&schema, CLOSED).expect("generation should repeat")
        );
        assert_eq!(first, include_str!("../tests/expected/track.hpp"));
        assert!(!first.contains("#include <variant>"));
    }

    #[test]
    fn schema_set_matches_dependency_order_golden() {
        let source =
            generate(&codegen_order_schema(), CLOSED).expect("C++ generation should succeed");
        assert_eq!(source, include_str!("../tests/expected/codegen_order.hpp"));
        assert!(
            source.find("class IncludedId").unwrap()
                < source.find("struct RecordFirstInSource").unwrap()
        );
        assert!(
            source.find("enum class IncludedQuality").unwrap()
                < source.find("struct RecordFirstInSource").unwrap()
        );
    }

    #[test]
    fn preserves_order_naming_and_cardinality_semantics() {
        let source = generate(&track_schema(), CLOSED).expect("C++ generation should succeed");
        assert!(source.contains("namespace oms::track"));
        assert!(source.contains("class TrackId"));
        assert!(
            source.contains("min_value = 1;\n    static constexpr std::int64_t max_value = 65535;")
        );
        assert!(source.contains("Unknown,\n    Tentative,\n    Confirmed,"));
        assert!(source.contains("std::optional<std::string> callsign;"));
        assert!(source.contains("BoundedVector<std::int64_t, 0, 8> sensor_ids;"));
        assert!(source.contains("TrackId id;\n    TrackQuality quality;"));
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
        assert!(source.contains("struct TrackId"));
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
                "float"
            } else {
                "double"
            }));
        }

        // Task 033 supersedes the Task 022 blanket rejection for the bound-only
        // subset: a named lower-inclusive Float64 is now lowered as a checked
        // wrapper with no public unchecked constructor.
        let mut schema = track_schema();
        schema.types[0].kind = TypeKind::Primitive(PrimitiveKind::Float64);
        schema.types[0].constraints = ConstraintSet {
            min_inclusive: Some(NumericValue::Float64(
                ams_gra_oms_ir::Float64Value::from_value(0.0),
            )),
            ..ConstraintSet::default()
        };
        let source = generate(&schema, CLOSED).expect("Task 033 bounded float must render");
        assert!(source.contains("static constexpr double min_value = 0.0;"));
        assert!(source.contains("static std::optional<TrackId> create(double value) noexcept {"));
        assert!(source.contains("if (value >= min_value) {"));
        // Unchecked construction must not be reachable publicly.
        assert!(source.contains("private:\n    explicit TrackId(double value) noexcept"));
        // The binary64 host guard is still required for a constrained schema.
        assert!(source.contains("OMS Float64 requires IEEE binary64 double"));

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
        assert!(error.message.contains("unsupported C++ IR construct"));
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
        assert!(source.contains("std::vector<std::uint8_t>"));

        let mut schema = track_schema();
        schema.types[0].kind = TypeKind::Primitive(PrimitiveKind::Binary);
        schema.types[0].constraints = ConstraintSet::default();
        let source = generate(&schema, CLOSED).expect("unconstrained named binary must generate");
        assert!(source.contains(
            "explicit TrackId(std::vector<std::uint8_t> value) : value_(std::move(value)) {}"
        ));
        assert!(source.contains("const std::vector<std::uint8_t>& value() const noexcept"));
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
                .contains("unsupported C++ IR construct: constraints on")
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
                .contains("unsupported C++ IR construct: field constraints on")
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
                "unsupported C++ IR construct: type reference Primitive({kind:?})"
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
                    .contains("unsupported C++ IR construct: constraints on")
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
                message.contains("unsupported C++ IR construct: constraints on")
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
                .contains("unsupported C++ IR construct: abstract type")
        );
    }

    #[test]
    fn uninhabited_abstract_optional_field_is_elided_without_fake_payload() {
        let source = generate(&uninhabited_optional_schema(), CLOSED)
            .expect("absent-only occurrence should lower");
        assert!(!source.contains("SidecarPoint"));
        assert!(!source.contains("widget"));
        assert!(source.contains("struct Holder {\n    std::string required;\n};"));
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
        assert!(source.contains("struct SidecarPoint {\n    std::variant<"));
        assert!(source.contains("std::optional<SidecarPoint> widget;"));
    }

    #[test]
    fn inherited_uninhabited_field_is_elided_on_the_concrete_descendant() {
        let source = generate(&uninhabited_inherited_schema(), CLOSED)
            .expect("inherited absent-only occurrence should lower");
        assert!(!source.contains("SidecarPoint"));
        assert!(source.contains("struct ConcreteHolder {\n    std::string required;\n};"));
    }

    #[test]
    fn uninhabited_optional_field_composes_with_task_024_closed_sum() {
        let source = generate(&uninhabited_composes_closed_sum_schema(), CLOSED)
            .expect("Task 026 composition with Task 024 closed sum should lower");
        assert!(source.contains("struct Parent {\n    std::variant<"));
        assert!(source.contains("struct ConcreteChild {\n};"));
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
    /// 024 sum for the repeated 0..* extension point, and the generated header
    /// compiles under strict C++17 while constructing a PrivateA value inside
    /// the repeated collection. The same composed schema still fails closed
    /// under `open-extensions`.
    #[test]
    fn task029_private_overlay_closes_the_sum_but_not_the_world() {
        let source = generate(&overlay_composed_schema(&["private-a.xsd"]), CLOSED)
            .expect("an explicit overlay must close the extension point");
        assert!(source.contains("struct ExtensionBase {"));
        assert!(source.contains("std::variant<\n        PrivateA\n    > value;"));
        assert!(
            source.contains("UnboundedVector<ExtensionBase, 0> extensions;"),
            "the repeated 0..* field must still be generated: {source}"
        );

        use std::fs;
        use std::process::Command;
        let directory = std::env::temp_dir().join("ams-gra-oms-task029-overlay");
        fs::create_dir_all(&directory).expect("create C++ probe directory");
        fs::write(directory.join("overlay.hpp"), &source).expect("write generated C++ header");
        fs::write(
            directory.join("overlay.cpp"),
            r#"#include "overlay.hpp"

int probe() {
    using namespace urn::overlay;
    ExtensionBase value{PrivateA{"l", "p"}};
    auto extensions = UnboundedVector<ExtensionBase, 0>::create({value});
    Container container{"c", *extensions};
    return static_cast<int>(container.extensions.values().size());
}
"#,
        )
        .expect("write C++ probe unit");
        let status = Command::new("c++")
            .args([
                "-std=c++17",
                "-Wall",
                "-Wextra",
                "-pedantic-errors",
                "-fsyntax-only",
            ])
            .arg(directory.join("overlay.cpp"))
            .status()
            .expect("C++ compiler must be available");
        fs::remove_dir_all(&directory).expect("remove C++ probe directory");
        assert!(
            status.success(),
            "overlay-composed closed sum must compile under strict C++17"
        );

        let message = generate(&overlay_composed_schema(&["private-a.xsd"]), OPEN)
            .expect_err("section 25: an overlay must not imply a closed world")
            .to_string();
        assert!(
            message.contains("ExtensionBase") && message.contains("open-extensions"),
            "open diagnostic must name target and policy: {message}"
        );
    }

    /// Task 029 section 31: closed-sum alternative order follows the caller's
    /// overlay order, which is explicit deterministic input. Overlays are
    /// never sorted by filesystem path.
    #[test]
    fn task029_two_overlays_order_variants_by_caller_order() {
        let forward = generate(
            &overlay_composed_schema(&["private-a.xsd", "private-b.xsd"]),
            CLOSED,
        )
        .expect("two overlays should compose");
        assert!(
            forward.contains("std::variant<\n        PrivateA,\n        PrivateB\n    > value;")
        );

        let reversed = generate(
            &overlay_composed_schema(&["private-b.xsd", "private-a.xsd"]),
            CLOSED,
        )
        .expect("reversed overlays should compose");
        assert!(
            reversed.contains("std::variant<\n        PrivateB,\n        PrivateA\n    > value;")
        );
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
    /// would declare the same C++ type twice.
    #[test]
    fn converging_declaration_names_are_rejected() {
        let schema = preflight_fixture("backend-name-preflight-declaration-collision.xsd");
        let message = generate(&schema, CLOSED)
            .expect_err("converging declaration names must be rejected")
            .message;
        assert!(message.contains("TrackReport"), "{message}");
    }

    /// An inherited member and a locally declared member that snake_case to
    /// the same identifier. Effective structural projection accepts them
    /// because the XSD names differ; only generated-name policy catches this.
    #[test]
    fn converging_inherited_member_names_are_rejected() {
        let schema = preflight_fixture("backend-name-preflight-inherited-collision.xsd");
        let message = generate(&schema, CLOSED)
            .expect_err("converging inherited member names must be rejected")
            .message;
        assert!(message.contains("track_id"), "{message}");
    }

    /// A member named `class` snake_cases onto a C++ keyword.
    #[test]
    fn reserved_word_member_is_rejected() {
        let schema = preflight_fixture("backend-name-preflight-reserved.xsd");
        let message = generate(&schema, CLOSED)
            .expect_err("a C++ keyword member must be rejected")
            .message;
        assert!(message.contains("reserved word"), "{message}");
    }

    /// C++ is case-sensitive, so a case-only difference is NOT a C++
    /// collision. Asserting this keeps the shared preflight from silently
    /// applying Ada's rule everywhere.
    #[test]
    fn case_only_difference_is_accepted_by_cpp() {
        let schema = preflight_fixture("backend-name-preflight-ada-case-collision.xsd");
        assert!(generate(&schema, CLOSED).is_ok());
    }

    /// The control must still render, and the generated header must actually
    /// compile under strict C++17: a preflight that rejected everything would
    /// pass the negative tests above while being useless.
    #[test]
    fn preflight_control_renders_and_compiles() {
        let source = generate(
            &preflight_fixture("backend-name-preflight-control.xsd"),
            CLOSED,
        )
        .expect("safe generated names must render");
        assert!(source.contains("struct TrackReport"), "{source}");

        use std::fs;
        use std::process::Command;
        let directory = std::env::temp_dir().join("ams-gra-oms-preflight-cpp-control");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create C++ probe directory");
        fs::write(directory.join("generated.hpp"), &source).expect("write generated header");
        fs::write(
            directory.join("probe.cpp"),
            "#include \"generated.hpp\"\nint main() { return 0; }\n",
        )
        .expect("write C++ probe");
        let status = Command::new("c++")
            .current_dir(&directory)
            .args([
                "-std=c++17",
                "-Wall",
                "-Wextra",
                "-pedantic-errors",
                "-o",
                "probe",
                "probe.cpp",
            ])
            .status()
            .expect("C++ compiler must be available");
        let _ = fs::remove_dir_all(&directory);
        assert!(status.success(), "strict C++17 compile must succeed");
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
