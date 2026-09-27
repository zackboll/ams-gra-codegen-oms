//! Tasks 047-048: the typed Rust service API wrapper and its
//! publish/subscribe facade.
//!
//! Renders an already-lowered, language-neutral [`ServiceApiModel`] into Rust
//! syntax. Nothing here interprets a Service Contract: kind, direction,
//! mandate, topic, message identity, order, and (Task 048) the one facade
//! operation per OMS exchange all arrive decided.

use crate::{error, rust_type};
use ams_gra_oms_codegen_core::{
    BackendLanguage, CodegenError, RUST_SERVICE_CODEC_FILE, RUST_SERVICE_CODEC_MODULE,
    ServiceApiFacadeNames, ServiceApiModel, ServiceApiOmsBinding, ServiceApiOmsOperation,
    rust_model_file_name, service_api_exchange_scope_name, service_api_fixed_names,
    service_api_function_scope_name, validate_service_api_artifacts, validate_service_api_names,
};
use ams_gra_oms_ir::SchemaIr;
use std::fmt::Write as _;

/// The stable wrapper entrypoint file name.
pub const SERVICE_API_FILE: &str = service_api_fixed_names(LANGUAGE).file;

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
/// Task 048: when the service has any OMS exchange, the root also exposes
/// the generic `PublishAdapter` / `SubscribeAdapter` contracts (Task 049:
/// re-exported from the stable `ams-gra-oms-runtime-api` crate rather than
/// declared per wrapper, see [`RUNTIME_API_CRATE`]), and each OMS
/// exchange gains exactly one forwarding operation decided in codegen-core:
/// `publish` for an output, `subscribe` for an input. The operation supplies
/// the endpoint's routing metadata to a caller-supplied adapter; nothing
/// here connects, encodes, or dispatches.
///
/// # Errors
///
/// Returns an error when a wrapper name fails the shared naming preflight or
/// the model cannot be spelled.
pub fn generate_service_api(
    model: &ServiceApiModel,
    schema: &SchemaIr,
) -> Result<String, CodegenError> {
    render_service_api(model, schema, false)
}

/// Task 050: the same wrapper, additionally mounting the generated
/// `service_codec.rs` as the sibling module `service_codec` when the service
/// has OMS exchanges. Without OMS exchanges no codec exists and the output is
/// exactly [`generate_service_api`]'s.
///
/// # Errors
///
/// Exactly as [`generate_service_api`].
pub fn generate_service_api_with_codec(
    model: &ServiceApiModel,
    schema: &SchemaIr,
) -> Result<String, CodegenError> {
    render_service_api(model, schema, model.has_oms_exchanges())
}

fn render_service_api(
    model: &ServiceApiModel,
    schema: &SchemaIr,
    mount_codec: bool,
) -> Result<String, CodegenError> {
    // Re-run the shared preflight: a caller that skipped readiness must
    // still fail closed instead of emitting source that cannot compile.
    validate_service_api_names(model, LANGUAGE).map_err(|name| error(name.to_string()))?;
    // Task 047 corrective: the wrapper must also be emittable BESIDE the
    // model -- no shared output path, no model/wrapper name conflict -- so a
    // direct caller that skipped readiness cannot get source known to
    // conflict with the model it references.
    validate_service_api_artifacts(model, schema, LANGUAGE)
        .map_err(|artifact| error(artifact.to_string()))?;
    let fixed = service_api_fixed_names(LANGUAGE);
    let model_module = fixed
        .model_module
        .ok_or_else(|| error("Rust service API requires a model module name"))?;

    let facade = model.has_oms_exchanges();
    let mut output = String::from(if facade {
        concat!(
            "// Generated typed service API (Tasks 047-049).\n",
            "//\n",
            "// Endpoint descriptors plus a typed publish/subscribe facade that\n",
            "// forwards each operation to a caller-supplied adapter. This file itself\n",
            "// sends, receives, encodes, decodes, dispatches, and connects to nothing.\n",
            "// Compile it as the crate root of the generated service, with the\n",
            "// `ams-gra-oms-runtime-api` crate available (it owns the adapter traits).\n\n",
        )
    } else {
        concat!(
            "// Generated typed service API descriptors (Task 047).\n",
            "//\n",
            "// Compile-time endpoint metadata only. This file sends, receives,\n",
            "// encodes, decodes, subscribes, publishes, dispatches, and connects to\n",
            "// nothing. Compile it as the crate root of the generated service.\n\n",
        )
    });
    if model.emits_type_model() {
        writeln!(
            output,
            "#[path = \"{}\"]\npub mod {model_module};\n",
            rust_model_file_name(schema)?
        )
        .expect("writing to String cannot fail");
    }
    if mount_codec {
        writeln!(
            output,
            "#[path = \"{RUST_SERVICE_CODEC_FILE}\"]\npub mod {RUST_SERVICE_CODEC_MODULE};\n"
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
    if facade {
        adapter_contracts(&mut output, &fixed.facade)?;
    }

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
                operation(
                    &mut output,
                    &fixed.facade,
                    fixed.topic,
                    fixed.payload,
                    binding,
                )?;
            }
            output.push_str("        }\n");
        }
        output.push_str("    }\n");
    }
    output.push_str("}\n");
    Ok(output)
}

/// The Rust crate name (Cargo's `-`→`_` mapping of the package
/// `ams-gra-oms-runtime-api`) that owns the stable adapter contracts.
///
/// A generated Rust wrapper with at least one OMS exchange must be compiled
/// with this crate available (`--extern ams_gra_oms_runtime_api=...`, or a
/// Cargo dependency on `ams-gra-oms-runtime-api`). Zero-OMS wrappers never
/// reference it.
pub const RUNTIME_API_CRATE: &str = "ams_gra_oms_runtime_api";

/// The two generic adapter contracts, exposed once in the wrapper root.
///
/// Task 049: the traits are no longer declared per wrapper. They are
/// re-exported from the stable runtime API crate ([`RUNTIME_API_CRATE`]), so
/// one runtime implementation serves every generated service while the
/// application-facing path `service_api::{PublishAdapter, SubscribeAdapter}`
/// is unchanged. The trait definitions themselves (moved verbatim) still
/// return the adapter's own associated `Output` and require no `Send`,
/// `Sync`, `'static`, or executor; `SubscribeAdapter` still takes the handler
/// type as a trait parameter so a runtime may add bounds without the
/// generated service naming them.
///
/// The path is absolute (`::crate_name`) so no generated module can shadow it.
fn adapter_contracts(
    output: &mut String,
    facade: &ServiceApiFacadeNames,
) -> Result<(), CodegenError> {
    let RustFacade {
        publish_adapter,
        subscribe_adapter,
        ..
    } = RustFacade::new(facade)?;
    writeln!(
        output,
        "\n    /// The stable runtime adapter contracts behind every generated\n\
         \x20   /// `publish` / `subscribe` operation, re-exported from the\n\
         \x20   /// `ams-gra-oms-runtime-api` crate so any runtime implementing them\n\
         \x20   /// serves this service. Compile this file with that crate available.\n\
         \x20   pub use ::{RUNTIME_API_CRATE}::{{{publish_adapter}, {subscribe_adapter}}};"
    )
    .expect("writing to String cannot fail");
    Ok(())
}

/// The Rust-only façade spellings, unwrapped once. Every one is required
/// for Rust; a missing one is a defect in the shared fixed-name table.
///
/// Task 049: the trait-profile parameter spellings (`P`, the hook
/// parameters) are no longer rendered here -- the trait definitions live in
/// `ams-gra-oms-runtime-api` -- so only the names this file still writes are
/// unwrapped.
struct RustFacade {
    publish_adapter: &'static str,
    subscribe_adapter: &'static str,
    adapter: &'static str,
    adapter_type: &'static str,
    handler_type: &'static str,
}

impl RustFacade {
    fn new(facade: &ServiceApiFacadeNames) -> Result<Self, CodegenError> {
        let required = |name: Option<&'static str>, what: &str| {
            name.ok_or_else(|| error(format!("Rust service API requires a {what} name")))
        };
        let parameters = &facade.parameters;
        Ok(Self {
            publish_adapter: required(facade.publish_adapter, "publish adapter")?,
            subscribe_adapter: required(facade.subscribe_adapter, "subscribe adapter")?,
            adapter: required(parameters.adapter, "adapter parameter")?,
            adapter_type: required(parameters.adapter_type, "adapter type parameter")?,
            handler_type: required(parameters.handler_type, "handler type parameter")?,
        })
    }
}

/// The one forwarding operation of an OMS exchange, after its `Payload`.
///
/// Exactly one of `publish` / `subscribe` is emitted, as decided by
/// [`ServiceApiOmsBinding::operation`]. Neither takes any routing string
/// from the application; both name the payload only through the local
/// `Payload` alias.
fn operation(
    output: &mut String,
    facade: &ServiceApiFacadeNames,
    topic: &str,
    payload: &str,
    binding: &ServiceApiOmsBinding,
) -> Result<(), CodegenError> {
    let RustFacade {
        publish_adapter,
        subscribe_adapter,
        adapter,
        adapter_type: a,
        handler_type: h,
        ..
    } = RustFacade::new(facade)?;
    let (value, handler) = (facade.parameters.value, facade.parameters.handler);
    // exchange -> function -> service_api root.
    let root = "super::super";
    // Every OMS endpoint carries its resolved global message identity, taken
    // verbatim from the binding (never from the payload, topic, or type name).
    let message = binding.message_name();
    output.push('\n');
    constant(output, 3, facade.message_namespace, &message.namespace_uri);
    constant(output, 3, facade.message_name, &message.local_name);
    match binding.operation() {
        ServiceApiOmsOperation::Publish => {
            let publish = facade.publish;
            writeln!(
                output,
                "\n            /// Publish one `{payload}` on this endpoint's topic through `{adapter}`,\n\
                 \x20           /// supplying the resolved message identity and topic.\n\
                 \x20           pub fn {publish}<{a}>({adapter}: &mut {a}, {value}: &{payload}) -> {a}::Output\n\
                 \x20           where\n\
                 \x20               {a}: {root}::{publish_adapter}<{payload}> + ?Sized,\n\
                 \x20           {{\n\
                 \x20               {adapter}.{publish}({}, {}, {topic}, {value})\n\
                 \x20           }}",
                facade.message_namespace, facade.message_name
            )
            .expect("writing to String cannot fail");
        }
        ServiceApiOmsOperation::Subscribe => {
            let group = binding.subscription_group().map_or_else(
                || "None".to_owned(),
                |group| format!("Some(\"{}\")", string_literal_body(group)),
            );
            writeln!(
                output,
                "            pub const {}: Option<&str> = {group};",
                facade.subscription_group
            )
            .expect("writing to String cannot fail");
            let subscribe = facade.subscribe;
            writeln!(
                output,
                "\n            /// Subscribe `{handler}` to this endpoint's `{payload}` messages through\n\
                 \x20           /// `{adapter}`, supplying the resolved message identity, topic, and group.\n\
                 \x20           pub fn {subscribe}<{a}, {h}>({adapter}: &mut {a}, {handler}: {h}) -> {a}::Output\n\
                 \x20           where\n\
                 \x20               {a}: {root}::{subscribe_adapter}<{payload}, {h}> + ?Sized,\n\
                 \x20               {h}: FnMut(&{payload}),\n\
                 \x20           {{\n\
                 \x20               {adapter}.{subscribe}(\n\
                 \x20                   {},\n\
                 \x20                   {},\n\
                 \x20                   {topic},\n\
                 \x20                   {},\n\
                 \x20                   {handler},\n\
                 \x20               )\n\
                 \x20           }}",
                facade.message_namespace, facade.message_name, facade.subscription_group
            )
            .expect("writing to String cannot fail");
        }
    }
    Ok(())
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
