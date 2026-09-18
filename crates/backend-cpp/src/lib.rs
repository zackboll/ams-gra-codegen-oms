//! Minimal C++17 type generation from normalized schema IR.

use ams_gra_oms_codegen_core::{Backend, CodegenError, GeneratedFile, plan_type_declarations};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, NumericValue, PrimitiveKind, SchemaIr, TypeDecl, TypeKind, TypeRef,
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
    let mut output = String::from(
        "#pragma once\n\n\
         #include <cstddef>\n\
         #include <cstdint>\n\
         #include <optional>\n\
         #include <string>\n\
         #include <utility>\n\
         #include <vector>\n\n",
    );
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
    for declaration in declarations {
        render_declaration(&mut output, declaration)?;
    }
    writeln!(output, "}}  // namespace {namespace}").expect("writing to String cannot fail");
    Ok(output)
}

fn render_declaration(output: &mut String, declaration: &TypeDecl) -> Result<(), CodegenError> {
    let name = upper_camel(&declaration.name.local_name)?;
    match &declaration.kind {
        TypeKind::Primitive(PrimitiveKind::SignedInteger) => {
            let (min, max) = inclusive_bounds(&declaration.constraints, &name)?;
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
        TypeKind::Record { fields } => {
            writeln!(output, "struct {name} {{").expect("writing to String cannot fail");
            for field in fields {
                let field_name = snake_case(&field.name)?;
                let base = cpp_type(&field.type_ref)?;
                let field_type = match field.cardinality {
                    Cardinality::REQUIRED_ONE => base,
                    Cardinality::OPTIONAL_ONE => format!("std::optional<{base}>"),
                    Cardinality {
                        min_occurs,
                        max_occurs: Some(max),
                    } if max > 1 => {
                        format!("BoundedVector<{base}, {min_occurs}, {max}>")
                    }
                    _ => return unsupported(format!("cardinality on field {field_name}")),
                };
                writeln!(output, "    {field_type} {field_name};")
                    .expect("writing to String cannot fail");
            }
            output.push_str("};\n\n");
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
        if declaration.is_abstract {
            return unsupported(format!("abstract type {}", declaration.name.local_name));
        }
        if matches!(
            declaration.kind,
            TypeKind::Primitive(PrimitiveKind::Float32 | PrimitiveKind::Float64)
        ) && has_numeric_constraints(&declaration.constraints)
        {
            return unsupported(format!(
                "floating constraints on {}",
                declaration.name.local_name
            ));
        }
        if matches!(
            declaration.kind,
            TypeKind::Record { .. } | TypeKind::Choice { .. }
        ) && matches!(
            declaration.base_type.as_ref().map(|base| &base.target),
            Some(TypeRefTarget::Named(_))
        ) {
            return unsupported(format!(
                "inherited structural type {}",
                declaration.name.local_name
            ));
        }
        reject_extra_constraints(&declaration.constraints, &declaration.name.local_name)?;
        if let TypeKind::Record { fields } = &declaration.kind {
            for field in fields {
                if field.nillable {
                    return unsupported(format!("nillable field {}", field.name));
                }
                if field.constraints != ConstraintSet::default() {
                    return unsupported(format!("field constraints on {}", field.name));
                }
            }
        }
    }
    Ok(())
}

fn has_numeric_constraints(constraints: &ConstraintSet) -> bool {
    constraints.min_inclusive.is_some()
        || constraints.max_inclusive.is_some()
        || constraints.min_exclusive.is_some()
        || constraints.max_exclusive.is_some()
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
        TypeRefTarget::Primitive(PrimitiveKind::String) => Ok("std::string".to_owned()),
        TypeRefTarget::Named(name) => upper_camel(&name.local_name),
        other => unsupported(format!("type reference {other:?}")),
    }
}

fn inclusive_bounds(constraints: &ConstraintSet, name: &str) -> Result<(i128, i128), CodegenError> {
    match (constraints.min_inclusive, constraints.max_inclusive) {
        (Some(NumericValue::Integer(min)), Some(NumericValue::Integer(max)))
            if min <= max && i64::try_from(min).is_ok() && i64::try_from(max).is_ok() =>
        {
            Ok((min, max))
        }
        _ => unsupported(format!("integer bounds on {name}")),
    }
}

fn reject_extra_constraints(constraints: &ConstraintSet, name: &str) -> Result<(), CodegenError> {
    if constraints.min_exclusive.is_some()
        || constraints.max_exclusive.is_some()
        || constraints.length.is_some()
        || constraints.min_length.is_some()
        || constraints.max_length.is_some()
        || !constraints.patterns.is_empty()
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
    use ams_gra_oms_xsd_frontend::{load_schema_document, load_schema_set};
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

    #[test]
    fn track_matches_golden_and_is_deterministic() {
        let schema = track_schema();
        let first = generate(&schema).expect("C++ generation should succeed");
        assert_eq!(first, generate(&schema).expect("generation should repeat"));
        assert_eq!(first, include_str!("../tests/expected/track.hpp"));
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
        let error = generate(&schema).expect_err("choice must not be omitted");
        assert!(
            error
                .message
                .starts_with("unsupported C++ IR construct: type TrackId")
        );
    }

    #[test]
    fn floating_fields_fail_explicitly() {
        for kind in [PrimitiveKind::Float32, PrimitiveKind::Float64] {
            let mut schema = floating_schema();
            let TypeKind::Record { fields } = &mut schema.types[0].kind else {
                panic!("floating fixture should contain a record");
            };
            fields.truncate(1);
            fields[0].type_ref = TypeRef::primitive(kind);
            let error = generate(&schema).expect_err("floating generation must remain unsupported");
            assert!(error.message.contains(&format!(
                "unsupported C++ IR construct: type reference Primitive({kind:?})"
            )));
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
    fn inherited_structural_types_fail_explicitly() {
        for choice in [false, true] {
            let mut schema = track_schema();
            let derived_index = schema
                .types
                .iter()
                .position(|declaration| matches!(declaration.kind, TypeKind::Record { .. }))
                .unwrap();
            let mut base = schema.types[derived_index].clone();
            base.name.local_name = "Structural_Base".to_owned();
            base.kind = TypeKind::Record { fields: Vec::new() };
            schema.types[derived_index].base_type = Some(TypeRef::named(base.name.clone()));
            if choice {
                let TypeKind::Record { fields } = &schema.types[derived_index].kind else {
                    unreachable!()
                };
                schema.types[derived_index].kind = TypeKind::Choice {
                    alternatives: vec![fields[0].clone()],
                };
            }
            schema.types.push(base);
            let error = generate(&schema).expect_err("inherited structure must be rejected");
            assert!(
                error
                    .message
                    .contains("unsupported C++ IR construct: inherited structural type")
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
