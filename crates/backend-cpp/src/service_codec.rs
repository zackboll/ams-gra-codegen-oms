//! CPP-OMS-001: generated C++17 OMS JSON payload codecs
//! (OMSC-SPC-013 Rev B section 6.1).
//!
//! Consumes only the lowered [`ServiceApiModel`] and its projected
//! [`SchemaIr`]. Every C++ spelling it emits (type, enum constant, record
//! member) comes from the SAME functions the model renderer uses
//! (`upper_camel`, `record_field_name`, `generated_enum_variant_name`,
//! `integral_domain`), so a model rename changes the codec identically. The
//! OMS JSON member key never comes from those: it is always the element's own
//! QName (`FieldDecl::wire_name`) through the shared formatter, so a host
//! language escape can never leak onto the wire.
//!
//! The supported subset is decided ONCE, in codegen-core
//! (`analyze_service_codec`): Boolean, integral, plain String, enumeration and
//! concrete Record declarations with required, optional and repeated members.
//! This renderer calls that analyzer first and refuses anything it rejects, so
//! readiness (`service-check`) and generation can never disagree.
//!
//! Validation authority stays with the generated model: every decoded value is
//! built through its checked factory (`Type::create`,
//! `BoundedVector::create`, `UnboundedVector::create`,
//! `BoundedInteger::create`). No bound or cardinality is duplicated here.
//!
//! Per-declaration helpers are named `encode_tNNN` / `decode_tNNN` from the
//! stable emission order, never from schema names, so they add no public API
//! and need no name preflight. The public surface is `CodecError`,
//! `CodecResult<T>`, and one `encode_<Payload>` / `decode_<Payload>` pair per
//! unique selected payload type.

use crate::service_api::string_literal_body;
use crate::{
    choice_alternative_name, error, integral_domain, named_integral_domain, namespace_name,
    record_field_name, upper_camel,
};
use ams_gra_oms_codegen_core::{
    BackendLanguage, CPP_SERVICE_CODEC_FILE, CodegenError, GeneratedFile, InclusiveIntegralDomain,
    ServiceApiModel, TypeEmission, analyze_service_codec, cpp_model_header_name,
    effective_choice_alternatives, effective_record_fields, generated_enum_variant_name,
    oms_json_member_name, oms_json_type_name, plan_type_emissions,
};
use ams_gra_oms_ir::{
    FieldDecl, OccurrenceShape, PrimitiveKind, QualifiedName, SchemaIr, TypeDecl, TypeKind,
    TypeRefTarget,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;

/// The generated codec header file name.
pub const SERVICE_CODEC_FILE: &str = CPP_SERVICE_CODEC_FILE;

/// Fixed runtime: reader, writer, result types. Opens (and leaves open)
/// `namespace service_codec { namespace detail {`.
const RUNTIME: &str = include_str!("codec_runtime.hpp");

/// Render `service_codec.hpp` for a service with at least one OMS exchange.
///
/// Fails closed with the shared codec analyzer's reason, so a direct caller
/// that skipped readiness cannot obtain a codec for an unsupported schema. A
/// service with no OMS exchange needs no codec and yields no file.
///
/// # Errors
///
/// A [`CodegenError`] naming the `service codec boundary` or the construct the
/// renderer cannot represent.
pub fn generate_service_codec(
    model: &ServiceApiModel,
    schema: &SchemaIr,
) -> Result<Vec<GeneratedFile>, CodegenError> {
    if !model.has_oms_exchanges() {
        return Ok(Vec::new());
    }
    let world = model.world();
    let readiness = analyze_service_codec(model, schema, BackendLanguage::Cpp, world)
        .map_err(|failure| error(format!("service codec boundary: {failure}")))?;
    if let Some(blocker) = readiness.blocker {
        return Err(error(format!("service codec boundary: {blocker}")));
    }
    let emissions = plan_type_emissions(schema, world)?;
    let emitted: Vec<&TypeEmission<'_>> = emissions
        .iter()
        .filter(|emission| emission.emits_own_top_level_name())
        .collect();
    let mut ids = BTreeMap::new();
    for (index, emission) in emitted.iter().enumerate() {
        let TypeEmission::Declaration(declaration) = emission else {
            return Err(error(
                "service codec: abstract value reached the C++ renderer",
            ));
        };
        ids.insert(declaration.name.clone(), index);
    }
    let renderer = Renderer { schema, ids };

    let mut output = RUNTIME.replace(
        "@MODEL_INCLUDE@",
        &format!("#include \"{}\"", cpp_model_header_name(schema)?),
    );
    let namespace = namespace_name(schema)?;
    writeln!(output, "namespace M = ::{namespace};\n").expect("infallible");
    for (index, emission) in emitted.iter().enumerate() {
        let TypeEmission::Declaration(declaration) = emission else {
            unreachable!("checked above");
        };
        renderer.declaration(&mut output, index, declaration)?;
    }
    output.push_str("}  // namespace detail\n");

    // One encode/decode pair per UNIQUE payload type, in contract
    // first-occurrence order.
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
        let symbol = upper_camel(&payload.local_name)?;
        let cpp = format!("::{namespace}::{symbol}");
        let local = string_literal_body(&payload.local_name);
        writeln!(
            output,
            "\n/// Encode one {symbol} payload body as JSON text.\n\
             inline CodecResult<std::string> encode_{symbol}(const {cpp}& value) {{\n    \
             std::string out;\n    CodecError error;\n    \
             const detail::Path root{{nullptr, \"{local}\", 0, false}};\n    \
             if (!detail::encode_t{id:03}(value, out, error, &root)) {{\n        \
             return CodecResult<std::string>::failure(std::move(error));\n    }}\n    \
             return CodecResult<std::string>::success(std::move(out));\n}}\n\n\
             /// Decode one {symbol} payload body from UTF-8 JSON text.\n\
             inline CodecResult<{cpp}> decode_{symbol}(std::string_view json) {{\n    \
             CodecError error;\n    detail::Value tree;\n    \
             if (!detail::parse_json(json, tree, error)) {{\n        \
             return CodecResult<{cpp}>::failure(std::move(error));\n    }}\n    \
             const detail::Path root{{nullptr, \"{local}\", 0, false}};\n    \
             std::optional<{cpp}> decoded = detail::decode_t{id:03}(tree, error, &root);\n    \
             if (!decoded) {{\n        \
             return CodecResult<{cpp}>::failure(std::move(error));\n    }}\n    \
             return CodecResult<{cpp}>::success(std::move(*decoded));\n}}"
        )
        .expect("infallible");
    }
    output.push_str("\n}  // namespace service_codec\n");
    Ok(vec![GeneratedFile {
        relative_path: PathBuf::from(SERVICE_CODEC_FILE),
        contents: output,
    }])
}

/// The OMS JSON member key of one stored Record field, from its OWN element
/// QName through the shared formatter.
fn member_key(field: &FieldDecl) -> Result<String, CodegenError> {
    let wire = field.wire_name();
    let namespace = wire.namespace_uri.filter(|namespace| !namespace.is_empty());
    namespace
        .map(|namespace| oms_json_member_name(namespace, wire.local_name))
        .ok_or_else(|| {
            error(format!(
                "service codec boundary: unqualified element member {} has no OMS JSON name",
                wire.local_name
            ))
        })
}

/// A JSON string literal body (already quoted) for a generated constant.
fn json_quoted(text: &str) -> String {
    let mut quoted = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            control if (control as u32) < 0x20 => {
                write!(quoted, "\\u{:04x}", control as u32).expect("infallible");
            }
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

/// How one member's (or one repetition item's) value is stored and coded.
enum Base {
    Bool,
    String,
    Signed(Option<(i64, i64)>),
    Unsigned(Option<(u64, u64)>),
    /// A direct `xs:dateTime` / `xs:duration` member: the model's
    /// `XmlSchemaDateTime` / `XmlSchemaDuration` checked text carrier.
    Text {
        cpp: &'static str,
        what: &'static str,
    },
    /// A direct `xs:hexBinary` member: `std::vector<std::uint8_t>`.
    Hex,
    Named {
        id: usize,
        cpp: String,
    },
}

struct Renderer<'a> {
    schema: &'a SchemaIr,
    ids: BTreeMap<QualifiedName, usize>,
}

impl Renderer<'_> {
    fn id(&self, name: &QualifiedName) -> Result<usize, CodegenError> {
        self.ids.get(name).copied().ok_or_else(|| {
            error(format!(
                "service codec boundary: {} is not an emitted declaration",
                name.local_name
            ))
        })
    }

    fn base(&self, field: &FieldDecl) -> Result<Base, CodegenError> {
        match &field.type_ref.target {
            TypeRefTarget::Primitive(PrimitiveKind::Boolean) => Ok(Base::Bool),
            TypeRefTarget::Primitive(PrimitiveKind::String) => Ok(Base::String),
            TypeRefTarget::Primitive(
                kind @ (PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger),
            ) => Ok(
                match integral_domain(*kind, &field.constraints, &field.name)? {
                    Some(InclusiveIntegralDomain::Signed { min, max }) => {
                        Base::Signed(Some((min, max)))
                    }
                    Some(InclusiveIntegralDomain::Unsigned { min, max }) => {
                        Base::Unsigned(Some((min, max)))
                    }
                    None if *kind == PrimitiveKind::SignedInteger => Base::Signed(None),
                    None => Base::Unsigned(None),
                },
            ),
            TypeRefTarget::Primitive(PrimitiveKind::DateTime) => Ok(Base::Text {
                cpp: "M::XmlSchemaDateTime",
                what: "dateTime",
            }),
            TypeRefTarget::Primitive(PrimitiveKind::Duration) => Ok(Base::Text {
                cpp: "M::XmlSchemaDuration",
                what: "duration",
            }),
            TypeRefTarget::Primitive(PrimitiveKind::Binary) => Ok(Base::Hex),
            TypeRefTarget::Primitive(kind) => Err(error(format!(
                "service codec boundary: member {} is {kind:?}, which has no C++ codec mapping",
                field.name
            ))),
            TypeRefTarget::Named(name) => Ok(Base::Named {
                id: self.id(name)?,
                cpp: format!("M::{}", upper_camel(&name.local_name)?),
            }),
        }
    }

    /// The model's C++ spelling of a base value (relative to alias `M`).
    fn cpp(base: &Base) -> String {
        match base {
            Base::Bool => "bool".to_owned(),
            Base::String => "std::string".to_owned(),
            Base::Signed(None) => "std::int64_t".to_owned(),
            Base::Unsigned(None) => "std::uint64_t".to_owned(),
            Base::Signed(Some((min, max))) => format!(
                "M::BoundedInteger<std::int64_t, {}, {}>",
                crate::cpp_signed_bound(*min),
                crate::cpp_signed_bound(*max)
            ),
            Base::Unsigned(Some((min, max))) => format!(
                "M::BoundedInteger<std::uint64_t, {}, {}>",
                crate::cpp_unsigned_bound(*min),
                crate::cpp_unsigned_bound(*max)
            ),
            Base::Text { cpp, .. } => (*cpp).to_owned(),
            Base::Hex => "std::vector<std::uint8_t>".to_owned(),
            Base::Named { cpp, .. } => cpp.clone(),
        }
    }

    /// A `bool`-valued C++ expression encoding `access` to `out`.
    fn encode(base: &Base, access: &str, path: &str) -> String {
        match base {
            Base::Bool => format!("put_bool(out, {access})"),
            Base::String => format!("put_string(out, {access}, error, {path})"),
            Base::Signed(None) => format!("put_i64(out, {access})"),
            Base::Unsigned(None) => format!("put_u64(out, {access})"),
            Base::Signed(Some(_)) => format!("put_i64(out, {access}.value())"),
            Base::Unsigned(Some(_)) => format!("put_u64(out, {access}.value())"),
            Base::Text { .. } => format!("put_string(out, {access}.value(), error, {path})"),
            Base::Hex => format!("put_hex_binary(out, {access})"),
            Base::Named { id, .. } => format!("encode_t{id:03}({access}, out, error, {path})"),
        }
    }

    /// An expression of type `std::optional<base>` decoding `value`.
    fn decode(base: &Base, value: &str, path: &str) -> String {
        match base {
            Base::Bool => format!("get_bool({value}, error, {path})"),
            Base::String => format!("get_string({value}, error, {path})"),
            Base::Signed(None) => format!("get_i64({value}, error, {path})"),
            Base::Unsigned(None) => format!("get_u64({value}, error, {path})"),
            Base::Signed(Some(_)) => format!(
                "get_checked_i64<{}>({value}, error, {path})",
                Self::cpp(base)
            ),
            Base::Unsigned(Some(_)) => format!(
                "get_checked_u64<{}>({value}, error, {path})",
                Self::cpp(base)
            ),
            Base::Text { cpp, what } => {
                format!("get_checked_text<{cpp}>({value}, error, {path}, \"{what}\")")
            }
            Base::Hex => format!("get_hex_binary({value}, error, {path})"),
            Base::Named { id, .. } => format!("decode_t{id:03}({value}, error, {path})"),
        }
    }

    fn declaration(
        &self,
        output: &mut String,
        id: usize,
        declaration: &TypeDecl,
    ) -> Result<(), CodegenError> {
        let local = &declaration.name.local_name;
        let cpp = format!("M::{}", upper_camel(local)?);
        match &declaration.kind {
            TypeKind::Primitive(PrimitiveKind::Boolean) => {
                writeln!(
                    output,
                    "// {local} (named Boolean)\n\
                     inline bool encode_t{id:03}(const {cpp}& value, std::string& out, CodecError& error, const Path* path) {{\n    \
                     static_cast<void>(error);\n    static_cast<void>(path);\n    return put_bool(out, value.value());\n}}\n\n\
                     inline std::optional<{cpp}> decode_t{id:03}(const Value& value, CodecError& error, const Path* path) {{\n    \
                     const std::optional<bool> flag = get_bool(value, error, path);\n    \
                     if (!flag) {{\n        return std::nullopt;\n    }}\n    return {cpp}(*flag);\n}}\n"
                )
                .expect("infallible");
            }
            TypeKind::Primitive(
                kind @ (PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger),
            ) => {
                let domain = named_integral_domain(*kind, &declaration.constraints, local)?;
                let (put, get) = match domain {
                    Some(InclusiveIntegralDomain::Signed { .. }) => ("put_i64", "get_checked_i64"),
                    Some(InclusiveIntegralDomain::Unsigned { .. }) => {
                        ("put_u64", "get_checked_u64")
                    }
                    None => {
                        return Err(error(format!(
                            "service codec boundary: {local} has no integral domain"
                        )));
                    }
                };
                writeln!(
                    output,
                    "// {local} (named integral; the generated factory validates)\n\
                     inline bool encode_t{id:03}(const {cpp}& value, std::string& out, CodecError& error, const Path* path) {{\n    \
                     static_cast<void>(error);\n    static_cast<void>(path);\n    return {put}(out, value.value());\n}}\n\n\
                     inline std::optional<{cpp}> decode_t{id:03}(const Value& value, CodecError& error, const Path* path) {{\n    \
                     return {get}<{cpp}>(value, error, path);\n}}\n",
                    put = put,
                    get = get,
                )
                .expect("infallible");
            }
            TypeKind::Enumeration { variants } => {
                let mut encode = String::new();
                let mut decode = String::new();
                for variant in variants {
                    let constant =
                        generated_enum_variant_name(BackendLanguage::Cpp, &variant.wire_value)
                            .ok_or_else(|| {
                                error(format!(
                                    "service codec: invalid C++ enumeration wire value {:?}",
                                    variant.wire_value
                                ))
                            })?;
                    let wire = string_literal_body(&variant.wire_value);
                    writeln!(
                        encode,
                        "    if (value == {cpp}::{constant}) {{\n        wire = std::string_view(\"{wire}\");\n        known = true;\n    }}"
                    )
                    .expect("infallible");
                    writeln!(
                        decode,
                        "    if (*text == \"{wire}\") {{\n        return {cpp}::{constant};\n    }}"
                    )
                    .expect("infallible");
                }
                writeln!(
                    output,
                    "// {local} (enumeration: exact XSD wire values)\n\
                     inline bool encode_t{id:03}(const {cpp}& value, std::string& out, CodecError& error, const Path* path) {{\n    \
                     std::string_view wire;\n    bool known = false;\n{encode}    \
                     if (!known) {{\n        return fail_bool(error, path, \"value is not an enumerator of {local}\");\n    }}\n    \
                     return put_string(out, wire, error, path);\n}}\n\n\
                     inline std::optional<{cpp}> decode_t{id:03}(const Value& value, CodecError& error, const Path* path) {{\n    \
                     const std::string* text = get_text(value, error, path);\n    \
                     if (text == nullptr) {{\n        return std::nullopt;\n    }}\n{decode}    \
                     return fail(error, path, \"value is not a member of enumeration {local}\");\n}}\n"
                )
                .expect("infallible");
            }
            TypeKind::Primitive(
                kind @ (PrimitiveKind::String | PrimitiveKind::DateTime | PrimitiveKind::Duration),
            ) => {
                // A checked text carrier: UUID / schema-version / visible-ASCII /
                // whitespace-visible / NATO profile, DateTimeZulu or Duration.
                // The generated `create(std::string_view)` is the only authority.
                let what = match kind {
                    PrimitiveKind::String => "String profile",
                    PrimitiveKind::DateTime => "dateTime",
                    _ => "duration",
                };
                writeln!(
                    output,
                    "// {local} (checked {what}; the generated factory validates)\n\
                     inline bool encode_t{id:03}(const {cpp}& value, std::string& out, CodecError& error, const Path* path) {{\n    \
                     return put_string(out, value.value(), error, path);\n}}\n\n\
                     inline std::optional<{cpp}> decode_t{id:03}(const Value& value, CodecError& error, const Path* path) {{\n    \
                     return get_checked_text<{cpp}>(value, error, path, \"{local}\");\n}}\n"
                )
                .expect("infallible");
            }
            TypeKind::Primitive(PrimitiveKind::Binary) => {
                writeln!(
                    output,
                    "// {local} (named hexBinary: canonical uppercase, case-insensitive on input)\n\
                     inline bool encode_t{id:03}(const {cpp}& value, std::string& out, CodecError& error, const Path* path) {{\n    \
                     static_cast<void>(error);\n    static_cast<void>(path);\n    return put_hex_binary(out, value.value());\n}}\n\n\
                     inline std::optional<{cpp}> decode_t{id:03}(const Value& value, CodecError& error, const Path* path) {{\n    \
                     std::optional<std::vector<std::uint8_t>> octets = get_hex_binary(value, error, path);\n    \
                     if (!octets) {{\n        return std::nullopt;\n    }}\n    return {cpp}(std::move(*octets));\n}}\n"
                )
                .expect("infallible");
            }
            TypeKind::Choice { .. } => self.choice(output, id, declaration, &cpp)?,
            TypeKind::Record { .. } => self.record(output, id, declaration, &cpp)?,
            other => {
                return Err(error(format!(
                    "service codec boundary: {local}: {other:?} is not in the C++ codec subset"
                )));
            }
        }
        Ok(())
    }

    /// A Choice is an object with exactly one selected member, keyed by the
    /// alternative's own element QName (OMSC-SPC-013 Rev B section 6.1). The
    /// model's variant alternative is selected by its nested wrapper type.
    fn choice(
        &self,
        output: &mut String,
        id: usize,
        declaration: &TypeDecl,
        cpp: &str,
    ) -> Result<(), CodegenError> {
        let local = &declaration.name.local_name;
        let alternatives = effective_choice_alternatives(self.schema, &declaration.name)
            .map_err(|failure| error(format!("service codec: {local}: {failure:?}")))?;
        let mut encode = String::new();
        let mut decode = String::new();
        let mut allowed = Vec::new();
        for (index, alternative) in alternatives.into_iter().enumerate() {
            if alternative.cardinality.shape() != OccurrenceShape::RequiredOne {
                return Err(error(format!(
                    "service codec boundary: {local}.{} is not a required single Choice alternative",
                    alternative.name
                )));
            }
            let wrapper = format!("{cpp}::{}", choice_alternative_name(&alternative.name)?);
            let key = member_key(alternative)?;
            let key_literal = string_literal_body(&key);
            let raw_key = string_literal_body(&format!("{}:", json_quoted(&key)));
            allowed.push(format!("std::string_view(\"{key_literal}\")"));
            let base = self.base(alternative)?;
            let selected = format!("selected{index}");
            let expression = Self::encode(&base, &format!("{selected}->value"), "&p");
            let keyword = if index == 0 { "if" } else { "} else if" };
            // Scalar alternatives never name their path; declaring it would
            // trip -Wunused-variable under the strict consumer flags.
            let path_decl = if expression.contains("&p") {
                format!("        const Path p{{path, \"{key_literal}\", 0, false}};\n")
            } else {
                String::new()
            };
            writeln!(
                encode,
                "    {keyword} (const auto* {selected} = std::get_if<{wrapper}>(&value.value)) {{\n{path_decl}        \
                 put_raw(out, std::string_view(\"{raw_key}\"));\n        \
                 if (!{expression}) {{\n            return false;\n        }}"
            )
            .expect("infallible");
            let decoded = Self::decode(&base, "chosen->value", "&p");
            let single = Self::cpp(&base);
            writeln!(
                decode,
                "    if (chosen->key == \"{key_literal}\") {{\n        \
                 const Path p{{path, \"{key_literal}\", 0, false}};\n        \
                 std::optional<{single}> item = {decoded};\n        \
                 if (!item) {{\n            return std::nullopt;\n        }}\n        \
                 return {cpp}{{{wrapper}{{std::move(*item)}}}};\n    }}"
            )
            .expect("infallible");
        }
        let own_type = string_literal_body(&oms_json_type_name(
            &declaration.name.namespace_uri,
            &declaration.name.local_name,
        ));
        writeln!(
            output,
            "// {local} (Choice: exactly one selected member)\n\
             inline bool encode_t{id:03}(const {cpp}& value, std::string& out, CodecError& error, const Path* path) {{\n    \
             out += '{{';\n{encode}    }} else {{\n        \
             return fail_bool(error, path, \"Choice holds no alternative\");\n    }}\n    \
             out += '}}';\n    return true;\n}}\n\n\
             inline std::optional<{cpp}> decode_t{id:03}(const Value& value, CodecError& error, const Path* path) {{\n    \
             if (!expect_kind(value, Value::Kind::Object, error, path)) {{\n        return std::nullopt;\n    }}\n    \
             if (!check_members(value, error, path, \"{own_type}\", {{{}}})) {{\n        return std::nullopt;\n    }}\n    \
             const Member* chosen = find_choice_member(value, error, path);\n    \
             if (chosen == nullptr) {{\n        return std::nullopt;\n    }}\n{decode}    \
             return fail(error, path, \"unknown member\");\n}}\n",
            allowed.join(", "),
        )
        .expect("infallible");
        Ok(())
    }

    fn record(
        &self,
        output: &mut String,
        id: usize,
        declaration: &TypeDecl,
        cpp: &str,
    ) -> Result<(), CodegenError> {
        let local = &declaration.name.local_name;
        let fields = effective_record_fields(self.schema, &declaration.name)
            .map_err(|failure| error(format!("service codec: {local}: {failure:?}")))?;
        let mut encode = String::new();
        let mut decode = String::new();
        let mut allowed = Vec::new();
        let mut initializers = Vec::new();
        for (index, field) in fields.into_iter().enumerate() {
            let member = record_field_name(&field.name)?;
            let key = member_key(field)?;
            let key_literal = string_literal_body(&key);
            let raw_key = string_literal_body(&format!("{}:", json_quoted(&key)));
            allowed.push(format!("std::string_view(\"{key_literal}\")"));
            let base = self.base(field)?;
            let shape = field.cardinality.shape();
            // ---- encode ----
            let single_encode = |access: &str, path: &str| Self::encode(&base, access, path);
            match shape {
                OccurrenceShape::RequiredOne | OccurrenceShape::OptionalOne => {
                    let optional = shape == OccurrenceShape::OptionalOne;
                    let access = if optional {
                        format!("(*value.{member})")
                    } else {
                        format!("value.{member}")
                    };
                    let expression = single_encode(&access, "&p");
                    let path_decl = if expression.contains("&p") {
                        format!("        const Path p{{path, \"{key_literal}\", 0, false}};\n")
                    } else {
                        String::new()
                    };
                    let body = format!(
                        "{path_decl}        put_separator(out, first);\n        put_raw(out, std::string_view(\"{raw_key}\"));\n        if (!{expression}) {{\n            return false;\n        }}\n"
                    );
                    if optional {
                        writeln!(encode, "    if (value.{member}) {{\n{body}    }}")
                    } else {
                        writeln!(encode, "    {{\n{body}    }}")
                    }
                    .expect("infallible");
                }
                _ => {
                    let items = format!("value.{member}.values()");
                    let expression = single_encode("items[i]", "&pi");
                    let needs_item_path = expression.contains("&pi");
                    let p_decl = if needs_item_path {
                        format!("        const Path p{{path, \"{key_literal}\", 0, false}};\n")
                    } else {
                        String::new()
                    };
                    let pi_decl = if needs_item_path {
                        "            const Path pi{&p, std::string_view(), i, true};\n"
                    } else {
                        ""
                    };
                    writeln!(
                        encode,
                        "    {{\n        const auto& items = {items};\n        if (!items.empty()) {{\n{p_decl}            put_separator(out, first);\n            put_raw(out, std::string_view(\"{raw_key}\"));\n            out += '[';\n            for (std::size_t i = 0; i < items.size(); ++i) {{\n                if (i != 0) {{\n                    out += ',';\n                }}\n{pi_decl}                if (!{expression}) {{\n                    return false;\n                }}\n            }}\n            out += ']';\n        }}\n    }}"
                    )
                    .expect("infallible");
                }
            }
            // ---- decode ----
            let local_name = format!("f{index}");
            let single = Self::cpp(&base);
            match shape {
                OccurrenceShape::RequiredOne => {
                    let expression = Self::decode(&base, "*m", "&p");
                    writeln!(
                        decode,
                        "    std::optional<{single}> {local_name};\n    {{\n        const Value* m = find_member(value, \"{key_literal}\");\n        if (m == nullptr) {{\n            return fail(error, path, \"missing required member \\\"{key_literal}\\\"\");\n        }}\n        const Path p{{path, \"{key_literal}\", 0, false}};\n        {local_name} = {expression};\n        if (!{local_name}) {{\n            return std::nullopt;\n        }}\n    }}"
                    )
                    .expect("infallible");
                    initializers.push(format!("std::move(*{local_name})"));
                }
                OccurrenceShape::OptionalOne => {
                    let expression = Self::decode(&base, "*m", "&p");
                    writeln!(
                        decode,
                        "    std::optional<{single}> {local_name};\n    {{\n        const Value* m = find_member(value, \"{key_literal}\");\n        if (m != nullptr) {{\n            const Path p{{path, \"{key_literal}\", 0, false}};\n            {local_name} = {expression};\n            if (!{local_name}) {{\n                return std::nullopt;\n            }}\n        }}\n    }}"
                    )
                    .expect("infallible");
                    initializers.push(format!("std::move({local_name})"));
                }
                _ => {
                    let container = match shape {
                        OccurrenceShape::Bounded { min, max } => {
                            format!("M::BoundedVector<{single}, {min}, {max}>")
                        }
                        OccurrenceShape::Unbounded { min } => format!(
                            "M::UnboundedVector<{single}, {}>",
                            crate::cpp_unsigned_bound(min)
                        ),
                        _ => {
                            return Err(error(format!(
                                "service codec boundary: {local}.{} has an unsupported occurrence shape",
                                field.name
                            )));
                        }
                    };
                    let expression = Self::decode(&base, "m->array[i]", "&pi");
                    writeln!(
                        decode,
                        "    std::optional<{container}> {local_name};\n    {{\n        const Value* m = find_member(value, \"{key_literal}\");\n        const Path p{{path, \"{key_literal}\", 0, false}};\n        std::vector<{single}> items;\n        if (m != nullptr) {{\n            if (!expect_kind(*m, Value::Kind::Array, error, &p)) {{\n                return std::nullopt;\n            }}\n            items.reserve(m->array.size());\n            for (std::size_t i = 0; i < m->array.size(); ++i) {{\n                const Path pi{{&p, std::string_view(), i, true}};\n                std::optional<{single}> item = {expression};\n                if (!item) {{\n                    return std::nullopt;\n                }}\n                items.push_back(std::move(*item));\n            }}\n        }}\n        const std::size_t count = items.size();\n        {local_name} = {container}::create(std::move(items));\n        if (!{local_name}) {{\n            return fail_cardinality(error, &p, count);\n        }}\n    }}"
                    )
                    .expect("infallible");
                    initializers.push(format!("std::move(*{local_name})"));
                }
            }
        }
        let own_type = string_literal_body(&oms_json_type_name(
            &declaration.name.namespace_uri,
            &declaration.name.local_name,
        ));
        writeln!(
            output,
            "// {local} (concrete Record; effective inherited members, no nested base)\n\
             inline bool encode_t{id:03}(const {cpp}& value, std::string& out, CodecError& error, const Path* path) {{\n    \
             static_cast<void>(value);\n    static_cast<void>(error);\n    static_cast<void>(path);\n    \
             bool first = true;\n    out += '{{';\n{encode}    \
             static_cast<void>(first);\n    out += '}}';\n    return true;\n}}\n\n\
             inline std::optional<{cpp}> decode_t{id:03}(const Value& value, CodecError& error, const Path* path) {{\n    \
             if (!expect_kind(value, Value::Kind::Object, error, path)) {{\n        return std::nullopt;\n    }}\n    \
             if (!check_members(value, error, path, \"{own_type}\", {{{}}})) {{\n        return std::nullopt;\n    }}\n{decode}    \
             return {cpp}{{{}}};\n}}\n",
            allowed.join(", "),
            initializers.join(", "),
        )
        .expect("infallible");
        Ok(())
    }
}
