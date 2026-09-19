//! Minimal C++17 type generation from normalized schema IR.

use ams_gra_oms_codegen_core::{
    Backend, CodegenError, GeneratedFile, InclusiveIntegralDomain, effective_choice_alternatives,
    effective_record_fields, inclusive_integral_domain, plan_type_declarations,
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

    fn generate(&self, schema: &SchemaIr) -> Result<Vec<GeneratedFile>, CodegenError> {
        let contents = generate(schema)?;
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
/// # Errors
///
/// Returns an error when the IR contains a construct this initial backend
/// cannot represent without losing semantics.
pub fn generate(schema: &SchemaIr) -> Result<String, CodegenError> {
    let declarations = plan_type_declarations(schema)?;
    validate_schema(schema)?;
    let namespace = namespace_name(schema)?;
    let variant_header = if schema
        .types
        .iter()
        .any(|declaration| matches!(declaration.kind, TypeKind::Choice { .. }))
    {
        "#include <variant>\n"
    } else {
        ""
    };
    let limits_header =
        if schema_needs_limits(schema) || schema.types.iter().any(has_unbounded_occurrence) {
            "#include <limits>\n"
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
        limits_header,
    );
    output.push_str(variant_header);
    output.push('\n');
    if schema_has_floating(schema) {
        output.push_str("static_assert(std::numeric_limits<float>::digits == 24 && std::numeric_limits<float>::is_iec559, \"OMS Float32 requires IEEE binary32 float\");\nstatic_assert(std::numeric_limits<double>::digits == 53 && std::numeric_limits<double>::is_iec559, \"OMS Float64 requires IEEE binary64 double\");\n\n");
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
    if schema.types.iter().any(has_unbounded_occurrence) {
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
    if schema.types.iter().any(has_direct_integral_range) {
        output.push_str(concat!(
            "template <typename T, T Min, T Max>\nclass BoundedInteger {\npublic:\n",
            "    static std::optional<BoundedInteger> create(T value) noexcept {\n",
            "        if (value < Min || value > Max) return std::nullopt;\n",
            "        return BoundedInteger(value);\n    }\n",
            "    T value() const noexcept { return value_; }\nprivate:\n",
            "    explicit BoundedInteger(T value) noexcept : value_(value) {}\n    T value_;\n};\n\n",
        ));
    }
    for declaration in declarations {
        render_declaration(&mut output, schema, declaration)?;
    }
    writeln!(output, "}}  // namespace {namespace}").expect("writing to String cannot fail");
    Ok(output)
}

fn render_declaration(
    output: &mut String,
    schema: &SchemaIr,
    declaration: &TypeDecl,
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
        TypeKind::Primitive(PrimitiveKind::Float32) => {
            reject_any_constraints(&declaration.constraints, &name)?;
            writeln!(output, "class {name} {{\npublic:\n    explicit {name}(float value) noexcept : value_(value) {{}}\n    float value() const noexcept {{ return value_; }}\nprivate:\n    float value_;\n}};\n").expect("writing to String cannot fail");
        }
        TypeKind::Primitive(PrimitiveKind::Float64) => {
            reject_any_constraints(&declaration.constraints, &name)?;
            writeln!(output, "class {name} {{\npublic:\n    explicit {name}(double value) noexcept : value_(value) {{}}\n    double value() const noexcept {{ return value_; }}\nprivate:\n    double value_;\n}};\n").expect("writing to String cannot fail");
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

fn validate_schema(schema: &SchemaIr) -> Result<(), CodegenError> {
    let namespace = schema
        .namespaces
        .first()
        .ok_or_else(|| error("C++ generation requires one namespace"))?;
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
            && (!matches!(declaration.kind, TypeKind::Record { .. })
                || !schema.types.iter().any(|candidate| {
                    candidate.base_type.as_ref().is_some_and(|base| {
                        matches!(&base.target, TypeRefTarget::Named(name) if name == &declaration.name)
                    })
                }))
        {
            return unsupported(format!("abstract type {}", declaration.name.local_name));
        }
        if matches!(
            declaration.kind,
            TypeKind::Primitive(PrimitiveKind::Float32 | PrimitiveKind::Float64)
        ) && declaration.constraints != ConstraintSet::default()
        {
            return unsupported(format!(
                "floating constraints on {}",
                declaration.name.local_name
            ));
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
                if field.nillable {
                    return unsupported(format!("nillable field {}", field.name));
                }
                if let TypeRefTarget::Named(target) = &field.type_ref.target {
                    if schema
                        .types
                        .iter()
                        .any(|candidate| candidate.name == *target && candidate.is_abstract)
                    {
                        return unsupported(format!(
                            "abstract structural value reference {}",
                            target.local_name
                        ));
                    }
                }
            }
        }
        if matches!(declaration.kind, TypeKind::Choice { .. }) {
            let alternatives = effective_choice_alternatives(schema, &declaration.name).map_err(
                |projection_error| {
                    error(format!("unsupported C++ IR construct: {projection_error}"))
                },
            )?;
            validate_choice_alternatives(schema, alternatives)?;
        }
    }
    for message in &schema.messages {
        if let TypeRefTarget::Named(target) = &message.payload_type.target {
            if schema
                .types
                .iter()
                .any(|candidate| candidate.name == *target && candidate.is_abstract)
            {
                return unsupported(format!("abstract message payload {}", target.local_name));
            }
        }
    }
    Ok(())
}

fn validate_choice_alternatives(
    schema: &SchemaIr,
    alternatives: Vec<&ams_gra_oms_ir::FieldDecl>,
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
        if let TypeRefTarget::Named(target) = &alternative.type_ref.target {
            if schema
                .types
                .iter()
                .any(|candidate| candidate.name == *target && candidate.is_abstract)
            {
                return unsupported(format!(
                    "abstract structural value reference {}",
                    target.local_name
                ));
            }
        }
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

fn has_unbounded_occurrence(declaration: &TypeDecl) -> bool {
    match &declaration.kind {
        TypeKind::Record { fields } => fields,
        TypeKind::Choice { alternatives } => alternatives,
        _ => return false,
    }
    .iter()
    .any(|field| matches!(field.cardinality.shape(), OccurrenceShape::Unbounded { .. }))
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

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::NumericValue;
    use ams_gra_oms_xsd_frontend::{load_schema_document, load_schema_set};
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
        let source = generate(&unbounded_schema()).expect("unbounded cardinality should generate");
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
        let source = generate(&schema).expect("full unbounded minimum should generate");
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
        let source = generate(&integral_schema()).expect("integral scalars should generate");
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
        let source = generate(&full_integral_boundary_schema())
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
            let error = generate(&schema).expect_err("unsupported integral semantics must fail");
            assert!(
                error.message.contains(diagnostic),
                "{label}: {}",
                error.message
            );
        }
    }

    #[test]
    fn lowers_choice_as_named_variant_and_accepts_empty_record_ancestry() {
        let source = generate(&choice_schema()).expect("supported Choice should generate");
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
        assert!(
            generate(&collision)
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
            generate(&nillable)
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
            generate(&constrained)
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
            generate(&abstract_target)
                .unwrap_err()
                .message
                .contains("abstract structural value reference Base")
        );

        let source =
            generate(&repeated_choice_schema()).expect("finite repeated Choice must generate");
        assert!(source.contains("struct Items { BoundedVector<Token, 0, 3> value; };"));
    }

    #[test]
    fn lowers_effective_record_fields_and_omits_abstract_ancestor() {
        let source = generate(&inheritance_schema()).expect("pure Record inheritance is supported");
        assert!(!source.contains("struct Base"));
        let leaf = source.find("struct Leaf").unwrap();
        let fields = &source[leaf..];
        assert!(fields.find("base_optional").unwrap() < fields.find("base_values").unwrap());
        assert!(fields.find("base_values").unwrap() < fields.find("linked").unwrap());
        assert!(fields.find("linked").unwrap() < fields.find("local").unwrap());
    }

    #[test]
    fn rejects_abstract_value_references_and_choice_segments() {
        assert!(
            generate(&abstract_value_schema())
                .unwrap_err()
                .message
                .contains("abstract structural value reference Base")
        );
        assert!(
            generate(&choice_boundary_schema())
                .unwrap_err()
                .message
                .contains("contains Record segment Base while lowering Choice")
        );
    }

    #[test]
    fn track_matches_golden_and_is_deterministic() {
        let schema = track_schema();
        let first = generate(&schema).expect("C++ generation should succeed");
        assert_eq!(first, generate(&schema).expect("generation should repeat"));
        assert_eq!(first, include_str!("../tests/expected/track.hpp"));
        assert!(!first.contains("#include <variant>"));
    }

    #[test]
    fn schema_set_matches_dependency_order_golden() {
        let source = generate(&codegen_order_schema()).expect("C++ generation should succeed");
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
        let source = generate(&track_schema()).expect("C++ generation should succeed");
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
        let source = generate(&schema).expect("Choice must render");
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
            let source = generate(&schema).expect("unconstrained floating generation must succeed");
            assert!(source.contains(if kind == PrimitiveKind::Float32 {
                "float"
            } else {
                "double"
            }));
        }

        let mut schema = track_schema();
        schema.types[0].kind = TypeKind::Primitive(PrimitiveKind::Float64);
        schema.types[0].constraints = ConstraintSet {
            min_inclusive: Some(NumericValue::Float64(
                ams_gra_oms_ir::Float64Value::from_value(0.0),
            )),
            ..ConstraintSet::default()
        };
        let error = generate(&schema).expect_err("constrained float must remain unsupported");
        assert!(error.message.contains("unsupported C++ IR construct"));
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
            let error = generate(&schema).expect_err("temporal generation must remain unsupported");
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
            let error = generate(&schema).expect_err("constraints must not be discarded");
            assert!(
                error
                    .message
                    .contains("unsupported C++ IR construct: constraints on")
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
            let error = generate(&schema).expect_err("lexical constraints must be rejected");
            assert!(
                error
                    .message
                    .contains("unsupported C++ IR construct: constraints on")
            );
        }
    }

    #[test]
    fn abstract_types_fail_explicitly() {
        let mut schema = track_schema();
        schema.types[0].is_abstract = true;
        let error = generate(&schema).expect_err("abstract type must be rejected");
        assert!(
            error
                .message
                .contains("unsupported C++ IR construct: abstract type")
        );
    }
}
