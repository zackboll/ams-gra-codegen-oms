//! Standalone Python 3.11+ models for a deliberately bounded Schema IR subset.
//!
//! Model and service API preflight live with the renderers. Readiness exercises
//! these same renderers; the shared three-language coverage inventory is unchanged.

mod names;
mod service_api;
mod service_codec;
pub use service_api::{SERVICE_API_FILE, generate_service_api};
pub use service_codec::{SERVICE_CODEC_FILE, generate_service_codec};

use ams_gra_oms_codegen_core::{
    Backend, CodegenError, DirectTemporalProfile, FloatingBoundKind, FloatingDomain, GeneratedFile,
    GenerationWorld, InclusiveIntegralDomain, ServiceApiModel, StringProfile, TemporalProfile,
    TypeEmission, abstract_value_targets, direct_temporal_profile, effective_record_fields,
    floating_domain, inclusive_integral_domain, plan_type_emissions, string_profile,
    temporal_profile,
};
use ams_gra_oms_ir::{
    ConstraintSet, OccurrenceShape, PrimitiveKind, SchemaIr, TypeDecl, TypeKind, TypeRef,
    TypeRefTarget,
};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::PathBuf;

pub const MODEL_FILE: &str = "model.py";

#[derive(Debug, Default, Clone, Copy)]
pub struct PythonBackend;

impl Backend for PythonBackend {
    fn name(&self) -> &'static str {
        "python"
    }

    fn generate(
        &self,
        schema: &SchemaIr,
        world: GenerationWorld,
    ) -> Result<Vec<GeneratedFile>, CodegenError> {
        Ok(vec![GeneratedFile {
            relative_path: PathBuf::from(MODEL_FILE),
            contents: generate(schema, world)?,
        }])
    }

    fn generate_service_api(
        &self,
        model: &ServiceApiModel,
        schema: &SchemaIr,
    ) -> Result<Vec<GeneratedFile>, CodegenError> {
        Ok(vec![GeneratedFile {
            relative_path: PathBuf::from(SERVICE_API_FILE),
            contents: generate_service_api(model, schema)?,
        }])
    }
}

/// Generate one standalone model, requiring an explicit generation world.
///
/// # Errors
/// Returns an error for invalid IR, unsafe/colliding Python names, cyclic
/// dependencies, or any semantics outside the documented PY-001 subset.
/// No files are written, and no partial artifacts are returned on failure.
pub fn generate(schema: &SchemaIr, world: GenerationWorld) -> Result<String, CodegenError> {
    // The shared planner validates IR and enforces the explicit world contract.
    // In particular OpenExtensions cannot silently acquire closed abstract sums.
    let emissions = plan_type_emissions(schema, world)?;
    preflight(schema, &emissions)?;
    let mut output = include_str!("runtime.py").to_owned();
    if uses_datetime(schema) {
        output.push_str(include_str!("datetime_runtime.py"));
    }
    if uses_float64(schema) {
        output.push_str(include_str!("float64_runtime.py"));
    }
    if uses_visible_ascii(schema) {
        output.push_str(include_str!("visible_ascii_runtime.py"));
    }
    if uses_nato_special_words(schema) {
        output.push_str(include_str!("nato_special_words_runtime.py"));
    }
    if schema
        .types
        .iter()
        .any(|declaration| header_profile(declaration).is_some())
    {
        output.push_str(include_str!("header_runtime.py"));
    }
    for declaration in emitted_declarations(&emissions)? {
        render(&mut output, schema, declaration)?;
    }
    Ok(output)
}

fn error(reason: impl std::fmt::Display) -> CodegenError {
    CodegenError {
        message: format!("unsupported Python IR construct: {reason}"),
    }
}

fn unconstrained(constraints: &ConstraintSet) -> Result<(), CodegenError> {
    if constraints != &ConstraintSet::default() {
        return Err(error("unsupported constraints"));
    }
    Ok(())
}

// Admission is the shared exact normalized-facet proof, never a type name.
// Only named declarations gain carriers; field-local constrained strings keep
// their existing unsupported boundary.
fn header_profile(declaration: &TypeDecl) -> Option<&'static str> {
    let TypeKind::Primitive(PrimitiveKind::String) = declaration.kind else {
        return None;
    };
    match string_profile(PrimitiveKind::String, &declaration.constraints) {
        Ok(Some(StringProfile::UciSchemaVersion)) => Some("_uci_schema_version"),
        Ok(Some(StringProfile::UniversallyUniqueIdentifier)) => Some("_uci_uuid"),
        _ => None,
    }
}

fn visible_ascii_profile(declaration: &TypeDecl) -> Option<(u64, u64)> {
    if declaration.kind != TypeKind::Primitive(PrimitiveKind::String) {
        return None;
    }
    match string_profile(PrimitiveKind::String, &declaration.constraints) {
        Ok(Some(StringProfile::VisibleAscii {
            min_length,
            max_length,
        })) => Some((min_length, max_length)),
        _ => None,
    }
}

fn nato_special_words_profile(declaration: &TypeDecl) -> bool {
    declaration.kind == TypeKind::Primitive(PrimitiveKind::String)
        && matches!(
            string_profile(PrimitiveKind::String, &declaration.constraints),
            Ok(Some(StringProfile::NatoSpecialWords))
        )
}

fn uses_nato_special_words(schema: &SchemaIr) -> bool {
    schema.types.iter().any(nato_special_words_profile)
}

// A named edge must retain the same proven, fixed lexical domain. Do not
// infer the parent's semantics from a leaf's effective facets or its name.
fn nato_special_words_ancestry(
    schema: &SchemaIr,
    declaration: &TypeDecl,
) -> Result<(), CodegenError> {
    let mut current = declaration;
    let mut seen = BTreeSet::new();
    loop {
        if current.is_abstract
            || !seen.insert(&current.name)
            || !nato_special_words_profile(current)
        {
            return Err(error("unproven NATO SpecialWords restriction ancestry"));
        }
        match current.base_type.as_ref().map(|base| &base.target) {
            Some(TypeRefTarget::Primitive(PrimitiveKind::String)) | None => return Ok(()),
            Some(TypeRefTarget::Named(name)) => {
                current = schema
                    .types
                    .iter()
                    .find(|candidate| candidate.name == *name)
                    .ok_or_else(|| error("missing NATO SpecialWords restriction base"))?;
            }
            _ => return Err(error("incompatible NATO SpecialWords restriction base")),
        }
    }
}

fn uses_visible_ascii(schema: &SchemaIr) -> bool {
    schema
        .types
        .iter()
        .any(|declaration| visible_ascii_profile(declaration).is_some())
}

// Effective constraints must prove a contained domain at every named edge.
// Never infer a parent's proof from a leaf's facets or its declaration name.
fn visible_ascii_ancestry(schema: &SchemaIr, declaration: &TypeDecl) -> Result<(), CodegenError> {
    let mut current = declaration;
    let mut seen = BTreeSet::new();
    loop {
        if current.is_abstract || !seen.insert(&current.name) {
            return Err(error("unproven visible-ASCII named restriction ancestry"));
        }
        let (min, max) = visible_ascii_profile(current)
            .ok_or_else(|| error("unproven visible-ASCII restriction profile"))?;
        match current.base_type.as_ref().map(|base| &base.target) {
            Some(TypeRefTarget::Primitive(PrimitiveKind::String)) | None => return Ok(()),
            Some(TypeRefTarget::Named(name)) => {
                let parent = schema
                    .types
                    .iter()
                    .find(|candidate| candidate.name == *name)
                    .ok_or_else(|| error("missing visible-ASCII restriction base"))?;
                let (parent_min, parent_max) = visible_ascii_profile(parent)
                    .ok_or_else(|| error("unproven visible-ASCII restriction base"))?;
                if min < parent_min || max > parent_max {
                    return Err(error("incompatible effective visible-ASCII constraints"));
                }
                current = parent;
            }
            _ => return Err(error("incompatible visible-ASCII restriction base")),
        }
    }
}

// Demand is semantic, never a declaration name. The selected schema contains
// ancestry declarations too, so inherited direct fields are included.
fn uses_float64(schema: &SchemaIr) -> bool {
    let direct = |reference: &TypeRef| {
        matches!(
            reference.target,
            TypeRefTarget::Primitive(PrimitiveKind::Float64)
        )
    };
    schema
        .messages
        .iter()
        .any(|message| direct(&message.payload_type))
        || schema
            .types
            .iter()
            .any(|declaration| match &declaration.kind {
                TypeKind::Primitive(PrimitiveKind::Float64) => true,
                TypeKind::Alias(target) => direct(target),
                TypeKind::Record { fields } => fields.iter().any(|field| direct(&field.type_ref)),
                _ => false,
            })
}

fn uses_datetime(schema: &SchemaIr) -> bool {
    schema
        .types
        .iter()
        .any(|declaration| match &declaration.kind {
            TypeKind::Primitive(PrimitiveKind::DateTime) => true,
            TypeKind::Record { .. } if !declaration.is_abstract => {
                record_fields(schema, declaration).is_ok_and(|fields| {
                    fields.iter().any(|field| {
                        field.type_ref.target == TypeRefTarget::Primitive(PrimitiveKind::DateTime)
                    })
                })
            }
            _ => false,
        })
}

fn datetime_profile(constraints: &ConstraintSet) -> Result<(), CodegenError> {
    match temporal_profile(PrimitiveKind::DateTime, constraints).map_err(error)? {
        Some(TemporalProfile::DateTimeZulu) => Ok(()),
        _ => Err(error("unproven named DateTime profile")),
    }
}

// Every effective restriction must retain the same proven Zulu domain. Never
// flatten incompatible ancestry or trust only the leaf's primitive kind.
fn datetime_ancestry(schema: &SchemaIr, declaration: &TypeDecl) -> Result<(), CodegenError> {
    let mut current = declaration;
    let mut seen = BTreeSet::new();
    loop {
        if !seen.insert(&current.name)
            || current.is_abstract
            || current.kind != TypeKind::Primitive(PrimitiveKind::DateTime)
        {
            return Err(error("unproven DateTime named restriction ancestry"));
        }
        datetime_profile(&current.constraints)?;
        match current.base_type.as_ref().map(|base| &base.target) {
            Some(TypeRefTarget::Primitive(PrimitiveKind::DateTime)) | None => return Ok(()),
            Some(TypeRefTarget::Named(name)) => {
                current = schema
                    .types
                    .iter()
                    .find(|item| item.name == *name)
                    .ok_or_else(|| error("missing DateTime restriction base"))?;
            }
            _ => return Err(error("incompatible DateTime restriction base")),
        }
    }
}

// Both renderers consume exactly the shared domain. Binary64 bits, rather than
// display text, are the source of every generated bound.
fn float64_arguments(constraints: &ConstraintSet, prefix: &str) -> Result<String, CodegenError> {
    let domain = floating_domain(PrimitiveKind::Float64, constraints).map_err(error)?;
    let Some(FloatingDomain::Float64 { lower, upper }) = domain else {
        return Ok("None, True, None, True".to_owned());
    };
    let bound = |bound: Option<ams_gra_oms_codegen_core::Float64Bound>| {
        bound.map_or_else(
            || "None, True".to_owned(),
            |bound| {
                format!(
                    "{prefix}_float64_bound(0x{:016x}), {}",
                    bound.value.value().to_bits(),
                    if bound.kind == FloatingBoundKind::Inclusive {
                        "True"
                    } else {
                        "False"
                    }
                )
            },
        )
    };
    Ok(format!("{}, {}", bound(lower), bound(upper)))
}

// Only compatible Float64 primitive chains with effective (not local) domains
// are admitted. Prove every child domain is contained in its immediate parent;
// malformed externally assembled IR cannot drop an inherited constraint.
fn float64_ancestry(schema: &SchemaIr, declaration: &TypeDecl) -> Result<(), CodegenError> {
    let mut current = declaration;
    let mut seen = BTreeSet::new();
    loop {
        if !seen.insert(&current.name)
            || current.is_abstract
            || current.kind != TypeKind::Primitive(PrimitiveKind::Float64)
        {
            return Err(error("unproven Float64 named restriction ancestry"));
        }
        let child = floating_domain(PrimitiveKind::Float64, &current.constraints).map_err(error)?;
        match current.base_type.as_ref().map(|base| &base.target) {
            Some(TypeRefTarget::Primitive(PrimitiveKind::Float64)) | None => return Ok(()),
            Some(TypeRefTarget::Named(name)) => {
                let parent = schema
                    .types
                    .iter()
                    .find(|candidate| candidate.name == *name)
                    .ok_or_else(|| error("missing Float64 restriction base"))?;
                if parent.kind != TypeKind::Primitive(PrimitiveKind::Float64) {
                    return Err(error("incompatible Float64 restriction base"));
                }
                if let Some(FloatingDomain::Float64 { lower, upper }) =
                    floating_domain(PrimitiveKind::Float64, &parent.constraints).map_err(error)?
                {
                    let Some(FloatingDomain::Float64 {
                        lower: child_lower,
                        upper: child_upper,
                    }) = child
                    else {
                        return Err(error("unresolved effective Float64 constraints"));
                    };
                    for (parent_bound, child_bound, is_lower) in
                        [(lower, child_lower, true), (upper, child_upper, false)]
                    {
                        if let Some(parent_bound) = parent_bound {
                            let compatible = child_bound.is_some_and(|bound| {
                                let p = parent_bound.value.value();
                                let c = bound.value.value();
                                (if is_lower { c > p } else { c < p })
                                    || (c == p
                                        && (parent_bound.kind == FloatingBoundKind::Inclusive
                                            || bound.kind == FloatingBoundKind::Exclusive))
                            });
                            if !compatible {
                                return Err(error("unresolved effective Float64 constraints"));
                            }
                        }
                    }
                }
                current = parent;
            }
            _ => return Err(error("incompatible Float64 restriction base")),
        }
    }
}

// Returns both the type annotation and the complete runtime check arguments.
fn primitive(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
) -> Result<(String, String), CodegenError> {
    match kind {
        PrimitiveKind::DateTime => {
            if direct_temporal_profile(kind, constraints)
                .map_err(|reason| error(format!("{reason:?}")))?
                != Some(DirectTemporalProfile::DateTime)
            {
                return Err(error("unproven direct DateTime profile"));
            }
            Ok(("str".to_owned(), "str".to_owned()))
        }
        PrimitiveKind::Float64 => {
            // Field-local facets have no established lowering. Named carriers
            // are classified separately against their effective declaration.
            unconstrained(constraints)?;
            Ok(("float".to_owned(), "float".to_owned()))
        }
        PrimitiveKind::Boolean | PrimitiveKind::String => {
            unconstrained(constraints)?;
            let name = if kind == PrimitiveKind::Boolean {
                "bool"
            } else {
                "str"
            };
            Ok((name.to_owned(), name.to_owned()))
        }
        PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger => {
            let domain = inclusive_integral_domain(kind, constraints).map_err(error)?;
            let bounds = match domain {
                Some(InclusiveIntegralDomain::Signed { min, max }) => format!("{min}, {max}"),
                Some(InclusiveIntegralDomain::Unsigned { min, max }) => format!("{min}, {max}"),
                // Python can represent the normalized unbounded signed integer
                // value space directly, without introducing an i64 truncation.
                None if kind == PrimitiveKind::SignedInteger => "None, None".to_owned(),
                None => "0, None".to_owned(),
            };
            Ok(("int".to_owned(), format!("int, {bounds}")))
        }
        _ => Err(error(format!("primitive {kind:?}"))),
    }
}

fn reference(
    reference: &TypeRef,
    constraints: &ConstraintSet,
) -> Result<(String, String), CodegenError> {
    match &reference.target {
        TypeRefTarget::Primitive(kind) => primitive(*kind, constraints),
        TypeRefTarget::Named(name) => {
            unconstrained(constraints)?;
            let symbol = names::symbol(&name.local_name, false);
            Ok((symbol.clone(), symbol))
        }
    }
}

fn preflight(schema: &SchemaIr, emissions: &[TypeEmission<'_>]) -> Result<(), CodegenError> {
    if schema.namespaces.len() != 1 {
        return Err(error("exactly one schema namespace is required"));
    }
    // The shared classifier includes fields, alternatives, aliases, lists and
    // message payloads, but never base_type edges. Inspect the original schema:
    // the planner may elide Task 026 absent-only targets, which Python still
    // does not support. No known descendant is an implicit concrete default.
    if let Some(target) = abstract_value_targets(schema).first() {
        return Err(error(format!(
            "abstract value representations are unsupported: {}",
            target.name.local_name
        )));
    }
    let declarations = emitted_declarations(emissions)?;
    names::validate(schema, &declarations)?;
    let datetime_demand = uses_datetime(schema);
    let mut scalar_names = BTreeSet::new();
    let mut available = BTreeSet::new();
    for declaration in declarations {
        let qualified = (|| -> Result<(), CodegenError> {
            // Primitive base provenance is not structural inheritance. Named
            // restriction ancestry is deferred rather than silently flattened.
            if declaration.kind == TypeKind::Primitive(PrimitiveKind::Float64) {
                float64_ancestry(schema, declaration)?;
            } else if declaration.kind == TypeKind::Primitive(PrimitiveKind::DateTime) {
                datetime_ancestry(schema, declaration)?;
            } else if visible_ascii_profile(declaration).is_some() {
                visible_ascii_ancestry(schema, declaration)?;
            } else if nato_special_words_profile(declaration) {
                nato_special_words_ancestry(schema, declaration)?;
            } else if let Some(base) = &declaration.base_type {
                match (&declaration.kind, &base.target) {
                    (TypeKind::Primitive(kind), TypeRefTarget::Primitive(base_kind))
                        if kind == base_kind => {}
                    (
                        TypeKind::Enumeration { .. },
                        TypeRefTarget::Primitive(PrimitiveKind::String),
                    ) => {}
                    (TypeKind::Record { .. }, TypeRefTarget::Named(_)) => {}
                    _ => return Err(error("inheritance / named restriction ancestry")),
                }
            }
            match &declaration.kind {
                TypeKind::Primitive(kind) => {
                    if *kind == PrimitiveKind::Float64 {
                        floating_domain(*kind, &declaration.constraints).map_err(error)?;
                    } else if *kind == PrimitiveKind::DateTime {
                        datetime_profile(&declaration.constraints)?;
                    } else if header_profile(declaration).is_none()
                        && visible_ascii_profile(declaration).is_none()
                        && !nato_special_words_profile(declaration)
                    {
                        primitive(*kind, &declaration.constraints)?;
                    }
                    if *kind != PrimitiveKind::DateTime
                        && visible_ascii_profile(declaration).is_none()
                        && !nato_special_words_profile(declaration)
                    {
                        scalar_names.insert(declaration.name.clone());
                    }
                }
                TypeKind::Alias(target) => {
                    if target.target == TypeRefTarget::Primitive(PrimitiveKind::DateTime) {
                        return Err(error("DateTime aliases are unsupported"));
                    }
                    unconstrained(&declaration.constraints)?;
                    if let TypeRefTarget::Named(name) = &target.target
                        && !scalar_names.contains(name)
                    {
                        return Err(error("aliases must target supported scalar declarations"));
                    }
                    reference(target, &ConstraintSet::default())?;
                    scalar_names.insert(declaration.name.clone());
                }
                TypeKind::Enumeration { .. } => {
                    unconstrained(&declaration.constraints)?;
                    scalar_names.insert(declaration.name.clone());
                }
                TypeKind::Record { .. } => {
                    unconstrained(&declaration.constraints)?;
                    for field in record_fields(schema, declaration)? {
                        if field.nillable {
                            return Err(error(format!("nillable field {}", field.name)));
                        }
                        if matches!(field.cardinality.shape(), OccurrenceShape::Unbounded { .. }) {
                            return Err(error(format!("unbounded field {}", field.name)));
                        }
                        reference(&field.type_ref, &field.constraints).map_err(|failure| {
                            if datetime_demand {
                                error(format!("field {}: {}", field.name, failure.message))
                            } else {
                                failure
                            }
                        })?;
                        if let TypeRefTarget::Named(name) = &field.type_ref.target
                            && !available.contains(name)
                        {
                            return Err(error(format!(
                                "field {} requires a preceding concrete emitted type {}",
                                field.name, name.local_name
                            )));
                        }
                    }
                }
                TypeKind::Choice { .. } | TypeKind::List { .. } => {
                    return Err(error("Choice / List declarations"));
                }
            }
            available.insert(declaration.name.clone());
            Ok(())
        })();
        qualified.map_err(|failure| {
            if datetime_demand {
                CodegenError {
                    message: format!(
                        "{{{}}}{}: {}",
                        declaration.name.namespace_uri,
                        declaration.name.local_name,
                        failure.message
                    ),
                }
            } else {
                failure
            }
        })?;
    }
    // Messages are not rendered, but their payload cannot escape subset checks.
    for message in &schema.messages {
        reference(&message.payload_type, &ConstraintSet::default())?;
    }
    Ok(())
}

// This Python-specific emission surface is shared by naming, models and codecs.
// The shared planner's no-value-target shortcut retains ancestry declarations.
fn emitted_declarations<'a>(
    emissions: &[TypeEmission<'a>],
) -> Result<Vec<&'a TypeDecl>, CodegenError> {
    let mut declarations = Vec::new();
    for emission in emissions {
        match emission {
            TypeEmission::Declaration(declaration)
                if declaration.is_abstract
                    && matches!(declaration.kind, TypeKind::Record { .. }) => {}
            TypeEmission::Declaration(declaration) if !declaration.is_abstract => {
                declarations.push(*declaration);
            }
            _ => return Err(error("abstract value representations are unsupported")),
        }
    }
    Ok(declarations)
}

fn record_fields<'a>(
    schema: &'a SchemaIr,
    declaration: &TypeDecl,
) -> Result<Vec<&'a ams_gra_oms_ir::FieldDecl>, CodegenError> {
    effective_record_fields(schema, &declaration.name).map_err(error)
}

fn render(
    output: &mut String,
    schema: &SchemaIr,
    declaration: &TypeDecl,
) -> Result<(), CodegenError> {
    let name = names::symbol(&declaration.name.local_name, false);
    let identity = format!(
        "({}, {})",
        names::literal(&declaration.name.namespace_uri),
        names::literal(&declaration.name.local_name)
    );
    if let TypeKind::Enumeration { variants } = &declaration.kind {
        writeln!(output, "\n\n@_unique\nclass {name}(_Enum):").unwrap();
        writeln!(output, "    __schema_name__ = {identity}").unwrap();
        for variant in variants {
            writeln!(
                output,
                "    {} = {}",
                names::symbol(&variant.wire_value, false),
                names::literal(&variant.wire_value)
            )
            .unwrap();
        }
        return Ok(());
    }
    writeln!(
        output,
        "\n\n@_dataclass(frozen=True, slots=True, kw_only={})\nclass {name}:",
        if matches!(declaration.kind, TypeKind::Record { .. }) {
            "True"
        } else {
            "False"
        }
    )
    .unwrap();
    writeln!(
        output,
        "    __schema_name__: _ClassVar[tuple[str, str]] = {identity}"
    )
    .unwrap();
    match &declaration.kind {
        TypeKind::Primitive(_) | TypeKind::Alias(_) => {
            if declaration.kind == TypeKind::Primitive(PrimitiveKind::DateTime) {
                writeln!(output, "    value: str\n\n    def __post_init__(self) -> None:\n        object.__setattr__(self, 'value', _check_datetime(self.value, True))").unwrap();
                return Ok(());
            }
            if declaration.kind == TypeKind::Primitive(PrimitiveKind::Float64) {
                let args = float64_arguments(&declaration.constraints, "")?;
                writeln!(output, "    value: float\n\n    def __post_init__(self) -> None:\n        _check_float64(self.value, {args})").unwrap();
                return Ok(());
            }
            if let Some(validator) = header_profile(declaration) {
                writeln!(output, "    value: str\n\n    def __post_init__(self) -> None:\n        _check(self.value, str)\n        {validator}(self.value)").unwrap();
                return Ok(());
            }
            if let Some((min, max)) = visible_ascii_profile(declaration) {
                writeln!(output, "    value: str\n\n    def __post_init__(self) -> None:\n        _check_visible_ascii(self.value, {min}, {max})").unwrap();
                return Ok(());
            }
            if nato_special_words_profile(declaration) {
                writeln!(output, "    value: str\n\n    def __post_init__(self) -> None:\n        _check_nato_special_words(self.value)").unwrap();
                return Ok(());
            }
            let (annotation, check) = match &declaration.kind {
                TypeKind::Primitive(kind) => primitive(*kind, &declaration.constraints)?,
                TypeKind::Alias(target) => reference(target, &ConstraintSet::default())?,
                _ => unreachable!(),
            };
            writeln!(output, "    value: {annotation}\n\n    def __post_init__(self) -> None:\n        _check(self.value, {check})").unwrap();
        }
        TypeKind::Record { .. } => {
            let fields = record_fields(schema, declaration)?;
            for field in &fields {
                let symbol = names::symbol(&field.name, true);
                let (base, _) = reference(&field.type_ref, &field.constraints)?;
                let (annotation, default) = match field.cardinality.shape() {
                    OccurrenceShape::RequiredOne => (base, String::new()),
                    OccurrenceShape::OptionalOne => {
                        (format!("{base} | None"), "default=None, ".to_owned())
                    }
                    OccurrenceShape::Bounded { min, .. } => (
                        format!("tuple[{base}, ...]"),
                        if min == 0 {
                            "default=(), ".to_owned()
                        } else {
                            String::new()
                        },
                    ),
                    OccurrenceShape::Unbounded { .. } => unreachable!("preflight"),
                };
                let namespace = field
                    .wire_namespace_uri
                    .as_deref()
                    .map_or_else(|| "None".to_owned(), names::literal);
                writeln!(output, "    {symbol}: {annotation} = _field({default}metadata={{'schema_name': {}, 'schema_namespace': {namespace}}})", names::literal(&field.name)).unwrap();
            }
            writeln!(output, "\n    def __post_init__(self) -> None:").unwrap();
            if fields.is_empty() {
                writeln!(output, "        pass").unwrap();
            }
            for field in fields {
                let symbol = names::symbol(&field.name, true);
                let (_, check) = reference(&field.type_ref, &field.constraints)?;
                match field.cardinality.shape() {
                    OccurrenceShape::RequiredOne => writeln!(output, "        _check(self.{symbol}, {check})").unwrap(),
                    OccurrenceShape::OptionalOne => writeln!(output, "        if self.{symbol} is not None:\n            _check(self.{symbol}, {check})").unwrap(),
                    OccurrenceShape::Bounded { min, max } => {
                        writeln!(output, "        _check(self.{symbol}, tuple)\n        if not {min} <= len(self.{symbol}) <= {max}:\n            raise ValueError({})\n        for item in self.{symbol}:\n            _check(item, {check})", names::literal(&format!("{symbol}: expected {min}..{max} items"))).unwrap();
                    }
                    OccurrenceShape::Unbounded { .. } => unreachable!("preflight"),
                }
                if field.type_ref.target == TypeRefTarget::Primitive(PrimitiveKind::DateTime) {
                    let value = match field.cardinality.shape() {
                        OccurrenceShape::RequiredOne => format!("_check_datetime(self.{symbol})"),
                        OccurrenceShape::OptionalOne => format!(
                            "_check_datetime(self.{symbol}) if self.{symbol} is not None else None"
                        ),
                        OccurrenceShape::Bounded { .. } => {
                            format!("tuple(_check_datetime(item) for item in self.{symbol})")
                        }
                        OccurrenceShape::Unbounded { .. } => unreachable!("preflight"),
                    };
                    writeln!(
                        output,
                        "        object.__setattr__(self, {}, {value})",
                        names::literal(&symbol)
                    )
                    .unwrap();
                }
            }
        }
        _ => unreachable!("preflight"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::{Float32Value, Float64Value, NumericValue};

    #[test]
    fn float64_bounds_are_shared_classifications_and_exact_bits() {
        for bits in [
            0,
            1,
            0x000f_ffff_ffff_ffff,
            0x0010_0000_0000_0000,
            0x3ff0_0000_0000_0001,
            0x7fef_ffff_ffff_ffff,
            0x8000_0000_0000_0000,
        ] {
            for inclusive in [true, false] {
                let mut constraints = ConstraintSet::default();
                let bound = Some(NumericValue::Float64(Float64Value::from_bits(bits)));
                if inclusive {
                    constraints.min_inclusive = bound;
                } else {
                    constraints.min_exclusive = bound;
                }
                let rendered = float64_arguments(&constraints, "").unwrap();
                assert!(
                    rendered.starts_with(&format!("_float64_bound(0x{bits:016x})")),
                    "{rendered}"
                );
                assert!(rendered.ends_with(if inclusive {
                    ", True, None, True"
                } else {
                    ", False, None, True"
                }));
            }
        }
        assert!(
            float64_arguments(
                &ConstraintSet {
                    min_inclusive: Some(NumericValue::Float32(Float32Value::from_value(0.0))),
                    ..ConstraintSet::default()
                },
                ""
            )
            .is_err()
        );
        for constraints in [
            ConstraintSet {
                min_inclusive: Some(NumericValue::Float64(Float64Value::from_value(0.0))),
                min_exclusive: Some(NumericValue::Float64(Float64Value::from_value(1.0))),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                min_length: Some(1),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                max_inclusive: Some(NumericValue::Float64(Float64Value::from_value(
                    f64::INFINITY,
                ))),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                min_inclusive: Some(NumericValue::Float64(Float64Value::from_value(2.0))),
                max_inclusive: Some(NumericValue::Float64(Float64Value::from_value(1.0))),
                ..ConstraintSet::default()
            },
        ] {
            assert!(float64_arguments(&constraints, "").is_err());
        }
        assert!(
            primitive(
                PrimitiveKind::Float64,
                &ConstraintSet {
                    min_inclusive: Some(NumericValue::Float64(Float64Value::from_value(0.0))),
                    ..ConstraintSet::default()
                }
            )
            .is_err(),
            "direct field facets must not gain named-carrier semantics"
        );
    }

    #[test]
    fn backend_identity_and_empty_schema_fail_closed() {
        assert_eq!(PythonBackend.name(), "python");
        assert!(
            PythonBackend
                .generate(&SchemaIr::empty(), GenerationWorld::OpenExtensions)
                .is_err()
        );
    }

    #[test]
    fn unsupported_primitives_and_facets_fail_closed() {
        for kind in [
            PrimitiveKind::Decimal,
            PrimitiveKind::Float32,
            PrimitiveKind::Binary,
            PrimitiveKind::Time,
            PrimitiveKind::Duration,
        ] {
            assert!(primitive(kind, &ConstraintSet::default()).is_err());
        }
        assert!(
            primitive(
                PrimitiveKind::String,
                &ConstraintSet {
                    max_length: Some(8),
                    ..ConstraintSet::default()
                }
            )
            .is_err()
        );
    }
}
