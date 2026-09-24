//! Minimal Ada type generation from normalized schema IR.

use ams_gra_oms_codegen_core::{
    ADA_PORTABLE_POSITIVE_INDEX_MAX, AbstractValueProjection, Backend, BackendLanguage,
    CodegenError, EffectiveValueMember, FloatingDomain, GeneratedFile, GenerationWorld,
    InclusiveIntegralDomain, StringProfile, TemporalProfile, TypeEmission,
    abstract_value_projection_for_ref, ada_record_field_uses_optional_wrapper, backend_preflight,
    constrains_string, effective_choice_alternatives, effective_record_fields,
    field_storage_semantics, float32_literal, float64_literal, floating_domain,
    inclusive_integral_domain, is_temporal_primitive, plan_type_emissions,
    schema_emits_ada_binary_vectors, schema_emits_string_profile_carrier,
    schema_emits_temporal_carrier, schema_emits_unbounded_sequence_support, string_profile,
    temporal_profile,
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
        // Task 036: a package body is emitted only when some declaration
        // actually needs one, so a schema with no body-requiring feature keeps
        // its existing single-`.ads` file set and no empty `.adb` appears.
        if let Some(body) = generate_body(schema, world)? {
            files.push(GeneratedFile {
                relative_path: PathBuf::from(format!("{file_stem}.adb")),
                contents: body,
            });
        }
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
    // Task 033: predicate checks follow the assertion policy in force where a
    // conversion is written, so the generated spec states its own policy. It is
    // emitted only when some declaration actually carries a floating predicate,
    // which keeps Task 022 unconstrained output byte-identical, and it is a
    // configuration pragma on this unit alone -- no repository-wide compiler
    // flag is involved.
    let mut output = String::new();
    if schema_has_constrained_floating(schema) {
        output.push_str("pragma Assertion_Policy (Dynamic_Predicate => Check);\n\n");
    }
    output.push_str("with Ada.Strings.Unbounded;\n");
    let needs_binary = schema_emits_ada_binary_vectors(schema);
    if schema_emits_unbounded_sequence_support(schema) || needs_binary {
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
    // Task 033 correction: a constrained floating type is declared `private`
    // in the visible part and completed here, so the completion -- the derived
    // IEEE type, its predicate, and the checked conversions -- is never
    // nameable by a client. Only constrained floats contribute, so a schema
    // without them produces no `private` part at all and Task 022 output stays
    // byte-identical.
    let mut private_part = String::new();
    // Task 036: the temporal validator is a real algorithm rather than an
    // expression function, so it needs a package body. The body text is
    // accumulated here and emitted only if some declaration actually produced
    // one, which keeps every other schema's single-`.ads` output unchanged.
    let mut body = String::new();
    for emission in emissions {
        match emission {
            TypeEmission::Declaration(declaration) => render_declaration(
                &mut output,
                &mut private_part,
                &mut body,
                schema,
                declaration,
                world,
            )?,
            TypeEmission::AbstractValue(projection) => {
                render_abstract_value(&mut output, &projection)?
            }
        }
    }
    if !private_part.is_empty() {
        output.push_str("private\n\n");
        output.push_str(&private_part);
    }
    writeln!(output, "end {package};").expect("writing to String cannot fail");
    Ok(output)
}

/// Generate the Ada package **body**, when the schema needs one.
///
/// Returns `None` when no declaration requires a body, so a schema without a
/// Task 036 feature keeps its existing generated file set exactly. An empty
/// `.adb` is never emitted.
///
/// # Errors
///
/// Returns an error for the same IR constructs [`generate`] rejects; the two
/// are always called on the same schema and agree by construction.
pub fn generate_body(
    schema: &SchemaIr,
    world: GenerationWorld,
) -> Result<Option<String>, CodegenError> {
    // Task 037 generalizes this predicate rather than adding a second
    // body-generation mechanism: a schema needs a body when it emits *any*
    // validator-backed carrier. A schema requiring neither keeps its existing
    // single-`.ads` output exactly.
    if !schema_emits_temporal_carrier(schema) && !schema_emits_string_profile_carrier(schema) {
        return Ok(None);
    }
    validate_schema(schema, world)?;
    let emissions = plan_type_emissions(schema, world)?;
    let package = package_name(schema)?;
    let mut discard_spec = String::new();
    let mut discard_private = String::new();
    let mut body = String::new();
    for emission in emissions {
        if let TypeEmission::Declaration(declaration) = emission {
            render_declaration(
                &mut discard_spec,
                &mut discard_private,
                &mut body,
                schema,
                declaration,
                world,
            )?;
        }
    }
    if body.is_empty() {
        return Ok(None);
    }
    let mut output = String::new();
    writeln!(output, "package body {package} is\n").expect("writing to String cannot fail");
    output.push_str(&body);
    writeln!(output, "end {package};").expect("writing to String cannot fail");
    Ok(Some(output))
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
    private_part: &mut String,
    body: &mut String,
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
        TypeKind::Primitive(kind @ (PrimitiveKind::Float32 | PrimitiveKind::Float64)) => {
            render_floating_declaration(
                output,
                private_part,
                *kind,
                &declaration.constraints,
                &name,
            )?;
        }
        TypeKind::Primitive(
            kind @ (PrimitiveKind::DateTime | PrimitiveKind::Time | PrimitiveKind::Duration),
        ) => {
            render_temporal_declaration(
                output,
                private_part,
                body,
                *kind,
                &declaration.constraints,
                &name,
            )?;
        }
        // Task 037: a *constrained* named String is routed to the shared
        // classifier. An unconstrained one is deliberately not handled here and
        // falls through to the existing generic path, so ordinary
        // `Unbounded_String` output is byte-for-byte unchanged.
        TypeKind::Primitive(PrimitiveKind::String)
            if constrains_string(&declaration.constraints) =>
        {
            render_string_profile_declaration(
                output,
                private_part,
                body,
                &declaration.constraints,
                &name,
            )?;
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
                } else if ada_record_field_uses_optional_wrapper(field) {
                    render_optional_helper(output, &name, field)?;
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
                    // Task 034/035: a non-nillable optional named value, or a
                    // direct primitive whose value representation this backend
                    // already implements, is stored in this field's own
                    // generated discriminated wrapper, emitted just above under
                    // the same emitted owner.
                    Cardinality::OPTIONAL_ONE if ada_record_field_uses_optional_wrapper(field) => {
                        format!("{name}_{field_name}_Optional")
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
    // Shared global preflight: the single-namespace boundary and generated
    // host-language name safety, including Ada's case-insensitive identity and
    // the flat-package helper type names derived from member names.
    // Capability/readiness analysis consults the same rules, so a READY verdict
    // cannot disagree with what happens here.
    if let Err(preflight) = backend_preflight(schema, BackendLanguage::Ada, world) {
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
                    "unsupported Ada IR construct: {reason} on {}",
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
        // Task 037: a named constrained String declaration is classified by the
        // shared helper. Only the authoritative UCI schema-version profile is
        // lowered; every other constrained String shape still fails closed,
        // here, before any output is produced. An *unconstrained* String is not
        // a Task 037 declaration and is untouched by this branch.
        if let TypeKind::Primitive(kind @ PrimitiveKind::String) = declaration.kind
            && constrains_string(&declaration.constraints)
        {
            match string_profile(kind, &declaration.constraints) {
                Ok(Some(StringProfile::UciSchemaVersion))
                | Ok(Some(StringProfile::UniversallyUniqueIdentifier))
                | Ok(Some(StringProfile::VisibleAscii { .. })) => {}
                Ok(None) => unreachable!("constrains_string gates this branch"),
                Err(reason) => {
                    return unsupported(format!("{reason} on {}", declaration.name.local_name));
                }
            }
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
                // Task 034: nillability is a third state -- present-and-nil --
                // that neither the existing storage shapes nor the new
                // optional wrapper can express, so it fails closed here
                // exactly as it already does in backend-rust and backend-cpp.
                // Coverage has always treated a nillable field as
                // unrenderable; before Task 034 an Ada nillable *named*
                // `0..1` field was rejected only incidentally, by the
                // cardinality arm this task replaces, so the rule is now
                // stated directly rather than relying on that side effect.
                if field.nillable {
                    return unsupported(format!("nillable field {}", field.name));
                }
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
    // Alternative-name collisions are no longer checked here: the shared
    // backend name preflight owns that policy for every generated region --
    // including Ada's case-insensitive identity -- so keeping a second
    // Ada-local copy would let the two drift.
    for alternative in alternatives {
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

/// Emit the Task 034 optional wrapper for one field of one emitted owner.
///
/// The representation is an Ada discriminated record whose discriminant *is*
/// the presence flag:
///
/// ```ada
/// type Owner_Field_Optional (Is_Present : Boolean := False) is record
///    case Is_Present is
///       when False => null;
///       when True  => Value : Target;
///    end case;
/// end record;
/// ```
///
/// Properties this deliberately has, and which future Phase 4 SPARK work is
/// meant to be able to rely on: the discriminant alone decides whether `Value`
/// exists, so absence and presence are explicit, distinguishable states; there
/// is no access type, no heap allocation introduced by the wrapper itself, no
/// unchecked conversion, and no in-band sentinel standing in for absence.
/// Reading `Value` when `Is_Present` is `False` is a `Constraint_Error`, not a
/// silently wrong value. No proof annotation is added by this task.
///
/// The wrapper is generated **per emitted field** rather than from a shared
/// generic runtime Optional. That keeps the task self-contained and
/// allocation-free beyond whatever `Target` itself owns, works even when
/// `Target` is one of this backend's `private` or otherwise constrained
/// generated types, and avoids committing to a runtime package that does not
/// exist yet.
///
/// `owner` is the **emitted** declaration's Ada name. For a field inherited
/// from a non-emitted abstract ancestor that is the concrete descendant, which
/// is the scope where the component is really rendered.
fn render_optional_helper(
    output: &mut String,
    owner: &str,
    field: &ams_gra_oms_ir::FieldDecl,
) -> Result<(), CodegenError> {
    let field_name = ada_identifier(&field.name)?;
    let value_type = ada_field_base(field)?;
    writeln!(
        output,
        "   type {owner}_{field_name}_Optional (Is_Present : Boolean := False) is record\n\
         \x20     case Is_Present is\n\
         \x20        when False => null;\n\
         \x20        when True  => Value : {value_type};\n\
         \x20     end case;\n\
         \x20  end record;\n"
    )
    .expect("writing to String cannot fail");
    Ok(())
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

/// Render a named Float32/Float64 declaration.
///
/// Unconstrained output is exactly Task 022's: a plain derived IEEE type, with
/// no predicate and no assertion pragma, because nothing needs checking.
///
/// A Task 033 bound-only declaration is lowered as a **private** type. The
/// visible part shows only `type T is private`, `Create`, and `Value`; the
/// derived IEEE type and its `Dynamic_Predicate` live in the private part.
///
/// The private representation is the Task 033 correction. A publicly derived
/// numeric type left two ordinary client paths that manufactured invalid
/// values with no diagnostic at all when the client was compiled without
/// `-gnata`: the direct conversion `T (0.0)`, and inherited arithmetic such as
/// `A + B` returning `T` after two valid constructions. Hiding the derivation
/// removes both from the public surface -- they become compile errors, not
/// runtime surprises -- so the invariant no longer depends on either the
/// client's assertion policy or the client's restraint.
///
/// No public arithmetic is re-exported. Checked operations over constrained
/// wrappers, if ever wanted, are a deliberate later design.
///
/// The predicate is deliberately **not** an Ada `range` subtype. A finite
/// `range` would exclude `+Infinity` from a lower-only XSD constraint that
/// actually admits it, silently narrowing the schema's domain; a predicate
/// expressed as ordinary comparisons keeps IEEE semantics exactly -- NaN fails
/// every bound, one-sided infinities behave normally, and both zero signs
/// compare equal.
///
/// `Create` is the single checked construction boundary. Predicate
/// enforcement follows the `Assertion_Policy` in force **where the conversion
/// is written**, not where the type is declared. Because `Create`'s
/// expression-function completion is written inside this spec, under the
/// spec's own `pragma Assertion_Policy (Dynamic_Predicate => Check)`, the
/// conversion sits on the generated side of that boundary and is checked
/// regardless of the client's flags. That pragma is emitted once per package
/// (see `generate`) and affects only generated units -- no repository-wide
/// compiler flag is changed.
///
/// `Value` is read-only extraction of the underlying IEEE scalar; it cannot
/// construct.
/// Whether any named declaration will emit a floating `Dynamic_Predicate`.
///
/// Only a *supported* bound-only domain counts. An unsupported facet shape is
/// rejected by `validate_schema` before any output is produced, so it can never
/// reach rendering, and a declaration whose classification errors here must not
/// cause a pragma to be emitted for output that will not exist.
fn schema_has_constrained_floating(schema: &SchemaIr) -> bool {
    schema.types.iter().any(|declaration| {
        matches!(
            declaration.kind,
            TypeKind::Primitive(kind @ (PrimitiveKind::Float32 | PrimitiveKind::Float64))
                if floating_domain(kind, &declaration.constraints)
                    .is_ok_and(|domain| domain.is_some())
        )
    })
}

fn render_floating_declaration(
    output: &mut String,
    private_part: &mut String,
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
    name: &str,
) -> Result<(), CodegenError> {
    let base = if kind == PrimitiveKind::Float32 {
        "Interfaces.IEEE_Float_32"
    } else {
        "Interfaces.IEEE_Float_64"
    };
    let Some(domain) = floating_domain(kind, constraints)
        .map_err(|reason| error(format!("unsupported Ada IR construct: {reason} on {name}")))?
    else {
        // Task 022 output, unchanged byte for byte.
        writeln!(output, "   type {name} is new {base};\n").expect("writing to String cannot fail");
        return Ok(());
    };

    let mut clauses = Vec::new();
    match domain {
        FloatingDomain::Float32 { lower, upper } => {
            if let Some(bound) = lower {
                clauses.push(format!(
                    "{name} {} {}",
                    bound.kind.lower_operator(),
                    float32_literal(bound.value.value())
                ));
            }
            if let Some(bound) = upper {
                clauses.push(format!(
                    "{name} {} {}",
                    bound.kind.upper_operator(),
                    float32_literal(bound.value.value())
                ));
            }
        }
        FloatingDomain::Float64 { lower, upper } => {
            if let Some(bound) = lower {
                clauses.push(format!(
                    "{name} {} {}",
                    bound.kind.lower_operator(),
                    float64_literal(bound.value.value())
                ));
            }
            if let Some(bound) = upper {
                clauses.push(format!(
                    "{name} {} {}",
                    bound.kind.upper_operator(),
                    float64_literal(bound.value.value())
                ));
            }
        }
    }

    // Visible part: an opaque handle plus the two operations a client may use.
    // No conversion, no arithmetic, no field is nameable from here.
    writeln!(
        output,
        concat!(
            "   type {name} is private;\n\n",
            "   function Create (Value : {base}) return {name};\n\n",
            "   function Value (Item : {name}) return {base};\n",
        ),
        name = name,
        base = base,
    )
    .expect("writing to String cannot fail");

    // Private completion: the derived IEEE type carries the predicate, and the
    // conversions are written here, inside this unit, under this unit's own
    // `pragma Assertion_Policy (Dynamic_Predicate => Check)`. That is what
    // makes the check independent of the client's `-gnata`.
    writeln!(
        private_part,
        concat!(
            "   type {name} is new {base}\n",
            "     with Dynamic_Predicate =>\n",
            "       {predicate};\n\n",
            "   function Create (Value : {base}) return {name}\n",
            "   is ({name} (Value));\n\n",
            "   function Value (Item : {name}) return {base}\n",
            "   is ({base} (Item));\n",
        ),
        name = name,
        base = base,
        predicate = clauses.join("\n       and then "),
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

/// Render a named constrained String declaration into spec, private part, body.
///
/// Only the shared classifier's one supported profile is lowered; every other
/// constrained String shape fails closed rather than being approximated.
///
/// # Representation
///
/// A **validated lexical carrier** whose private completion reuses the
/// package's existing owned-string representation
/// (`Ada.Strings.Unbounded.Unbounded_String`), exactly as Task 036's carrier
/// does. The stored text is the caller's input unchanged: this profile
/// inherits `whiteSpace = preserve`, so nothing is trimmed or collapsed.
///
/// # Equality
///
/// The private type inherits Ada's predefined equality, which compares the
/// stored representation. Unlike the Task 036 DateTime carrier, that *is* the
/// correct XML Schema semantics here: for `xs:string` the value space is the
/// set of lexical forms, so stored-text equality is genuine value equality.
/// The generated comment says so. No ordering operator is declared, because
/// XML Schema defines no order relation on `string`.
///
/// # Why a body
///
/// The validator is a real algorithm, not an expression function, so `Create`
/// is completed in a package body. Every parser helper is **nested inside
/// `Create`**, so no new package-scope identifier is introduced and nothing
/// new is owed to generated-name preflight beyond the `Create` / `Value`
/// overloads the shared name model already registers.
fn render_string_profile_declaration(
    output: &mut String,
    private_part: &mut String,
    body: &mut String,
    constraints: &ConstraintSet,
    name: &str,
) -> Result<(), CodegenError> {
    let (profile, summary, facets) = match string_profile(PrimitiveKind::String, constraints) {
        Ok(Some(StringProfile::UciSchemaVersion)) => (
            StringProfile::UciSchemaVersion,
            "A validated UCI schema-version string.",
            "schema-version pattern and both length facets".to_owned(),
        ),
        Ok(Some(StringProfile::UniversallyUniqueIdentifier)) => (
            StringProfile::UniversallyUniqueIdentifier,
            "A validated UCI UUID string.",
            "UUID pattern and the length facet".to_owned(),
        ),
        Ok(Some(profile @ StringProfile::VisibleAscii { .. })) => (
            profile,
            "A validated visible-ASCII string.",
            "[ -~] character class and both length facets".to_owned(),
        ),
        Ok(None) => return unsupported(format!("unconstrained String on {name}")),
        Err(reason) => return unsupported(format!("{reason} on {name}")),
    };
    // Visible part: an opaque handle plus the two operations a client may use.
    // The representation is not nameable from here, so no aggregate or
    // conversion can bypass `Create`.
    writeln!(
        output,
        concat!(
            "   --  {summary}\n",
            "   --  Predefined \"=\" compares the stored representation, which for\n",
            "   --  xs:string IS XML Schema value equality.\n",
            "   type {name} is private;\n\n",
            "   --  Raises Constraint_Error unless Value matches the authoritative\n",
            "   --  {facets}.\n",
            "   function Create (Value : String) return {name};\n\n",
            "   --  The stored representation, exactly as supplied.\n",
            "   function Value (Item : {name}) return String;\n",
        ),
        summary = summary,
        facets = facets,
        name = name,
    )
    .expect("writing to String cannot fail");

    writeln!(
        private_part,
        concat!(
            "   type {name} is record\n",
            "      --  Task 040: an explicitly failing component default. An ordinary\n",
            "      --  default declaration of this carrier would otherwise produce an\n",
            "      --  empty string, which this profile's own validator rejects. The\n",
            "      --  default is a raise expression, so enforcement is part of the\n",
            "      --  language's initialization semantics and does not depend on\n",
            "      --  -gnata, Assertion_Policy, or any client-side check.\n",
            "      --\n",
            "      --  Create builds this record with an explicit named aggregate after\n",
            "      --  validation succeeds, so legitimate construction never evaluates\n",
            "      --  this default. Copy and assignment from an already valid carrier\n",
            "      --  are likewise unaffected.\n",
            "      Text : Standard.Ada.Strings.Unbounded.Unbounded_String :=\n",
            "        raise Standard.Program_Error\n",
            "          with \"{name} requires initialization from Create\";\n",
            "   end record;\n",
        ),
        name = name,
    )
    .expect("writing to String cannot fail");

    let rendered = match profile {
        StringProfile::UciSchemaVersion => ADA_SCHEMA_VERSION_BODY.replace("{name}", name),
        StringProfile::UniversallyUniqueIdentifier => ADA_UUID_BODY.replace("{name}", name),
        // The only parameterized profile: the classifier has already proven
        // that these bounds are exactly the ones the declaration's own pattern
        // quantifier states, so substituting them cannot widen the type.
        StringProfile::VisibleAscii {
            min_length,
            max_length,
        } => ADA_VISIBLE_ASCII_BODY
            .replace("{name}", name)
            .replace("{min_length}", &min_length.to_string())
            .replace("{max_length}", &max_length.to_string()),
    };
    body.push_str(&rendered);
    Ok(())
}

/// The generated Ada body for one visible-ASCII carrier, with `{name}` and the
/// bounds substituted.
///
/// # Validation order
///
/// 1. the `minLength`/`maxLength` facets;
/// 2. the authoritative `[ -~]` character class, tested per character.
///
/// Both are enforced; the facets are checked explicitly rather than assumed
/// redundant, so no facet is silently lost.
///
/// # No regular-expression engine
///
/// `GNAT.Regpat` is deliberately not used: its syntax is Perl-derived, not XML
/// Schema. The authoritative expression is one character class under one
/// bounded quantifier, so membership is a length test plus an independent
/// per-character range test. Patterns are anchored, which testing every
/// character of the slice enforces directly.
///
/// # The range is ordinal, never locale-sensitive
///
/// Membership is `Item in ' ' .. '~'`, a comparison on `Character`'s position
/// in Latin-1, whose first 128 positions are ASCII. No
/// `Ada.Characters.Handling` classification is consulted, so DEL and every
/// Latin-1 character above U+007E are rejected regardless of environment.
///
/// # SPACE is an ordinary member
///
/// This profile inherits `whiteSpace = preserve` and its class contains SPACE,
/// so leading, trailing, interior, and all-space values are valid when their
/// lengths fit, and are stored unchanged. TAB, LF, and CR are outside the
/// class and are rejected.
///
/// # Character counting
///
/// Ada's `String` is an array of `Character`, so `'Length` is already a
/// character count and matches the XSD facet directly.
///
/// # Scope
///
/// Every helper is declared in `Create`'s own declarative part, so no
/// package-scope identifier is introduced beyond the `Create` / `Value`
/// overloads the shared name model already registers.
const ADA_VISIBLE_ASCII_BODY: &str = r##"
   function Create (Value : String) return {name} is

      --  minLength, which is also the pattern quantifier's minimum.
      Min_Length : constant := {min_length};

      --  maxLength, which is also the pattern quantifier's maximum.
      Max_Length : constant := {max_length};

      --  The [ -~] class: the inclusive interval U+0020 SPACE .. U+007E TILDE.
      --
      --  SPACE is inside the class; DEL, TAB, LF, CR, every other control, and
      --  every Latin-1 character above '~' are outside it.
      function Is_Visible (Item : Character) return Boolean is
        (Item in ' ' .. '~');

      --  The character class over the whole value. Anchored by construction:
      --  every character must be a member.
      function Matches_Pattern (Text : String) return Boolean is
      begin
         for Item of Text loop
            if not Is_Visible (Item) then
               return False;
            end if;
         end loop;
         return True;
      end Matches_Pattern;

   begin
      --  The whole gate: both length facets AND the character class. The
      --  stored text is the input unchanged -- this profile inherits
      --  whiteSpace = preserve, so nothing is trimmed and leading or trailing
      --  spaces are preserved exactly as supplied.
      if Value'Length not in Min_Length .. Max_Length
        or else not Matches_Pattern (Value)
      then
         raise Standard.Constraint_Error
           with "invalid visible-ASCII string";
      end if;
      return {name}'
        (Text => Standard.Ada.Strings.Unbounded.To_Unbounded_String (Value));
   end Create;

   function Value (Item : {name}) return String is
   begin
      return Standard.Ada.Strings.Unbounded.To_String (Item.Text);
   end Value;
"##;

/// The generated Ada body for one UUID carrier, `{name}` substituted.
///
/// # Validation order
///
/// 1. the `length` facet;
/// 2. the authoritative pattern, evaluated positionally.
///
/// Both are enforced; the facet is checked explicitly rather than assumed
/// redundant, so no facet is silently lost.
///
/// # No regular-expression engine
///
/// `GNAT.Regpat` is deliberately not used: its syntax is Perl-derived, not XML
/// Schema. Both branches of the authoritative expression are fixed-width, so
/// every position's character class is determined by its offset alone and the
/// decision is a bounded positional test. Patterns are anchored, which the
/// fixed length requirement enforces directly.
///
/// The nil branch is checked first and separately, because it is **not**
/// redundant: the general branch requires a version nibble in `[1-5]` and a
/// variant nibble in `[89abAB]`, and the nil UUID has `0` in both.
///
/// # Character counting
///
/// Ada's `String` is an array of `Character`, so `'Length` is already a
/// character count and matches the XSD facet directly.
///
/// # Scope
///
/// Every helper is declared in `Create`'s own declarative part, so no
/// package-scope identifier is introduced beyond the `Create` / `Value`
/// overloads the shared name model already registers.
const ADA_UUID_BODY: &str = r##"
   function Create (Value : String) return {name} is

      Length : constant := 36;

      --  The one value the pattern's nil branch accepts.
      Nil_Uuid : constant String := "00000000-0000-0000-0000-000000000000";

      function Matches_Pattern (Text : String) return Boolean;

      --  Decide the authoritative pattern positionally:
      --    (0{8}(-0{4}){3}-0{12})
      --    |([a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[1-5][a-fA-F0-9]{3}
      --      -[89abAB][a-fA-F0-9]{3}-[a-fA-F0-9]{12})
      function Matches_Pattern (Text : String) return Boolean is

         function Is_Hex (Item : Character) return Boolean is
           (Item in '0' .. '9' or else Item in 'a' .. 'f'
            or else Item in 'A' .. 'F');

      begin
         --  Branch A: the exact nil literal, which branch B rejects.
         if Text = Nil_Uuid then
            return True;
         end if;

         --  Branch B: 8-4-4-4-12 with constrained version and variant
         --  nibbles. Offset is zero-based over the fixed 36-character width.
         for Offset in 0 .. Length - 1 loop
            declare
               Item : constant Character := Text (Text'First + Offset);
            begin
               case Offset is
                  when 8 | 13 | 18 | 23 =>
                     if Item /= '-' then
                        return False;
                     end if;

                  --  The version nibble: [1-5].
                  when 14 =>
                     if Item not in '1' .. '5' then
                        return False;
                     end if;

                  --  The variant nibble: [89abAB].
                  when 19 =>
                     if Item not in '8' | '9' | 'a' | 'b' | 'A' | 'B' then
                        return False;
                     end if;

                  when others =>
                     if not Is_Hex (Item) then
                        return False;
                     end if;
               end case;
            end;
         end loop;
         return True;
      end Matches_Pattern;

   begin
      --  The whole gate: the length facet AND the pattern. The stored text is
      --  the input unchanged -- this profile inherits whiteSpace = preserve,
      --  so nothing is trimmed and hexadecimal letter case is preserved.
      if Value'Length /= Length or else not Matches_Pattern (Value) then
         raise Standard.Constraint_Error
           with "invalid UUID";
      end if;
      return {name}'
        (Text => Standard.Ada.Strings.Unbounded.To_Unbounded_String (Value));
   end Create;

   function Value (Item : {name}) return String is
   begin
      return Standard.Ada.Strings.Unbounded.To_String (Item.Text);
   end Value;
"##;

/// The generated Ada body for one schema-version carrier, `{name}` substituted.
///
/// # Validation order
///
/// 1. the authoritative pattern, evaluated structurally;
/// 2. the `minLength`/`maxLength` facets.
///
/// Both are enforced; the facets are checked explicitly rather than assumed
/// redundant, so no facet is silently lost.
///
/// # No regular-expression engine
///
/// `GNAT.Regpat` is deliberately not used: its syntax is Perl-derived, not XML
/// Schema. The authoritative expression is a concatenation of bounded pieces
/// whose alphabets are disjoint from the literals that follow them, so it is
/// decided by one left-to-right scan. Patterns are anchored, implemented by
/// requiring the scan to end exactly at the string's last index.
///
/// # Character counting
///
/// Ada's `String` is an array of `Character`, so `'Length` is already a
/// character count and matches the XSD facet directly. Every character the
/// pattern admits is ASCII, so this also coincides with the byte count other
/// backends use.
///
/// # Scope
///
/// Every helper is declared in `Create`'s own declarative part, so Task 037
/// introduces no package-scope identifier beyond `Create` and `Value` -- both
/// of which the shared name model already registers.
const ADA_SCHEMA_VERSION_BODY: &str = r##"
   function Create (Value : String) return {name} is

      Min_Length : constant := 7;
      Max_Length : constant := 57;

      function Matches_Pattern (Text : String) return Boolean;

      --  Decide the authoritative pattern in one pass:
      --    [0-9]{3}\.[0-9]{1,2}(\.[0-9]{1,2})([a-z]{1,2})?(_[a-zA-Z0-9\-]{1,45})?
      function Matches_Pattern (Text : String) return Boolean is

         At_Index : Positive := Text'First;

         function Is_Digit (Item : Character) return Boolean is
           (Item in '0' .. '9');

         function Is_Lower (Item : Character) return Boolean is
           (Item in 'a' .. 'z');

         --  The [a-zA-Z0-9\-] class of the optional underscore tail.
         function Is_Tail (Item : Character) return Boolean is
           (Is_Digit (Item) or else Is_Lower (Item)
            or else Item in 'A' .. 'Z' or else Item = '-');

         function Run
           (Min, Max : Natural; Accepts : not null access
              function (Item : Character) return Boolean)
            return Natural;
         function Literal (Expected : Character) return Boolean;

         --  Consume up to Max characters satisfying Accepts, returning the
         --  count. At_Index advances only by what was consumed, and 0 is
         --  returned without consuming anything when fewer than Min match, so
         --  a caller checking against Min is never left mid-piece.
         function Run
           (Min, Max : Natural; Accepts : not null access
              function (Item : Character) return Boolean)
            return Natural
         is
            Taken : Natural := 0;
         begin
            while Taken < Max
              and then At_Index + Taken <= Text'Last
              and then Accepts (Text (At_Index + Taken))
            loop
               Taken := Taken + 1;
            end loop;
            if Taken < Min then
               return 0;
            end if;
            At_Index := At_Index + Taken;
            return Taken;
         end Run;

         --  Consume one expected literal character, if present.
         function Literal (Expected : Character) return Boolean is
         begin
            if At_Index <= Text'Last and then Text (At_Index) = Expected then
               At_Index := At_Index + 1;
               return True;
            end if;
            return False;
         end Literal;

         Digit_Access : constant not null access
           function (Item : Character) return Boolean := Is_Digit'Access;
         Lower_Access : constant not null access
           function (Item : Character) return Boolean := Is_Lower'Access;
         Tail_Access  : constant not null access
           function (Item : Character) return Boolean := Is_Tail'Access;

         Ignored : Natural;
      begin
         --  [0-9]{3}
         if Run (3, 3, Digit_Access) /= 3 then
            return False;
         end if;
         --  \.
         if not Literal ('.') then
            return False;
         end if;
         --  [0-9]{1,2}
         if Run (1, 2, Digit_Access) = 0 then
            return False;
         end if;
         --  (\.[0-9]{1,2}) -- parenthesized but unquantified, so REQUIRED.
         if not Literal ('.') then
            return False;
         end if;
         if Run (1, 2, Digit_Access) = 0 then
            return False;
         end if;
         --  ([a-z]{1,2})? -- optional, so a zero-length run is acceptable.
         Ignored := Run (0, 2, Lower_Access);
         --  (_[a-zA-Z0-9\-]{1,45})? -- optional as a whole, but once the '_'
         --  is present at least one tail character is required.
         if Literal ('_') and then Run (1, 45, Tail_Access) = 0 then
            return False;
         end if;
         --  Patterns are anchored: the whole literal must be consumed.
         return At_Index = Text'Last + 1;
      end Matches_Pattern;

   begin
      --  The pattern AND both length facets. Value is stored exactly as
      --  supplied: this profile inherits whiteSpace = preserve.
      if not Matches_Pattern (Value)
        or else Value'Length < Min_Length
        or else Value'Length > Max_Length
      then
         raise Standard.Constraint_Error
           with "invalid schema version";
      end if;
      return {name}'
        (Text => Standard.Ada.Strings.Unbounded.To_Unbounded_String (Value));
   end Create;

   function Value (Item : {name}) return String is
   begin
      return Standard.Ada.Strings.Unbounded.To_String (Item.Text);
   end Value;
"##;

/// Render a named temporal declaration into the spec, private part, and body.
///
/// Only the shared classifier's one supported profile is lowered; `Time`,
/// `Duration`, an unconstrained `DateTime`, and every other facet shape fail
/// closed rather than being approximated.
///
/// # Representation
///
/// A **validated lexical carrier** whose private completion reuses the
/// package's existing owned-string representation
/// (`Ada.Strings.Unbounded.Unbounded_String`). Deliberately not
/// `Ada.Calendar.Time`: that type's year range and sub-second precision are
/// implementation-bounded and narrower than XML Schema's, so it would silently
/// reject valid values and discard wire-level information.
///
/// # Equality
///
/// The private type inherits Ada's predefined equality as a consequence of the
/// language model. That equality compares the **stored normalized lexical
/// representation** and is not XML Schema dateTime value-space equality: two
/// distinct legal spellings can denote one value. The generated comment says
/// so, and no ordering operator is declared.
///
/// # Why a body
///
/// The validator is a real algorithm, not an expression function, so `Create`
/// is completed in a package body. Every parser helper is **nested inside
/// `Create`**, so no new package-scope identifier is introduced and nothing
/// new is owed to generated-name preflight.
fn render_temporal_declaration(
    output: &mut String,
    private_part: &mut String,
    body: &mut String,
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
    name: &str,
) -> Result<(), CodegenError> {
    match temporal_profile(kind, constraints) {
        Ok(Some(TemporalProfile::DateTimeZulu)) => {}
        Ok(None) => return unsupported(format!("non-temporal primitive on {name}")),
        Err(reason) => return unsupported(format!("{reason} on {name}")),
    }
    // Visible part: an opaque handle plus the two operations a client may use.
    // The representation is not nameable from here, so no aggregate or
    // conversion can bypass `Create`.
    writeln!(
        output,
        concat!(
            "   --  A validated XML Schema dateTime restricted to the Zulu timezone.\n",
            "   --  Predefined \"=\" compares the stored normalized lexical\n",
            "   --  representation; it is NOT XML Schema value-space equality.\n",
            "   type {name} is private;\n\n",
            "   --  Raises Constraint_Error unless the whitespace-normalized value is\n",
            "   --  a valid lexical dateTime whose timezone is 'Z'.\n",
            "   function Create (Value : String) return {name};\n\n",
            "   --  The stored normalized lexical representation.\n",
            "   function Value (Item : {name}) return String;\n",
        ),
        name = name,
    )
    .expect("writing to String cannot fail");

    writeln!(
        private_part,
        concat!(
            "   type {name} is record\n",
            "      --  Task 040: an explicitly failing component default. An ordinary\n",
            "      --  default declaration of this carrier would otherwise produce an\n",
            "      --  empty string, which is not a valid Zulu dateTime. The default is\n",
            "      --  a raise expression, so enforcement is part of the language's\n",
            "      --  initialization semantics and does not depend on -gnata,\n",
            "      --  Assertion_Policy, or any client-side check.\n",
            "      --\n",
            "      --  Create builds this record with an explicit named aggregate after\n",
            "      --  validation succeeds, so legitimate construction never evaluates\n",
            "      --  this default. Copy and assignment from an already valid carrier\n",
            "      --  are likewise unaffected.\n",
            "      Lexical : Standard.Ada.Strings.Unbounded.Unbounded_String :=\n",
            "        raise Standard.Program_Error\n",
            "          with \"{name} requires initialization from Create\";\n",
            "   end record;\n",
        ),
        name = name,
    )
    .expect("writing to String cannot fail");

    body.push_str(&ADA_DATE_TIME_ZULU_BODY.replace("{name}", name));
    Ok(())
}

/// The generated Ada body for one DateTime Zulu carrier, `{name}` substituted.
///
/// # Validation order
///
/// 1. XML Schema `collapse` normalization (section 4.3.6), fixed for
///    `dateTime`;
/// 2. the full `dateTime` lexical grammar and calendar rules (3.2.7.1);
/// 3. the UCI `.+Z` Zulu restriction.
///
/// A bare "ends with 'Z'" test would accept `garbageZ`, so the base grammar is
/// checked first and Zulu is the last gate rather than the only one.
///
/// # No fixed-width year
///
/// The year is validated and its leap-year properties computed from decimal
/// digits, never converted to `Integer` or `Ada.Calendar.Year_Number`. XML
/// Schema admits a four-or-more digit year with no upper bound, so a valid
/// date must not become invalid because it exceeds a host numeric type.
///
/// # Scope
///
/// Every helper is declared in `Create`'s own declarative part, so Task 036
/// introduces no package-scope identifier beyond `Create` and `Value` -- both
/// of which the shared name model already registers.
const ADA_DATE_TIME_ZULU_BODY: &str = r##"
   function Create (Value : String) return {name} is

      function Collapse (Raw : String) return String;
      function Is_Zulu_Date_Time (Text : String) return Boolean;

      --  XML Schema "collapse": tab/LF/CR become spaces, runs of spaces are
      --  squeezed to one, and leading/trailing spaces are removed.
      function Collapse (Raw : String) return String is
         Result        : String (1 .. Raw'Length);
         Last          : Natural := 0;
         Pending_Space : Boolean := False;
      begin
         for Index in Raw'Range loop
            if Raw (Index) = ' '
              or else Raw (Index) = Character'Val (9)
              or else Raw (Index) = Character'Val (10)
              or else Raw (Index) = Character'Val (13)
            then
               Pending_Space := Last > 0;
            else
               if Pending_Space then
                  Last := Last + 1;
                  Result (Last) := ' ';
                  Pending_Space := False;
               end if;
               Last := Last + 1;
               Result (Last) := Raw (Index);
            end if;
         end loop;
         return Result (1 .. Last);
      end Collapse;

      function Is_Zulu_Date_Time (Text : String) return Boolean is

         function Is_Digit (Item : Character) return Boolean is
           (Item in '0' .. '9');

         function Two_Digits
           (Text : String; From : Positive; Out_Value : out Natural)
            return Boolean;
         function Days_In_Month (Month : Natural; Leap : Boolean) return Natural;
         function Is_Leap_Year (Text : String; From, To : Positive) return Boolean;
         function Is_Time_Of_Day (Text : String; From : Positive) return Boolean;

         --  Exactly two ASCII digits at Text (From .. From + 1).
         function Two_Digits
           (Text : String; From : Positive; Out_Value : out Natural)
            return Boolean is
         begin
            Out_Value := 0;
            if From + 1 > Text'Last
              or else not Is_Digit (Text (From))
              or else not Is_Digit (Text (From + 1))
            then
               return False;
            end if;
            Out_Value :=
              (Character'Pos (Text (From)) - Character'Pos ('0')) * 10
              + (Character'Pos (Text (From + 1)) - Character'Pos ('0'));
            return True;
         end Two_Digits;

         --  maximumDayInMonthFor, XML Schema 1.0 Part 2 Appendix E.
         function Days_In_Month (Month : Natural; Leap : Boolean) return Natural is
         begin
            case Month is
               when 1 | 3 | 5 | 7 | 8 | 10 | 12 => return 31;
               when 4 | 6 | 9 | 11              => return 30;
               when 2                           =>
                  if Leap then
                     return 29;
                  else
                     return 28;
                  end if;
               when others                      => return 0;
            end case;
         end Days_In_Month;

         --  Leap year from decimal digits: divisible by 400, or by 4 but not
         --  100. Divisibility by 4 depends only on the last two digits (100 is
         --  itself a multiple of 4), and the "divisible by 400" case is decided
         --  by reducing the remaining digits modulo 4 one at a time. Nothing is
         --  converted to a whole-year integer, so any digit count stays exact.
         function Is_Leap_Year (Text : String; From, To : Positive) return Boolean is
            Last_Two         : constant Natural :=
              (Character'Pos (Text (To - 1)) - Character'Pos ('0')) * 10
              + (Character'Pos (Text (To)) - Character'Pos ('0'));
            Divisible_By_4   : constant Boolean := Last_Two mod 4 = 0;
            Divisible_By_100 : constant Boolean := Last_Two = 0;
            Remainder        : Natural := 0;
         begin
            if Divisible_By_100 then
               for Index in From .. To - 2 loop
                  Remainder :=
                    (Remainder * 10
                     + (Character'Pos (Text (Index)) - Character'Pos ('0')))
                    mod 4;
               end loop;
               return Remainder = 0;
            end if;
            return Divisible_By_4;
         end Is_Leap_Year;

         --  'T' hh ':' mm ':' ss ('.' s+)?
         function Is_Time_Of_Day (Text : String; From : Positive) return Boolean is
            Hour, Minute, Second : Natural;
            Fraction_Is_Zero     : Boolean := True;
         begin
            if Text'Last - From + 1 < 9
              or else Text (From) /= 'T'
              or else Text (From + 3) /= ':'
              or else Text (From + 6) /= ':'
            then
               return False;
            end if;
            if not Two_Digits (Text, From + 1, Hour)
              or else not Two_Digits (Text, From + 4, Minute)
              or else not Two_Digits (Text, From + 7, Second)
            then
               return False;
            end if;
            --  XML Schema 1.0 Part 2, Appendix D: the two digits of 'ss' "can
            --  have values from 0 to 60". 60 is the LEAP SECOND and is
            --  lexically legal; 61 never is, because the field itself stops at
            --  60. Minutes remain 00 .. 59 -- a leap second lengthens the
            --  second field, not the minute.
            --
            --  Appendix D adds that a 60 is "not sensible" away from March 31,
            --  June 30, September 30, or December 31 UTC, but prescribes that
            --  such a value "should [be] considered as added or subtracted
            --  from the following minute" -- a VALUE mapping, not a lexical
            --  rejection. So no calendar-position test is applied here, and no
            --  IERS leap-second table is needed: Appendix E states outright
            --  that a definition tracking real leap seconds "would need to be
            --  constantly updated".
            if Hour > 24 or else Minute > 59 or else Second > 60 then
               return False;
            end if;
            if From + 9 <= Text'Last then
               --  '.' s+ : the dot requires at least one digit after it.
               if Text (From + 9) /= '.' or else From + 10 > Text'Last then
                  return False;
               end if;
               for Index in From + 10 .. Text'Last loop
                  if not Is_Digit (Text (Index)) then
                     return False;
                  end if;
                  if Text (Index) /= '0' then
                     Fraction_Is_Zero := False;
                  end if;
               end loop;
            end if;
            --  Hour 24 is legal only as the exact instant 24:00:00(.0*).
            if Hour = 24
              and then (Minute /= 0 or else Second /= 0 or else not Fraction_Is_Zero)
            then
               return False;
            end if;
            return True;
         end Is_Time_Of_Day;

         Body_Last    : Natural;
         Year_From    : Positive;
         Year_To      : Natural;
         Cursor       : Natural;
         All_Zero     : Boolean := True;
         Leap         : Boolean;
         Month, Day   : Natural;
      begin
         --  The Zulu restriction, applied to the normalized form -- exactly
         --  where XML Schema applies a pattern facet.
         if Text'Length < 2 or else Text (Text'Last) /= 'Z' then
            return False;
         end if;
         Body_Last := Text'Last - 1;

         --  '-'? yyyy, with an unbounded digit count.
         if Text (Text'First) = '-' then
            Year_From := Text'First + 1;
         else
            Year_From := Text'First;
         end if;
         Cursor := Year_From;
         while Cursor <= Body_Last and then Is_Digit (Text (Cursor)) loop
            Cursor := Cursor + 1;
         end loop;
         Year_To := Cursor - 1;

         --  Four-or-more digits.
         if Year_To - Year_From + 1 < 4 then
            return False;
         end if;
         --  If more than four digits, leading zeros are prohibited.
         if Year_To - Year_From + 1 > 4 and then Text (Year_From) = '0' then
            return False;
         end if;
         --  '0000' is not a valid lexical representation in XML Schema 1.0,
         --  with or without a sign.
         for Index in Year_From .. Year_To loop
            if Text (Index) /= '0' then
               All_Zero := False;
            end if;
         end loop;
         if All_Zero then
            return False;
         end if;
         Leap := Is_Leap_Year (Text, Year_From, Year_To);

         --  '-' mm '-' dd, all fixed width.
         if Year_To + 6 > Body_Last
           or else Text (Year_To + 1) /= '-'
           or else Text (Year_To + 4) /= '-'
         then
            return False;
         end if;
         if not Two_Digits (Text, Year_To + 2, Month)
           or else not Two_Digits (Text, Year_To + 5, Day)
         then
            return False;
         end if;
         if Month < 1 or else Month > 12
           or else Day < 1 or else Day > Days_In_Month (Month, Leap)
         then
            return False;
         end if;

         --  The base grammar must hold for everything before the timezone;
         --  this is what stops "garbageZ" from being accepted.
         return Is_Time_Of_Day (Text (Text'First .. Body_Last), Year_To + 7);
      end Is_Zulu_Date_Time;

      --  Declared after the helper bodies so Ada's declare-before-use rule is
      --  satisfied without a separate elaboration step.
      Normalized : constant String := Collapse (Value);

   begin
      if not Is_Zulu_Date_Time (Normalized) then
         raise Constraint_Error
           with "not a valid XML Schema dateTime in the Zulu timezone";
      end if;
      return {name}'
        (Lexical =>
           Standard.Ada.Strings.Unbounded.To_Unbounded_String (Normalized));
   end Create;

   function Value (Item : {name}) return String is
     (Standard.Ada.Strings.Unbounded.To_String (Item.Lexical));
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

    /// Task 033 sections 27--29: the generated predicates must be **really**
    /// enforced, not decorative.
    ///
    /// The probe is compiled *without* `-gnata` on purpose. That is the whole
    /// point: a client that does not enable assertions must still not be able
    /// to build an out-of-domain value through the generated `Create`, because
    /// the spec carries its own `Assertion_Policy`. If the pragma or `Create`
    /// were dropped, this test would fail rather than silently pass with an
    /// unenforced predicate.
    ///
    /// Skipped only where GNAT is absent, matching the Task 029 probe policy;
    /// the generated-text assertions above it are unconditional.
    #[test]
    fn constrained_floats_enforce_predicates_under_gnat() {
        let source =
            generate(&constrained_floating_schema(), CLOSED).expect("fixture must generate");

        // Width and representation are structural.
        assert!(source.starts_with("pragma Assertion_Policy (Dynamic_Predicate => Check);\n"));

        // The visible part is opaque: only the private type and the two
        // operations. The derived IEEE type must not be nameable from there.
        let (visible, private_part) = source
            .split_once("\nprivate\n")
            .expect("a constrained floating schema must emit a private part");
        assert!(visible.contains("type FloatUnitInterval is private;"));
        assert!(visible.contains("type DoubleAltitude is private;"));
        assert!(visible.contains(
            "function Create (Value : Interfaces.IEEE_Float_32) return FloatUnitInterval;"
        ));
        assert!(visible.contains(
            "function Value (Item : FloatUnitInterval) return Interfaces.IEEE_Float_32;"
        ));
        assert!(
            visible.contains(
                "function Create (Value : Interfaces.IEEE_Float_64) return DoubleAltitude;"
            )
        );
        // No public derivation, predicate, or conversion anywhere visible.
        assert!(!visible.contains("is new Interfaces.IEEE_Float_32"));
        assert!(!visible.contains("is new Interfaces.IEEE_Float_64"));
        assert!(!visible.contains("'Base"));
        // The only `Dynamic_Predicate` mention before `private` is the unit's
        // own configuration pragma, never a visible predicate aspect.
        assert!(!visible.contains("with Dynamic_Predicate"));

        // Width and the effective domains live in the private completion.
        assert!(private_part.contains("type FloatUnitInterval is new Interfaces.IEEE_Float_32"));
        assert!(private_part.contains("type DoubleAltitude is new Interfaces.IEEE_Float_64"));
        assert!(private_part.contains("DoubleAltitude >= -6378237.0;"));
        assert!(private_part.contains("DoublePositive > 0.0;"));
        // The named chain enforces its effective inherited domain.
        assert!(
            private_part.contains("DerivedFloat >= 0.0\n       and then DerivedFloat <= 10.0;")
        );

        use std::fs;
        use std::process::Command;
        if Command::new("gnatmake").arg("--version").output().is_err() {
            return;
        }
        let directory = std::env::temp_dir().join("ams-gra-oms-task033-ada-bounds");
        fs::create_dir_all(&directory).expect("create Ada probe directory");
        fs::write(
            directory.join("constrained.ads"),
            "package Constrained is\nend Constrained;\n",
        )
        .expect("write Ada parent package");
        fs::write(directory.join("constrained-floating.ads"), source)
            .expect("write generated Ada spec");
        fs::write(directory.join("probe.adb"), ADA_BOUNDS_PROBE).expect("write Ada probe");

        let status = Command::new("gnatmake")
            .current_dir(&directory)
            .args(["-q", "probe.adb"])
            .status()
            .expect("GNAT reported a version, so it must be runnable");
        assert!(status.success(), "generated Ada spec must compile");
        let run = Command::new(directory.join("probe"))
            .status()
            .expect("compiled probe must run");
        fs::remove_dir_all(&directory).expect("remove Ada probe directory");
        assert!(
            run.success(),
            "generated predicates must be enforced at runtime"
        );
    }

    /// Task 033 correction: the two bypasses that the original publicly derived
    /// representation permitted must now be *compile* errors.
    ///
    /// Before the correction both of these compiled cleanly without `-gnata`
    /// and produced out-of-domain values: `DoublePositive (0.0)` yielded `0.0`
    /// in a `> 0.0` type, and `A + B` yielded `1.5` in a `[0.0, 1.0]` type. A
    /// runtime exception would not be an acceptable outcome here -- the point
    /// is that the unchecked surface does not exist publicly at all, so the
    /// probes must be rejected by the compiler.
    ///
    /// Each probe is checked for its *intended* diagnostic, not merely for
    /// failure, so an unrelated syntax error in the fixture cannot make this
    /// test pass vacuously.
    #[test]
    fn constrained_floats_reject_public_bypasses_under_gnat() {
        let source =
            generate(&constrained_floating_schema(), CLOSED).expect("fixture must generate");

        use std::fs;
        use std::process::Command;
        if Command::new("gnatmake").arg("--version").output().is_err() {
            return;
        }
        let directory = std::env::temp_dir().join("ams-gra-oms-task033-ada-bypass");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create Ada probe directory");
        fs::write(
            directory.join("constrained.ads"),
            "package Constrained is\nend Constrained;\n",
        )
        .expect("write Ada parent package");
        fs::write(directory.join("constrained-floating.ads"), source)
            .expect("write generated Ada spec");

        // (unit, source, substring the rejection must mention)
        let probes: [(&str, &str, &str); 2] = [
            (
                "direct_conversion",
                ADA_DIRECT_CONVERSION_BYPASS,
                "invalid conversion",
            ),
            (
                "inherited_arithmetic",
                ADA_INHERITED_ARITHMETIC_BYPASS,
                "no applicable operator",
            ),
        ];
        for (unit, probe_source, expected) in probes {
            let file = format!("{unit}.adb");
            fs::write(directory.join(&file), probe_source).expect("write Ada bypass probe");
            let output = Command::new("gnatmake")
                .current_dir(&directory)
                .args(["-q", &file])
                .output()
                .expect("GNAT reported a version, so it must be runnable");
            assert!(
                !output.status.success(),
                "{unit} bypass must not compile against the private representation"
            );
            let diagnostics = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                diagnostics.contains(expected),
                "{unit} must be rejected for representation hiding \
                 (expected a diagnostic mentioning {expected:?}), got:\n{diagnostics}"
            );
        }
        fs::remove_dir_all(&directory).expect("remove Ada probe directory");
    }

    /// The direct-conversion bypass: there is no public conversion into the
    /// private type, so the scalar literal has nothing to convert to.
    const ADA_DIRECT_CONVERSION_BYPASS: &str = r#"with Constrained.Floating; use Constrained.Floating;

procedure Direct_Conversion is
   Bad : DoublePositive := DoublePositive (0.0);
begin
   null;
end Direct_Conversion;
"#;

    /// The inherited-arithmetic bypass: a private type inherits no numeric
    /// operators, so two valid values cannot be combined into an invalid one.
    const ADA_INHERITED_ARITHMETIC_BYPASS: &str = r#"with Constrained.Floating; use Constrained.Floating;

procedure Inherited_Arithmetic is
   A : FloatUnitInterval := Create (0.75);
   B : FloatUnitInterval := Create (0.75);
   C : FloatUnitInterval := A + B;
begin
   null;
end Inherited_Arithmetic;
"#;

    /// The runtime assertions compiled against the generated spec above.
    ///
    /// Each `Accepts_*` wrapper converts through the generated `Create` and
    /// reports whether the predicate held, so accept and reject are both
    /// observed rather than only the failure path.
    const ADA_BOUNDS_PROBE: &str = r#"with Constrained.Floating; use Constrained.Floating;
with Interfaces;
use type Interfaces.IEEE_Float_32;
use type Interfaces.IEEE_Float_64;
with Ada.Text_IO; use Ada.Text_IO;

procedure Probe is

   subtype F32 is Interfaces.IEEE_Float_32;
   subtype F64 is Interfaces.IEEE_Float_64;

   --  Every wrapper goes through the public `Create`, which is the only
   --  construction path the private representation offers. Each also asserts
   --  that `Value` returns exactly what was accepted, so the round-trip is
   --  observed rather than assumed.
   function Accepts_Unit (Item : F32) return Boolean is
      Held : FloatUnitInterval;
   begin
      Held := Create (Item);
      return Value (Held) = Item;
   exception
      when others => return False;
   end Accepts_Unit;

   function Accepts_Lower (Item : F32) return Boolean is
      Held : FloatLowerInclusive;
   begin
      Held := Create (Item);
      return Value (Held) = Item;
   exception
      when others => return False;
   end Accepts_Lower;

   function Accepts_Altitude (Item : F64) return Boolean is
      Held : DoubleAltitude;
   begin
      Held := Create (Item);
      return Value (Held) = Item or else Item /= Item;
   exception
      when others => return False;
   end Accepts_Altitude;

   function Accepts_Positive (Item : F64) return Boolean is
      Held : DoublePositive;
   begin
      Held := Create (Item);
      return Value (Held) = Item;
   exception
      when others => return False;
   end Accepts_Positive;

   function Accepts_Upper_Exclusive (Item : F64) return Boolean is
      Held : DoubleUpperExclusive;
   begin
      Held := Create (Item);
      return Value (Held) = Item;
   exception
      when others => return False;
   end Accepts_Upper_Exclusive;

   function Accepts_Derived (Item : F64) return Boolean is
      Held : DerivedFloat;
   begin
      Held := Create (Item);
      return Value (Held) = Item;
   exception
      when others => return False;
   end Accepts_Derived;

   procedure Check (Label : String; Actual : Boolean; Expected : Boolean) is
   begin
      if Actual /= Expected then
         Put_Line ("FAIL: " & Label);
         raise Program_Error;
      end if;
   end Check;

   --  Built at run time so the compiler cannot fold the comparison away.
   Zero : F64 := 0.0;
   pragma Volatile (Zero);
   Nan_Value : F64;
   pragma Volatile (Nan_Value);
   Big : F64 := F64'Last;
   pragma Volatile (Big);
   Pos_Inf : F64;
   pragma Volatile (Pos_Inf);
begin
   Nan_Value := Zero / Zero;
   Pos_Inf := Big * 2.0;

   --  Float32 two-sided inclusive.
   Check ("unit 0.5 accepted", Accepts_Unit (0.5), True);
   Check ("unit 0.0 accepted", Accepts_Unit (0.0), True);
   Check ("unit 1.0 accepted", Accepts_Unit (1.0), True);
   Check ("unit 1.5 rejected", Accepts_Unit (1.5), False);
   Check ("unit -0.5 rejected", Accepts_Unit (-0.5), False);

   --  Float32 lower inclusive, including signed zero.
   Check ("lower 0.0 accepted", Accepts_Lower (0.0), True);
   Check ("lower -0.0 accepted", Accepts_Lower (-0.0), True);
   Check ("lower -1.0 rejected", Accepts_Lower (-1.0), False);

   --  Float64 lower inclusive: the authoritative AltitudeType bound.
   Check ("altitude bound accepted", Accepts_Altitude (-6378237.0), True);
   Check ("altitude below rejected", Accepts_Altitude (-6378238.0), False);
   Check ("altitude 100.5 accepted", Accepts_Altitude (100.5), True);

   --  Float64 lower EXCLUSIVE: section 29's required shape.
   Check ("positive 0.0 rejected", Accepts_Positive (0.0), False);
   Check ("positive -0.0 rejected", Accepts_Positive (-0.0), False);
   Check ("positive 1.0e-300 accepted", Accepts_Positive (1.0e-300), True);

   --  Float64 upper exclusive.
   Check ("upper 1.0 rejected", Accepts_Upper_Exclusive (1.0), False);
   Check ("upper 0.999 accepted", Accepts_Upper_Exclusive (0.999), True);

   --  Named chain: the derived effective domain is what is enforced.
   Check ("derived 0.0 accepted", Accepts_Derived (0.0), True);
   Check ("derived 10.0 accepted", Accepts_Derived (10.0), True);
   Check ("derived -0.5 rejected", Accepts_Derived (-0.5), False);
   Check ("derived 10.5 rejected", Accepts_Derived (10.5), False);

   --  NaN satisfies no bound.
   Check ("NaN rejected", Accepts_Altitude (Nan_Value), False);
   Check ("NaN rejected (exclusive)", Accepts_Positive (Nan_Value), False);

   --  One-sided infinity keeps IEEE ordering.
   Check ("+Inf accepted by lower-only", Accepts_Altitude (Pos_Inf), True);
   Check ("-Inf rejected by lower-only", Accepts_Altitude (-Pos_Inf), False);
   Check ("-Inf accepted by upper-only",
          Accepts_Upper_Exclusive (-Pos_Inf), True);
   Check ("+Inf rejected by upper-only",
          Accepts_Upper_Exclusive (Pos_Inf), False);

   Put_Line ("ok");
end Probe;
"#;

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
            "subtype Container_ZeroOrMoreNamed_Sequence is Container_ZeroOrMoreNamed_Vectors.Vector;"
        ));
        assert!(source.contains("array (Positive range 1 .. 1) of Container_OneOrMoreNamed_Item;"));
        assert!(source.contains("array (Positive range 1 .. 2) of Container_TwoOrMoreNamed_Item;"));
        assert!(
            source.contains("Additional : Container_TwoOrMoreNamed_Additional_Vectors.Vector;")
        );
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
        // `Foo` and `foo` are one Ada identifier. This is now diagnosed by the
        // shared backend name preflight rather than by an Ada-local duplicate
        // check, so the assertion is on the semantic outcome -- the collision
        // is rejected and both spellings are named -- rather than on the exact
        // prose of the superseded local diagnostic.
        let message = generate(&collision, CLOSED)
            .expect_err("Ada case-insensitive alternative collision must be rejected")
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

        // Task 022 unconstrained output carries no predicate and no assertion
        // pragma; Task 033 must not churn it.
        let mut unconstrained_schema = track_schema();
        unconstrained_schema.types[0].kind = TypeKind::Primitive(PrimitiveKind::Float64);
        unconstrained_schema.types[0].constraints = ConstraintSet::default();
        let unconstrained =
            generate(&unconstrained_schema, CLOSED).expect("unconstrained generation");
        assert!(unconstrained.contains("type Track_Id is new Interfaces.IEEE_Float_64;"));
        assert!(!unconstrained.contains("Dynamic_Predicate"));
        assert!(!unconstrained.contains("Assertion_Policy"));
        assert!(!unconstrained.contains("function Create"));

        // Task 033 supersedes the Task 022 blanket rejection for the bound-only
        // subset: a named lower-inclusive Float64 becomes a derived IEEE type
        // with an enforced predicate.
        let mut schema = track_schema();
        schema.types[0].kind = TypeKind::Primitive(PrimitiveKind::Float64);
        schema.types[0].constraints = ConstraintSet {
            min_inclusive: Some(NumericValue::Float64(
                ams_gra_oms_ir::Float64Value::from_value(0.0),
            )),
            ..ConstraintSet::default()
        };
        let source = generate(&schema, CLOSED).expect("Task 033 bounded float must render");
        assert!(source.starts_with("pragma Assertion_Policy (Dynamic_Predicate => Check);\n"));
        let (visible, private_part) = source
            .split_once("\nprivate\n")
            .expect("a bounded float must emit a private part");
        // Public surface: opaque type, checked construction, read-only access.
        assert!(visible.contains("type Track_Id is private;"));
        assert!(
            visible.contains("function Create (Value : Interfaces.IEEE_Float_64) return Track_Id;")
        );
        assert!(
            visible.contains("function Value (Item : Track_Id) return Interfaces.IEEE_Float_64;")
        );
        assert!(!visible.contains("is new Interfaces.IEEE_Float_64"));
        assert!(!visible.contains("with Dynamic_Predicate"));
        // Representation and predicate are hidden in the completion.
        assert!(private_part.contains("type Track_Id is new Interfaces.IEEE_Float_64"));
        assert!(private_part.contains("with Dynamic_Predicate =>\n       Track_Id >= 0.0;"));
        // A finite `range` subtype would wrongly exclude +Infinity from this
        // lower-only constraint, so the floating declaration must not use one.
        assert!(!source.contains("type Track_Id is new Interfaces.IEEE_Float_64 range"));
        assert!(!source.contains("subtype Track_Id"));

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
        assert!(error.message.contains("unsupported Ada IR construct"));
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

    /// Task 035 supersedes the Task 025 control that asserted optional direct
    /// Binary was unsupported: an unconstrained optional Binary now renders in
    /// the shared per-field wrapper over `Binary_Vectors.Vector`.
    ///
    /// A *constrained* optional Binary must still fail closed, because the Ada
    /// backend has no field-local Binary facet lowering and Task 035 adds none.
    #[test]
    fn optional_binary_field_uses_a_wrapper_but_constrained_stays_unsupported() {
        let mut schema = floating_schema();
        let TypeKind::Record { fields } = &mut schema.types[0].kind else {
            panic!("floating fixture should contain a record");
        };
        fields.truncate(1);
        fields[0].type_ref = TypeRef::primitive(PrimitiveKind::Binary);
        fields[0].cardinality = Cardinality::OPTIONAL_ONE;
        let member = fields[0].name.clone();
        let owner = schema.types[0].name.local_name.clone();
        let source = generate(&schema, CLOSED).expect("optional binary field must render");
        assert!(
            source.contains(&format!(
                "type {owner}_{member}_Optional (Is_Present : Boolean := False) is record"
            )),
            "{source}"
        );
        // The wrapper stores the backend's existing Binary representation.
        assert!(
            source.contains("when True  => Value : Binary_Vectors.Vector;"),
            "{source}"
        );

        // Same field, plus a field-local length facet: still fail-closed.
        let TypeKind::Record { fields } = &mut schema.types[0].kind else {
            panic!("floating fixture should contain a record");
        };
        fields[0].constraints = ConstraintSet {
            length: Some(4),
            ..ConstraintSet::default()
        };
        let error = generate(&schema, CLOSED)
            .expect_err("a constrained optional binary field must stay unsupported");
        // The diagnostic names the unlowerable *facet*, not the optionality:
        // the field-local length is the real reason, and Task 035 preserves that
        // attribution rather than blaming the occurrence.
        assert!(
            error
                .message
                .contains("unsupported Ada IR construct: field constraints on"),
            "{}",
            error.message
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
            // A constrained String now reports through the Task 037
            // classifier, which names the facet profile rather than saying
            // only "constraints". Either way it fails closed:
            // is not the one supported profile.
            assert!(
                error
                    .message
                    .contains("unsupported Ada IR construct: constraints on")
                    || error.message.contains(
                        "unsupported constrained String declaration: unsupported facet profile"
                    ),
                "unexpected diagnostic for {kind:?}: {}",
                error.message
            );
        }
    }

    /// Lexical constraints still fail before rendering everywhere Task 036 has
    /// not implemented a validator.
    ///
    /// `DateTime` + `.+Z` is deliberately absent from this list: that exact
    /// pair is the one Task 036 profile the generator now fully enforces, and
    /// the wording below distinguishes it from `Time` carrying the *same*
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
                message.contains("unsupported Ada IR construct: constraints on")
                    || message.contains("unsupported temporal declaration: Time")
                    || message.contains(
                        "unsupported constrained String declaration: unsupported facet profile"
                    ),
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
    fn future_descendant_reclassifies_uninhabited_target_and_stores_the_optional_value() {
        // Once a concrete descendant exists the target is inhabited again and
        // Task 024's ordinary closed-sum lowering applies.
        //
        // Before Task 034 this stopped at Ada's optional-named-value gap and
        // the test asserted that failure. That gap is exactly what Task 034
        // removes, so the field now lowers to its own generated wrapper over
        // the Task 024 closed sum -- the reclassification result is stored
        // rather than rejected. Task 026's own firewall is untouched: the
        // *uninhabited* cases above still fail and still elide, and the field
        // only becomes storable here because a concrete descendant exists.
        let source = generate(&uninhabited_future_descendant_schema(), CLOSED)
            .expect("an inhabited optional named value must now lower");
        assert!(
            source
                .contains("type Holder_Widget_Optional (Is_Present : Boolean := False) is record"),
            "{source}"
        );
        assert!(
            source.contains("when True  => Value : SidecarPoint;"),
            "{source}"
        );
        assert!(
            source.contains("Widget : Holder_Widget_Optional;"),
            "{source}"
        );
        // The target really is the Task 024 closed sum, not a fake payload.
        assert!(source.contains("type SidecarPoint_Kind is"), "{source}");
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

    /// Compile the overlay-composed Ada spec, appending a `PrivateA` value to
    /// the generated repeated collection, when GNAT is installed.
    ///
    /// The caller's generated-text assertions are unconditional; only this
    /// compile probe is skipped when GNAT is absent. Unlike `c++`, GNAT is not
    /// present on the stock CI image, and a missing optional toolchain must not
    /// be reported as a lowering regression. The probe still runs in every
    /// environment that has GNAT, including local development.
    fn compile_overlay_probe_if_gnat_available(source: &str) {
        use std::fs;
        use std::process::Command;

        if Command::new("gnatmake").arg("--version").output().is_err() {
            return;
        }

        let directory = std::env::temp_dir().join("ams-gra-oms-task029-overlay-ada");
        fs::create_dir_all(&directory).expect("create Ada probe directory");
        fs::write(directory.join("urn.ads"), "package Urn is\nend Urn;\n")
            .expect("write Ada parent package");
        fs::write(directory.join("urn-overlay.ads"), source).expect("write generated Ada spec");
        fs::write(
            directory.join("probe.adb"),
            r#"with Urn.Overlay; use Urn.Overlay;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;

procedure Probe is
   Value : constant ExtensionBase :=
     (Kind => PrivateA_Kind,
      PrivateA_Value =>
        (Label => To_Unbounded_String ("l"),
         PrivateCodeA => To_Unbounded_String ("p")));
   Items : Container_Extensions_Sequence;
   Holder : Container;
begin
   Items.Append (Value);
   Holder := (Name => To_Unbounded_String ("c"), Extensions => Items);
   pragma Assert (Natural (Holder.Extensions.Length) = 1);
end Probe;
"#,
        )
        .expect("write Ada probe unit");

        let status = Command::new("gnatmake")
            .current_dir(&directory)
            .args(["-gnatwa", "-gnata", "probe.adb"])
            .status()
            .expect("GNAT reported a version, so it must be runnable");
        fs::remove_dir_all(&directory).expect("remove Ada probe directory");
        assert!(
            status.success(),
            "overlay-composed closed sum must compile under GNAT"
        );
    }

    /// Task 029 sections 29/30: supplying the private derived type through an
    /// explicit overlay -- with no edit to the public root -- closes the Task
    /// 024 sum for the repeated 0..* extension point, and where GNAT is
    /// available the generated spec compiles while a PrivateA value is appended
    /// to the repeated collection. The same composed schema still fails closed
    /// under `open-extensions`.
    #[test]
    fn task029_private_overlay_closes_the_sum_but_not_the_world() {
        let source = generate(&overlay_composed_schema(&["private-a.xsd"]), CLOSED)
            .expect("an explicit overlay must close the extension point");
        assert!(source.contains("type ExtensionBase (Kind : ExtensionBase_Kind"));
        assert!(source.contains("PrivateA_Value : PrivateA;"));
        assert!(
            source.contains("Extensions : Container_Extensions_Sequence;"),
            "the repeated 0..* field must still be generated: {source}"
        );

        compile_overlay_probe_if_gnat_available(&source);

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
        assert!(forward.contains("      PrivateA_Kind,\n      PrivateB_Kind);"));

        let reversed = generate(
            &overlay_composed_schema(&["private-b.xsd", "private-a.xsd"]),
            CLOSED,
        )
        .expect("reversed overlays should compose");
        assert!(reversed.contains("      PrivateB_Kind,\n      PrivateA_Kind);"));
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

    /// Ada identifiers are case-insensitive, so `Track` and `TRACK` are one
    /// type name and the generated package would declare it twice.
    #[test]
    fn case_only_declaration_collision_is_rejected() {
        let schema = preflight_fixture("backend-name-preflight-ada-case-collision.xsd");
        let message = generate(&schema, CLOSED)
            .expect_err("Ada case-only declaration collision must be rejected")
            .message;
        assert!(
            message.contains("\"Track\"") && message.contains("\"TRACK\""),
            "{message}"
        );
    }

    /// An inherited component and a locally declared component that differ
    /// only by case are one Ada component. Effective structural projection
    /// accepts them because the XSD names differ.
    #[test]
    fn case_only_inherited_component_collision_is_rejected() {
        let schema = preflight_fixture("backend-name-preflight-inherited-collision.xsd");
        let message = generate(&schema, CLOSED)
            .expect_err("Ada case-only component collision must be rejected")
            .message;
        assert!(
            message.contains("track_id") && message.contains("Track_Id"),
            "{message}"
        );
    }

    /// A type named `Record` lands on an Ada reserved word. GNAT rejects the
    /// previously generated spec with "reserved word \"record\" cannot be used
    /// as identifier", so this must fail before any output is produced.
    #[test]
    fn reserved_word_declaration_is_rejected() {
        let schema = preflight_fixture("backend-name-preflight-reserved.xsd");
        let message = generate(&schema, CLOSED)
            .expect_err("an Ada reserved word declaration must be rejected")
            .message;
        assert!(message.contains("reserved word"), "{message}");
    }

    /// Ada keeps both spellings verbatim and they differ by more than case, so
    /// the Rust/C++ upper-camel convergence is NOT an Ada collision. Asserting
    /// this keeps the shared preflight from applying one backend's rule
    /// everywhere.
    #[test]
    fn upper_camel_convergence_is_not_an_ada_collision() {
        let schema = preflight_fixture("backend-name-preflight-declaration-collision.xsd");
        assert!(generate(&schema, CLOSED).is_ok());
    }

    /// The control must still render, and where GNAT is available the
    /// generated spec must actually compile: a preflight that rejected
    /// everything would pass the negative tests above while being useless.
    #[test]
    fn preflight_control_renders_and_compiles() {
        let source = generate(
            &preflight_fixture("backend-name-preflight-control.xsd"),
            CLOSED,
        )
        .expect("safe generated names must render");
        assert!(source.contains("type TrackReport is record"), "{source}");
        assert!(source.contains("TrackId"), "{source}");

        use std::fs;
        use std::process::Command;
        if Command::new("gnatmake").arg("--version").output().is_err() {
            // CI installs GNAT and sets this, so a silent skip there is a CI
            // defect rather than an acceptable outcome.
            assert!(
                std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                "AMS_GRA_REQUIRE_GNAT is set but GNAT is not runnable"
            );
            return;
        }

        let directory = std::env::temp_dir().join("ams-gra-oms-preflight-ada-control");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create Ada probe directory");
        // The fixture namespace `urn:preflight:test` yields package
        // `Preflight.Test`, so the child unit's file name must match.
        fs::write(
            directory.join("preflight.ads"),
            "package Preflight is\nend Preflight;\n",
        )
        .expect("write Ada parent package");
        fs::write(directory.join("preflight-test.ads"), &source).expect("write generated Ada spec");
        let status = Command::new("gnatmake")
            .current_dir(&directory)
            .args(["-gnatwa", "-c", "preflight-test.ads"])
            .status()
            .expect("GNAT reported a version, so it must be runnable");
        let _ = fs::remove_dir_all(&directory);
        assert!(status.success(), "generated Ada spec must compile");
    }

    /// Generated-name preflight tracks *emitted* entities, not raw Schema IR
    /// declarations. An ancestry-only abstract Record is folded into its
    /// descendants and never emitted, so its local name does not occupy the
    /// generated package -- even when that name is `Optional_String`, which
    /// backend-ada always emits as a support type.
    ///
    /// The generated spec must therefore contain exactly one
    /// `Optional_String` and compile under GNAT.
    #[test]
    fn ancestry_only_abstract_support_name_renders_and_compiles_under_gnat() {
        let schema = preflight_fixture("backend-ancestry-only-ada-support-name.xsd");
        assert!(
            ams_gra_oms_codegen_core::validate_backend_names(&schema, BackendLanguage::Ada, CLOSED)
                .is_ok(),
            "an abstract base Ada never emits must not reserve Optional_String"
        );
        let source = generate(&schema, CLOSED).expect("ancestry-only base must render");
        // Exactly one declaration of the support type, and none from the
        // schema-owned abstract base.
        assert_eq!(
            source.matches("type Optional_String").count(),
            1,
            "{source}"
        );
        // The inherited field really is folded into the emitted descendant.
        assert!(source.contains("type Derived is record"), "{source}");
        assert!(source.contains("Inherited"), "{source}");

        use std::fs;
        use std::process::Command;
        if Command::new("gnatmake").arg("--version").output().is_err() {
            assert!(
                std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                "AMS_GRA_REQUIRE_GNAT is set but GNAT is not runnable"
            );
            return;
        }
        let directory = std::env::temp_dir().join("ams-gra-oms-ancestry-ada-control");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create Ada probe directory");
        // Namespace `urn:preflight:ancestryada` yields package
        // `Preflight.Ancestryada`.
        fs::write(
            directory.join("preflight.ads"),
            "package Preflight is\nend Preflight;\n",
        )
        .expect("write Ada parent package");
        fs::write(directory.join("preflight-ancestryada.ads"), &source)
            .expect("write generated Ada spec");
        let status = Command::new("gnatmake")
            .current_dir(&directory)
            .args(["-gnatwa", "-c", "preflight-ancestryada.ads"])
            .status()
            .expect("GNAT reported a version, so it must be runnable");
        let _ = fs::remove_dir_all(&directory);
        assert!(status.success(), "generated Ada spec must compile");
    }

    /// Ada repeated helpers are named after the **emitted** owner. `Base` is
    /// abstract ancestry that backend-ada never writes, so it emits no
    /// `Base_Items_Array`; the inherited field's helpers appear under
    /// `Derived`. A user type spelled `Base_Items_Array` must therefore be
    /// accepted, render, and compile under GNAT.
    #[test]
    fn a_non_emitted_abstract_owner_emits_no_helper_and_compiles_under_gnat() {
        let schema = preflight_fixture("backend-abstract-helper-owner.xsd");
        assert!(
            ams_gra_oms_codegen_core::validate_backend_names(&schema, BackendLanguage::Ada, CLOSED)
                .is_ok(),
            "no helper is emitted under a non-emitted abstract Record owner"
        );
        let source = generate(&schema, CLOSED).expect("the control schema must render");
        // The helper Ada really emits carries the emitted descendant's stem.
        assert!(source.contains("type Derived_Items_Array"), "{source}");
        assert!(source.contains("type Derived_Items_Sequence"), "{source}");
        // `Base_Items_Array` appears exactly once, as the user declaration --
        // never as a generated helper under the non-emitted abstract base.
        assert_eq!(
            source.matches("type Base_Items_Array").count(),
            1,
            "{source}"
        );
        assert!(!source.contains("Base_Items_Sequence"), "{source}");
        // The abstract base itself is not written at all.
        assert!(!source.contains("type Base is"), "{source}");

        use std::fs;
        use std::process::Command;
        if Command::new("gnatmake").arg("--version").output().is_err() {
            assert!(
                std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                "AMS_GRA_REQUIRE_GNAT is set but GNAT is not runnable"
            );
            return;
        }
        let directory = std::env::temp_dir().join("ams-gra-oms-helper-owner-ada-control");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create Ada probe directory");
        // Namespace `urn:preflight:helperowner` yields package
        // `Preflight.Helperowner`.
        fs::write(
            directory.join("preflight.ads"),
            "package Preflight is\nend Preflight;\n",
        )
        .expect("write Ada parent package");
        fs::write(directory.join("preflight-helperowner.ads"), &source)
            .expect("write generated Ada spec");
        let status = Command::new("gnatmake")
            .current_dir(&directory)
            .args(["-gnatwa", "-c", "preflight-helperowner.ads"])
            .status()
            .expect("GNAT reported a version, so it must be runnable");
        let _ = fs::remove_dir_all(&directory);
        assert!(status.success(), "generated Ada spec must compile");
    }

    /// The counterpart: the inherited field's helper really is emitted under
    /// the concrete descendant, so colliding with *that* spelling is rejected.
    /// This proves the corrective did not simply stop validating inherited
    /// members.
    #[test]
    fn an_inherited_helper_under_the_emitted_descendant_is_still_rejected() {
        let schema = preflight_fixture("backend-abstract-helper-owner-collision.xsd");
        let message = generate(&schema, CLOSED)
            .expect_err("the real emitted helper name must still collide")
            .message;
        assert!(message.contains("Derived_Items_Array"), "{message}");
    }

    /// Task 026 counterpart under Ada: an elided zero-descendant target
    /// reserves neither its own name nor an `Optional_String_Kind` companion,
    /// so the generated support type keeps the identifier.
    #[test]
    fn task026_elided_target_does_not_reserve_its_own_ada_name() {
        let schema = preflight_fixture("backend-elided-target-support-name.xsd");
        assert!(
            ams_gra_oms_codegen_core::validate_backend_names(&schema, BackendLanguage::Ada, CLOSED)
                .is_ok(),
            "a Task 026 elided target must not reserve its own Ada name"
        );
        let source = generate(&schema, CLOSED).expect("elided target must render");
        assert_eq!(
            source.matches("type Optional_String").count(),
            1,
            "{source}"
        );
        // No `_Kind` companion is manufactured for an elided target.
        assert!(!source.contains("Optional_String_Kind"), "{source}");

        use std::fs;
        use std::process::Command;
        if Command::new("gnatmake").arg("--version").output().is_err() {
            assert!(
                std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                "AMS_GRA_REQUIRE_GNAT is set but GNAT is not runnable"
            );
            return;
        }
        let directory = std::env::temp_dir().join("ams-gra-oms-elided-ada-control");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create Ada probe directory");
        fs::write(
            directory.join("preflight.ads"),
            "package Preflight is\nend Preflight;\n",
        )
        .expect("write Ada parent package");
        fs::write(directory.join("preflight-elided.ads"), &source)
            .expect("write generated Ada spec");
        let status = Command::new("gnatmake")
            .current_dir(&directory)
            .args(["-gnatwa", "-c", "preflight-elided.ads"])
            .status()
            .expect("GNAT reported a version, so it must be runnable");
        let _ = fs::remove_dir_all(&directory);
        assert!(status.success(), "generated Ada spec must compile");
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

    // ---------------------------------------------------------------
    // Task 034 -- optional named Record values
    // ---------------------------------------------------------------

    /// An Ada probe that exercises **both** states of both generated wrappers.
    ///
    /// The absent state is the default discriminant, so `Absent_Mode` is
    /// written with no aggregate at all -- that is the point of defaulting
    /// `Is_Present` to `False`. The present state carries a real `Value`, and
    /// the probe reads it back, so presence is not merely constructible but
    /// actually usable.
    ///
    /// Reading `Value` from an absent wrapper must raise `Constraint_Error`
    /// rather than yield a sentinel; the probe asserts that too, which is what
    /// makes "absence and presence are explicit states" a checked claim
    /// instead of a comment.
    const ADA_OPTIONAL_PROBE: &str = r#"with Ada.Strings.Unbounded;
with Optional.Named;
procedure Probe is
   use Optional.Named;
   Absent_Mode    : Container_Maybe_Mode_Optional;
   Present_Mode   : constant Container_Maybe_Mode_Optional :=
     (Is_Present => True, Value => Active);
   Absent_Details : Container_Maybe_Details_Optional;
begin
   if Absent_Mode.Is_Present then
      raise Program_Error with "default wrapper must be absent";
   end if;
   if not Present_Mode.Is_Present or else Present_Mode.Value /= Active then
      raise Program_Error with "present wrapper must carry its value";
   end if;
   if Absent_Details.Is_Present then
      raise Program_Error with "default record wrapper must be absent";
   end if;

   declare
      Present_Details : constant Container_Maybe_Details_Optional :=
        (Is_Present => True,
         Value      =>
           (Label => Standard.Ada.Strings.Unbounded.To_Unbounded_String ("x"),
            Count => 7));
   begin
      if Present_Details.Value.Count /= 7 then
         raise Program_Error with "present record wrapper must carry its value";
      end if;
   end;

   --  Absence is a real state, not a sentinel: reading through it is an error.
   declare
      Ignored : Mode;
   begin
      Ignored := Absent_Mode.Value;
      raise Program_Error with "reading an absent value must be checked";
   exception
      when Constraint_Error => null;
   end;
end Probe;
"#;

    /// Task 035 runtime probe for optional **direct primitive** wrappers.
    ///
    /// Exercises Boolean, UnsignedInteger, and Float64 in both presence states,
    /// and additionally *uses* the Binary wrapper so the generated type is
    /// proven to compile against `Binary_Vectors.Vector` rather than merely
    /// being emitted as text.
    ///
    /// As in Task 034, the absent state is written with no aggregate at all --
    /// that is the point of defaulting `Is_Present` to `False` -- and reading
    /// `Value` through an absent wrapper must raise `Constraint_Error` rather
    /// than yield a sentinel.
    const ADA_PRIMITIVE_OPTIONAL_PROBE: &str = r#"with Interfaces;
with Optional.Primitive;
procedure Probe is
   use Optional.Primitive;
   --  The float wrappers store `Interfaces.IEEE_Float_*`, whose operators are
   --  declared in `Interfaces`; the probe compares values, so it needs them.
   use Interfaces;
   Absent_Bool      : PrimitiveOptionals_Maybe_Bool_Optional;
   Present_Bool     : constant PrimitiveOptionals_Maybe_Bool_Optional :=
     (Is_Present => True, Value => True);
   Absent_Unsigned  : PrimitiveOptionals_Maybe_Unsigned_Optional;
   Present_Unsigned : constant PrimitiveOptionals_Maybe_Unsigned_Optional :=
     (Is_Present => True, Value => 4_294_967_295);
   Absent_F64       : PrimitiveOptionals_Maybe_F64_Optional;
   Present_F64      : constant PrimitiveOptionals_Maybe_F64_Optional :=
     (Is_Present => True, Value => 2.5);
   Absent_Binary    : PrimitiveOptionals_Maybe_Binary_Optional;
begin
   --  Absence is the default for every generated wrapper.
   if Absent_Bool.Is_Present
     or else Absent_Unsigned.Is_Present
     or else Absent_F64.Is_Present
     or else Absent_Binary.Is_Present
   then
      raise Program_Error with "default wrapper must be absent";
   end if;

   --  Presence retains the supplied value.
   if not Present_Bool.Is_Present or else Present_Bool.Value /= True then
      raise Program_Error with "present Boolean wrapper must carry its value";
   end if;
   if not Present_Unsigned.Is_Present
     or else Present_Unsigned.Value /= 4_294_967_295
   then
      raise Program_Error with "present Unsigned wrapper must carry its value";
   end if;
   if not Present_F64.Is_Present or else Present_F64.Value /= 2.5 then
      raise Program_Error with "present Float64 wrapper must carry its value";
   end if;

   --  The Binary wrapper is really usable over Binary_Vectors.Vector.
   declare
      Octets : Binary_Vectors.Vector;
   begin
      Octets.Append (16#AB#);
      Octets.Append (16#CD#);
      declare
         Present_Binary : constant PrimitiveOptionals_Maybe_Binary_Optional :=
           (Is_Present => True, Value => Octets);
      begin
         if Natural (Present_Binary.Value.Length) /= 2 then
            raise Program_Error with "present Binary wrapper must carry its value";
         end if;
      end;
   end;

   --  Absence is a real state, not a sentinel: reading through it is an error.
   declare
      Ignored : Interfaces.Unsigned_64;
   begin
      Ignored := Absent_Unsigned.Value;
      raise Program_Error with "reading an absent value must be checked";
   exception
      when Constraint_Error => null;
   end;
end Probe;
"#;

    /// Task 035 core control: every optional **direct primitive** field inside
    /// the supported subset lowers to its own generated discriminated wrapper,
    /// direct optional String keeps the shared `Optional_String`, and the result
    /// compiles **and runs** under GNAT in both presence states.
    #[test]
    fn optional_direct_primitive_values_render_and_execute_under_gnat() {
        let schema = preflight_fixture("backend-optional-primitive-values.xsd");
        let source = generate(&schema, CLOSED).expect("optional direct primitives must render");

        // Every supported direct primitive gains a wrapper, storing the exact
        // representation the backend already uses for that primitive.
        for (member, value_type) in [
            ("Maybe_Bool", "Boolean"),
            (
                "Maybe_Signed",
                "Long_Long_Integer range -9_223_372_036_854_775_808 .. 9_223_372_036_854_775_807",
            ),
            (
                "Maybe_Unsigned",
                "Interfaces.Unsigned_64 range 0 .. 4294967295",
            ),
            ("Maybe_F32", "Interfaces.IEEE_Float_32"),
            ("Maybe_F64", "Interfaces.IEEE_Float_64"),
            ("Maybe_Binary", "Binary_Vectors.Vector"),
        ] {
            assert!(
                source.contains(&format!(
                    "type PrimitiveOptionals_{member}_Optional (Is_Present : Boolean := False) is record"
                )),
                "{member} must gain a wrapper:\n{source}"
            );
            assert!(
                source.contains(&format!("when True  => Value : {value_type};")),
                "{member} must store {value_type}:\n{source}"
            );
            assert!(
                source.contains(&format!(
                    "      {member} : PrimitiveOptionals_{member}_Optional;"
                )),
                "{member} must use its wrapper:\n{source}"
            );
        }

        // Direct optional String is UNCHANGED: it keeps the shared
        // `Optional_String` and gains no per-field wrapper, so Task 035 causes
        // no churn in already generated output.
        assert!(
            source.contains("      Maybe_String : Optional_String;"),
            "{source}"
        );
        assert!(
            !source.contains("PrimitiveOptionals_Maybe_String_Optional"),
            "direct optional String must not gain a per-field wrapper:\n{source}"
        );

        // Task 035 changes the optional occurrence only: required direct
        // primitive fields are still stored directly, with no wrapper.
        assert!(
            source.contains("      Required_Bool : Boolean;"),
            "{source}"
        );
        assert!(
            source.contains(
                "      Required_Unsigned : Interfaces.Unsigned_64 range 0 .. 4294967295;"
            ),
            "{source}"
        );
        assert!(
            source.contains("      Required_F64 : Interfaces.IEEE_Float_64;"),
            "{source}"
        );
        for member in ["Required_Bool", "Required_Unsigned", "Required_F64"] {
            assert!(
                !source.contains(&format!("{member}_Optional")),
                "{member} must not gain a wrapper:\n{source}"
            );
        }

        // Each wrapper is declared before the record that uses it, and no
        // shared generic Optional abstraction was introduced.
        let record = source
            .find("type PrimitiveOptionals is record")
            .expect("PrimitiveOptionals must be emitted");
        for member in [
            "Maybe_Bool",
            "Maybe_Signed",
            "Maybe_Unsigned",
            "Maybe_F32",
            "Maybe_F64",
            "Maybe_Binary",
        ] {
            let helper = format!("type PrimitiveOptionals_{member}_Optional");
            assert!(
                source.find(&helper).expect("helper must be emitted") < record,
                "{helper} must precede the record that uses it:\n{source}"
            );
        }
        assert!(!source.contains("generic"), "{source}");

        use std::fs;
        use std::process::Command;
        if Command::new("gnatmake").arg("--version").output().is_err() {
            assert!(
                std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                "AMS_GRA_REQUIRE_GNAT is set but GNAT is not runnable"
            );
            return;
        }
        let directory = std::env::temp_dir().join("ams-gra-oms-task035-ada-primitive");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create Ada probe directory");
        // Namespace `urn:optional:primitive` yields package `Optional.Primitive`.
        fs::write(
            directory.join("optional.ads"),
            "package Optional is\nend Optional;\n",
        )
        .expect("write Ada parent package");
        fs::write(directory.join("optional-primitive.ads"), &source)
            .expect("write generated Ada spec");
        fs::write(directory.join("probe.adb"), ADA_PRIMITIVE_OPTIONAL_PROBE)
            .expect("write Ada probe");
        let status = Command::new("gnatmake")
            .current_dir(&directory)
            .args(["-q", "probe.adb"])
            .status()
            .expect("GNAT reported a version, so it must be runnable");
        assert!(status.success(), "generated Ada spec must compile");
        let run = Command::new(directory.join("probe"))
            .status()
            .expect("compiled probe must run");
        let _ = fs::remove_dir_all(&directory);
        assert!(
            run.success(),
            "both wrapper states must behave as generated"
        );
    }

    /// Task 035 inherited-owner control, shaped like the UCI `MissionID_Type` /
    /// `VersionedID_Type` boundary this task unblocked. The wrapper is named
    /// after the **emitted** owner, so `Base` -- ancestry-only and never written
    /// -- emits no `Base_Version_Optional`, and a user type carrying that
    /// spelling must be accepted, render, and compile.
    #[test]
    fn an_inherited_direct_primitive_optional_uses_the_emitted_owner_under_gnat() {
        let schema = preflight_fixture("backend-optional-primitive-inherited.xsd");
        assert!(
            ams_gra_oms_codegen_core::validate_backend_names(&schema, BackendLanguage::Ada, CLOSED)
                .is_ok(),
            "no optional wrapper is emitted under a non-emitted abstract Record owner"
        );
        let source = generate(&schema, CLOSED).expect("the inherited control must render");

        assert!(
            source.contains(
                "type Derived_Version_Optional (Is_Present : Boolean := False) is record"
            ),
            "{source}"
        );
        assert!(
            source.contains("      Version : Derived_Version_Optional;"),
            "{source}"
        );
        // `Base_Version_Optional` appears exactly once, as the user declaration
        // -- never as a generated wrapper under the non-emitted abstract base.
        assert_eq!(
            source.matches("type Base_Version_Optional").count(),
            1,
            "{source}"
        );
        assert!(!source.contains("type Base is"), "{source}");

        use std::fs;
        use std::process::Command;
        if Command::new("gnatmake").arg("--version").output().is_err() {
            assert!(
                std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                "AMS_GRA_REQUIRE_GNAT is set but GNAT is not runnable"
            );
            return;
        }
        let directory = std::env::temp_dir().join("ams-gra-oms-task035-ada-inherited");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create Ada probe directory");
        // `urn:optional:primitive-inherited` splits on non-alphanumerics, so the
        // last two components yield the package `Primitive.Inherited`.
        let package = package_name(&schema).expect("the fixture namespace must map");
        let (parent, _) = package
            .split_once('.')
            .expect("the generated package is always a child unit");
        fs::write(
            directory.join(format!("{}.ads", parent.to_ascii_lowercase())),
            format!("package {parent} is\nend {parent};\n"),
        )
        .expect("write Ada parent package");
        let stem = package.to_ascii_lowercase().replace('.', "-");
        fs::write(directory.join(format!("{stem}.ads")), &source)
            .expect("write generated Ada spec");
        let status = Command::new("gnatmake")
            .current_dir(&directory)
            .args(["-gnatwa", "-c", &format!("{stem}.ads")])
            .status()
            .expect("GNAT reported a version, so it must be runnable");
        let _ = fs::remove_dir_all(&directory);
        assert!(status.success(), "generated Ada spec must compile");
    }

    /// Task 035 collision control: the wrapper Ada really emits occupies a real
    /// top-level name, so a user declaration spelled the same way must be
    /// rejected -- with attribution naming both responsible declarations.
    ///
    /// Rust and C++ are controls: neither emits the Ada helper, so the spelling
    /// stays a free top-level name for them.
    #[test]
    fn a_direct_primitive_optional_wrapper_name_collision_is_rejected() {
        let schema = preflight_fixture("backend-optional-primitive-collision.xsd");
        let error =
            ams_gra_oms_codegen_core::validate_backend_names(&schema, BackendLanguage::Ada, CLOSED)
                .expect_err("the emitted wrapper genuinely occupies this name");
        assert!(
            error.to_string().contains("Owner_Version_Optional"),
            "{error}"
        );
        // Generation fails for the same reason, through the shared preflight.
        let generated = generate(&schema, CLOSED)
            .expect_err("Ada generation must not emit a doubly declared name");
        assert!(
            generated.message.contains("Owner_Version_Optional"),
            "{}",
            generated.message
        );
        // Rust and C++ never emit this helper, so the spelling stays free.
        for language in [BackendLanguage::Rust, BackendLanguage::Cpp] {
            assert!(
                ams_gra_oms_codegen_core::validate_backend_names(&schema, language, CLOSED).is_ok(),
                "{language:?} must not reserve the Ada optional wrapper spelling"
            );
        }
    }

    /// Task 035 negative controls, all applied to the same optional direct
    /// primitive field so the only difference is the property under test.
    ///
    /// Each of these must stay fail-closed: Task 035 adds an occurrence
    /// representation, not nillability, not field-local constraint lowering, and
    /// not primitive expansion.
    #[test]
    fn direct_primitive_optional_boundaries_remain_unsupported() {
        // Start from the Task 035 control, which renders, and keep exactly one
        // optional direct primitive field so each boundary is isolated.
        let mut schema = preflight_fixture("backend-optional-primitive-values.xsd");
        let record = schema
            .types
            .iter()
            .position(|declaration| declaration.name.local_name == "PrimitiveOptionals")
            .expect("the control fixture declares PrimitiveOptionals");
        let TypeKind::Record { fields } = &mut schema.types[record].kind else {
            panic!("PrimitiveOptionals is a record");
        };
        fields.retain(|field| field.name == "Maybe_Unsigned");
        assert_eq!(fields.len(), 1, "one isolated optional field");
        assert!(
            generate(&schema, CLOSED).is_ok(),
            "the isolated control must render before each boundary is applied"
        );

        // Nillable: nil is a third state the two-state wrapper cannot express.
        let TypeKind::Record { fields } = &mut schema.types[record].kind else {
            panic!("PrimitiveOptionals is a record");
        };
        fields[0].nillable = true;
        let error =
            generate(&schema, CLOSED).expect_err("a nillable optional must stay unsupported");
        // The diagnostic names nillability, which is the actual reason.
        assert!(
            error
                .message
                .contains("unsupported Ada IR construct: nillable field"),
            "{}",
            error.message
        );

        // A field-local integral shape Task 020 cannot lower: no silent facet
        // loss into an unconstrained wrapper.
        let TypeKind::Record { fields } = &mut schema.types[record].kind else {
            panic!("PrimitiveOptionals is a record");
        };
        fields[0].nillable = false;
        fields[0].constraints = ConstraintSet {
            min_exclusive: Some(ams_gra_oms_ir::NumericValue::Integer(0)),
            ..ConstraintSet::default()
        };
        let error = generate(&schema, CLOSED)
            .expect_err("an unlowerable integral facet must stay unsupported");
        assert!(
            error.message.contains("unsupported Ada IR construct"),
            "{}",
            error.message
        );

        // Temporal and Decimal: occurrence storage does not grant primitive
        // support, and the diagnostic keeps blaming the primitive/target rather
        // than claiming optionality is the root problem.
        for kind in [
            PrimitiveKind::DateTime,
            PrimitiveKind::Time,
            PrimitiveKind::Duration,
            PrimitiveKind::Decimal,
        ] {
            let TypeKind::Record { fields } = &mut schema.types[record].kind else {
                panic!("PrimitiveOptionals is a record");
            };
            fields[0].type_ref = TypeRef::primitive(kind);
            fields[0].constraints = ConstraintSet::default();
            let error = generate(&schema, CLOSED)
                .expect_err("an unsupported primitive must stay unsupported when optional");
            assert!(
                error.message.contains("type reference"),
                "{kind:?} must still be rejected for its primitive type, not its \
                 cardinality: {}",
                error.message
            );
        }
    }

    /// Task 035 is Record fields only. An optional direct primitive **Choice
    /// alternative** stays fail-closed: a Choice's exclusivity is already
    /// carried by its generated `Kind` discriminant, and nothing here justifies
    /// giving one alternative a second nested discriminant.
    #[test]
    fn optional_direct_primitive_choice_alternatives_remain_unsupported() {
        let mut schema = choice_schema();
        let TypeKind::Choice { alternatives } = &mut schema.types[1].kind else {
            panic!("choice fixture should contain a choice");
        };
        alternatives[0].type_ref = TypeRef::primitive(PrimitiveKind::UnsignedInteger);
        alternatives[0].constraints = ConstraintSet::default();
        alternatives[0].cardinality = Cardinality::OPTIONAL_ONE;
        let error = generate(&schema, CLOSED)
            .expect_err("an optional direct primitive Choice alternative must stay unsupported");
        assert!(
            error
                .message
                .contains("unsupported Ada IR construct: cardinality on Choice alternative"),
            "{}",
            error.message
        );
    }

    /// Task 034 core control: a non-nillable `0..1` named Enumeration field and
    /// a non-nillable `0..1` named Record field each lower to their own
    /// generated discriminated wrapper, and the result compiles **and runs**
    /// under GNAT in both presence states.
    #[test]
    fn optional_named_values_render_and_execute_under_gnat() {
        let schema = preflight_fixture("backend-optional-named-values.xsd");
        let source = generate(&schema, CLOSED).expect("optional named values must render");

        // Both wrappers exist, named after the emitted owner.
        assert!(
            source.contains(
                "type Container_Maybe_Mode_Optional (Is_Present : Boolean := False) is record"
            ),
            "{source}"
        );
        assert!(
            source.contains(
                "type Container_Maybe_Details_Optional (Is_Present : Boolean := False) is record"
            ),
            "{source}"
        );
        assert!(source.contains("when True  => Value : Mode;"), "{source}");
        assert!(
            source.contains("when True  => Value : Details;"),
            "{source}"
        );
        // The components really use the wrappers.
        assert!(
            source.contains("Maybe_Mode : Container_Maybe_Mode_Optional;"),
            "{source}"
        );
        assert!(
            source.contains("Maybe_Details : Container_Maybe_Details_Optional;"),
            "{source}"
        );
        // Task 034 changes the optional occurrence only: the required named
        // field is still stored directly, with no wrapper.
        assert!(source.contains("Required_Mode : Mode;"), "{source}");
        assert!(!source.contains("Required_Mode_Optional"), "{source}");
        // Each wrapper is declared before the record that uses it.
        let container = source
            .find("type Container is record")
            .expect("Container must be emitted");
        for helper in [
            "type Container_Maybe_Mode_Optional",
            "type Container_Maybe_Details_Optional",
        ] {
            assert!(
                source.find(helper).expect("helper must be emitted") < container,
                "{helper} must precede the record that uses it:\n{source}"
            );
        }
        // No shared generic Optional abstraction was introduced.
        assert!(!source.contains("generic"), "{source}");

        use std::fs;
        use std::process::Command;
        if Command::new("gnatmake").arg("--version").output().is_err() {
            assert!(
                std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                "AMS_GRA_REQUIRE_GNAT is set but GNAT is not runnable"
            );
            return;
        }
        let directory = std::env::temp_dir().join("ams-gra-oms-task034-ada-optional");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create Ada probe directory");
        // Namespace `urn:optional:named` yields package `Optional.Named`.
        fs::write(
            directory.join("optional.ads"),
            "package Optional is\nend Optional;\n",
        )
        .expect("write Ada parent package");
        fs::write(directory.join("optional-named.ads"), &source).expect("write generated Ada spec");
        fs::write(directory.join("probe.adb"), ADA_OPTIONAL_PROBE).expect("write Ada probe");
        let status = Command::new("gnatmake")
            .current_dir(&directory)
            .args(["-q", "probe.adb"])
            .status()
            .expect("GNAT reported a version, so it must be runnable");
        assert!(status.success(), "generated Ada spec must compile");
        let run = Command::new(directory.join("probe"))
            .status()
            .expect("compiled probe must run");
        let _ = fs::remove_dir_all(&directory);
        assert!(
            run.success(),
            "both wrapper states must behave as generated"
        );
    }

    /// The Task 034 wrapper is named after the **emitted** owner, exactly like
    /// a repeated helper. `Base` is ancestry-only and is never written, so it
    /// emits no `Base_Maybe_Optional`; the inherited field's wrapper appears
    /// under `Derived`. A user type spelled `Base_Maybe_Optional` must
    /// therefore be accepted, render, and compile.
    #[test]
    fn a_non_emitted_abstract_owner_emits_no_optional_wrapper_and_compiles_under_gnat() {
        let schema = preflight_fixture("backend-optional-named-inherited.xsd");
        assert!(
            ams_gra_oms_codegen_core::validate_backend_names(&schema, BackendLanguage::Ada, CLOSED)
                .is_ok(),
            "no optional wrapper is emitted under a non-emitted abstract Record owner"
        );
        let source = generate(&schema, CLOSED).expect("the inherited control must render");

        // The wrapper Ada really emits carries the emitted descendant's stem.
        assert!(
            source
                .contains("type Derived_Maybe_Optional (Is_Present : Boolean := False) is record"),
            "{source}"
        );
        assert!(
            source.contains("Maybe : Derived_Maybe_Optional;"),
            "{source}"
        );
        // `Base_Maybe_Optional` appears exactly once, as the user declaration
        // -- never as a generated wrapper under the non-emitted abstract base.
        assert_eq!(
            source.matches("type Base_Maybe_Optional").count(),
            1,
            "{source}"
        );
        // The abstract base itself is not written at all.
        assert!(!source.contains("type Base is"), "{source}");

        use std::fs;
        use std::process::Command;
        if Command::new("gnatmake").arg("--version").output().is_err() {
            assert!(
                std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                "AMS_GRA_REQUIRE_GNAT is set but GNAT is not runnable"
            );
            return;
        }
        let directory = std::env::temp_dir().join("ams-gra-oms-task034-ada-inherited");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create Ada probe directory");
        // Namespace `urn:optional:inherited` yields `Optional.Inherited`.
        fs::write(
            directory.join("optional.ads"),
            "package Optional is\nend Optional;\n",
        )
        .expect("write Ada parent package");
        fs::write(directory.join("optional-inherited.ads"), &source)
            .expect("write generated Ada spec");
        let status = Command::new("gnatmake")
            .current_dir(&directory)
            .args(["-gnatwa", "-c", "optional-inherited.ads"])
            .status()
            .expect("GNAT reported a version, so it must be runnable");
        let _ = fs::remove_dir_all(&directory);
        assert!(status.success(), "generated Ada spec must compile");
    }

    /// Task 026 x Task 034 composition. `EmptyBase` has zero concrete
    /// descendants and is held only as an absent-only optional field, so the
    /// renderer removes `Holder.Maybe` before emitting anything: no record
    /// component, and no `Holder_Maybe_Optional` wrapper. Generated-name
    /// preflight must classify the field the same way, otherwise it reserves
    /// a wrapper nothing writes and falsely rejects the user declaration that
    /// legitimately carries that spelling.
    #[test]
    fn a_task026_elided_optional_field_emits_no_component_or_wrapper_and_compiles_under_gnat() {
        let schema = preflight_fixture("backend-optional-named-elided.xsd");
        assert!(
            ams_gra_oms_codegen_core::validate_backend_names(&schema, BackendLanguage::Ada, CLOSED)
                .is_ok(),
            "an elided field reserves neither its component name nor a wrapper"
        );
        let source = generate(&schema, CLOSED).expect("the elided composition must render");

        // `Holder_Maybe_Optional` appears exactly once, as the user record --
        // never as the Task 034 two-state wrapper the renderer would write for
        // a genuinely stored optional named field.
        assert!(
            !source.contains("type Holder_Maybe_Optional (Is_Present : Boolean := False)"),
            "{source}"
        );
        assert_eq!(
            source.matches("type Holder_Maybe_Optional").count(),
            1,
            "{source}"
        );
        // No `Maybe` component survives in `Holder`, and the elided target
        // itself is written nowhere.
        assert!(!source.contains("Maybe :"), "{source}");
        assert!(!source.contains("type EmptyBase"), "{source}");
        // The sibling stored field is untouched, proving the elision is
        // field-scoped rather than a whole-record bail-out.
        assert!(
            source.contains("      Required : Standard.Ada.Strings"),
            "{source}"
        );

        use std::fs;
        use std::process::Command;
        if Command::new("gnatmake").arg("--version").output().is_err() {
            assert!(
                std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                "AMS_GRA_REQUIRE_GNAT is set but GNAT is not runnable"
            );
            return;
        }
        let directory = std::env::temp_dir().join("ams-gra-oms-task034-ada-elided");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create Ada probe directory");
        // Namespace `urn:optional:elided` yields `Optional.Elided`.
        fs::write(
            directory.join("optional.ads"),
            "package Optional is\nend Optional;\n",
        )
        .expect("write Ada parent package");
        fs::write(directory.join("optional-elided.ads"), &source)
            .expect("write generated Ada spec");
        let status = Command::new("gnatmake")
            .current_dir(&directory)
            .args(["-gnatwa", "-c", "optional-elided.ads"])
            .status()
            .expect("GNAT reported a version, so it must be runnable");
        let _ = fs::remove_dir_all(&directory);
        assert!(status.success(), "generated Ada spec must compile");
    }

    /// Regression 5 at the generation boundary: under open-extensions the
    /// abstract value itself is unrepresentable, so the authoritative failure
    /// is the semantic diagnostic -- never a manufactured wrapper collision.
    #[test]
    fn an_open_world_abstract_optional_field_reports_the_semantic_diagnostic() {
        let schema = preflight_fixture("backend-optional-named-open-abstract.xsd");
        let error = generate(&schema, GenerationWorld::OpenExtensions)
            .expect_err("an open-world abstract value must fail closed");
        assert!(
            error.message.contains("open-extensions")
                && error
                    .message
                    .contains("external derived types cannot be represented"),
            "the open-world semantics must be the reported cause: {error:?}"
        );
        assert!(
            !error.message.contains("Holder_Maybe_Optional"),
            "a phantom wrapper name must not be the reported cause: {error:?}"
        );
        // Closed-world control: there the field is genuinely stored, so the
        // very same user spelling really is a collision.
        let closed = generate(&schema, CLOSED)
            .expect_err("a stored optional wrapper name must not be duplicated");
        assert!(
            closed.message.contains("Holder_Maybe_Optional"),
            "{closed:?}"
        );
    }

    /// When the owner really *is* emitted, the wrapper name is really taken,
    /// so a user declaration spelled the same way is a genuine flat-package
    /// collision. Generation must fail closed on the shared preflight rather
    /// than emit a duplicate identifier and let GNAT find it.
    #[test]
    fn an_emitted_optional_wrapper_name_collides_with_a_user_declaration() {
        let schema = preflight_fixture("backend-optional-named-collision.xsd");
        let error =
            ams_gra_oms_codegen_core::validate_backend_names(&schema, BackendLanguage::Ada, CLOSED)
                .expect_err("an emitted wrapper name must not be silently duplicated");
        assert!(
            error.to_string().contains("Owner_Maybe_Optional"),
            "{error}"
        );
        // Generation refuses too, on the same shared policy.
        generate(&schema, CLOSED).expect_err("generation must reject the colliding schema");
    }

    /// The `Container` fixture's fields, for the fail-closed controls below.
    fn optional_named_fixture_field(
        index: usize,
        mutate: impl FnOnce(&mut ams_gra_oms_ir::FieldDecl),
    ) -> SchemaIr {
        let mut schema = preflight_fixture("backend-optional-named-values.xsd");
        let container = schema
            .types
            .iter_mut()
            .find(|declaration| declaration.name.local_name == "Container")
            .expect("Container must exist");
        let TypeKind::Record { fields } = &mut container.kind else {
            panic!("Container should be a Record");
        };
        assert_eq!(fields[index].name, "Maybe_Mode");
        mutate(&mut fields[index]);
        schema
    }

    /// Nillability is a third state the two-state wrapper cannot express, so
    /// it stays fail-closed, and it is diagnosed *as* nillability rather than
    /// being silently accepted by the new optional path.
    #[test]
    fn a_nillable_optional_named_field_remains_unsupported() {
        let schema = optional_named_fixture_field(1, |field| field.nillable = true);
        let error = generate(&schema, CLOSED).expect_err("a nillable value must fail closed");
        assert!(
            error.message.contains("nillable field Maybe_Mode"),
            "{error:?}"
        );
    }

    /// A field-local constraint on an optional named value has no lowering, so
    /// it stays outside the Task 034 subset and must not quietly render as if
    /// the facet did not exist.
    ///
    /// It is rejected by the pre-existing field-constraint rule rather than by
    /// the optional path, which is the correct attribution: the problem is the
    /// unlowerable facet, not the optionality. `ada_emits_optional_wrapper`
    /// independently excludes non-default constraints, so the two agree and no
    /// wrapper is emitted for such a field even if that rule were reached.
    #[test]
    fn a_locally_constrained_optional_named_field_remains_unsupported() {
        let schema =
            optional_named_fixture_field(1, |field| field.constraints.max_length = Some(4));
        let error =
            generate(&schema, CLOSED).expect_err("a locally constrained value must fail closed");
        assert!(
            error.message.contains("field constraints on Maybe_Mode"),
            "{error:?}"
        );
        // The constraint is the stated reason, not the occurrence.
        assert!(!error.message.contains("cardinality"), "{error:?}");
    }

    /// Task 034 changes the *occurrence* representation only; it does not make
    /// an unsupported target kind supported. An optional named **temporal**
    /// value must still fail, and the diagnostic must identify the target
    /// capability rather than claim optionality is unsupported.
    ///
    /// This is the shape of the real UCI case: `Acceleration3D_Type.Timestamp`
    /// is an optional named reference to `DateTimeType`. After Task 034 it no
    /// longer fails because it is optional; it fails because `DateTimeType` is
    /// a temporal primitive this project does not lower yet.
    #[test]
    fn an_optional_unsupported_target_fails_on_the_target_not_the_occurrence() {
        let mut schema = preflight_fixture("backend-optional-named-values.xsd");
        // Retarget `Mode` at a temporal primitive, which Ada does not lower.
        let mode = schema
            .types
            .iter_mut()
            .find(|declaration| declaration.name.local_name == "Mode")
            .expect("Mode must exist");
        mode.kind = TypeKind::Primitive(PrimitiveKind::DateTime);
        let error =
            generate(&schema, CLOSED).expect_err("an unsupported target must still fail closed");
        assert!(
            error.message.contains("Mode"),
            "the target must be named: {error:?}"
        );
        assert!(
            !error.message.contains("cardinality"),
            "optionality must not be blamed: {error:?}"
        );
    }

    /// Task 034 is **Record fields only**. A Choice alternative's exclusivity
    /// is already carried by the generated `Kind` discriminant, and no
    /// authoritative evidence justifies giving one alternative a second,
    /// nested discriminant, so the optional Choice-alternative shape is left
    /// exactly as it was.
    #[test]
    fn optional_named_choice_alternatives_are_unchanged() {
        let mut schema = choice_schema();
        let TypeKind::Choice { alternatives } = &mut schema.types[1].kind else {
            panic!("expected a Choice");
        };
        alternatives[0].cardinality = Cardinality::OPTIONAL_ONE;
        let error = generate(&schema, CLOSED)
            .expect_err("Task 034 must not enable optional Choice alternatives");
        assert!(
            error
                .message
                .contains("cardinality on Choice alternative First"),
            "{error:?}"
        );
    }
}
