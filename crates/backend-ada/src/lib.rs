//! Minimal Ada type generation from normalized schema IR.

use ams_gra_oms_codegen_core::{Backend, CodegenError, GeneratedFile, plan_type_declarations};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, PrimitiveKind, SchemaIr, TypeDecl, TypeKind, TypeRef, TypeRefTarget,
};
use std::fmt::Write as _;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Copy)]
pub struct AdaBackend;

impl Backend for AdaBackend {
    fn name(&self) -> &'static str {
        "ada"
    }

    fn generate(&self, schema: &SchemaIr) -> Result<Vec<GeneratedFile>, CodegenError> {
        let contents = generate(schema)?;
        let package = package_name(schema)?;
        let file_stem = package.to_ascii_lowercase().replace('.', "-");
        let mut files = Vec::new();
        if let Some((parent, _)) = package.split_once('.') {
            files.push(GeneratedFile {
                relative_path: PathBuf::from(format!("{}.ads", parent.to_ascii_lowercase())),
                contents: format!("package {parent} is\nend {parent};\n"),
            });
        }
        files.push(GeneratedFile {
            relative_path: PathBuf::from(format!("{file_stem}.ads")),
            contents,
        });
        Ok(files)
    }
}

/// Generate one Ada package specification from normalized schema IR.
///
/// # Errors
///
/// Returns an error when the IR contains a construct this initial backend
/// cannot represent without losing semantics.
pub fn generate(schema: &SchemaIr) -> Result<String, CodegenError> {
    let declarations = plan_type_declarations(schema)?;
    validate_schema(schema)?;
    let package = package_name(schema)?;
    let mut output = String::from("with Ada.Strings.Unbounded;\n\n");
    writeln!(output, "package {package} is\n").expect("writing to String cannot fail");
    output.push_str(concat!(
        "   type Optional_String (Is_Present : Boolean := False) is record\n",
        "      case Is_Present is\n",
        "         when False => null;\n",
        "         when True  => Value : Ada.Strings.Unbounded.Unbounded_String;\n",
        "      end case;\n",
        "   end record;\n\n",
    ));
    for declaration in declarations {
        render_declaration(&mut output, declaration)?;
    }
    writeln!(output, "end {package};").expect("writing to String cannot fail");
    Ok(output)
}

fn render_declaration(output: &mut String, declaration: &TypeDecl) -> Result<(), CodegenError> {
    let name = ada_identifier(&declaration.name.local_name)?;
    match &declaration.kind {
        TypeKind::Primitive(PrimitiveKind::SignedInteger) => {
            let (min, max) = inclusive_bounds(&declaration.constraints, &name)?;
            writeln!(
                output,
                "   type {name} is range {} .. {};\n",
                ada_number(min),
                ada_number(max)
            )
            .expect("writing to String cannot fail");
        }
        TypeKind::Enumeration { variants } => {
            if variants.is_empty() {
                return unsupported(format!("empty enumeration {name}"));
            }
            writeln!(output, "   type {name} is").expect("writing to String cannot fail");
            for (index, variant) in variants.iter().enumerate() {
                let variant_name = ada_identifier(&variant.wire_value)?;
                let prefix = if index == 0 { "     (" } else { "      " };
                let suffix = if index + 1 == variants.len() {
                    ");"
                } else {
                    ","
                };
                writeln!(output, "{prefix}{variant_name}{suffix}")
                    .expect("writing to String cannot fail");
            }
            output.push('\n');
        }
        TypeKind::Record { fields } => {
            for field in fields {
                if let Some(max) = repeated_max(field.cardinality) {
                    let field_name = ada_identifier(&field.name)?;
                    let item_type = ada_type(&field.type_ref)?;
                    writeln!(
                        output,
                        "   type {field_name}_Array is array (Positive range 1 .. {max}) of {item_type};\n\
                         \x20  type {field_name}_Sequence is record\n\
                         \x20     Length : Natural range 0 .. {max} := 0;\n\
                         \x20     Items  : {field_name}_Array;\n\
                         \x20  end record;\n"
                    )
                    .expect("writing to String cannot fail");
                }
            }
            writeln!(output, "   type {name} is record").expect("writing to String cannot fail");
            for field in fields {
                let field_name = ada_identifier(&field.name)?;
                let field_type = match field.cardinality {
                    Cardinality::REQUIRED_ONE => ada_type(&field.type_ref)?,
                    Cardinality::OPTIONAL_ONE
                        if field.type_ref.target
                            == TypeRefTarget::Primitive(PrimitiveKind::String) =>
                    {
                        "Optional_String".to_owned()
                    }
                    cardinality if repeated_max(cardinality).is_some() => {
                        format!("{field_name}_Sequence")
                    }
                    _ => return unsupported(format!("cardinality on field {field_name}")),
                };
                writeln!(output, "      {field_name} : {field_type};")
                    .expect("writing to String cannot fail");
            }
            output.push_str("   end record;\n\n");
        }
        other => return unsupported(format!("type {name}: {other:?}")),
    }
    Ok(())
}

fn validate_schema(schema: &SchemaIr) -> Result<(), CodegenError> {
    let namespace = schema
        .namespaces
        .first()
        .ok_or_else(|| error("Ada generation requires one namespace"))?;
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
        reject_extra_constraints(&declaration.constraints, &declaration.name.local_name)?;
        if let TypeKind::Record { fields } = &declaration.kind {
            for field in fields {
                if field.nillable {
                    return unsupported(format!("nillable field {}", field.name));
                }
                reject_any_constraints(&field.constraints, &field.name)?;
            }
        }
    }
    Ok(())
}

fn package_name(schema: &SchemaIr) -> Result<String, CodegenError> {
    let uri = &schema
        .namespaces
        .first()
        .ok_or_else(|| error("Ada generation requires one namespace"))?
        .uri;
    let parts = uri
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return unsupported(format!("namespace URI {uri}"));
    }
    Ok(format!(
        "{}.{}",
        ada_title(parts[parts.len() - 2])?,
        ada_title(parts[parts.len() - 1])?
    ))
}

fn ada_type(type_ref: &TypeRef) -> Result<String, CodegenError> {
    match &type_ref.target {
        TypeRefTarget::Primitive(PrimitiveKind::SignedInteger) => {
            Ok("Long_Long_Integer".to_owned())
        }
        TypeRefTarget::Primitive(PrimitiveKind::String) => {
            Ok("Ada.Strings.Unbounded.Unbounded_String".to_owned())
        }
        TypeRefTarget::Named(name) => ada_identifier(&name.local_name),
        other => unsupported(format!("type reference {other:?}")),
    }
}

fn repeated_max(cardinality: Cardinality) -> Option<u64> {
    match cardinality {
        Cardinality {
            min_occurs: 0,
            max_occurs: Some(max),
        } if max > 1 => Some(max),
        _ => None,
    }
}

fn inclusive_bounds(constraints: &ConstraintSet, name: &str) -> Result<(i128, i128), CodegenError> {
    match (constraints.min_inclusive, constraints.max_inclusive) {
        (Some(min), Some(max)) if min <= max => Ok((min, max)),
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

fn reject_any_constraints(constraints: &ConstraintSet, name: &str) -> Result<(), CodegenError> {
    if constraints != &ConstraintSet::default() {
        return unsupported(format!("field constraints on {name}"));
    }
    Ok(())
}

fn ada_identifier(value: &str) -> Result<String, CodegenError> {
    if !value.is_empty()
        && value.as_bytes()[0].is_ascii_alphabetic()
        && value
            .bytes()
            .all(|character| character.is_ascii_alphanumeric() || character == b'_')
        && !value.contains("__")
        && !value.ends_with('_')
    {
        Ok(value.to_owned())
    } else {
        unsupported(format!("Ada identifier {value:?}"))
    }
}

fn ada_title(value: &str) -> Result<String, CodegenError> {
    let mut characters = value.chars();
    let first = characters
        .next()
        .ok_or_else(|| error("empty Ada namespace component"))?;
    ada_identifier(&format!(
        "{}{}",
        first.to_ascii_uppercase(),
        characters.as_str()
    ))
}

fn ada_number(value: i128) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut grouped = String::new();
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push('_');
        }
        grouped.push(character);
    }
    if value < 0 {
        format!("-{grouped}")
    } else {
        grouped
    }
}

fn unsupported<T>(construct: String) -> Result<T, CodegenError> {
    Err(error(format!("unsupported Ada IR construct: {construct}")))
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

    #[test]
    fn track_matches_golden_and_is_deterministic() {
        let schema = track_schema();
        let first = generate(&schema).expect("Ada generation should succeed");
        let second = generate(&schema).expect("Ada generation should be repeatable");
        assert_eq!(first, second);
        assert_eq!(first, include_str!("../tests/expected/track.ads"));
    }

    #[test]
    fn schema_set_matches_dependency_order_golden() {
        let source = generate(&codegen_order_schema()).expect("Ada generation should succeed");
        assert_eq!(source, include_str!("../tests/expected/codegen_order.ads"));
        assert!(
            source.find("type Included_Id").unwrap() < source.find("type Record_First").unwrap()
        );
        assert!(
            source.find("type Included_Quality").unwrap()
                < source.find("type Record_First").unwrap()
        );
    }

    #[test]
    fn preserves_order_and_cardinality_semantics() {
        let source = generate(&track_schema()).expect("Ada generation should succeed");
        assert!(source.contains("type Track_Id is range 1 .. 65_535;"));
        assert!(source.contains("(Unknown,\n      Tentative,\n      Confirmed);"));
        assert!(source.contains("Callsign : Optional_String;"));
        assert!(source.contains("Length : Natural range 0 .. 8 := 0;"));
        assert!(source.contains("Id : Track_Id;\n      Quality : Track_Quality;"));
    }

    #[test]
    fn unsupported_construct_fails_explicitly() {
        let mut schema = track_schema();
        schema.types[0].kind = TypeKind::Choice {
            alternatives: Vec::new(),
        };
        let error = generate(&schema).expect_err("choice must not be omitted");
        assert!(
            error
                .message
                .starts_with("unsupported Ada IR construct: type Track_Id")
        );
    }
}
