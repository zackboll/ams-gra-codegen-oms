//! Task 050: generated Rust OMS JSON payload codecs (OMSC-SPC-013 Rev B §6.1).
//!
//! Consumes only the lowered [`ServiceApiModel`], the projected [`SchemaIr`],
//! and the [`GenerationWorld`]. Every Rust spelling it emits -- type, field,
//! enum variant, Choice variant, abstract-value variant -- comes from the
//! SAME functions the model renderer uses (`super::upper_camel` for type
//! names, Task 054 `super::record_field_name` / `super::choice_alternative_name`
//! for structural members, `generated_enum_variant_name` for enumerations),
//! so a model rename changes the codec identically. The OMS JSON member key
//! never comes from those: it is always `member_key` (the element's own
//! QName), so a host-language escape can never leak onto the wire. Effective inheritance, storage elision, and
//! closed abstract sums come from the shared codegen-core helpers.
//!
//! Validation authority stays with the generated model: every decoded value
//! is built through its generated checked constructor (`Type::new`,
//! `BoundedVec::new`, `UnboundedVec::new`, `XmlSchemaDateTime::new`,
//! `XmlSchemaDuration::new`,
//! `BoundedI64/U64::new`). No bound, facet, or cardinality is duplicated here.
//!
//! Recursive helpers are named `encode_tNNN` / `decode_tNNN` from the stable
//! emission order, never from schema names, so they add no public API and
//! need no name preflight. The public surface is `ServiceCodec` and its
//! `OmsJsonCodec<P>` impls.

use super::{choice_alternative_name, error, record_field_name, upper_camel};
use ams_gra_oms_codegen_core::{
    BackendLanguage, CodegenError, DirectTemporalProfile, EffectiveValueMember, GenerationWorld,
    InclusiveIntegralDomain, ServiceApiModel, TypeEmission, analyze_service_codec,
    binary_length_domain, direct_temporal_profile, effective_choice_alternatives,
    effective_record_fields, field_storage_semantics, floating_domain, generated_enum_variant_name,
    inclusive_integral_domain, oms_json_member_name, oms_json_type_name, plan_type_emissions,
    service_api_fixed_names,
};
use ams_gra_oms_ir::{
    BinaryLexicalEncoding, FieldDecl, OccurrenceShape, PrimitiveKind, QualifiedName, SchemaIr,
    TypeDecl, TypeKind, TypeRefTarget, declaration_binary_encoding, resolve_binary_encoding,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// The generated codec file name.
pub use ams_gra_oms_codegen_core::RUST_SERVICE_CODEC_FILE as SERVICE_CODEC_FILE;

/// Fixed private prelude of every generated codec file. Its names are
/// module-private and schema-independent.
const PRELUDE: &str = r#"// Generated OMS JSON payload codec (Task 050, OMSC-SPC-013 Rev B section 6.1).
//
// Encodes and decodes the PAYLOAD BODY of each selected OMS message; the
// runtime owns the one-member global-element envelope. Every decoded value is
// constructed through the generated model's checked constructors, so the
// model -- not this file -- decides whether a value inhabits its schema type.
//
// Decode receives an already-parsed serde_json::Value, so duplicate JSON
// object keys have been collapsed by the parser before this code runs; this
// codec does not (and cannot) detect them.
#![allow(dead_code)]

use ams_gra_oms_runtime_rust::{CodecError, OmsJsonCodec};
use serde_json::{Map, Value};

/// The generated OMS JSON codec for every selected payload of this service.
#[derive(Debug, Default, Clone, Copy)]
pub struct ServiceCodec;

fn invalid(path: &str, message: &str) -> CodecError {
    CodecError::new(format!("{path}: {message}"))
}

fn json_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn mismatch(path: &str, expected: &str, value: &Value) -> CodecError {
    invalid(path, &format!("expected {expected}, found {}", json_kind(value)))
}

fn expect_object<'v>(value: &'v Value, path: &str) -> Result<&'v Map<String, Value>, CodecError> {
    value.as_object().ok_or_else(|| mismatch(path, "object", value))
}

/// Reject unknown members. `$type` is accepted only when it names the
/// concrete type being decoded (a derived type would lose content).
fn check_members(
    object: &Map<String, Value>,
    path: &str,
    own_type: &str,
    allowed: &[&str],
) -> Result<(), CodecError> {
    for (key, value) in object {
        if key == "$type" {
            if value.as_str() != Some(own_type) {
                return Err(invalid(path, &format!("$type {value} is not {own_type:?}")));
            }
        } else if !allowed.contains(&key.as_str()) {
            return Err(invalid(path, &format!("unknown member {key:?}")));
        }
    }
    Ok(())
}

fn required<'v>(object: &'v Map<String, Value>, name: &str, path: &str) -> Result<&'v Value, CodecError> {
    object
        .get(name)
        .ok_or_else(|| invalid(path, &format!("missing required member {name:?}")))
}

/// A repeated member: absent means zero occurrences; present must be an array.
fn repeated<'v>(object: &'v Map<String, Value>, name: &str, path: &str) -> Result<&'v [Value], CodecError> {
    match object.get(name) {
        None => Ok(&[]),
        Some(Value::Array(items)) => Ok(items),
        Some(other) => Err(mismatch(&format!("{path}.{name}"), "array", other)),
    }
}

fn expect_array<'v>(value: &'v Value, path: &str) -> Result<&'v [Value], CodecError> {
    match value {
        Value::Array(items) => Ok(items),
        other => Err(mismatch(path, "array", other)),
    }
}

fn cardinality(path: &str, count: usize) -> CodecError {
    invalid(path, &format!("{count} occurrence(s) rejected by the generated sequence constructor"))
}

fn rejected(path: &str, carrier: &str) -> CodecError {
    invalid(path, &format!("value rejected by generated {carrier}::new"))
}

fn dec_bool(value: &Value, path: &str) -> Result<bool, CodecError> {
    value.as_bool().ok_or_else(|| mismatch(path, "boolean", value))
}

fn dec_i64(value: &Value, path: &str) -> Result<i64, CodecError> {
    match value {
        Value::Number(number) => number
            .as_i64()
            .ok_or_else(|| invalid(path, &format!("{number} is not a signed 64-bit integer"))),
        other => Err(mismatch(path, "integer number", other)),
    }
}

fn dec_u64(value: &Value, path: &str) -> Result<u64, CodecError> {
    match value {
        Value::Number(number) => number
            .as_u64()
            .ok_or_else(|| invalid(path, &format!("{number} is not an unsigned 64-bit integer"))),
        other => Err(mismatch(path, "integer number", other)),
    }
}

fn special_float(value: &Value, path: &str) -> Result<f64, CodecError> {
    match value.as_str() {
        Some("NaN") => Ok(f64::NAN),
        Some("Infinity") => Ok(f64::INFINITY),
        Some("-Infinity") => Ok(f64::NEG_INFINITY),
        Some(other) => Err(invalid(
            path,
            &format!("{other:?} is not a number or one of \"NaN\", \"Infinity\", \"-Infinity\""),
        )),
        None => Err(mismatch(path, "number or special float string", value)),
    }
}

fn dec_f64(value: &Value, path: &str) -> Result<f64, CodecError> {
    match value {
        Value::Number(number) => number
            .as_f64()
            .filter(|parsed| parsed.is_finite())
            .ok_or_else(|| invalid(path, &format!("{number} is not a finite double"))),
        other => special_float(other, path),
    }
}

fn dec_f32(value: &Value, path: &str) -> Result<f32, CodecError> {
    match value {
        // Parse the JSON number text directly as f32 (one correct rounding).
        Value::Number(number) => number
            .to_string()
            .parse::<f32>()
            .ok()
            .filter(|parsed| parsed.is_finite())
            .ok_or_else(|| invalid(path, &format!("{number} is not a finite float"))),
        #[allow(clippy::cast_possible_truncation)]
        other => special_float(other, path).map(|special| special as f32),
    }
}

fn enc_f64(value: f64) -> Value {
    if value.is_nan() {
        Value::String("NaN".to_owned())
    } else if value == f64::INFINITY {
        Value::String("Infinity".to_owned())
    } else if value == f64::NEG_INFINITY {
        Value::String("-Infinity".to_owned())
    } else {
        serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number)
    }
}

fn enc_f32(value: f32) -> Value {
    if value.is_finite() {
        // Shortest f32 spelling, so the JSON number denotes exactly `value`.
        value
            .to_string()
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map_or(Value::Null, Value::Number)
    } else {
        enc_f64(f64::from(value))
    }
}

fn dec_str<'v>(value: &'v Value, path: &str) -> Result<&'v str, CodecError> {
    value.as_str().ok_or_else(|| mismatch(path, "string", value))
}

fn with_type(mut value: Value, concrete: &str) -> Value {
    if let Value::Object(object) = &mut value {
        object.insert("$type".to_owned(), Value::String(concrete.to_owned()));
    }
    value
}
"#;

/// Task 052: the ONE shared private `xs:hexBinary` lexical mapping, appended
/// only when the codec surface contains a HexBinary value, so a codec with no
/// Binary is byte-identical to Task 050/051 output. It is generated payload
/// lexical mapping, not a runtime dependency.
const HEX_BINARY_HELPERS: &str = r#"
/// XML Schema 2E 3.2.15.2 canonical `hexBinary`: two UPPERCASE hexadecimal
/// digits per octet, no separator, no prefix.
fn encode_hex_binary(octets: &[u8]) -> Value {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut text = String::with_capacity(octets.len() * 2);
    for &octet in octets {
        text.push(char::from(DIGITS[usize::from(octet >> 4)]));
        text.push(char::from(DIGITS[usize::from(octet & 0x0F)]));
    }
    Value::String(text)
}

/// XML Schema 2E `hexBinary` lexical parse of one JSON string. First the
/// fixed `whiteSpace=collapse` normalization (4.3.6): TAB/LF/CR become SPACE,
/// SPACE runs collapse to one, leading/trailing SPACE are removed; then the
/// result must be pairs of case-insensitive ASCII hexadecimal digits
/// (3.2.15.1). An internal space survives collapse and is rejected.
fn decode_hex_binary(value: &Value, path: &str) -> Result<Vec<u8>, CodecError> {
    let raw = value.as_str().ok_or_else(|| mismatch(path, "hexBinary string", value))?;
    let mut collapsed = String::with_capacity(raw.len());
    for word in raw
        .split(|c| matches!(c, ' ' | '\t' | '\n' | '\r'))
        .filter(|word| !word.is_empty())
    {
        if !collapsed.is_empty() {
            collapsed.push(' ');
        }
        collapsed.push_str(word);
    }
    let nibble = |digit: u8| match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    };
    let nibbles = collapsed
        .bytes()
        .map(nibble)
        .collect::<Option<Vec<u8>>>()
        .ok_or_else(|| invalid(path, &format!("{raw:?} is not hexBinary: only hexadecimal digits [0-9A-Fa-f] are allowed")))?;
    if nibbles.len() % 2 != 0 {
        return Err(invalid(path, &format!("{raw:?} is not hexBinary: odd number of hexadecimal digits")));
    }
    Ok(nibbles.chunks_exact(2).map(|pair| (pair[0] << 4) | pair[1]).collect())
}
"#;

/// Render `service_codec.rs` for a service with at least one OMS exchange.
///
/// Fails closed with the shared codec preflight's reason, so a direct caller
/// that skipped readiness cannot obtain a codec for an unsupported schema.
///
/// # Errors
///
/// A [`CodegenError`] naming the `service codec boundary` or the construct
/// the renderer cannot represent.
pub fn generate_service_codec(
    model: &ServiceApiModel,
    schema: &SchemaIr,
    world: GenerationWorld,
) -> Result<String, CodegenError> {
    let readiness = analyze_service_codec(model, schema, BackendLanguage::Rust, world)
        .map_err(|failure| error(format!("service codec boundary: {failure}")))?;
    if let Some(blocker) = readiness.blocker {
        return Err(error(format!("service codec boundary: {blocker}")));
    }
    if !readiness.has_oms_exchanges {
        return Err(error(
            "service codec boundary: a service with no OMS exchange has no codec",
        ));
    }
    let model_module = service_api_fixed_names(BackendLanguage::Rust)
        .model_module
        .ok_or_else(|| error("Rust service API requires a model module name"))?;
    let emissions = plan_type_emissions(schema, world)?;
    let emitted: Vec<&TypeEmission<'_>> = emissions
        .iter()
        .filter(|emission| emission.emits_own_top_level_name())
        .collect();
    let mut ids = BTreeMap::new();
    for (index, emission) in emitted.iter().enumerate() {
        ids.insert(emission_name(emission).clone(), index);
    }
    let renderer = Renderer {
        schema,
        world,
        model: format!("super::{model_module}"),
        ids,
    };

    let mut output = String::from(PRELUDE);
    // One impl per UNIQUE payload type, in contract first-occurrence order.
    let mut seen = BTreeSet::new();
    for binding in model
        .functions()
        .iter()
        .flat_map(|function| function.exchanges())
        .filter_map(|exchange| exchange.oms_binding())
    {
        let payload = binding.payload_name();
        if !seen.insert(payload.clone()) {
            continue;
        }
        let id = renderer.id(payload)?;
        let rust = format!("{}::{}", renderer.model, upper_camel(&payload.local_name)?);
        let path = &payload.local_name;
        writeln!(
            output,
            "\nimpl OmsJsonCodec<{rust}> for ServiceCodec {{\n    \
             fn encode_payload(&self, value: &{rust}) -> Result<Value, CodecError> {{\n        \
             Ok(encode_t{id:03}(value))\n    }}\n\n    \
             fn decode_payload(&self, value: &Value) -> Result<{rust}, CodecError> {{\n        \
             decode_t{id:03}(value, {path:?})\n    }}\n}}"
        )
        .expect("writing to String cannot fail");
    }
    for (index, emission) in emitted.iter().enumerate() {
        renderer.emission(&mut output, index, emission)?;
    }
    // Task 052: one shared hex helper pair, only when a HexBinary value is
    // actually encoded, so every Binary-free codec is unchanged byte for byte.
    if output.contains("encode_hex_binary(") {
        output.push_str(HEX_BINARY_HELPERS);
    }
    Ok(output)
}

/// Task 051: the OMS JSON member key of one stored Record field or Choice
/// alternative, from its OWN element QName (`FieldDecl::wire_name`) through
/// the shared formatter. Never the Rust spelling and never the owner's,
/// message's, or member type's namespace.
fn member_key(field: &FieldDecl) -> Result<String, CodegenError> {
    let wire = field.wire_name();
    let namespace = wire.namespace_uri.ok_or_else(|| {
        error(format!(
            "service codec boundary: {} has an unqualified local element with no \
             evidenced OMS JSON member-name mapping",
            field.name
        ))
    })?;
    Ok(oms_json_member_name(namespace, wire.local_name))
}

/// The `"$type"` value of a concrete complexType: its OWN declaration QName.
fn type_name(name: &QualifiedName) -> String {
    oms_json_type_name(&name.namespace_uri, &name.local_name)
}

/// `text` embedded in a generated `format!` string literal: Rust-escaped,
/// with `{`/`}` doubled so a Clark key (`{urn:x}Name`) stays literal. A bare
/// NCName is returned unchanged, so OAM output is byte-identical.
fn format_literal(text: &str) -> String {
    let quoted = format!("{text:?}");
    quoted[1..quoted.len() - 1]
        .replace('{', "{{")
        .replace('}', "}}")
}

fn emission_name<'a>(emission: &'a TypeEmission<'_>) -> &'a QualifiedName {
    match emission {
        TypeEmission::Declaration(declaration) => &declaration.name,
        TypeEmission::AbstractValue(projection) => &projection.declaration.name,
    }
}

struct Renderer<'s> {
    schema: &'s SchemaIr,
    world: GenerationWorld,
    /// `super::model`: the codec is mounted beside the model module.
    model: String,
    ids: BTreeMap<QualifiedName, usize>,
}

/// How one member's base value is stored, mirroring `super::rust_field_base`.
enum Base {
    Signed(Option<(i64, i64)>),
    Unsigned(Option<(u64, u64)>),
    Boolean,
    Float32,
    Float64,
    String,
    DirectDateTime,
    /// Task 057: the direct `XmlSchemaDuration` checked lexical carrier.
    DirectDuration,
    /// Task 052: a direct `Vec<u8>` whose provenance resolved to hexBinary.
    HexBinary,
    Named(usize),
}

impl Renderer<'_> {
    fn id(&self, name: &QualifiedName) -> Result<usize, CodegenError> {
        self.ids.get(name).copied().ok_or_else(|| {
            error(format!(
                "service codec: {} is not a generated model type",
                name.local_name
            ))
        })
    }

    fn rust(&self, name: &QualifiedName) -> Result<String, CodegenError> {
        Ok(format!(
            "{}::{}",
            self.model,
            upper_camel(&name.local_name)?
        ))
    }

    fn base(&self, field: &FieldDecl) -> Result<Base, CodegenError> {
        let kind = match &field.type_ref.target {
            TypeRefTarget::Named(name) => return Ok(Base::Named(self.id(name)?)),
            TypeRefTarget::Primitive(kind) => *kind,
        };
        let domain = || {
            inclusive_integral_domain(kind, &field.constraints)
                .map_err(|reason| error(format!("service codec: {reason} on {}", field.name)))
        };
        Ok(match kind {
            PrimitiveKind::SignedInteger => Base::Signed(match domain()? {
                Some(InclusiveIntegralDomain::Signed { min, max }) => Some((min, max)),
                _ => None,
            }),
            PrimitiveKind::UnsignedInteger => Base::Unsigned(match domain()? {
                Some(InclusiveIntegralDomain::Unsigned { min, max }) => Some((min, max)),
                _ => None,
            }),
            PrimitiveKind::Boolean => Base::Boolean,
            PrimitiveKind::Float32 => Base::Float32,
            PrimitiveKind::Float64 => Base::Float64,
            PrimitiveKind::String => Base::String,
            PrimitiveKind::DateTime => match direct_temporal_profile(kind, &field.constraints) {
                Ok(Some(DirectTemporalProfile::DateTime)) => Base::DirectDateTime,
                _ => {
                    return Err(error(format!(
                        "service codec: direct DateTime constraints on {}",
                        field.name
                    )));
                }
            },
            PrimitiveKind::Duration => match direct_temporal_profile(kind, &field.constraints) {
                Ok(Some(DirectTemporalProfile::Duration)) => Base::DirectDuration,
                _ => {
                    return Err(error(format!(
                        "service codec: direct Duration constraints on {}",
                        field.name
                    )));
                }
            },
            // Task 052: only RESOLVED hexBinary provenance selects the hex
            // mapping (readiness has already rejected anything else).
            PrimitiveKind::Binary => match resolve_binary_encoding(self.schema, &field.type_ref) {
                Some(BinaryLexicalEncoding::HexBinary) => Base::HexBinary,
                _ => {
                    return Err(error(format!(
                        "service codec boundary: {} is Binary without hexBinary provenance",
                        field.name
                    )));
                }
            },
            other => {
                return Err(error(format!(
                    "service codec boundary: {} is {other:?}, which has no codec mapping",
                    field.name
                )));
            }
        })
    }

    /// Expression of type `Value` encoding `x: &Base`.
    fn encode_base(&self, base: &Base, x: &str) -> String {
        match base {
            Base::Signed(Some(_)) | Base::Unsigned(Some(_)) => format!("Value::from({x}.get())"),
            Base::Signed(None) | Base::Unsigned(None) => format!("Value::from(*{x})"),
            Base::Boolean => format!("Value::Bool(*{x})"),
            Base::Float32 => format!("enc_f32(*{x})"),
            Base::Float64 => format!("enc_f64(*{x})"),
            Base::String => format!("Value::String({x}.clone())"),
            Base::DirectDateTime | Base::DirectDuration => {
                format!("Value::String({x}.as_str().to_owned())")
            }
            Base::HexBinary => format!("encode_hex_binary({x})"),
            Base::Named(id) => format!("encode_t{id:03}({x})"),
        }
    }

    /// Expression of type `Result<Base, CodecError>` decoding `v: &Value`
    /// at path `p: &str`.
    fn decode_base(&self, base: &Base, v: &str, p: &str) -> String {
        let model = &self.model;
        match base {
            Base::Signed(Some((min, max))) => format!(
                "dec_i64({v}, {p}).and_then(|n| {model}::BoundedI64::<{min}, {max}>::new(n).ok_or_else(|| rejected({p}, \"BoundedI64\")))"
            ),
            Base::Unsigned(Some((min, max))) => format!(
                "dec_u64({v}, {p}).and_then(|n| {model}::BoundedU64::<{min}, {max}>::new(n).ok_or_else(|| rejected({p}, \"BoundedU64\")))"
            ),
            Base::Signed(None) => format!("dec_i64({v}, {p})"),
            Base::Unsigned(None) => format!("dec_u64({v}, {p})"),
            Base::Boolean => format!("dec_bool({v}, {p})"),
            Base::Float32 => format!("dec_f32({v}, {p})"),
            Base::Float64 => format!("dec_f64({v}, {p})"),
            Base::String => format!("dec_str({v}, {p}).map(str::to_owned)"),
            Base::DirectDateTime => format!(
                "dec_str({v}, {p}).and_then(|s| {model}::XmlSchemaDateTime::new(s).ok_or_else(|| rejected({p}, \"XmlSchemaDateTime\")))"
            ),
            // Task 057: the generated checked constructor is the only
            // authority on the duration lexical space; no grammar is here.
            Base::DirectDuration => format!(
                "dec_str({v}, {p}).and_then(|s| {model}::XmlSchemaDuration::new(s).ok_or_else(|| rejected({p}, \"XmlSchemaDuration\")))"
            ),
            Base::HexBinary => format!("decode_hex_binary({v}, {p})"),
            Base::Named(id) => format!("decode_t{id:03}({v}, {p})"),
        }
    }

    fn emission(
        &self,
        output: &mut String,
        id: usize,
        emission: &TypeEmission<'_>,
    ) -> Result<(), CodegenError> {
        match emission {
            TypeEmission::AbstractValue(projection) => self.abstract_value(
                output,
                id,
                projection.declaration,
                &projection.concrete_descendants,
            ),
            TypeEmission::Declaration(declaration) => match &declaration.kind {
                TypeKind::Primitive(kind) => self.primitive(output, id, declaration, *kind),
                TypeKind::Enumeration { variants } => {
                    let rust = self.rust(&declaration.name)?;
                    let local = &declaration.name.local_name;
                    let mut encode = String::new();
                    let mut decode = String::new();
                    for variant in variants {
                        let name =
                            generated_enum_variant_name(BackendLanguage::Rust, &variant.wire_value)
                                .ok_or_else(|| {
                                    error(format!(
                                        "invalid Rust enumeration wire value {:?}",
                                        variant.wire_value
                                    ))
                                })?;
                        let wire = &variant.wire_value;
                        writeln!(encode, "        {rust}::{name} => {wire:?},")
                            .expect("infallible");
                        writeln!(decode, "        {wire:?} => Ok({rust}::{name}),")
                            .expect("infallible");
                    }
                    writeln!(
                        output,
                        "\n// {local} (enumeration: exact XSD wire values)\nfn encode_t{id:03}(value: &{rust}) -> Value {{\n    Value::String(match value {{\n{encode}    }}.to_owned())\n}}\n\nfn decode_t{id:03}(value: &Value, path: &str) -> Result<{rust}, CodecError> {{\n    match dec_str(value, path)? {{\n{decode}        other => Err(invalid(path, &format!(\"{{other:?}} is not a {local} value\"))),\n    }}\n}}"
                    )
                    .expect("infallible");
                    Ok(())
                }
                TypeKind::Record { .. } => self.record(output, id, declaration),
                TypeKind::Choice { .. } => self.choice(output, id, declaration),
                other => Err(error(format!(
                    "service codec: unsupported declaration kind {other:?}"
                ))),
            },
        }
    }

    fn primitive(
        &self,
        output: &mut String,
        id: usize,
        declaration: &TypeDecl,
        kind: PrimitiveKind,
    ) -> Result<(), CodegenError> {
        let rust = self.rust(&declaration.name)?;
        let local = &declaration.name.local_name;
        let checked = |decode: &str| {
            format!(
                "{decode}.and_then(|x| {rust}::new(x).ok_or_else(|| rejected(path, {local:?})))"
            )
        };
        let (encode, decode) = match kind {
            PrimitiveKind::SignedInteger => (
                "Value::from(value.get())".to_owned(),
                checked("dec_i64(value, path)"),
            ),
            PrimitiveKind::UnsignedInteger => (
                "Value::from(value.get())".to_owned(),
                checked("dec_u64(value, path)"),
            ),
            PrimitiveKind::Boolean => (
                "Value::Bool(value.get())".to_owned(),
                format!("dec_bool(value, path).map({rust}::new)"),
            ),
            PrimitiveKind::Float32 | PrimitiveKind::Float64 => {
                let (enc, dec) = if kind == PrimitiveKind::Float32 {
                    ("enc_f32", "dec_f32(value, path)")
                } else {
                    ("enc_f64", "dec_f64(value, path)")
                };
                // Unconstrained floats keep an infallible `new`; bounded ones
                // return `Option` and the generated bound decides.
                let bounded = floating_domain(kind, &declaration.constraints)
                    .map_err(|reason| error(format!("service codec: {reason} on {local}")))?
                    .is_some();
                (
                    format!("{enc}(value.get())"),
                    if bounded {
                        checked(dec)
                    } else {
                        format!("{dec}.map({rust}::new)")
                    },
                )
            }
            PrimitiveKind::String
            | PrimitiveKind::DateTime
            | PrimitiveKind::Time
            | PrimitiveKind::Duration => (
                "Value::String(value.as_str().to_owned())".to_owned(),
                checked("dec_str(value, path)"),
            ),
            // Task 052: a named Binary through its public model API only
            // (`as_slice`, `new`). Task 053: the SHARED classifier decides
            // which `new` exists. Unconstrained keeps the infallible
            // `.map(T::new)`, byte-identical to Task 052. A length-constrained
            // declaration goes through the generated checked `T::new`, which
            // stays the only authority on octet count: no bound is restated
            // here, and a lexically valid value of the wrong length is a
            // model rejection at this member's path, not a hex error.
            PrimitiveKind::Binary
                if declaration_binary_encoding(self.schema, declaration)
                    == Some(BinaryLexicalEncoding::HexBinary) =>
            {
                let constrained = binary_length_domain(kind, &declaration.constraints)
                    .map_err(|reason| error(format!("service codec: {reason} on {local}")))?
                    .is_some();
                (
                    "encode_hex_binary(value.as_slice())".to_owned(),
                    if constrained {
                        checked("decode_hex_binary(value, path)")
                    } else {
                        format!("decode_hex_binary(value, path).map({rust}::new)")
                    },
                )
            }
            other => {
                return Err(error(format!(
                    "service codec boundary: {local} is {other:?}, which has no codec mapping"
                )));
            }
        };
        writeln!(
            output,
            "\n// {local} (named {kind:?}; the generated constructor validates)\nfn encode_t{id:03}(value: &{rust}) -> Value {{\n    {encode}\n}}\n\nfn decode_t{id:03}(value: &Value, path: &str) -> Result<{rust}, CodecError> {{\n    {decode}\n}}"
        )
        .expect("infallible");
        Ok(())
    }

    /// Encode statements inserting one stored member into `object`.
    fn encode_member(&self, field: &FieldDecl, access: &str) -> Result<String, CodegenError> {
        let base = self.base(field)?;
        let key = &member_key(field)?;
        Ok(match field.cardinality.shape() {
            OccurrenceShape::RequiredOne => format!(
                "    object.insert({key:?}.to_owned(), {{ let x = &{access}; {} }});\n",
                self.encode_base(&base, "x")
            ),
            OccurrenceShape::OptionalOne => format!(
                "    if let Some(x) = &{access} {{\n        object.insert({key:?}.to_owned(), {});\n    }}\n",
                self.encode_base(&base, "x")
            ),
            // Zero occurrences contribute no member at all.
            _ => format!(
                "    if !{access}.as_slice().is_empty() {{\n        object.insert({key:?}.to_owned(), Value::Array({access}.as_slice().iter().map(|x| {}).collect()));\n    }}\n",
                self.encode_base(&base, "x")
            ),
        })
    }

    /// Expression of the member's full Rust storage type decoded from
    /// `object` (a `Map`) at parent path `path`.
    fn decode_member(&self, field: &FieldDecl) -> Result<String, CodegenError> {
        let base = self.base(field)?;
        let key = &member_key(field)?;
        let path_key = format_literal(key);
        let model = &self.model;
        let p = format!("&format!(\"{{path}}.{path_key}\")");
        Ok(match field.cardinality.shape() {
            OccurrenceShape::RequiredOne => format!(
                "{}?",
                self.decode_base(&base, &format!("required(object, {key:?}, path)?"), &p)
            ),
            OccurrenceShape::OptionalOne => format!(
                "match object.get({key:?}) {{ None => None, Some(v) => Some({}?) }}",
                self.decode_base(&base, "v", &p)
            ),
            shape => {
                let wrapper = if matches!(shape, OccurrenceShape::Unbounded { .. }) {
                    "UnboundedVec"
                } else {
                    "BoundedVec"
                };
                format!(
                    "{{ let items = repeated(object, {key:?}, path)?; {model}::{wrapper}::new(items.iter().enumerate().map(|(i, v)| {}).collect::<Result<Vec<_>, _>>()?).ok_or_else(|| cardinality({p}, items.len()))? }}",
                    self.decode_base(
                        &base,
                        "v",
                        &format!("&format!(\"{{path}}.{path_key}[{{i}}]\")")
                    )
                )
            }
        })
    }

    fn record(
        &self,
        output: &mut String,
        id: usize,
        declaration: &TypeDecl,
    ) -> Result<(), CodegenError> {
        let rust = self.rust(&declaration.name)?;
        let local = &declaration.name.local_name;
        let fields = effective_record_fields(self.schema, &declaration.name)
            .map_err(|failure| error(format!("service codec: {local}: {failure:?}")))?;
        let mut encode = String::new();
        let mut decode = String::new();
        let mut allowed = Vec::new();
        for field in fields {
            let stored = field_storage_semantics(self.schema, field, self.world)
                .map_err(|failure| error(format!("service codec: {failure}")))?;
            if matches!(stored, EffectiveValueMember::AbsentOnly(_)) {
                // Task 026: no storage, never encoded, and any member is
                // rejected on decode as unknown.
                continue;
            }
            // Task 054: host API member (possibly escaped, e.g. `field_type`)
            // versus wire key (always the source QName, e.g. "Type").
            let member = record_field_name(&field.name)?;
            encode.push_str(&self.encode_member(field, &format!("value.{member}"))?);
            writeln!(decode, "        {member}: {},", self.decode_member(field)?)
                .expect("infallible");
            allowed.push(format!("{:?}", member_key(field)?));
        }
        // The only `$type` a concrete Record accepts is its own type QName.
        let own_type = type_name(&declaration.name);
        writeln!(
            output,
            "\n// {local} (concrete Record; effective inherited fields, no nested base)\nfn encode_t{id:03}(value: &{rust}) -> Value {{\n    let mut object = Map::new();\n{encode}    let _ = value;\n    Value::Object(object)\n}}\n\nfn decode_t{id:03}(value: &Value, path: &str) -> Result<{rust}, CodecError> {{\n    let object = expect_object(value, path)?;\n    check_members(object, path, {own_type:?}, &[{}])?;\n    Ok({rust} {{\n{decode}    }})\n}}",
            allowed.join(", ")
        )
        .expect("infallible");
        Ok(())
    }

    fn choice(
        &self,
        output: &mut String,
        id: usize,
        declaration: &TypeDecl,
    ) -> Result<(), CodegenError> {
        let rust = self.rust(&declaration.name)?;
        let local = &declaration.name.local_name;
        let alternatives = effective_choice_alternatives(self.schema, &declaration.name)
            .map_err(|failure| error(format!("service codec: {local}: {failure:?}")))?;
        let mut encode = String::new();
        let mut decode = String::new();
        let mut allowed = Vec::new();
        for alternative in alternatives {
            // Rust variant from the shared Task 054 helper (as the model
            // does); wire key from the element's own QName, never escaped.
            let variant = choice_alternative_name(&alternative.name)?;
            let key = &member_key(alternative)?;
            let path_key = format_literal(key);
            let base = self.base(alternative)?;
            let p = format!("&format!(\"{{path}}.{path_key}\")");
            let (enc, dec) = match alternative.cardinality.shape() {
                OccurrenceShape::RequiredOne => (
                    self.encode_base(&base, "x"),
                    format!("{}?", self.decode_base(&base, "member", &p)),
                ),
                shape => {
                    let wrapper = if matches!(shape, OccurrenceShape::Unbounded { .. }) {
                        "UnboundedVec"
                    } else {
                        "BoundedVec"
                    };
                    (
                        format!(
                            "Value::Array(x.as_slice().iter().map(|x| {}).collect())",
                            self.encode_base(&base, "x")
                        ),
                        format!(
                            "{{ let items = expect_array(member, {p})?; {}::{wrapper}::new(items.iter().enumerate().map(|(i, v)| {}).collect::<Result<Vec<_>, _>>()?).ok_or_else(|| cardinality({p}, items.len()))? }}",
                            self.model,
                            self.decode_base(
                                &base,
                                "v",
                                &format!("&format!(\"{{path}}.{path_key}[{{i}}]\")")
                            )
                        ),
                    )
                }
            };
            writeln!(encode, "        {rust}::{variant}(x) => ({key:?}, {enc}),")
                .expect("infallible");
            writeln!(decode, "        {key:?} => {rust}::{variant}({dec}),").expect("infallible");
            allowed.push(format!("{key:?}"));
        }
        writeln!(
            output,
            "\n// {local} (Choice: exactly one selected member)\nfn encode_t{id:03}(value: &{rust}) -> Value {{\n    let (key, member) = match value {{\n{encode}    }};\n    let mut object = Map::new();\n    object.insert(key.to_owned(), member);\n    Value::Object(object)\n}}\n\nfn decode_t{id:03}(value: &Value, path: &str) -> Result<{rust}, CodecError> {{\n    let object = expect_object(value, path)?;\n    check_members(object, path, {own_type:?}, &[{allowed}])?;\n    let mut selected = object.iter().filter(|(key, _)| key.as_str() != \"$type\");\n    let (Some((key, member)), None) = (selected.next(), selected.next()) else {{\n        return Err(invalid(path, \"a Choice requires exactly one selected member\"));\n    }};\n    Ok(match key.as_str() {{\n{decode}        other => return Err(invalid(path, &format!(\"unknown member {{other:?}}\"))),\n    }})\n}}",
            allowed = allowed.join(", "),
            own_type = type_name(&declaration.name)
        )
        .expect("infallible");
        Ok(())
    }

    fn abstract_value(
        &self,
        output: &mut String,
        id: usize,
        base: &TypeDecl,
        descendants: &[&TypeDecl],
    ) -> Result<(), CodegenError> {
        let rust = self.rust(&base.name)?;
        let local = &base.name.local_name;
        let mut encode = String::new();
        let mut decode = String::new();
        for descendant in descendants {
            let variant = upper_camel(&descendant.name.local_name)?;
            // Task 051: the concrete TYPE declaration's own QName is
            // authoritative for `$type`; encode and decode use the same value.
            let concrete = &type_name(&descendant.name);
            let inner = self.id(&descendant.name)?;
            writeln!(
                encode,
                "        {rust}::{variant}(x) => with_type(encode_t{inner:03}(x), {concrete:?}),"
            )
            .expect("infallible");
            writeln!(
                decode,
                "        {concrete:?} => Ok({rust}::{variant}(decode_t{inner:03}(value, path)?)),"
            )
            .expect("infallible");
        }
        writeln!(
            output,
            "\n// {local} (closed abstract value: concrete object plus \"$type\")\nfn encode_t{id:03}(value: &{rust}) -> Value {{\n    match value {{\n{encode}    }}\n}}\n\nfn decode_t{id:03}(value: &Value, path: &str) -> Result<{rust}, CodecError> {{\n    let object = expect_object(value, path)?;\n    let concrete = match object.get(\"$type\") {{\n        Some(Value::String(name)) => name.as_str(),\n        Some(other) => return Err(mismatch(path, \"$type string\", other)),\n        None => return Err(invalid(path, \"abstract {local} requires a $type member\")),\n    }};\n    match concrete {{\n{decode}        other => Err(invalid(path, &format!(\"$type {{other:?}} is not a known concrete {local}\"))),\n    }}\n}}"
        )
        .expect("infallible");
        Ok(())
    }
}
