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
    Backend, CodegenError, GeneratedFile, GenerationWorld, InclusiveIntegralDomain,
    ServiceApiModel, StringProfile, TypeEmission, inclusive_integral_domain, plan_type_emissions,
    string_profile,
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
    if schema
        .types
        .iter()
        .any(|declaration| header_profile(declaration).is_some())
    {
        output.push_str(include_str!("header_runtime.py"));
    }
    for emission in emissions {
        let TypeEmission::Declaration(declaration) = emission else {
            return Err(error("abstract value representations are unsupported"));
        };
        render(&mut output, declaration)?;
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

// Returns both the type annotation and the complete runtime check arguments.
fn primitive(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
) -> Result<(String, String), CodegenError> {
    match kind {
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
    // Inspect the original declarations as well: the shared planner can elide
    // uninhabited abstract targets under ClosedSchemaSet. Python does not yet
    // implement that absent-only representation either.
    if schema
        .types
        .iter()
        .any(|declaration| declaration.is_abstract)
    {
        return Err(error("abstract declarations are unsupported"));
    }
    names::validate(schema)?;
    let mut scalar_names = BTreeSet::new();
    for emission in emissions {
        let TypeEmission::Declaration(declaration) = emission else {
            return Err(error("abstract value representations are unsupported"));
        };
        if declaration.is_abstract {
            return Err(error("abstract declarations are unsupported"));
        }
        // Primitive base provenance is not structural inheritance. Named
        // restriction ancestry is deferred rather than silently flattened.
        if let Some(base) = &declaration.base_type {
            match (&declaration.kind, &base.target) {
                (TypeKind::Primitive(kind), TypeRefTarget::Primitive(base_kind))
                    if kind == base_kind => {}
                (TypeKind::Enumeration { .. }, TypeRefTarget::Primitive(PrimitiveKind::String)) => {
                }
                _ => return Err(error("inheritance / named restriction ancestry")),
            }
        }
        match &declaration.kind {
            TypeKind::Primitive(kind) => {
                if header_profile(declaration).is_none() {
                    primitive(*kind, &declaration.constraints)?;
                }
                scalar_names.insert(declaration.name.clone());
            }
            TypeKind::Alias(target) => {
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
            TypeKind::Record { fields } => {
                unconstrained(&declaration.constraints)?;
                for field in fields {
                    if field.nillable {
                        return Err(error(format!("nillable field {}", field.name)));
                    }
                    if matches!(field.cardinality.shape(), OccurrenceShape::Unbounded { .. }) {
                        return Err(error(format!("unbounded field {}", field.name)));
                    }
                    reference(&field.type_ref, &field.constraints)?;
                }
            }
            TypeKind::Choice { .. } | TypeKind::List { .. } => {
                return Err(error("Choice / List declarations"));
            }
        }
    }
    // Messages are not rendered, but their payload cannot escape subset checks.
    for message in &schema.messages {
        reference(&message.payload_type, &ConstraintSet::default())?;
    }
    Ok(())
}

fn render(output: &mut String, declaration: &TypeDecl) -> Result<(), CodegenError> {
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
            if let Some(validator) = header_profile(declaration) {
                writeln!(output, "    value: str\n\n    def __post_init__(self) -> None:\n        _check(self.value, str)\n        {validator}(self.value)").unwrap();
                return Ok(());
            }
            let (annotation, check) = match &declaration.kind {
                TypeKind::Primitive(kind) => primitive(*kind, &declaration.constraints)?,
                TypeKind::Alias(target) => reference(target, &ConstraintSet::default())?,
                _ => unreachable!(),
            };
            writeln!(output, "    value: {annotation}\n\n    def __post_init__(self) -> None:\n        _check(self.value, {check})").unwrap();
        }
        TypeKind::Record { fields } => {
            for field in fields {
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
            }
        }
        _ => unreachable!("preflight"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
            PrimitiveKind::Float64,
            PrimitiveKind::Binary,
            PrimitiveKind::DateTime,
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
