//! Minimal C++17 type generation from normalized schema IR.

use ams_gra_oms_codegen_core::{
    Backend, CodegenError, GeneratedFile, effective_choice_alternatives, effective_record_fields,
    plan_type_declarations,
};
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
    let variant_header = if schema
        .types
        .iter()
        .any(|declaration| matches!(declaration.kind, TypeKind::Choice { .. }))
    {
        "#include <variant>\n"
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
    output.push_str(variant_header);
    output.push('\n');
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
        ) && has_numeric_constraints(&declaration.constraints)
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
                if field.constraints != ConstraintSet::default() {
                    return unsupported(format!("field constraints on {}", field.name));
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
        if alternative.constraints != ConstraintSet::default() {
            return unsupported(format!("field constraints on {}", alternative.name));
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
    let base = cpp_type(&field.type_ref)?;
    match field.cardinality {
        Cardinality::REQUIRED_ONE => Ok(base),
        Cardinality::OPTIONAL_ONE => Ok(format!("std::optional<{base}>")),
        Cardinality {
            min_occurs,
            max_occurs: Some(max),
        } if max > 1 => Ok(format!("BoundedVector<{base}, {min_occurs}, {max}>")),
        _ => unsupported(format!("cardinality on Choice alternative {}", field.name)),
    }
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

    #[test]
    fn lowers_choice_as_named_variant_and_accepts_empty_record_ancestry() {
        let source = generate(&choice_schema()).expect("supported Choice should generate");
        assert!(source.contains("#include <variant>"));
        assert!(source.contains("struct First { Token value; };\n    struct Second { Token value; };\n\n    std::variant<First, Second> value;"));
        assert!(source.contains("Selection selected;"));
        assert!(source.contains("std::variant<Left, Right> value;"));
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
