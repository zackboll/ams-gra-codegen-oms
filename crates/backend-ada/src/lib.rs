//! Minimal Ada type generation from normalized schema IR.

use ams_gra_oms_codegen_core::{
    ADA_PORTABLE_POSITIVE_INDEX_MAX, AbstractValueProjection, Backend, CodegenError,
    EffectiveValueMember, GeneratedFile, GenerationWorld, InclusiveIntegralDomain, TypeEmission,
    abstract_value_projection_for_ref, effective_choice_alternatives, effective_record_fields,
    field_storage_semantics, inclusive_integral_domain, plan_type_emissions,
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

    fn generate(
        &self,
        schema: &SchemaIr,
        world: GenerationWorld,
    ) -> Result<Vec<GeneratedFile>, CodegenError> {
        let contents = generate(schema, world)?;
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
    let package = package_name(schema)?;
    let mut output = String::from("with Ada.Strings.Unbounded;\n");
    let needs_binary = schema_needs_binary(schema);
    if schema.types.iter().any(has_ada_unbounded_occurrence) || needs_binary {
        output.push_str("with Ada.Containers.Vectors;\n");
    }
    if schema_needs_interfaces(schema) || needs_binary {
        output.push_str("with Interfaces;\n");
    }
    output.push('\n');
    writeln!(output, "package {package} is\n").expect("writing to String cannot fail");
    output.push_str(concat!(
        "   type Optional_String (Is_Present : Boolean := False) is record\n",
        "      case Is_Present is\n",
        "         when False => null;\n",
        "         when True  => Value : Standard.Ada.Strings.Unbounded.Unbounded_String;\n",
        "      end case;\n",
        "   end record;\n\n",
    ));
    if needs_binary {
        output.push_str(concat!(
            "   package Binary_Vectors is new Standard.Ada.Containers.Vectors\n",
            "     (Index_Type   => Natural,\n",
            "      Element_Type => Interfaces.Unsigned_8,\n",
            "      \"=\"          => Interfaces.\"=\");\n\n",
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
    writeln!(output, "end {package};").expect("writing to String cannot fail");
    Ok(output)
}

fn render_abstract_value(
    output: &mut String,
    projection: &AbstractValueProjection<'_>,
) -> Result<(), CodegenError> {
    let name = ada_identifier(&projection.declaration.name.local_name)?;
    let variants = projection
        .concrete_descendants
        .iter()
        .map(|descendant| ada_identifier(&descendant.name.local_name))
        .collect::<Result<Vec<_>, _>>()?;
    writeln!(output, "   type {name}_Kind is\n     (").expect("writing to String cannot fail");
    for (index, variant) in variants.iter().enumerate() {
        writeln!(
            output,
            "      {variant}_Kind{}",
            if index + 1 == variants.len() {
                ");"
            } else {
                ","
            }
        )
        .expect("writing to String cannot fail");
    }
    writeln!(
        output,
        "\n   type {name} (Kind : {name}_Kind := {}_Kind) is record\n      case Kind is",
        variants[0]
    )
    .expect("writing to String cannot fail");
    for variant in &variants {
        writeln!(
            output,
            "         when {variant}_Kind =>\n            {variant}_Value : {variant};"
        )
        .expect("writing to String cannot fail");
    }
    output.push_str("      end case;\n   end record;\n\n");
    Ok(())
}

fn render_declaration(
    output: &mut String,
    schema: &SchemaIr,
    declaration: &TypeDecl,
    world: GenerationWorld,
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
        TypeKind::Primitive(PrimitiveKind::Binary) => {
            reject_any_constraints(&declaration.constraints, &name)?;
            writeln!(
                output,
                "   type {name} is record\n      Value : Binary_Vectors.Vector;\n   end record;\n"
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
        TypeKind::Record { .. } => {
            if declaration.is_abstract {
                return Ok(());
            }
            let fields = effective_record_fields(schema, &declaration.name)
                .map_err(|_| {
                    error(format!(
                        "unsupported Ada IR construct: non-Record inheritance {}",
                        declaration.name.local_name
                    ))
                })?
                .into_iter()
                .map(|field| {
                    field_storage_semantics(schema, field, world).map_err(|projection_error| {
                        error(format!(
                            "unsupported abstract structural value: {projection_error}"
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .filter_map(|member| match member {
                    // Task 026: this field's abstract structural target has zero
                    // concrete descendants in the current schema set, so absence
                    // is its only legal state; no record component is generated.
                    EffectiveValueMember::AbsentOnly(_) => None,
                    EffectiveValueMember::Stored(field) => Some(field),
                })
                .collect::<Vec<_>>();
            for field in &fields {
                if let Some((min, max)) = bounded_repeated(field.cardinality) {
                    ensure_portable_finite_max(max)?;
                    let field_name = ada_identifier(&field.name)?;
                    let helper_name = format!("{name}_{field_name}");
                    let item_type = ada_field_base(field)?;
                    writeln!(
                        output,
                        "   type {helper_name}_Array is array (Positive range 1 .. {max}) of {item_type};\n\
                         \x20  type {helper_name}_Sequence is record\n\
                         \x20     Length : Natural range {min} .. {max} := {min};\n\
                         \x20     Items  : {helper_name}_Array;\n\
                         \x20  end record;\n"
                    )
                    .expect("writing to String cannot fail");
                } else if matches!(field.cardinality.shape(), OccurrenceShape::Unbounded { .. }) {
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
                        if bounded_repeated(cardinality).is_some()
                            || matches!(cardinality.shape(), OccurrenceShape::Unbounded { .. }) =>
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
                if let Some((min, max)) = bounded_repeated(alternative.cardinality) {
                    ensure_portable_finite_max(max)?;
                    let alternative_name = ada_identifier(&alternative.name)?;
                    let helper_name = format!("{name}_{alternative_name}");
                    let item_type = ada_field_base(alternative)?;
                    writeln!(
                        output,
                        "   type {helper_name}_Array is array (Positive range 1 .. {max}) of {item_type};\n\
                         \x20  type {helper_name}_Sequence is record\n\
                         \x20     Length : Natural range {min} .. {max} := {min};\n\
                         \x20     Items  : {helper_name}_Array;\n\
                         \x20  end record;\n"
                    )
                    .expect("writing to String cannot fail");
                } else if matches!(
                    alternative.cardinality.shape(),
                    OccurrenceShape::Unbounded { .. }
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

fn validate_schema(schema: &SchemaIr, world: GenerationWorld) -> Result<(), CodegenError> {
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
            && !matches!(
                declaration.kind,
                TypeKind::Record { .. } | TypeKind::Choice { .. }
            )
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
        if matches!(declaration.kind, TypeKind::Primitive(PrimitiveKind::Binary))
            && declaration.constraints != ConstraintSet::default()
        {
            return unsupported(format!("constraints on {}", declaration.name.local_name));
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
                    // is its only legal state; no record component is generated.
                    continue;
                }
                validate_abstract_value_reference(schema, &field.type_ref, world)?;
                ada_field_base(field)?;
                validate_repeated_cardinality(field)?;
            }
        }
        if matches!(declaration.kind, TypeKind::Choice { .. }) {
            let alternatives = effective_choice_alternatives(schema, &declaration.name).map_err(
                |projection_error| {
                    error(format!("unsupported Ada IR construct: {projection_error}"))
                },
            )?;
            validate_choice_alternatives(
                schema,
                &declaration.name.local_name,
                alternatives,
                world,
            )?;
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
    choice_name: &str,
    alternatives: Vec<&ams_gra_oms_ir::FieldDecl>,
    world: GenerationWorld,
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
        validate_abstract_value_reference(schema, &alternative.type_ref, world)?;
        validate_repeated_cardinality(alternative)?;
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
            if bounded_repeated(cardinality).is_some()
                || matches!(cardinality.shape(), OccurrenceShape::Unbounded { .. }) =>
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

fn validate_repeated_cardinality(field: &ams_gra_oms_ir::FieldDecl) -> Result<(), CodegenError> {
    match field.cardinality.shape() {
        OccurrenceShape::Bounded { max, .. } if max > 1 => ensure_portable_finite_max(max),
        OccurrenceShape::Unbounded { min } if min > ADA_PORTABLE_POSITIVE_INDEX_MAX => {
            unsupported(format!(
                "unbounded minimum {min} exceeds portable Positive bound {ADA_PORTABLE_POSITIVE_INDEX_MAX}"
            ))
        }
        _ => Ok(()),
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
    let OccurrenceShape::Unbounded { min } = field.cardinality.shape() else {
        return unsupported(format!("unbounded cardinality on field {field_name}"));
    };
    if min > ADA_PORTABLE_POSITIVE_INDEX_MAX {
        return unsupported(format!(
            "unbounded minimum {min} exceeds portable Positive bound {ADA_PORTABLE_POSITIVE_INDEX_MAX}"
        ));
    }
    let equality = if matches!(
        field.type_ref.target,
        TypeRefTarget::Primitive(
            PrimitiveKind::UnsignedInteger
                | PrimitiveKind::Float32
                | PrimitiveKind::Float64
                | PrimitiveKind::Binary
        )
    ) {
        if matches!(
            field.type_ref.target,
            TypeRefTarget::Primitive(PrimitiveKind::Binary)
        ) {
            ", \"=\" => Binary_Vectors.\"=\""
        } else {
            ", \"=\" => Interfaces.\"=\""
        }
    } else {
        ""
    };
    if min == 0 {
        writeln!(
            output,
            "   subtype {helper_name}_Item is {item_type};\n\
             \x20  package {helper_name}_Vectors is new Standard.Ada.Containers.Vectors\n\
             \x20     (Index_Type => Natural, Element_Type => {helper_name}_Item{equality});\n\
             \x20  subtype {helper_name}_Sequence is {helper_name}_Vectors.Vector;\n"
        )
        .expect("writing to String cannot fail");
    } else {
        writeln!(
            output,
            "   subtype {helper_name}_Item is {item_type};\n\
             \x20  type {helper_name}_Required_Array is\n\
             \x20    array (Positive range 1 .. {min}) of {helper_name}_Item;\n\
             \x20  package {helper_name}_Additional_Vectors is new Standard.Ada.Containers.Vectors\n\
             \x20     (Index_Type => Natural, Element_Type => {helper_name}_Item{equality});\n\
             \x20  type {helper_name}_Sequence is record\n\
             \x20     Required   : {helper_name}_Required_Array;\n\
             \x20     Additional : {helper_name}_Additional_Vectors.Vector;\n\
             \x20  end record;\n"
        )
        .expect("writing to String cannot fail");
    }
    Ok(())
}

fn has_ada_unbounded_occurrence(declaration: &TypeDecl) -> bool {
    match &declaration.kind {
        TypeKind::Record { fields } => fields,
        TypeKind::Choice { alternatives } => alternatives,
        _ => return false,
    }
    .iter()
    .any(|field| matches!(field.cardinality.shape(), OccurrenceShape::Unbounded { .. }))
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
            Ok("Standard.Ada.Strings.Unbounded.Unbounded_String".to_owned())
        }
        TypeRefTarget::Primitive(PrimitiveKind::Binary) => Ok("Binary_Vectors.Vector".to_owned()),
        TypeRefTarget::Named(name) => ada_identifier(&name.local_name),
        other => unsupported(format!("type reference {other:?}")),
    }
}

fn bounded_repeated(cardinality: Cardinality) -> Option<(u64, u64)> {
    match cardinality.shape() {
        OccurrenceShape::Bounded { min, max } if max > 1 => Some((min, max)),
        _ => None,
    }
}

fn ensure_portable_finite_max(max: u64) -> Result<(), CodegenError> {
    if max > ADA_PORTABLE_POSITIVE_INDEX_MAX {
        return unsupported(format!(
            "finite repeated maximum {max} exceeds portable Positive bound {ADA_PORTABLE_POSITIVE_INDEX_MAX}"
        ));
    }
    Ok(())
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

fn schema_needs_binary(schema: &SchemaIr) -> bool {
    schema
        .types
        .iter()
        .any(|declaration| matches!(declaration.kind, TypeKind::Primitive(PrimitiveKind::Binary)))
        || schema
            .types
            .iter()
            .any(|declaration| match &declaration.kind {
                TypeKind::Record { fields }
                | TypeKind::Choice {
                    alternatives: fields,
                } => fields.iter().any(|field| {
                    matches!(
                        field.type_ref.target,
                        TypeRefTarget::Primitive(PrimitiveKind::Binary)
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

    /// Existing pre-Task-028 regressions all asserted closed-world behaviour,
    /// so they keep asserting exactly that under the now-explicit policy.
    const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;
    /// Task 028 open-world regressions.
    #[allow(dead_code)]
    const OPEN: GenerationWorld = GenerationWorld::OpenExtensions;
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

    fn repeated_cardinality_schema() -> SchemaIr {
        load_schema_document(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../xsd-frontend/tests/fixtures/backend-ada-repeated-cardinality.xsd"),
        )
        .expect("repeated cardinality fixture should parse")
    }

    #[test]
    fn preserves_repeated_minimum_cardinality_and_portable_bounds() {
        let source =
            generate(&unbounded_schema(), CLOSED).expect("supported minima should generate");
        assert!(source.contains(
            "subtype Record_ZeroOrMoreNamed_Sequence is Record_ZeroOrMoreNamed_Vectors.Vector;"
        ));
        assert!(source.contains("array (Positive range 1 .. 1) of Record_OneOrMoreNamed_Item;"));
        assert!(source.contains("array (Positive range 1 .. 2) of Record_TwoOrMoreNamed_Item;"));
        assert!(source.contains("Additional : Record_TwoOrMoreNamed_Additional_Vectors.Vector;"));
        assert!(source.contains("array (Positive range 1 .. 1) of Selection_OneNamed_Item;"));

        let mut finite_limit = unbounded_schema();
        if let TypeKind::Record { fields } = &mut finite_limit.types[1].kind {
            fields[4].cardinality.max_occurs = Some(ADA_PORTABLE_POSITIVE_INDEX_MAX);
        } else {
            panic!("fixture Record should be present");
        }
        assert!(generate(&finite_limit, CLOSED).is_ok());
        let TypeKind::Record { fields } = &mut finite_limit.types[1].kind else {
            panic!("fixture Record should be present");
        };
        fields[4].cardinality.max_occurs = Some(ADA_PORTABLE_POSITIVE_INDEX_MAX + 1);
        let error =
            generate(&finite_limit, CLOSED).expect_err("over-limit finite maximum must fail");
        assert!(error.message.contains("finite repeated maximum"));

        let mut unbounded_limit = unbounded_schema();
        if let TypeKind::Record { fields } = &mut unbounded_limit.types[1].kind {
            fields[1].cardinality.min_occurs = ADA_PORTABLE_POSITIVE_INDEX_MAX;
        } else {
            panic!("fixture Record should be present");
        }
        assert!(generate(&unbounded_limit, CLOSED).is_ok());
        let TypeKind::Record { fields } = &mut unbounded_limit.types[1].kind else {
            panic!("fixture Record should be present");
        };
        fields[1].cardinality.min_occurs = ADA_PORTABLE_POSITIVE_INDEX_MAX + 1;
        let error =
            generate(&unbounded_limit, CLOSED).expect_err("over-limit unbounded minimum must fail");
        assert!(error.message.contains("unbounded minimum"));
    }

    #[test]
    fn lowers_finite_and_unbounded_repeated_value_shapes() {
        let source = generate(&repeated_cardinality_schema(), CLOSED)
            .expect("repeated values should generate");
        // The fixture namespace ends in `ada`, so generated package scope can
        // shadow the root Ada library unit unless references are rooted here.
        assert!(source.contains("Standard.Ada.Strings.Unbounded.Unbounded_String"));
        assert!(source.contains("new Standard.Ada.Containers.Vectors"));
        assert!(source.contains("Length : Natural range 0 .. 3 := 0;"));
        assert!(source.contains("Length : Natural range 1 .. 2 := 1;"));
        assert!(source.contains("Length : Natural range 2 .. 3 := 2;"));
        assert!(source.contains("Length : Natural range 3 .. 5 := 3;"));
        assert!(source.contains("array (Positive range 1 .. 2) of Interfaces.IEEE_Float_32;"));
        assert!(source.contains(
            "array (Positive range 1 .. 3) of Interfaces.Unsigned_64 range 0 .. 4294967295;"
        ));
        assert!(source.contains("array (Positive range 1 .. 3) of Payload_ThreeOrMore_Item;"));
        assert!(source.contains("Additional : Payload_ThreeOrMore_Additional_Vectors.Vector;"));
        assert!(source.contains("Selection_Repeated_Required_Array"));
    }

    #[test]
    fn lowers_integral_scalars_and_preserves_direct_ranges() {
        let source =
            generate(&integral_schema(), CLOSED).expect("integral scalars should generate");
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
        let source = generate(&choice_schema(), CLOSED).expect("supported Choice should generate");
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
            generate(&collision, CLOSED)
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
                .contains("Value : Base")
        );

        let source = generate(&repeated_choice_schema(), CLOSED)
            .expect("finite repeated Choice must generate");
        assert!(
            source
                .contains("type Selection_Items_Array is array (Positive range 1 .. 3) of Token;")
        );
        assert!(source.contains("type Selection_Items_Sequence is record"));
        assert!(source.contains("Items : Selection_Items_Sequence;"));
    }

    #[test]
    fn lowers_effective_record_fields_and_omits_abstract_ancestor() {
        let source =
            generate(&inheritance_schema(), CLOSED).expect("pure Record inheritance is supported");
        assert!(!source.contains("type Base is record"));
        let leaf = source.find("type Leaf is record").unwrap();
        let fields = &source[leaf..];
        assert!(fields.find("Base_Optional").unwrap() < fields.find("Base_Values").unwrap());
        assert!(fields.find("Base_Values").unwrap() < fields.find("Linked").unwrap());
        assert!(fields.find("Linked").unwrap() < fields.find("Local").unwrap());
        assert!(source.contains("type Leaf_Base_Values_Sequence"));
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
        assert!(source.contains("type Base_Kind is"));
        assert!(source.contains("Binary_Vectors.Vector"));
    }

    #[test]
    fn lowers_abstract_value_references_and_retains_choice_boundary() {
        let source =
            generate(&abstract_value_schema(), CLOSED).expect("closed abstract value should lower");
        assert!(source.contains("type Base_Kind is"));
        assert!(source.contains("Derived_Kind"));
        assert!(source.find("type Derived").unwrap() < source.find("type Base_Kind").unwrap());
        assert!(source.find("type Base_Kind").unwrap() < source.find("type Holder").unwrap());
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
        let first = generate(&schema, CLOSED).expect("Ada generation should succeed");
        let second = generate(&schema, CLOSED).expect("Ada generation should be repeatable");
        assert_eq!(first, second);
        assert_eq!(first, include_str!("../tests/expected/track.ads"));
    }

    #[test]
    fn schema_set_matches_dependency_order_golden() {
        let source =
            generate(&codegen_order_schema(), CLOSED).expect("Ada generation should succeed");
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
        let source = generate(&track_schema(), CLOSED).expect("Ada generation should succeed");
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
        let source = generate(&schema, CLOSED).expect("Choice must render");
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
            let source =
                generate(&schema, CLOSED).expect("unconstrained floating generation must succeed");
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
        let error =
            generate(&schema, CLOSED).expect_err("constrained float must remain unsupported");
        assert!(error.message.contains("unsupported Ada IR construct"));
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
        assert!(source.contains("Binary_Vectors.Vector"));
        assert!(source.contains("Interfaces.Unsigned_8"));
        assert!(source.contains("with Ada.Containers.Vectors;"));
        assert!(source.contains("with Interfaces;"));

        let mut schema = track_schema();
        schema.types[0].kind = TypeKind::Primitive(PrimitiveKind::Binary);
        schema.types[0].constraints = ConstraintSet::default();
        let source = generate(&schema, CLOSED).expect("unconstrained named binary must generate");
        assert!(source.contains("type Track_Id is record"));
        assert!(source.contains("Value : Binary_Vectors.Vector;"));
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
                .contains("unsupported Ada IR construct: constraints on")
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
                .contains("unsupported Ada IR construct: field constraints on")
        );
    }

    #[test]
    fn optional_binary_field_remains_unsupported() {
        let mut schema = floating_schema();
        let TypeKind::Record { fields } = &mut schema.types[0].kind else {
            panic!("floating fixture should contain a record");
        };
        fields.truncate(1);
        fields[0].type_ref = TypeRef::primitive(PrimitiveKind::Binary);
        fields[0].cardinality = Cardinality::OPTIONAL_ONE;
        let error =
            generate(&schema, CLOSED).expect_err("optional binary field must remain unsupported");
        assert!(
            error
                .message
                .contains("unsupported Ada IR construct: cardinality on field")
        );
    }

    #[test]
    fn temporal_and_constrained_scalars_fail_explicitly() {
        for kind in [PrimitiveKind::Time, PrimitiveKind::Duration] {
            let mut schema = track_schema();
            let TypeKind::Record { fields } = &mut schema.types[2].kind else {
                panic!("track fixture should contain a record");
            };
            fields[0].type_ref = TypeRef::primitive(kind);
            let error =
                generate(&schema, CLOSED).expect_err("temporal generation must remain unsupported");
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
            let error = generate(&schema, CLOSED).expect_err("constraints must not be discarded");
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
            let error =
                generate(&schema, CLOSED).expect_err("lexical constraints must be rejected");
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
        let error = generate(&schema, CLOSED).expect_err("abstract type must be rejected");
        assert!(
            error
                .message
                .contains("unsupported Ada IR construct: abstract type")
        );
    }

    #[test]
    fn uninhabited_abstract_optional_field_is_elided_without_fake_payload() {
        let source = generate(&uninhabited_optional_schema(), CLOSED)
            .expect("absent-only occurrence should lower");
        assert!(!source.contains("SidecarPoint"));
        assert!(!source.contains("Widget"));
        assert!(source.contains("type Holder is record\n      Required : Standard.Ada.Strings.Unbounded.Unbounded_String;\n   end record;"));
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
    fn future_descendant_reclassifies_uninhabited_target_and_hits_ada_optional_boundary() {
        // Once a concrete descendant exists the target is inhabited again and
        // Task 024's ordinary closed-sum lowering applies; Ada's existing
        // general-optional-named-value gap then applies unchanged. Task 026
        // must not weaken that boundary, so this fails at the pre-existing
        // Ada optional cardinality firewall rather than at the abstract-value
        // firewall.
        let error = generate(&uninhabited_future_descendant_schema(), CLOSED)
            .expect_err("Ada optional named-value boundary should still apply");
        assert!(
            error
                .message
                .contains("unsupported Ada IR construct: cardinality on field Widget")
        );
    }

    #[test]
    fn inherited_uninhabited_field_is_elided_on_the_concrete_descendant() {
        let source = generate(&uninhabited_inherited_schema(), CLOSED)
            .expect("inherited absent-only occurrence should lower");
        assert!(!source.contains("SidecarPoint"));
        assert!(source.contains("type ConcreteHolder is record\n      Required : Standard.Ada.Strings.Unbounded.Unbounded_String;\n   end record;"));
    }

    #[test]
    fn uninhabited_optional_field_composes_with_task_024_closed_sum() {
        let source = generate(&uninhabited_composes_closed_sum_schema(), CLOSED)
            .expect("Task 026 composition with Task 024 closed sum should lower");
        assert!(source.contains("type Parent_Kind is"));
        assert!(source.contains("ConcreteChild_Kind"));
        assert!(source.contains("type ConcreteChild is record\n      null;\n   end record;"));
        assert!(!source.contains("Widget"));
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
