//! Tasks 047-048: the typed C++17 service API wrapper and its
//! publish/subscribe facade.
//!
//! Renders an already-lowered, language-neutral [`ServiceApiModel`] into C++
//! syntax. Nothing here interprets a Service Contract: kind, direction,
//! mandate, topic, message identity, order, and (Task 048) the one facade
//! operation per OMS exchange all arrive decided.

use crate::{cpp_type, error, namespace_name};
use ams_gra_oms_codegen_core::{
    BackendLanguage, CodegenError, ServiceApiFacadeNames, ServiceApiFixedNames, ServiceApiModel,
    ServiceApiOmsBinding, ServiceApiOmsOperation, cpp_model_header_name,
    service_api_exchange_scope_name, service_api_fixed_names, service_api_function_scope_name,
    validate_service_api_artifacts, validate_service_api_names,
};
use ams_gra_oms_ir::SchemaIr;
use std::fmt::Write as _;

/// The stable wrapper entrypoint file name.
pub const SERVICE_API_FILE: &str = service_api_fixed_names(LANGUAGE).file;

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
/// Task 048: each OMS exchange additionally declares exactly one function
/// template, decided in codegen-core: `publish` for an output, `subscribe`
/// for an input. It forwards to a caller-supplied adapter (duck-typed: no
/// virtual base, no `std::function`) and returns whatever the adapter
/// returns. No class hierarchy, connection, codec, or dispatcher is
/// generated.
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
    // Task 047 corrective: the wrapper must also be emittable BESIDE the
    // model -- no shared output path, no model/wrapper name conflict -- so a
    // direct caller that skipped readiness cannot get source known to
    // conflict with the model it references.
    validate_service_api_artifacts(model, schema, LANGUAGE)
        .map_err(|artifact| error(artifact.to_string()))?;
    let fixed = service_api_fixed_names(LANGUAGE);

    let facade = model.has_oms_exchanges();
    let mut output = String::from(if facade {
        concat!(
            "// Generated typed service API (Tasks 047-048).\n",
            "//\n",
            "// Endpoint descriptors plus a typed publish/subscribe facade that\n",
            "// forwards each operation to a caller-supplied adapter. This header\n",
            "// itself sends, receives, encodes, decodes, dispatches, and connects to\n",
            "// nothing.\n\n",
            "#pragma once\n\n",
        )
    } else {
        concat!(
            "// Generated typed service API descriptors (Task 047).\n",
            "//\n",
            "// Compile-time endpoint metadata only. This header sends, receives,\n",
            "// encodes, decodes, subscribes, publishes, dispatches, and connects to\n",
            "// nothing.\n\n",
            "#pragma once\n\n",
        )
    });
    let model_namespace = if model.emits_type_model() {
        writeln!(output, "#include \"{}\"", cpp_model_header_name(schema)?)
            .expect("writing to String cannot fail");
        Some(namespace_name(schema)?)
    } else {
        None
    };
    if facade {
        output.push_str("#include <optional>\n");
    }
    output.push_str("#include <string_view>\n");
    if facade {
        output.push_str("#include <type_traits>\n#include <utility>\n");
    }
    writeln!(output, "\nnamespace {} {{\n", fixed.root).expect("writing to String cannot fail");
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
                operation(&mut output, &fixed, binding)?;
            }
            writeln!(output, "\n}}  // namespace {exchange_scope}")
                .expect("writing to String cannot fail");
        }
        writeln!(output, "\n}}  // namespace {scope}").expect("writing to String cannot fail");
    }
    writeln!(output, "\n}}  // namespace {}", fixed.root).expect("writing to String cannot fail");
    Ok(output)
}

/// The one forwarding operation of an OMS exchange, after its `Payload`.
///
/// Both templates take the adapter by forwarding reference and return
/// `decltype(auto)`, so the adapter alone decides the result, error, and
/// subscription-token types, and whether it is a reference, a handle, or a
/// temporary. `subscribe` names the payload to the adapter explicitly as
/// `adapter.template subscribe<Payload>(...)` and statically requires the
/// handler to be invocable with `const Payload&`; the handler is perfectly
/// forwarded, never type-erased or copied by the wrapper.
fn operation(
    output: &mut String,
    fixed: &ServiceApiFixedNames,
    binding: &ServiceApiOmsBinding,
) -> Result<(), CodegenError> {
    let ServiceApiFacadeNames {
        publish,
        subscribe,
        message_namespace,
        message_name,
        subscription_group,
        parameters,
        ..
    } = fixed.facade;
    let required = |name: Option<&'static str>, what: &str| {
        name.ok_or_else(|| error(format!("C++ service API requires a {what} name")))
    };
    let adapter = required(parameters.adapter, "adapter parameter")?;
    let adapter_type = required(parameters.adapter_type, "adapter type parameter")?;
    let (value, handler) = (parameters.value, parameters.handler);
    let (topic, payload) = (fixed.topic, fixed.payload);
    match binding.operation() {
        ServiceApiOmsOperation::Publish => {
            writeln!(
                output,
                "\n// Publish one {payload} on this endpoint's topic through `{adapter}`.\n\
                 template <typename {adapter_type}>\n\
                 decltype(auto) {publish}({adapter_type}&& {adapter}, const {payload}& {value}) {{\n\
                 \x20   return std::forward<{adapter_type}>({adapter}).{publish}({topic}, {value});\n\
                 }}"
            )
            .expect("writing to String cannot fail");
        }
        ServiceApiOmsOperation::Subscribe => {
            let handler_type = required(parameters.handler_type, "handler type parameter")?;
            let message = binding.message_name();
            output.push('\n');
            constant(output, message_namespace, &message.namespace_uri);
            constant(output, message_name, &message.local_name);
            let group = binding.subscription_group().map_or_else(
                || "std::nullopt".to_owned(),
                |group| format!("std::string_view(\"{}\")", string_literal_body(group)),
            );
            writeln!(
                output,
                "inline constexpr std::optional<std::string_view> {subscription_group} = {group};"
            )
            .expect("writing to String cannot fail");
            writeln!(
                output,
                "\n// Subscribe `{handler}` to this endpoint's {payload} messages through\n\
                 // `{adapter}`, supplying the resolved message identity, topic, and group.\n\
                 template <typename {adapter_type}, typename {handler_type}>\n\
                 decltype(auto) {subscribe}({adapter_type}&& {adapter}, {handler_type}&& {handler}) {{\n\
                 \x20   static_assert(std::is_invocable_v<{handler_type}&, const {payload}&>,\n\
                 \x20                 \"subscriber handler must accept const {payload}&\");\n\
                 \x20   return std::forward<{adapter_type}>({adapter}).template {subscribe}<{payload}>(\n\
                 \x20       {message_namespace}, {message_name}, {topic}, {subscription_group},\n\
                 \x20       std::forward<{handler_type}>({handler}));\n\
                 }}"
            )
            .expect("writing to String cannot fail");
        }
    }
    Ok(())
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
