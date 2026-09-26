//! Task 047: the typed C++17 service API wrapper.
//!
//! Renders an already-lowered, language-neutral [`ServiceApiModel`] into C++
//! syntax. Nothing here interprets a Service Contract: kind, direction,
//! mandate, topic, message identity, and order all arrive decided.

use crate::{cpp_type, error, model_header_name, namespace_name};
use ams_gra_oms_codegen_core::{
    BackendLanguage, CodegenError, ServiceApiModel, service_api_exchange_scope_name,
    service_api_fixed_names, service_api_function_scope_name, validate_service_api_names,
};
use ams_gra_oms_ir::SchemaIr;
use std::fmt::Write as _;

/// The stable wrapper entrypoint file name.
pub const SERVICE_API_FILE: &str = "service_api.hpp";

const LANGUAGE: BackendLanguage = BackendLanguage::Cpp;

/// Render the typed service API wrapper header.
///
/// When the service selects a UCI model the header includes the generated
/// model header, and each OMS exchange's `Payload` names the payload type
/// through the model's fully qualified namespace (`::outer::inner::Type`),
/// derived with this backend's own `namespace_name`. Constants are
/// `inline constexpr std::string_view`, so the header is ODR-safe across
/// translation units and needs no `.cpp`.
///
/// Metadata only: no class, function, publisher, subscriber, or runtime code.
///
/// # Errors
///
/// Returns an error when a wrapper name fails the shared naming preflight or
/// the model cannot be spelled.
pub fn generate_service_api(
    model: &ServiceApiModel,
    schema: &SchemaIr,
) -> Result<String, CodegenError> {
    validate_service_api_names(model, LANGUAGE).map_err(|name| error(name.to_string()))?;
    let fixed = service_api_fixed_names(LANGUAGE);

    let mut output = String::from(concat!(
        "// Generated typed service API descriptors (Task 047).\n",
        "//\n",
        "// Compile-time endpoint metadata only. This header sends, receives,\n",
        "// encodes, decodes, subscribes, publishes, dispatches, and connects to\n",
        "// nothing.\n\n",
        "#pragma once\n\n",
    ));
    let model_namespace = if model.emits_type_model() {
        writeln!(output, "#include \"{}\"", model_header_name(schema)?)
            .expect("writing to String cannot fail");
        Some(namespace_name(schema)?)
    } else {
        None
    };
    writeln!(
        output,
        "#include <string_view>\n\nnamespace {} {{\n",
        fixed.root
    )
    .expect("writing to String cannot fail");
    constant(&mut output, fixed.service_name, model.service_name());
    constant(&mut output, fixed.service_version, model.service_version());
    constant(
        &mut output,
        fixed.service_kind,
        model.service_kind().as_str(),
    );

    for function in model.functions() {
        let scope = service_api_function_scope_name(LANGUAGE, function.id())
            .map_err(|name| error(name.to_string()))?;
        writeln!(output, "\nnamespace {scope} {{\n").expect("writing to String cannot fail");
        constant(&mut output, fixed.id, function.id());
        constant(&mut output, fixed.name, function.name());

        for exchange in function.exchanges() {
            let exchange_scope =
                service_api_exchange_scope_name(LANGUAGE, function.id(), exchange.id())
                    .map_err(|name| error(name.to_string()))?;
            writeln!(output, "\nnamespace {exchange_scope} {{\n")
                .expect("writing to String cannot fail");
            constant(&mut output, fixed.id, exchange.id());
            constant(&mut output, fixed.kind, exchange.kind().as_str());
            constant(&mut output, fixed.direction, exchange.direction().as_str());
            constant(&mut output, fixed.mandate, exchange.mandate().as_str());
            if let Some(binding) = exchange.oms_binding() {
                constant(&mut output, fixed.topic, binding.topic());
                let namespace = model_namespace.as_deref().ok_or_else(|| {
                    error("C++ service API has an OMS payload but no generated model")
                })?;
                writeln!(
                    output,
                    "\nusing {} = ::{namespace}::{};",
                    fixed.payload,
                    cpp_type(binding.payload_type())?
                )
                .expect("writing to String cannot fail");
            }
            writeln!(output, "\n}}  // namespace {exchange_scope}")
                .expect("writing to String cannot fail");
        }
        writeln!(output, "\n}}  // namespace {scope}").expect("writing to String cannot fail");
    }
    writeln!(output, "\n}}  // namespace {}", fixed.root).expect("writing to String cannot fail");
    Ok(output)
}

fn constant(output: &mut String, name: &str, value: &str) {
    writeln!(
        output,
        "inline constexpr std::string_view {name} = \"{}\";",
        string_literal_body(value)
    )
    .expect("writing to String cannot fail");
}

/// The body of a C++ narrow string literal holding the UTF-8 bytes of `text`.
///
/// Printable ASCII is written literally (with `\`, `"`, and `?` escaped; the
/// last so no trigraph can form under older compilers). Every other byte
/// becomes a three-digit octal escape, which can never absorb a following
/// character the way a hexadecimal escape would.
fn string_literal_body(text: &str) -> String {
    let mut body = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'\\' => body.push_str("\\\\"),
            b'"' => body.push_str("\\\""),
            b'?' => body.push_str("\\?"),
            b' '..=b'~' => body.push(char::from(byte)),
            other => write!(body, "\\{other:03o}").expect("writing to String cannot fail"),
        }
    }
    body
}

#[cfg(test)]
mod tests {
    use super::string_literal_body;

    #[test]
    fn string_literals_escape_everything_non_printable() {
        assert_eq!(string_literal_body("plain"), "plain");
        assert_eq!(string_literal_body(r#"a"b\c?"#), r#"a\"b\\c\?"#);
        assert_eq!(string_literal_body("tab\there"), "tab\\011here");
        assert_eq!(string_literal_body("caf\u{e9}1"), "caf\\303\\2511");
    }
}
