//! Schema-specific OMS payload mapping, never dataclass introspection. Model
//! capability, type spelling and primitive domains remain the PY-001 rules.
use crate::{error, names};
use ams_gra_oms_codegen_core::{
    CodegenError, GeneratedFile, OAM_NAMESPACE, ServiceApiModel, oms_json_member_name,
    plan_type_emissions,
};
use ams_gra_oms_ir::{
    BinaryLexicalEncoding, ConstraintSet, FieldDecl, OccurrenceShape, PrimitiveKind, SchemaIr,
    TypeKind, TypeRef, TypeRefTarget, declaration_binary_encoding, resolve_binary_encoding,
};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::PathBuf;

pub const SERVICE_CODEC_FILE: &str = "service_codec.py";

fn unbounded_runtime(template: &str) -> Result<String, CodegenError> {
    let bounded_check = "if not lower <= len(value) <= upper:";
    if template.matches(bounded_check).count() != 1 {
        return Err(error(
            "Python OMS JSON codec: expected exactly one bounded sequence runtime check",
        ));
    }
    Ok(template.replace(
        bounded_check,
        "if len(value) < lower or (upper is not None and len(value) > upper):",
    ))
}

fn wire_key(field: &FieldDecl) -> Result<String, CodegenError> {
    let wire = field.wire_name();
    let namespace = wire.namespace_uri.filter(|namespace| !namespace.is_empty())
        .ok_or_else(|| error(format!("Python OMS JSON codec: unqualified element member {} has no established wire mapping", wire.local_name)))?;
    Ok(oms_json_member_name(namespace, wire.local_name))
}

fn reference(
    reference: &TypeRef,
    constraints: &ConstraintSet,
    value: &str,
    path: &str,
    encode: bool,
) -> Result<String, CodegenError> {
    match &reference.target {
        TypeRefTarget::Named(name) => Ok(format!(
            "_{}_{symbol}({value}, {path})",
            if encode { "encode" } else { "decode" },
            symbol = names::symbol(&name.local_name, false)
        )),
        TypeRefTarget::Primitive(kind) => {
            if *kind == PrimitiveKind::Duration {
                crate::primitive(*kind, constraints)?;
                return Ok(format!("_checked_duration({value}, {path})"));
            }
            if *kind == PrimitiveKind::Binary {
                crate::primitive(*kind, constraints)?;
                return Ok(format!(
                    "_binary_{}({value}, {path})",
                    if encode { "encode" } else { "decode" }
                ));
            }
            if *kind == PrimitiveKind::DateTime {
                crate::primitive(*kind, constraints)?;
                return Ok(format!("_checked_datetime({value}, {path})"));
            }
            if *kind == PrimitiveKind::Float64 {
                let args = crate::float64_arguments(constraints, "_model.")?;
                return Ok(format!(
                    "_{}_float64({value}, {path}, {args})",
                    if encode { "encode" } else { "decode" }
                ));
            }
            let (_, check) = crate::primitive(*kind, constraints)?;
            Ok(format!("_scalar({value}, {path}, {check})"))
        }
    }
}

/// Qualify and render the actual codec against the same authoritative service
/// model and selected schema as Python's model and wrapper. No contract parsing.
///
/// # Errors
/// Returns the real model/subset/naming error, or an unsupported wire provenance
/// blocker. Zero OMS exchanges need no codec and return no artifact.
pub fn generate_service_codec(
    model: &ServiceApiModel,
    schema: &SchemaIr,
) -> Result<Vec<GeneratedFile>, CodegenError> {
    if !model.has_oms_exchanges() {
        return Ok(Vec::new());
    }
    let emissions = plan_type_emissions(schema, model.world())?;
    crate::preflight(schema, &emissions)?;
    let declarations = crate::emitted_declarations(&emissions)?;
    // Render-time wire preflight is the readiness decision too.
    for declaration in &declarations {
        if declaration.kind == TypeKind::Primitive(PrimitiveKind::Binary)
            && declaration_binary_encoding(schema, declaration)
                != Some(BinaryLexicalEncoding::HexBinary)
        {
            return Err(error(format!(
                "Python OMS JSON codec: {}: unsupported or unknown Binary lexical provenance",
                declaration.name.local_name
            )));
        }
        // §6.1.4's special mapping requires both exact OAM wire identity and
        // the checked Task 038 value space. Name alone remains insufficient.
        if declaration.name.namespace_uri == OAM_NAMESPACE
            && declaration.name.local_name == "UniversallyUniqueIdentifierType"
            && crate::header_profile(declaration) != Some("_uci_uuid")
        {
            return Err(error(
                "Python OMS JSON codec: OAM UniversallyUniqueIdentifierType RFC 4122 mapping is unsupported",
            ));
        }
        if let TypeKind::Record { .. } = &declaration.kind {
            let mut keys = BTreeSet::new();
            for field in crate::record_fields(schema, declaration)? {
                let binary = match &field.type_ref.target {
                    TypeRefTarget::Primitive(PrimitiveKind::Binary) => true,
                    TypeRefTarget::Named(name) => schema.types.iter().any(|candidate| {
                        candidate.name == *name
                            && candidate.kind == TypeKind::Primitive(PrimitiveKind::Binary)
                    }),
                    _ => false,
                };
                if binary
                    && resolve_binary_encoding(schema, &field.type_ref)
                        != Some(BinaryLexicalEncoding::HexBinary)
                {
                    return Err(error(format!(
                        "Python OMS JSON codec: {}.{}: unsupported or unknown Binary lexical provenance",
                        declaration.name.local_name, field.name
                    )));
                }
                let key = wire_key(field)?;
                if key == "$type" || !keys.insert(key) {
                    return Err(error(
                        "Python OMS JSON codec: duplicate or reserved wire member",
                    ));
                }
            }
        }
    }
    let mut output = include_str!("codec_runtime.py").to_owned();
    // Keep the established runtime bytes for schemas without unbounded members.
    // The same sequence contract accepts None only for an absent upper bound.
    let unbounded = schema.types.iter().any(|declaration| {
        matches!(&declaration.kind, TypeKind::Record { fields }
            if fields.iter().any(|field| matches!(field.cardinality.shape(), OccurrenceShape::Unbounded { .. })))
    });
    if unbounded {
        output = unbounded_runtime(&output)?;
    }
    if crate::uses_datetime(schema) {
        output.push_str(include_str!("datetime_codec_runtime.py"));
    }
    if crate::uses_duration(schema) {
        output.push_str(include_str!("duration_codec_runtime.py"));
    }
    if crate::uses_float64(schema) {
        output.push_str(include_str!("float64_codec_runtime.py"));
    }
    if crate::uses_binary(schema) {
        output.push_str(include_str!("binary_codec_runtime.py"));
    }
    if schema.types.iter().any(|declaration| {
        crate::header_profile(declaration).is_some()
            || crate::visible_ascii_profile(declaration).is_some()
            || crate::nato_special_words_profile(declaration)
    }) {
        output.push_str(include_str!("header_codec_runtime.py"));
    }
    for declaration in declarations {
        let symbol = names::symbol(&declaration.name.local_name, false);
        if declaration.kind == TypeKind::Primitive(PrimitiveKind::Duration) {
            writeln!(output, "\n\ndef _encode_{symbol}(value, path):\n    _expect(value, _model.{symbol}, path)\n    try:\n        text = value.value\n    except AttributeError as error:\n        raise CodecError(f'{{path}}: missing Duration value') from error\n    return _checked_duration(text, path)\n\n\ndef _decode_{symbol}(value, path):\n    return _model.{symbol}(_checked_duration(value, path))").unwrap();
            continue;
        }
        if declaration.kind == TypeKind::Primitive(PrimitiveKind::Binary) {
            let args = crate::binary_arguments(&declaration.constraints)?;
            writeln!(output, "\n\ndef _encode_{symbol}(value, path):\n    _expect(value, _model.{symbol}, path)\n    try:\n        octets = value.value\n    except AttributeError as error:\n        raise CodecError(f'{{path}}: missing Binary value') from error\n    return _binary_encode(octets, path, {args})\n\n\ndef _decode_{symbol}(value, path):\n    return _model.{symbol}(_binary_decode(value, path, {args}))").unwrap();
            continue;
        }
        if declaration.kind == TypeKind::Primitive(PrimitiveKind::DateTime) {
            let access = if unbounded {
                "    try:\n        text = value.value\n    except AttributeError as error:\n        raise CodecError(f'{path}: missing DateTime value') from error\n    return _checked_datetime(text, path, True)"
            } else {
                "    return _checked_datetime(value.value, path, True)"
            };
            writeln!(output, "\n\ndef _encode_{symbol}(value, path):\n    _expect(value, _model.{symbol}, path)\n{access}\n\n\ndef _decode_{symbol}(value, path):\n    return _model.{symbol}(_checked_datetime(value, path, True))").unwrap();
            continue;
        }
        if crate::header_profile(declaration).is_some()
            || crate::visible_ascii_profile(declaration).is_some()
            || crate::nato_special_words_profile(declaration)
        {
            writeln!(output, "\n\ndef _encode_{symbol}(value, path):\n    return _header_text(value, path, _model.{symbol})\n\n\ndef _decode_{symbol}(value, path):\n    return _header_value(value, path, _model.{symbol})").unwrap();
            continue;
        }
        writeln!(
            output,
            "\n\ndef _encode_{symbol}(value, path):\n    _expect(value, _model.{symbol}, path)"
        )
        .unwrap();
        match &declaration.kind {
            TypeKind::Primitive(kind) => {
                let expr = reference(
                    &TypeRef::primitive(*kind),
                    &declaration.constraints,
                    "value.value",
                    "path",
                    true,
                )?;
                writeln!(output, "    return {expr}").unwrap();
            }
            TypeKind::Alias(target) => {
                let expr = reference(
                    target,
                    &ConstraintSet::default(),
                    "value.value",
                    "path",
                    true,
                )?;
                writeln!(output, "    return {expr}").unwrap();
            }
            TypeKind::Enumeration { variants } => {
                for variant in variants {
                    writeln!(
                        output,
                        "    if value is _model.{symbol}.{}:\n        return {}",
                        names::symbol(&variant.wire_value, false),
                        names::literal(&variant.wire_value)
                    )
                    .unwrap();
                }
                writeln!(
                    output,
                    "    raise CodecError(f'{{path}}: invalid enumeration value')"
                )
                .unwrap();
            }
            TypeKind::Record { .. } => {
                writeln!(output, "    result = {{}}").unwrap();
                for field in crate::record_fields(schema, declaration)? {
                    let key = names::literal(&wire_key(field)?);
                    let access = format!("value.{}", names::symbol(&field.name, true));
                    let path = format!("path + '.' + {key}");
                    let expr =
                        reference(&field.type_ref, &field.constraints, &access, &path, true)?;
                    match field.cardinality.shape() {
                        OccurrenceShape::RequiredOne => {
                            writeln!(output, "    result[{key}] = {expr}").unwrap()
                        }
                        OccurrenceShape::OptionalOne => writeln!(
                            output,
                            "    if {access} is not None:\n        result[{key}] = {expr}"
                        )
                        .unwrap(),
                        shape @ (OccurrenceShape::Bounded { .. }
                        | OccurrenceShape::Unbounded { .. }) => {
                            let (min, max) = sequence_bounds(shape);
                            let item_path = format!("({path}) + '[' + str(index) + ']' ");
                            let expr = reference(
                                &field.type_ref,
                                &field.constraints,
                                "item",
                                &item_path,
                                true,
                            )?;
                            writeln!(output, "    items = _sequence({access}, tuple, {path}, {min}, {max})\n    if items:\n        result[{key}] = [{expr} for index, item in enumerate(items)]").unwrap();
                        }
                    }
                }
                writeln!(output, "    return result").unwrap();
            }
            _ => unreachable!("model preflight"),
        }
        writeln!(output, "\n\ndef _decode_{symbol}(value, path):").unwrap();
        match &declaration.kind {
            TypeKind::Primitive(kind) => {
                let expr = reference(
                    &TypeRef::primitive(*kind),
                    &declaration.constraints,
                    "value",
                    "path",
                    false,
                )?;
                writeln!(output, "    return _model.{symbol}({expr})").unwrap();
            }
            TypeKind::Alias(target) => {
                let expr = reference(target, &ConstraintSet::default(), "value", "path", false)?;
                writeln!(output, "    return _model.{symbol}({expr})").unwrap();
            }
            TypeKind::Enumeration { variants } => {
                writeln!(output, "    _scalar(value, path, str)").unwrap();
                for variant in variants {
                    writeln!(
                        output,
                        "    if value == {}:\n        return _model.{symbol}.{}",
                        names::literal(&variant.wire_value),
                        names::symbol(&variant.wire_value, false)
                    )
                    .unwrap();
                }
                writeln!(
                    output,
                    "    raise CodecError(f'{{path}}: unknown enumeration wire value')"
                )
                .unwrap();
            }
            TypeKind::Record { .. } => {
                let fields = crate::record_fields(schema, declaration)?;
                let keys = fields
                    .iter()
                    .map(|field| wire_key(field).map(|key| names::literal(&key)))
                    .collect::<Result<Vec<_>, _>>()?;
                writeln!(
                    output,
                    "    _object(value, [{}], path)\n    return _model.{symbol}(",
                    keys.join(", ")
                )
                .unwrap();
                for field in fields {
                    let key = names::literal(&wire_key(field)?);
                    let path = format!("path + '.' + {key}");
                    let value = format!("_required(value, {key}, path)");
                    let expr =
                        reference(&field.type_ref, &field.constraints, &value, &path, false)?;
                    let expr = match field.cardinality.shape() {
                        OccurrenceShape::RequiredOne => expr,
                        OccurrenceShape::OptionalOne => {
                            format!("{expr} if {key} in value else None")
                        }
                        shape @ (OccurrenceShape::Bounded { .. }
                        | OccurrenceShape::Unbounded { .. }) => {
                            let (min, max) = sequence_bounds(shape);
                            let item_path = format!("({path}) + '[' + str(index) + ']' ");
                            let expr = reference(
                                &field.type_ref,
                                &field.constraints,
                                "item",
                                &item_path,
                                false,
                            )?;
                            format!(
                                "tuple({expr} for index, item in enumerate(_array(value, {key}, path, {min}, {max})))"
                            )
                        }
                    };
                    writeln!(
                        output,
                        "        {}={expr},",
                        names::symbol(&field.name, true)
                    )
                    .unwrap();
                }
                writeln!(output, "    )").unwrap();
            }
            _ => unreachable!("model preflight"),
        }
    }
    let mut payloads = BTreeSet::new();
    for function in model.functions() {
        for exchange in function.exchanges() {
            if let Some(binding) = exchange.oms_binding() {
                let name = binding.payload_name();
                if payloads.insert(name.clone()) {
                    let symbol = names::symbol(&name.local_name, false);
                    writeln!(output, "\n\ndef encode_{symbol}(payload: _model.{symbol}) -> str:\n    \"\"\"Encode a validated {symbol} payload body as deterministic Unicode JSON.\"\"\"\n    return _dumps(_encode_{symbol}(payload, '$'))\n\n\ndef decode_{symbol}(text: str) -> _model.{symbol}:\n    \"\"\"Decode strict JSON text into the exact frozen {symbol} model type.\"\"\"\n    return _decode_{symbol}(_loads(text), '$')").unwrap();
                }
            }
        }
    }
    Ok(vec![GeneratedFile {
        relative_path: PathBuf::from(SERVICE_CODEC_FILE),
        contents: output,
    }])
}

fn sequence_bounds(shape: OccurrenceShape) -> (u64, String) {
    match shape {
        OccurrenceShape::Bounded { min, max } => (min, max.to_string()),
        OccurrenceShape::Unbounded { min } => (min, "None".to_owned()),
        _ => unreachable!("repeated member"),
    }
}

#[cfg(test)]
mod tests {
    use super::unbounded_runtime;

    #[test]
    fn unbounded_runtime_replaces_exactly_one_check() {
        let template = include_str!("codec_runtime.py");
        let expected = template.replace(
            "if not lower <= len(value) <= upper:",
            "if len(value) < lower or (upper is not None and len(value) > upper):",
        );
        assert_eq!(unbounded_runtime(template).unwrap(), expected);
    }

    #[test]
    fn unbounded_runtime_rejects_missing_or_duplicate_checks() {
        let template = include_str!("codec_runtime.py");
        let missing = template.replace("if not lower <= len(value) <= upper:", "if False:");
        for invalid in [missing, template.repeat(2)] {
            let failure = unbounded_runtime(&invalid).unwrap_err();
            assert!(failure.to_string().contains("expected exactly one"));
        }
    }
}
