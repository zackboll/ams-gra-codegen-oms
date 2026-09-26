//! Task 047: the typed Rust service API wrapper.
//!
//! Renders an already-lowered, language-neutral [`ServiceApiModel`] into Rust
//! syntax. Nothing here interprets a Service Contract: kind, direction,
//! mandate, topic, message identity, and order all arrive decided.

use crate::{error, model_file_name, rust_type};
use ams_gra_oms_codegen_core::{
    BackendLanguage, CodegenError, ServiceApiModel, service_api_exchange_scope_name,
    service_api_fixed_names, service_api_function_scope_name, validate_service_api_names,
};
use ams_gra_oms_ir::SchemaIr;
use std::fmt::Write as _;

/// The stable wrapper entrypoint file name.
pub const SERVICE_API_FILE: &str = "service_api.rs";

const LANGUAGE: BackendLanguage = BackendLanguage::Rust;

/// Render the typed service API wrapper.
///
/// The file is a self-contained crate root. When the service selects a UCI
/// model it mounts the generated model file as `model` with `#[path]`, then
/// declares `service_api` with one nested module per contract function and
/// per exchange occurrence, in contract order. Each OMS exchange binds
/// `Payload` to the generated payload type, spelled with this backend's own
/// type-name rule. Payload paths are `super`-relative, not `crate::`, so the
/// tree resolves identically wherever the file is mounted.
///
/// Metadata only: no trait, function, publisher, subscriber, or runtime code.
///
/// # Errors
///
/// Returns an error when a wrapper name fails the shared naming preflight or
/// the model cannot be spelled.
pub fn generate_service_api(
    model: &ServiceApiModel,
    schema: &SchemaIr,
) -> Result<String, CodegenError> {
    // Re-run the shared preflight: a caller that skipped readiness must
    // still fail closed instead of emitting source that cannot compile.
    validate_service_api_names(model, LANGUAGE).map_err(|name| error(name.to_string()))?;
    let fixed = service_api_fixed_names(LANGUAGE);
    let model_module = fixed
        .model_module
        .ok_or_else(|| error("Rust service API requires a model module name"))?;

    let mut output = String::from(concat!(
        "// Generated typed service API descriptors (Task 047).\n",
        "//\n",
        "// Compile-time endpoint metadata only. This file sends, receives,\n",
        "// encodes, decodes, subscribes, publishes, dispatches, and connects to\n",
        "// nothing. Compile it as the crate root of the generated service.\n\n",
    ));
    if model.emits_type_model() {
        writeln!(
            output,
            "#[path = \"{}\"]\npub mod {model_module};\n",
            model_file_name(schema)?
        )
        .expect("writing to String cannot fail");
    }
    writeln!(output, "pub mod {} {{", fixed.root).expect("writing to String cannot fail");
    constant(&mut output, 1, fixed.service_name, model.service_name());
    constant(
        &mut output,
        1,
        fixed.service_version,
        model.service_version(),
    );
    constant(
        &mut output,
        1,
        fixed.service_kind,
        model.service_kind().as_str(),
    );

    for function in model.functions() {
        let scope = service_api_function_scope_name(LANGUAGE, function.id())
            .map_err(|name| error(name.to_string()))?;
        writeln!(output, "\n    pub mod {scope} {{").expect("writing to String cannot fail");
        constant(&mut output, 2, fixed.id, function.id());
        constant(&mut output, 2, fixed.name, function.name());

        for exchange in function.exchanges() {
            let scope = service_api_exchange_scope_name(LANGUAGE, function.id(), exchange.id())
                .map_err(|name| error(name.to_string()))?;
            writeln!(output, "\n        pub mod {scope} {{")
                .expect("writing to String cannot fail");
            constant(&mut output, 3, fixed.id, exchange.id());
            constant(&mut output, 3, fixed.kind, exchange.kind().as_str());
            constant(
                &mut output,
                3,
                fixed.direction,
                exchange.direction().as_str(),
            );
            constant(&mut output, 3, fixed.mandate, exchange.mandate().as_str());
            if let Some(binding) = exchange.oms_binding() {
                constant(&mut output, 3, fixed.topic, binding.topic());
                // exchange -> function -> service_api -> file root.
                writeln!(
                    output,
                    "\n            pub type {} = super::super::super::{model_module}::{};",
                    fixed.payload,
                    rust_type(binding.payload_type())?
                )
                .expect("writing to String cannot fail");
            }
            output.push_str("        }\n");
        }
        output.push_str("    }\n");
    }
    output.push_str("}\n");
    Ok(output)
}

fn constant(output: &mut String, depth: usize, name: &str, value: &str) {
    writeln!(
        output,
        "{}pub const {name}: &str = \"{}\";",
        "    ".repeat(depth),
        string_literal_body(value)
    )
    .expect("writing to String cannot fail");
}

/// The body of a Rust string literal holding `text` exactly.
///
/// Printable ASCII is written literally (with `\` and `"` escaped); anything
/// else, including controls and non-ASCII, becomes a `\u{..}` escape, so the
/// generated source is pure ASCII whatever display text the contract holds.
fn string_literal_body(text: &str) -> String {
    let mut body = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\\' => body.push_str("\\\\"),
            '"' => body.push_str("\\\""),
            ' '..='~' => body.push(character),
            other => write!(body, "\\u{{{:x}}}", u32::from(other))
                .expect("writing to String cannot fail"),
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
        assert_eq!(string_literal_body(r#"a"b\c"#), r#"a\"b\\c"#);
        assert_eq!(string_literal_body("tab\there"), "tab\\u{9}here");
        assert_eq!(string_literal_body("caf\u{e9}"), "caf\\u{e9}");
    }
}
