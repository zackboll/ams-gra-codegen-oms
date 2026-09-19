//! Minimal Rust type generation from normalized schema IR.

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
    if schema.types.iter().any(has_unbounded_occurrence) {
        output.push_str(concat!(
            "#[derive(Debug, Clone, PartialEq, Eq)]\n",
            "pub struct UnboundedVec<T, const MIN: u64>(Vec<T>);\n\n",
            "impl<T, const MIN: u64> UnboundedVec<T, MIN> {\n",
            "    pub fn new(values: Vec<T>) -> Option<Self> {\n",
            "        usize::try_from(MIN).is_ok_and(|min| values.len() >= min).then_some(Self(values))\n",
            "    }\n\n",
            "    pub fn as_slice(&self) -> &[T] { &self.0 }\n",
            "}\n\n",
        ));
    }
    if schema.types.iter().any(has_direct_integral_range) {
        output.push_str(concat!(
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]\n",
            "pub struct BoundedI64<const MIN: i64, const MAX: i64>(i64);\n",
            "impl<const MIN: i64, const MAX: i64> BoundedI64<MIN, MAX> {\n",
            "    pub const fn new(value: i64) -> Option<Self> { if value >= MIN && value <= MAX { Some(Self(value)) } else { None } }\n",
            "    pub const fn get(self) -> i64 { self.0 }\n}\n\n",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]\n",
            "pub struct BoundedU64<const MIN: u64, const MAX: u64>(u64);\n",
            "impl<const MIN: u64, const MAX: u64> BoundedU64<MIN, MAX> {\n",
            "    pub const fn new(value: u64) -> Option<Self> { if value >= MIN && value <= MAX { Some(Self(value)) } else { None } }\n",
            "    pub const fn get(self) -> u64 { self.0 }\n}\n\n",
        ));
    }
    for declaration in declarations {
        render_declaration(&mut output, schema, declaration)?;
    }
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
        TypeKind::Primitive(PrimitiveKind::UnsignedInteger) => {
            let Some(InclusiveIntegralDomain::Unsigned { min, max }) = integral_domain(
                PrimitiveKind::UnsignedInteger,
                &declaration.constraints,
                &name,
            )?
            else {
                return unsupported(format!("integer bounds on {name}"));
            };
            writeln!(output, "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub struct {name}(u64);\n\nimpl {name} {{\n    pub const MIN: u64 = {min};\n    pub const MAX: u64 = {max};\n\n    pub const fn new(value: u64) -> Option<Self> {{\n        if value >= Self::MIN && value <= Self::MAX {{ Some(Self(value)) }} else {{ None }}\n    }}\n\n    pub const fn get(self) -> u64 {{ self.0 }}\n}}\n").expect("writing to String cannot fail");
        }
        TypeKind::Primitive(PrimitiveKind::Boolean) => {
            reject_any_constraints(&declaration.constraints, &name)?;
            writeln!(output, "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub struct {name}(bool);\n\nimpl {name} {{\n    pub const fn new(value: bool) -> Self {{ Self(value) }}\n    pub const fn get(self) -> bool {{ self.0 }}\n}}\n").expect("writing to String cannot fail");
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
        TypeKind::Record { .. } => {
            if declaration.is_abstract {
                return Ok(());
            }
            writeln!(
                output,
                "#[derive(Debug, Clone, PartialEq, Eq)]\npub struct {name} {{"
            )
            .expect("writing to String cannot fail");
            for field in effective_record_fields(schema, &declaration.name).map_err(|_| {
                error(format!(
                    "unsupported Rust IR construct: non-Record inheritance {}",
                    declaration.name.local_name
                ))
            })? {
                let field_name = snake_case(&field.name)?;
                let base = rust_field_base(field)?;
                let field_type = match field.cardinality.shape() {
                    OccurrenceShape::RequiredOne => base,
                    OccurrenceShape::OptionalOne => format!("Option<{base}>"),
                    OccurrenceShape::Bounded { min, max } if max > 1 => {
                        format!("BoundedVec<{base}, {min}, {max}>")
                    }
                    OccurrenceShape::Unbounded { min } => format!("UnboundedVec<{base}, {min}>"),
                    _ => return unsupported(format!("cardinality on field {field_name}")),
                };
                writeln!(output, "    pub {field_name}: {field_type},")
                    .expect("writing to String cannot fail");
            }
            output.push_str("}\n");
        }
        TypeKind::Choice { .. } => {
            writeln!(
                output,
                "#[derive(Debug, Clone, PartialEq, Eq)]\npub enum {name} {{"
            )
            .expect("writing to String cannot fail");
            for alternative in effective_choice_alternatives(schema, &declaration.name).map_err(
                |projection_error| {
                    error(format!("unsupported Rust IR construct: {projection_error}"))
                },
            )? {
                let alternative_name = upper_camel(&alternative.name)?;
                writeln!(
                    output,
                    "    {alternative_name}({}),",
                    rust_field_type(alternative)?
                )
                .expect("writing to String cannot fail");
            }
            output.push_str("}\n\n");
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
                    "unsupported Rust IR construct: non-Record inheritance {}",
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
                    error(format!("unsupported Rust IR construct: {projection_error}"))
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
        rust_field_type(alternative)?;
    }
    Ok(())
}

fn rust_field_type(field: &ams_gra_oms_ir::FieldDecl) -> Result<String, CodegenError> {
    let base = rust_field_base(field)?;
    match field.cardinality.shape() {
        OccurrenceShape::RequiredOne => Ok(base),
        OccurrenceShape::OptionalOne => Ok(format!("Option<{base}>")),
        OccurrenceShape::Bounded { min, max } if max > 1 => {
            Ok(format!("BoundedVec<{base}, {min}, {max}>"))
        }
        OccurrenceShape::Unbounded { min } => Ok(format!("UnboundedVec<{base}, {min}>")),
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

fn has_numeric_constraints(constraints: &ConstraintSet) -> bool {
    constraints.min_inclusive.is_some()
        || constraints.max_inclusive.is_some()
        || constraints.min_exclusive.is_some()
        || constraints.max_exclusive.is_some()
}

fn rust_type(type_ref: &TypeRef) -> Result<String, CodegenError> {
    match &type_ref.target {
        TypeRefTarget::Primitive(PrimitiveKind::SignedInteger) => Ok("i64".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::UnsignedInteger) => Ok("u64".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::Boolean) => Ok("bool".to_owned()),
        TypeRefTarget::Primitive(PrimitiveKind::String) => Ok("String".to_owned()),
        TypeRefTarget::Named(name) => upper_camel(&name.local_name),
        other => unsupported(format!("type reference {other:?}")),
    }
}

fn integral_domain(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
    name: &str,
) -> Result<Option<InclusiveIntegralDomain>, CodegenError> {
    inclusive_integral_domain(kind, constraints)
        .map_err(|reason| error(format!("unsupported Rust IR construct: {reason} on {name}")))
}

fn rust_field_base(field: &ams_gra_oms_ir::FieldDecl) -> Result<String, CodegenError> {
    match field.type_ref.target {
        TypeRefTarget::Primitive(
            kind @ (PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger),
        ) => match integral_domain(kind, &field.constraints, &field.name)? {
            Some(InclusiveIntegralDomain::Signed { min, max }) => {
                Ok(format!("BoundedI64<{min}, {max}>"))
            }
            Some(InclusiveIntegralDomain::Unsigned { min, max }) => {
                Ok(format!("BoundedU64<{min}, {max}>"))
            }
            None => rust_type(&field.type_ref),
        },
        _ => {
            reject_any_constraints(&field.constraints, &field.name)?;
            rust_type(&field.type_ref)
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

fn reject_any_constraints(constraints: &ConstraintSet, name: &str) -> Result<(), CodegenError> {
    if constraints != &ConstraintSet::default() {
        return unsupported(format!("field constraints on {name}"));
    }
    Ok(())
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
    fn lowers_unbounded_records_choices_and_constrained_elements() {
        let source = generate(&unbounded_schema()).expect("unbounded cardinality should generate");
        assert!(source.contains("pub struct UnboundedVec<T, const MIN: u64>(Vec<T>);"));
        assert!(source.contains("usize::try_from(MIN).is_ok_and(|min| values.len() >= min)"));
        assert!(!source.contains("u64::try_from(values.len())"));
        assert!(source.contains("UnboundedVec<Item, 0>"));
        assert!(source.contains("UnboundedVec<Item, 1>"));
        assert!(source.contains("UnboundedVec<Item, 2>"));
        assert!(source.contains("UnboundedVec<BoundedU64<0, 255>, 0>"));
        assert!(source.contains("ManyByte(UnboundedVec<BoundedU64<0, 255>, 1>)"));
    }

    #[test]
    fn lowers_integral_scalars_and_preserves_direct_ranges() {
        let source = generate(&integral_schema()).expect("integral scalars should generate");
        assert!(source.contains("pub struct BoundedI64<const MIN: i64, const MAX: i64>"));
        assert!(source.contains("pub enabled: bool"));
        assert!(source.contains("pub byte_value: BoundedI64<-128, 127>"));
        assert!(source.contains("pub unsigned_byte_value: BoundedU64<0, 255>"));
        assert!(source.contains("BoundedVec<BoundedI64<-128, 127>, 0, 8>"));
        assert!(source.contains("TrueCase(bool)"));
        assert!(source.contains("SignedCase(BoundedI64<-32768, 32767>)"));
        assert!(source.contains("UnsignedCase(BoundedU64<0, 255>)"));
        assert!(source.contains("pub struct UnsignedBounded(u64);"));
        assert!(source.contains("pub struct NamedBoolean(bool);"));
    }

    #[test]
    fn lowers_choice_as_named_enum_and_accepts_empty_record_ancestry() {
        let source = generate(&choice_schema()).expect("supported Choice should generate");
        assert!(source.contains("pub enum Selection {\n    First(Token),\n    Second(Token),"));
        assert!(source.contains("pub struct Holder {\n    pub selected: Selection,"));
        assert!(
            source.contains("pub enum DerivedSelection {\n    Left(Token),\n    Right(Token),")
        );
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
        assert!(source.contains("Items(BoundedVec<Token, 0, 3>)"));
    }

    #[test]
    fn lowers_effective_record_fields_and_omits_abstract_ancestor() {
        let source = generate(&inheritance_schema()).expect("pure Record inheritance is supported");
        assert!(!source.contains("pub struct Base"));
        let leaf = source.find("pub struct Leaf").unwrap();
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
        let source = generate(&schema).expect("Choice must render");
        assert!(source.contains("pub enum TrackId"));
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

        let mut schema = track_schema();
        schema.types[0].kind = TypeKind::Primitive(PrimitiveKind::Float64);
        schema.types[0].constraints = ConstraintSet {
            min_inclusive: Some(NumericValue::Float64(
                ams_gra_oms_ir::Float64Value::from_value(0.0),
            )),
            ..ConstraintSet::default()
        };
        let error = generate(&schema).expect_err("constrained float must remain unsupported");
        assert!(error.message.contains("unsupported Rust IR construct"));
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
                "unsupported Rust IR construct: type reference Primitive({kind:?})"
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
                    .contains("unsupported Rust IR construct: constraints on")
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
                    .contains("unsupported Rust IR construct: constraints on")
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
