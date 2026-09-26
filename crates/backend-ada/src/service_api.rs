//! Task 047: the typed Ada service API wrapper.
//!
//! Renders an already-lowered, language-neutral [`ServiceApiModel`] into Ada
//! syntax. Nothing here interprets a Service Contract: kind, direction,
//! mandate, topic, message identity, and order all arrive decided.

use crate::{ada_type, error, package_name};
use ams_gra_oms_codegen_core::{
    BackendLanguage, CodegenError, ServiceApiModel, service_api_exchange_scope_name,
    service_api_fixed_names, service_api_function_scope_name, validate_service_api_names,
};
use ams_gra_oms_ir::SchemaIr;
use std::fmt::Write as _;

/// The stable wrapper entrypoint file name (GNAT's default naming for the
/// `Service_API` package specification).
pub const SERVICE_API_FILE: &str = "service_api.ads";

const LANGUAGE: BackendLanguage = BackendLanguage::Ada;

/// Render the typed service API wrapper as one standalone package spec.
///
/// When the service selects a UCI model the spec `with`s the generated model
/// package (named by this backend's own `package_name`), and each OMS
/// exchange declares `subtype Payload is <Package>.<Type>;`. Every other
/// declaration is a `constant String`, so no package body is required and
/// none is emitted.
///
/// Metadata only: no subprogram, task, publisher, subscriber, or runtime
/// dependency.
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
        "--  Generated typed service API descriptors (Task 047).\n",
        "--\n",
        "--  Compile-time endpoint metadata only. This package sends, receives,\n",
        "--  encodes, decodes, subscribes, publishes, dispatches, and connects to\n",
        "--  nothing, and it requires no package body.\n\n",
    ));
    let model_package = if model.emits_type_model() {
        let package = package_name(schema)?;
        writeln!(output, "with {package};\n").expect("writing to String cannot fail");
        Some(package)
    } else {
        None
    };
    writeln!(output, "package {} is\n", fixed.root).expect("writing to String cannot fail");
    constants(
        &mut output,
        1,
        &[
            (fixed.service_name, model.service_name()),
            (fixed.service_version, model.service_version()),
            (fixed.service_kind, model.service_kind().as_str()),
        ],
    );

    for function in model.functions() {
        let scope = service_api_function_scope_name(LANGUAGE, function.id())
            .map_err(|name| error(name.to_string()))?;
        writeln!(output, "\n   package {scope} is\n").expect("writing to String cannot fail");
        constants(
            &mut output,
            2,
            &[(fixed.id, function.id()), (fixed.name, function.name())],
        );

        for exchange in function.exchanges() {
            let exchange_scope =
                service_api_exchange_scope_name(LANGUAGE, function.id(), exchange.id())
                    .map_err(|name| error(name.to_string()))?;
            writeln!(output, "\n      package {exchange_scope} is\n")
                .expect("writing to String cannot fail");
            let mut values = vec![
                (fixed.id, exchange.id()),
                (fixed.kind, exchange.kind().as_str()),
                (fixed.direction, exchange.direction().as_str()),
                (fixed.mandate, exchange.mandate().as_str()),
            ];
            if let Some(binding) = exchange.oms_binding() {
                values.push((fixed.topic, binding.topic()));
            }
            constants(&mut output, 3, &values);
            if let Some(binding) = exchange.oms_binding() {
                let package = model_package.as_deref().ok_or_else(|| {
                    error("Ada service API has an OMS payload but no generated model")
                })?;
                writeln!(
                    output,
                    "\n         subtype {} is {package}.{};",
                    fixed.payload,
                    ada_type(binding.payload_type())?
                )
                .expect("writing to String cannot fail");
            }
            writeln!(output, "\n      end {exchange_scope};")
                .expect("writing to String cannot fail");
        }
        writeln!(output, "\n   end {scope};").expect("writing to String cannot fail");
    }
    writeln!(output, "\nend {};", fixed.root).expect("writing to String cannot fail");
    Ok(output)
}

/// Emit a block of `Name : constant String := ...;` declarations with their
/// colons aligned, at `depth` levels of three-space Ada indentation.
fn constants(output: &mut String, depth: usize, values: &[(&str, &str)]) {
    let width = values.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
    let indent = "   ".repeat(depth);
    for (name, value) in values {
        writeln!(
            output,
            "{indent}{name:<width$} : constant String := {};",
            string_expression(value)
        )
        .expect("writing to String cannot fail");
    }
}

/// An Ada `String` expression whose value is exactly the UTF-8 bytes of
/// `text`.
///
/// Printable ASCII goes into ordinary literals (with `"` doubled). Ada string
/// literals cannot hold control characters, so every byte outside printable
/// ASCII -- controls and each byte of a non-ASCII character -- is spliced in
/// as a `(1 => Character'Val (N))` aggregate, keeping the source pure ASCII
/// and the value identical to what the Rust and C++ wrappers carry.
fn string_expression(text: &str) -> String {
    let mut parts = Vec::new();
    let mut literal = String::new();
    for byte in text.bytes() {
        match byte {
            b'"' => literal.push_str("\"\""),
            b' '..=b'~' => literal.push(char::from(byte)),
            other => {
                if !literal.is_empty() {
                    parts.push(format!("\"{literal}\""));
                    literal.clear();
                }
                parts.push(format!("(1 => Character'Val ({other}))"));
            }
        }
    }
    if !literal.is_empty() || parts.is_empty() {
        parts.push(format!("\"{literal}\""));
    }
    parts.join(" & ")
}

#[cfg(test)]
mod tests {
    use super::string_expression;

    #[test]
    fn string_expressions_carry_the_exact_bytes() {
        assert_eq!(string_expression("plain"), "\"plain\"");
        assert_eq!(string_expression(""), "\"\"");
        assert_eq!(string_expression("a\"b"), "\"a\"\"b\"");
        assert_eq!(
            string_expression("a\tb"),
            "\"a\" & (1 => Character'Val (9)) & \"b\""
        );
        assert_eq!(
            string_expression("\u{e9}"),
            "(1 => Character'Val (195)) & (1 => Character'Val (169))"
        );
    }
}
