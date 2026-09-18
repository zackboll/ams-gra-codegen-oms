//! Minimal Rust type generation from normalized schema IR.

use ams_gra_oms_codegen_core::{Backend, CodegenError, GeneratedFile, plan_type_declarations};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, PrimitiveKind, SchemaIr, TypeDecl, TypeKind, TypeRef, TypeRefTarget,
};
use std::fmt::Write as _;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Copy)]
pub struct RustBackend;

impl Backend for RustBackend {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn generate(&self, schema: &SchemaIr) -> Result<Vec<GeneratedFile>, CodegenError> {
        let contents = generate(schema)?;
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
/// # Errors
///
/// Returns an error when the IR contains a construct this initial backend
/// cannot represent without losing semantics.
pub fn generate(schema: &SchemaIr) -> Result<String, CodegenError> {
    let declarations = plan_type_declarations(schema)?;
    validate_schema(schema)?;
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
    for declaration in declarations {
        render_declaration(&mut output, declaration)?;
    }
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
        TypeKind::Record { fields } => {
            writeln!(
                output,
                "#[derive(Debug, Clone, PartialEq, Eq)]\npub struct {name} {{"
            )
            .expect("writing to String cannot fail");
            for field in fields {
                let field_name = snake_case(&field.name)?;
                let base = rust_type(&field.type_ref)?;
                let field_type = match field.cardinality {
                    Cardinality::REQUIRED_ONE => base,
                    Cardinality::OPTIONAL_ONE => format!("Option<{base}>"),
                    Cardinality {
                        min_occurs,
                        max_occurs: Some(max),
                    } if max > 1 => {
                        format!("BoundedVec<{base}, {min_occurs}, {max}>")
                    }
                    _ => return unsupported(format!("cardinality on field {field_name}")),
                };
                writeln!(output, "    pub {field_name}: {field_type},")
                    .expect("writing to String cannot fail");
            }
            output.push_str("}\n");
        }
        other => return unsupported(format!("type {name}: {other:?}")),
    }
    Ok(())
}

fn validate_schema(schema: &SchemaIr) -> Result<(), CodegenError> {
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
        if declaration.is_abstract {
            return unsupported(format!("abstract type {}", declaration.name.local_name));
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

fn rust_type(type_ref: &TypeRef) -> Result<String, CodegenError> {
    match &type_ref.target {
        TypeRefTarget::Primitive(PrimitiveKind::SignedInteger) => Ok("i64".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::String) => Ok("String".to_owned()),
        TypeRefTarget::Named(name) => upper_camel(&name.local_name),
        other => unsupported(format!("type reference {other:?}")),
    }
}

fn inclusive_bounds(constraints: &ConstraintSet, name: &str) -> Result<(i128, i128), CodegenError> {
    match (constraints.min_inclusive, constraints.max_inclusive) {
        (Some(min), Some(max))
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
        let first = generate(&schema).expect("Rust generation should succeed");
        assert_eq!(first, generate(&schema).expect("generation should repeat"));
        assert_eq!(first, include_str!("../tests/expected/track.rs"));
    }

    #[test]
    fn schema_set_matches_dependency_order_golden() {
        let source = generate(&codegen_order_schema()).expect("Rust generation should succeed");
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
        let source = generate(&track_schema()).expect("Rust generation should succeed");
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
        let error = generate(&schema).expect_err("choice must not be omitted");
        assert!(
            error
                .message
                .starts_with("unsupported Rust IR construct: type TrackId")
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
                "unsupported Rust IR construct: type reference Primitive({kind:?})"
            )));
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
                    .contains("unsupported Rust IR construct: inherited structural type")
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
                .contains("unsupported Rust IR construct: abstract type")
        );
    }
}
