//! Minimal Ada type generation from normalized schema IR.

use ams_gra_oms_codegen_core::{
    Backend, CodegenError, GeneratedFile, InclusiveIntegralDomain, effective_choice_alternatives,
    effective_record_fields, inclusive_integral_domain, plan_type_declarations,
};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, OccurrenceShape, PrimitiveKind, SchemaIr, TypeDecl, TypeKind,
    TypeRef, TypeRefTarget,
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
    let mut output = String::from("with Ada.Strings.Unbounded;\n");
    if schema.types.iter().any(has_zero_unbounded_occurrence) {
        output.push_str("with Ada.Containers.Vectors;\n");
    }
    if schema_needs_interfaces(schema) {
        output.push_str("with Interfaces;\n");
    }
    output.push('\n');
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
        render_declaration(&mut output, schema, declaration)?;
    }
    writeln!(output, "end {package};").expect("writing to String cannot fail");
    Ok(output)
}

fn render_declaration(
    output: &mut String,
    schema: &SchemaIr,
    declaration: &TypeDecl,
) -> Result<(), CodegenError> {
    let name = ada_identifier(&declaration.name.local_name)?;
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
                "   type {name} is range {} .. {};\n",
                ada_number(i128::from(min)),
                ada_number(i128::from(max))
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
            writeln!(
                output,
                "   subtype {name} is Interfaces.Unsigned_64 range {min} .. {max};\n"
            )
            .expect("writing to String cannot fail");
        }
        TypeKind::Primitive(PrimitiveKind::Boolean) => {
            reject_any_constraints(&declaration.constraints, &name)?;
            writeln!(output, "   type {name} is new Boolean;\n")
                .expect("writing to String cannot fail");
        }
        TypeKind::Primitive(PrimitiveKind::Float32) => {
            reject_any_constraints(&declaration.constraints, &name)?;
            writeln!(output, "   type {name} is new Interfaces.IEEE_Float_32;\n")
                .expect("writing to String cannot fail");
        }
        TypeKind::Primitive(PrimitiveKind::Float64) => {
            reject_any_constraints(&declaration.constraints, &name)?;
            writeln!(output, "   type {name} is new Interfaces.IEEE_Float_64;\n")
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
        TypeKind::Record { .. } => {
            if declaration.is_abstract {
                return Ok(());
            }
            let fields = effective_record_fields(schema, &declaration.name).map_err(|_| {
                error(format!(
                    "unsupported Ada IR construct: non-Record inheritance {}",
                    declaration.name.local_name
                ))
            })?;
            for field in &fields {
                if let Some(max) = repeated_max(field.cardinality) {
                    let field_name = ada_identifier(&field.name)?;
                    let helper_name = format!("{name}_{field_name}");
                    let item_type = ada_field_base(field)?;
                    writeln!(
                        output,
                        "   type {helper_name}_Array is array (Positive range 1 .. {max}) of {item_type};\n\
                         \x20  type {helper_name}_Sequence is record\n\
                         \x20     Length : Natural range 0 .. {max} := 0;\n\
                         \x20     Items  : {helper_name}_Array;\n\
                         \x20  end record;\n"
                    )
                    .expect("writing to String cannot fail");
                } else if matches!(
                    field.cardinality.shape(),
                    OccurrenceShape::Unbounded { min: 0 }
                ) {
                    render_unbounded_helper(output, &name, field)?;
                }
            }
            writeln!(output, "   type {name} is record").expect("writing to String cannot fail");
            if fields.is_empty() {
                output.push_str("      null;\n");
            }
            for field in fields {
                let field_name = ada_identifier(&field.name)?;
                let field_type = match field.cardinality {
                    Cardinality::REQUIRED_ONE => ada_field_base(field)?,
                    Cardinality::OPTIONAL_ONE
                        if field.type_ref.target
                            == TypeRefTarget::Primitive(PrimitiveKind::String) =>
                    {
                        "Optional_String".to_owned()
                    }
                    cardinality
                        if repeated_max(cardinality).is_some()
                            || matches!(
                                cardinality.shape(),
                                OccurrenceShape::Unbounded { min: 0 }
                            ) =>
                    {
                        format!("{name}_{field_name}_Sequence")
                    }
                    _ => return unsupported(format!("cardinality on field {field_name}")),
                };
                writeln!(output, "      {field_name} : {field_type};")
                    .expect("writing to String cannot fail");
            }
            output.push_str("   end record;\n\n");
        }
        TypeKind::Choice { .. } => {
            let alternatives = effective_choice_alternatives(schema, &declaration.name).map_err(
                |projection_error| {
                    error(format!("unsupported Ada IR construct: {projection_error}"))
                },
            )?;
            let kind_name = format!("{name}_Kind");
            for alternative in &alternatives {
                if let Some(max) = repeated_max(alternative.cardinality) {
                    let alternative_name = ada_identifier(&alternative.name)?;
                    let helper_name = format!("{name}_{alternative_name}");
                    let item_type = ada_field_base(alternative)?;
                    writeln!(
                        output,
                        "   type {helper_name}_Array is array (Positive range 1 .. {max}) of {item_type};\n\
                         \x20  type {helper_name}_Sequence is record\n\
                         \x20     Length : Natural range 0 .. {max} := 0;\n\
                         \x20     Items  : {helper_name}_Array;\n\
                         \x20  end record;\n"
                    )
                    .expect("writing to String cannot fail");
                } else if matches!(
                    alternative.cardinality.shape(),
                    OccurrenceShape::Unbounded { min: 0 }
                ) {
                    render_unbounded_helper(output, &name, alternative)?;
                }
            }
            writeln!(output, "   type {kind_name} is").expect("writing to String cannot fail");
            output.push_str("      (");
            for (index, alternative) in alternatives.iter().enumerate() {
                let suffix = if index + 1 == alternatives.len() {
                    ");"
                } else {
                    ","
                };
                writeln!(
                    output,
                    "{}{}_Kind{suffix}",
                    if index == 0 { "" } else { "       " },
                    ada_identifier(&alternative.name)?
                )
                .expect("writing to String cannot fail");
            }
            writeln!(
                output,
                "\n   type {name} (Kind : {kind_name} := {}_Kind) is record\n      case Kind is",
                ada_identifier(&alternatives[0].name)?
            )
            .expect("writing to String cannot fail");
            for alternative in alternatives {
                let alternative_name = ada_identifier(&alternative.name)?;
                writeln!(
                    output,
                    "         when {alternative_name}_Kind =>\n            {alternative_name} : {};",
                    ada_field_type(&name, alternative)?
                )
                .expect("writing to String cannot fail");
            }
            output.push_str("      end case;\n   end record;\n\n");
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
                    "unsupported Ada IR construct: non-Record inheritance {}",
                    declaration.name.local_name
                ))
            })?;
            for field in fields {
                ada_field_base(field)?;
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
                    error(format!("unsupported Ada IR construct: {projection_error}"))
                },
            )?;
            validate_choice_alternatives(schema, &declaration.name.local_name, alternatives)?;
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
    choice_name: &str,
    alternatives: Vec<&ams_gra_oms_ir::FieldDecl>,
) -> Result<(), CodegenError> {
    let mut names = std::collections::BTreeSet::new();
    for alternative in alternatives {
        let name = ada_identifier(&alternative.name)?.to_ascii_lowercase();
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
        // Keep validation aligned with the Choice-qualified helper emitted below.
        ada_field_type(choice_name, alternative)?;
    }
    Ok(())
}

fn ada_field_type(
    choice_name: &str,
    field: &ams_gra_oms_ir::FieldDecl,
) -> Result<String, CodegenError> {
    match field.cardinality {
        Cardinality::REQUIRED_ONE => ada_field_base(field),
        Cardinality::OPTIONAL_ONE
            if field.type_ref.target == TypeRefTarget::Primitive(PrimitiveKind::String) =>
        {
            Ok("Optional_String".to_owned())
        }
        cardinality
            if repeated_max(cardinality).is_some()
                || matches!(cardinality.shape(), OccurrenceShape::Unbounded { min: 0 }) =>
        {
            Ok(format!(
                "{}_{}_Sequence",
                choice_name,
                ada_identifier(&field.name)?
            ))
        }
        _ => unsupported(format!("cardinality on Choice alternative {}", field.name)),
    }
}

fn render_unbounded_helper(
    output: &mut String,
    owner: &str,
    field: &ams_gra_oms_ir::FieldDecl,
) -> Result<(), CodegenError> {
    let field_name = ada_identifier(&field.name)?;
    let helper_name = format!("{owner}_{field_name}");
    let item_type = ada_field_base(field)?;
    let equality = if matches!(
        field.type_ref.target,
        TypeRefTarget::Primitive(
            PrimitiveKind::UnsignedInteger | PrimitiveKind::Float32 | PrimitiveKind::Float64
        )
    ) {
        ", \"=\" => Interfaces.\"=\""
    } else {
        ""
    };
    writeln!(
        output,
        "   subtype {helper_name}_Item is {item_type};\n\
         \x20  package {helper_name}_Vectors is new Ada.Containers.Vectors\n\
         \x20     (Index_Type => Natural, Element_Type => {helper_name}_Item{equality});\n\
         \x20  subtype {helper_name}_Sequence is {helper_name}_Vectors.Vector;\n"
    )
    .expect("writing to String cannot fail");
    Ok(())
}

fn has_zero_unbounded_occurrence(declaration: &TypeDecl) -> bool {
    match &declaration.kind {
        TypeKind::Record { fields } => fields,
        TypeKind::Choice { alternatives } => alternatives,
        _ => return false,
    }
    .iter()
    .any(|field| {
        matches!(
            field.cardinality.shape(),
            OccurrenceShape::Unbounded { min: 0 }
        )
    })
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
        TypeRefTarget::Primitive(PrimitiveKind::UnsignedInteger) => {
            Ok("Interfaces.Unsigned_64".to_owned())
        }
        TypeRefTarget::Primitive(PrimitiveKind::Boolean) => Ok("Boolean".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::Float32) => {
            Ok("Interfaces.IEEE_Float_32".to_owned())
        }
        TypeRefTarget::Primitive(PrimitiveKind::Float64) => {
            Ok("Interfaces.IEEE_Float_64".to_owned())
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

fn integral_domain(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
    name: &str,
) -> Result<Option<InclusiveIntegralDomain>, CodegenError> {
    inclusive_integral_domain(kind, constraints)
        .map_err(|reason| error(format!("unsupported Ada IR construct: {reason} on {name}")))
}

fn ada_field_base(field: &ams_gra_oms_ir::FieldDecl) -> Result<String, CodegenError> {
    match field.type_ref.target {
        TypeRefTarget::Primitive(
            kind @ (PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger),
        ) => match integral_domain(kind, &field.constraints, &field.name)? {
            Some(InclusiveIntegralDomain::Signed { min, max }) => Ok(format!(
                "Long_Long_Integer range {} .. {}",
                ada_number(i128::from(min)),
                ada_number(i128::from(max))
            )),
            Some(InclusiveIntegralDomain::Unsigned { min, max }) => {
                Ok(format!("Interfaces.Unsigned_64 range {min} .. {max}"))
            }
            None => ada_type(&field.type_ref),
        },
        _ => {
            reject_any_constraints(&field.constraints, &field.name)?;
            ada_type(&field.type_ref)
        }
    }
}

fn schema_needs_interfaces(schema: &SchemaIr) -> bool {
    schema.types.iter().any(|declaration| {
        matches!(
            declaration.kind,
            TypeKind::Primitive(
                PrimitiveKind::UnsignedInteger | PrimitiveKind::Float32 | PrimitiveKind::Float64
            )
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
                    TypeRefTarget::Primitive(
                        PrimitiveKind::UnsignedInteger
                            | PrimitiveKind::Float32
                            | PrimitiveKind::Float64
                    )
                )
            }),
            _ => false,
        })
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
    use ams_gra_oms_ir::NumericValue;
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
    fn rejects_positive_minimum_unbounded_cardinality() {
        let generation_error =
            generate(&unbounded_schema()).expect_err("Ada must preserve positive minimum");
        assert!(
            generation_error.message.contains("cardinality on"),
            "{}",
            generation_error.message
        );
    }

    #[test]
    fn lowers_integral_scalars_and_preserves_direct_ranges() {
        let source = generate(&integral_schema()).expect("integral scalars should generate");
        assert!(source.contains("with Interfaces;"));
        assert!(source.contains("Enabled : Boolean;"));
        assert!(source.contains("Byte_Value : Long_Long_Integer range -128 .. 127;"));
        assert!(source.contains("Unsigned_Byte_Value : Interfaces.Unsigned_64 range 0 .. 255;"));
        assert!(
            source.contains("array (Positive range 1 .. 8) of Long_Long_Integer range -128 .. 127")
        );
        assert!(source.contains("when True_Case_Kind =>\n            True_Case : Boolean;"));
        assert!(
            source.contains("subtype Unsigned_Bounded is Interfaces.Unsigned_64 range 1 .. 1000;")
        );
        assert!(source.contains("type Named_Boolean is new Boolean;"));
    }

    #[test]
    fn lowers_choice_as_discriminated_record_and_accepts_empty_record_ancestry() {
        let source = generate(&choice_schema()).expect("supported Choice should generate");
        assert!(
            source.contains("type Selection_Kind is\n      (First_Kind,\n       Second_Kind);")
        );
        assert!(source.contains("type Selection (Kind : Selection_Kind := First_Kind) is record"));
        assert!(source.contains("Selected : Selection;"));
        assert!(source.contains(
            "type DerivedSelection (Kind : DerivedSelection_Kind := Left_Kind) is record"
        ));
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
                .contains("duplicate Choice alternative identifier foo")
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
        assert!(
            source
                .contains("type Selection_Items_Array is array (Positive range 1 .. 3) of Token;")
        );
        assert!(source.contains("type Selection_Items_Sequence is record"));
        assert!(source.contains("Items : Selection_Items_Sequence;"));
    }

    #[test]
    fn lowers_effective_record_fields_and_omits_abstract_ancestor() {
        let source = generate(&inheritance_schema()).expect("pure Record inheritance is supported");
        assert!(!source.contains("type Base is record"));
        let leaf = source.find("type Leaf is record").unwrap();
        let fields = &source[leaf..];
        assert!(fields.find("Base_Optional").unwrap() < fields.find("Base_Values").unwrap());
        assert!(fields.find("Base_Values").unwrap() < fields.find("Linked").unwrap());
        assert!(fields.find("Linked").unwrap() < fields.find("Local").unwrap());
        assert!(source.contains("type Leaf_Base_Values_Sequence"));
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
        assert!(source.contains("type Track_Id"));
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
                "Interfaces.IEEE_Float_32"
            } else {
                "Interfaces.IEEE_Float_64"
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
        assert!(error.message.contains("unsupported Ada IR construct"));
    }

    #[test]
    fn temporal_and_constrained_scalars_fail_explicitly() {
        for kind in [PrimitiveKind::Time, PrimitiveKind::Duration] {
            let mut schema = track_schema();
            let TypeKind::Record { fields } = &mut schema.types[2].kind else {
                panic!("track fixture should contain a record");
            };
            fields[0].type_ref = TypeRef::primitive(kind);
            let error = generate(&schema).expect_err("temporal generation must remain unsupported");
            assert!(error.message.contains(&format!(
                "unsupported Ada IR construct: type reference Primitive({kind:?})"
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
                    .contains("unsupported Ada IR construct: constraints on")
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
                    .contains("unsupported Ada IR construct: constraints on")
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
                .contains("unsupported Ada IR construct: abstract type")
        );
    }
}
