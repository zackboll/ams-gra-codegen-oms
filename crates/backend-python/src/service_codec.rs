//! Schema-specific OMS payload mapping, never dataclass introspection. Model
//! capability, type spelling and primitive domains remain the PY-001 rules.
use crate::{error, names};
use ams_gra_oms_codegen_core::{
    CodegenError, GeneratedFile, OAM_NAMESPACE, ServiceApiModel, TypeEmission,
    oms_json_member_name, plan_type_emissions,
};
use ams_gra_oms_ir::{
    ConstraintSet, FieldDecl, OccurrenceShape, SchemaIr, TypeKind, TypeRef, TypeRefTarget,
};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::PathBuf;

pub const SERVICE_CODEC_FILE: &str = "service_codec.py";

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
    // Render-time wire preflight is the readiness decision too.
    for declaration in &schema.types {
        // §6.1.4 has a name-specific RFC 4122 mapping. An unconstrained PY-001
        // string carrier does not establish that mapping or its validation.
        if declaration.name.namespace_uri == OAM_NAMESPACE
            && declaration.name.local_name == "UniversallyUniqueIdentifierType"
        {
            return Err(error(
                "Python OMS JSON codec: OAM UniversallyUniqueIdentifierType RFC 4122 mapping is unsupported",
            ));
        }
        if let TypeKind::Record { fields } = &declaration.kind {
            let mut keys = BTreeSet::new();
            for field in fields {
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
    for emission in emissions {
        let TypeEmission::Declaration(declaration) = emission else {
            return Err(error("Python codec abstract value"));
        };
        let symbol = names::symbol(&declaration.name.local_name, false);
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
            TypeKind::Record { fields } => {
                writeln!(output, "    result = {{}}").unwrap();
                for field in fields {
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
                        OccurrenceShape::Bounded { min, max } => {
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
                        OccurrenceShape::Unbounded { .. } => unreachable!("model preflight"),
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
            TypeKind::Record { fields } => {
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
                        OccurrenceShape::Bounded { min, max } => {
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
                        OccurrenceShape::Unbounded { .. } => unreachable!("model preflight"),
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
