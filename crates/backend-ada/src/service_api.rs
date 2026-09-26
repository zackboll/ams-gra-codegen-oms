//! Tasks 047-048: the typed Ada service API wrapper and its
//! publish/subscribe facade.
//!
//! Renders an already-lowered, language-neutral [`ServiceApiModel`] into Ada
//! syntax. Nothing here interprets a Service Contract: kind, direction,
//! mandate, topic, message identity, order, and (Task 048) the one facade
//! operation per OMS exchange all arrive decided.

use crate::{ada_type, error, package_name};
use ams_gra_oms_codegen_core::{
    BackendLanguage, CodegenError, ServiceApiFixedNames, ServiceApiModel, ServiceApiOmsBinding,
    ServiceApiOmsOperation, service_api_exchange_scope_name, service_api_fixed_names,
    service_api_function_scope_name, validate_service_api_artifacts, validate_service_api_names,
};
use ams_gra_oms_ir::SchemaIr;
use std::fmt::Write as _;

/// The stable wrapper entrypoint file name (GNAT's default naming for the
/// `Service_API` package specification).
pub const SERVICE_API_FILE: &str = service_api_fixed_names(LANGUAGE).file;

const LANGUAGE: BackendLanguage = BackendLanguage::Ada;

/// Render the typed service API wrapper as one standalone package spec.
///
/// When the service selects a UCI model the spec `with`s the generated model
/// package (named by this backend's own `package_name`), and each OMS
/// exchange declares `subtype Payload is <Package>.<Type>;`. Every other
/// declaration is a `constant String`, so no package body is required and
/// none is emitted.
///
/// Task 048: each OMS exchange also declares exactly one generic façade
/// package, as decided in codegen-core: `Publisher` for an output and
/// `Subscriber` (plus its `Handler` interface) for an input. Every
/// operation is an expression function, so the spec still needs no body.
/// There is no task, protected object, access-type allocation, or runtime
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
    // Task 047 corrective: the wrapper must also be emittable BESIDE the
    // model -- no shared output path, no model/wrapper name conflict -- so a
    // direct caller that skipped readiness cannot get source known to
    // conflict with the model it references.
    validate_service_api_artifacts(model, schema, LANGUAGE)
        .map_err(|artifact| error(artifact.to_string()))?;
    let fixed = service_api_fixed_names(LANGUAGE);

    let mut output = String::from(if model.has_oms_exchanges() {
        concat!(
            "--  Generated typed service API (Tasks 047-048).\n",
            "--\n",
            "--  Endpoint descriptors plus a typed publish/subscribe facade whose\n",
            "--  generic packages forward each operation to a runtime hook supplied at\n",
            "--  instantiation. This package itself sends, receives, encodes, decodes,\n",
            "--  dispatches, and connects to nothing, and it requires no package body.\n\n",
        )
    } else {
        concat!(
            "--  Generated typed service API descriptors (Task 047).\n",
            "--\n",
            "--  Compile-time endpoint metadata only. This package sends, receives,\n",
            "--  encodes, decodes, subscribes, publishes, dispatches, and connects to\n",
            "--  nothing, and it requires no package body.\n\n",
        )
    });
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
                operation(&mut output, &fixed, binding)?;
            }
            writeln!(output, "\n      end {exchange_scope};")
                .expect("writing to String cannot fail");
        }
        writeln!(output, "\n   end {scope};").expect("writing to String cannot fail");
    }
    writeln!(output, "\nend {};", fixed.root).expect("writing to String cannot fail");
    Ok(output)
}

/// The one façade operation of an OMS exchange, after its `Payload`.
///
/// Rendered as a generic package the application instantiates with its
/// runtime hook: `Publisher` for an output, `Subscriber` for an input, as
/// decided by [`ServiceApiOmsBinding::operation`]. Each operation is an
/// expression function, so no package body is needed. The hook's result type
/// is a generic formal, so the runtime alone picks its status, error, or
/// subscription-token type (an Ada runtime may also raise exceptions). A
/// subscriber is any object whose type implements this endpoint's `Handler`
/// interface, passed by `not null access`: no heap allocation is required,
/// and a runtime that keeps the handler does so under Ada's accessibility
/// checks.
fn operation(
    output: &mut String,
    fixed: &ServiceApiFixedNames,
    binding: &ServiceApiOmsBinding,
) -> Result<(), CodegenError> {
    let facade = &fixed.facade;
    let parameters = &facade.parameters;
    let result = required(facade.adapter_result, "adapter result")?;
    let (topic, payload) = (fixed.topic, fixed.payload);
    let (value, receiver) = (parameters.value, parameters.handler);
    let hook_topic = required(parameters.hook_topic, "hook topic parameter")?;
    match binding.operation() {
        ServiceApiOmsOperation::Publish => {
            let publish_to = required(facade.publish_hook, "publish hook")?;
            let publish = required(facade.publish_operation, "publish operation")?;
            let publisher = facade.publish;
            writeln!(
                output,
                "\n         --  Bind this output endpoint to a runtime hook. {publish} supplies the\n\
                 \x20        --  topic; the runtime chooses {result}.\n\
                 \x20        generic\n\
                 \x20           type {result} (<>) is limited private;\n\
                 \x20           with function {publish_to}\n\
                 \x20             ({hook_topic} : String; {value} : {payload}) return {result};\n\
                 \x20        package {publisher} is\n\
                 \x20           function {publish} ({value} : {payload}) return {result} is\n\
                 \x20             ({publish_to} ({topic}, {value}));\n\
                 \x20        end {publisher};"
            )
            .expect("writing to String cannot fail");
        }
        ServiceApiOmsOperation::Subscribe => subscriber(output, fixed, binding, receiver)?,
    }
    Ok(())
}

fn required(name: Option<&'static str>, what: &str) -> Result<&'static str, CodegenError> {
    name.ok_or_else(|| error(format!("Ada service API requires a {what} name")))
}

/// The Subscribe half of [`operation`]: the routing constants, the
/// `Handler` interface, and the generic `Subscriber` package.
fn subscriber(
    output: &mut String,
    fixed: &ServiceApiFixedNames,
    binding: &ServiceApiOmsBinding,
    receiver: &str,
) -> Result<(), CodegenError> {
    let facade = &fixed.facade;
    let parameters = &facade.parameters;
    let (topic, payload) = (fixed.topic, fixed.payload);
    let result = required(facade.adapter_result, "adapter result")?;
    let handler = required(facade.handler, "handler interface")?;
    let handle = required(facade.handle, "handler primitive")?;
    let has_group = required(facade.has_subscription_group, "subscription group flag")?;
    let subscribe_to = required(facade.subscribe_hook, "subscribe hook")?;
    let subscribe = required(facade.subscribe_operation, "subscribe operation")?;
    let handle_self = required(parameters.handle_self, "handler self parameter")?;
    let handle_message = required(parameters.handle_message, "handler message parameter")?;
    let hooks = [
        (parameters.hook_message_namespace, "String"),
        (parameters.hook_message_name, "String"),
        (parameters.hook_topic, "String"),
        (parameters.hook_has_subscription_group, "Boolean"),
        (parameters.hook_subscription_group, "String"),
    ]
    .map(|(name, type_name)| required(name, "hook parameter").map(|name| (name, type_name)));
    let mut profile = Vec::with_capacity(hooks.len() + 1);
    for hook in hooks {
        profile.push(hook?);
    }
    let receiver_type = format!("not null access {handler}'Class");
    profile.push((receiver, receiver_type.as_str()));

    let message = binding.message_name();
    let group = binding.subscription_group();
    // Absence is carried as the flag, never as a fabricated group.
    let rows = [
        (
            facade.message_namespace,
            "String",
            string_expression(&message.namespace_uri),
        ),
        (
            facade.message_name,
            "String",
            string_expression(&message.local_name),
        ),
        (
            has_group,
            "Boolean",
            if group.is_some() { "True" } else { "False" }.to_owned(),
        ),
        (
            facade.subscription_group,
            "String",
            string_expression(group.unwrap_or_default()),
        ),
    ];
    let width = rows.iter().map(|(name, ..)| name.len()).max().unwrap_or(0);
    output.push('\n');
    for (name, type_name, value) in &rows {
        writeln!(
            output,
            "         {name:<width$} : constant {type_name} := {value};"
        )
        .expect("writing to String cannot fail");
    }

    let hook_width = profile
        .iter()
        .map(|(name, _)| name.len())
        .max()
        .unwrap_or(0);
    let hook_profile = profile
        .iter()
        .map(|(name, type_name)| format!("{name:<hook_width$} : {type_name}"))
        .collect::<Vec<_>>()
        .join(";\n               ");
    let subscriber = facade.subscribe;
    let (namespace, name, group_name) = (
        facade.message_namespace,
        facade.message_name,
        facade.subscription_group,
    );
    writeln!(
        output,
        "\n         --  A typed receiver of this endpoint's {payload} messages.\n\
         \x20        type {handler} is limited interface;\n\
         \x20        procedure {handle}\n\
         \x20          ({handle_self} : in out {handler}; {handle_message} : {payload}) is abstract;\n\
         \n\
         \x20        --  Bind this input endpoint to a runtime hook. {subscribe} supplies the\n\
         \x20        --  message identity, topic, and group; the runtime chooses {result}.\n\
         \x20        generic\n\
         \x20           type {result} (<>) is limited private;\n\
         \x20           with function {subscribe_to}\n\
         \x20             ({hook_profile})\n\
         \x20              return {result};\n\
         \x20        package {subscriber} is\n\
         \x20           function {subscribe}\n\
         \x20             ({receiver} : {receiver_type}) return {result} is\n\
         \x20             ({subscribe_to}\n\
         \x20                ({namespace}, {name}, {topic},\n\
         \x20                 {has_group}, {group_name}, {receiver}));\n\
         \x20        end {subscriber};"
    )
    .expect("writing to String cannot fail");
    Ok(())
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
