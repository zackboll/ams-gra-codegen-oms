//! Syntax rendering only: every endpoint, binding and operation is already
//! decided by ServiceApiModel. No contract, transport or codec interpretation.
use crate::{error, names};
use ams_gra_oms_codegen_core::{CodegenError, ServiceApiModel, ServiceApiOmsOperation};
use ams_gra_oms_ir::SchemaIr;
use std::collections::BTreeSet;
use std::fmt::Write as _;

pub const SERVICE_API_FILE: &str = "service_api.py";

fn scope(prefix: &str, id: &str) -> String {
    format!("{prefix}_{}", names::symbol(id, true))
}

fn insert(seen: &mut BTreeSet<String>, symbol: &str, original: &str) -> Result<(), CodegenError> {
    if !seen.insert(symbol.to_owned()) {
        return Err(error(format!(
            "Python service API name collision: {original:?} -> {symbol}"
        )));
    }
    Ok(())
}

fn validate(model: &ServiceApiModel, schema: &SchemaIr) -> Result<(), CodegenError> {
    if model.emits_type_model() {
        // Same model names and capability checks even for direct wrapper callers.
        let emissions = ams_gra_oms_codegen_core::plan_type_emissions(schema, model.world())?;
        crate::preflight(schema, &emissions)?;
    }
    let mut functions = BTreeSet::new();
    for function in model.functions() {
        insert(
            &mut functions,
            &scope("function", function.id()),
            function.id(),
        )?;
        let mut exchanges = BTreeSet::new();
        for exchange in function.exchanges() {
            insert(
                &mut exchanges,
                &scope("exchange", exchange.id()),
                exchange.id(),
            )?;
        }
    }
    // Fixed names are uppercase or private; generated scopes always start with
    // function_/exchange_, and model names are always qualified through _model.
    // Thus no fixed-name or model/wrapper scope hiding is possible.
    Ok(())
}

fn constant(output: &mut String, indent: usize, name: &str, value: &str) {
    writeln!(
        output,
        "{}{name} = {}",
        "    ".repeat(indent),
        names::literal(value)
    )
    .unwrap();
}

/// Generate an importable Python 3.11+ service API beside model.py.
/// Both package imports and standalone imports from the output directory work.
///
/// # Errors
/// Fails closed on model capability/naming errors or normalized scope collisions.
pub fn generate_service_api(
    model: &ServiceApiModel,
    schema: &SchemaIr,
) -> Result<String, CodegenError> {
    validate(model, schema)?;
    let mut output = String::from(
        "# Generated Python service API. No transport or serialization.\nfrom __future__ import annotations\n\n",
    );
    if model.emits_type_model() {
        output.push_str("if __package__:\n    from . import model as _model\nelse:\n    import model as _model\n\n");
    }
    if model.has_oms_exchanges() {
        output.push_str(include_str!("service_runtime.py"));
        output.push('\n');
    }
    constant(&mut output, 0, "SERVICE_NAME", model.service_name());
    constant(&mut output, 0, "SERVICE_VERSION", model.service_version());
    constant(
        &mut output,
        0,
        "SERVICE_KIND",
        model.service_kind().as_str(),
    );
    for function in model.functions() {
        writeln!(output, "\n\nclass {}:", scope("function", function.id())).unwrap();
        constant(&mut output, 1, "ID", function.id());
        constant(&mut output, 1, "NAME", function.name());
        for exchange in function.exchanges() {
            writeln!(output, "\n    class {}:", scope("exchange", exchange.id())).unwrap();
            constant(&mut output, 2, "ID", exchange.id());
            constant(&mut output, 2, "KIND", exchange.kind().as_str());
            constant(&mut output, 2, "DIRECTION", exchange.direction().as_str());
            constant(&mut output, 2, "MANDATE", exchange.mandate().as_str());
            if let Some(binding) = exchange.oms_binding() {
                constant(&mut output, 2, "TOPIC", binding.topic());
                constant(
                    &mut output,
                    2,
                    "MESSAGE_NAMESPACE",
                    &binding.message_name().namespace_uri,
                );
                constant(
                    &mut output,
                    2,
                    "MESSAGE_NAME",
                    &binding.message_name().local_name,
                );
                let group = binding
                    .subscription_group()
                    .map_or_else(|| "None".to_owned(), names::literal);
                let payload = format!(
                    "_model.{}",
                    names::symbol(&binding.payload_name().local_name, false)
                );
                writeln!(
                    output,
                    "        SUBSCRIPTION_GROUP = {group}\n        Payload = {payload}"
                )
                .unwrap();
                // An explicit return TypeVar preserves each adapter's result,
                // including a subscription handle, without creating one here.
                let route = format!(
                    "message_namespace={}, message_name={}, topic={}",
                    names::literal(&binding.message_name().namespace_uri),
                    names::literal(&binding.message_name().local_name),
                    names::literal(binding.topic())
                );
                match binding.operation() {
                    ServiceApiOmsOperation::Publish => {
                        writeln!(output, "\n        @staticmethod\n        def publish(adapter: PublishAdapter[{payload}, _Result], payload: {payload}) -> _Result:\n            return adapter.publish({route}, payload=payload)").unwrap();
                    }
                    ServiceApiOmsOperation::Subscribe => {
                        writeln!(output, "\n        @staticmethod\n        def subscribe(adapter: SubscribeAdapter[{payload}, _Result], handler: _Callable[[{payload}], None]) -> _Result:\n            return adapter.subscribe({route}, subscription_group={group}, handler=handler)").unwrap();
                    }
                }
            }
        }
        let exchanges = function
            .exchanges()
            .iter()
            .map(|exchange| scope("exchange", exchange.id()))
            .collect::<Vec<_>>();
        writeln!(output, "\n    EXCHANGES = ({})", tuple(&exchanges)).unwrap();
    }
    let functions = model
        .functions()
        .iter()
        .map(|function| scope("function", function.id()))
        .collect::<Vec<_>>();
    writeln!(output, "\n\nFUNCTIONS = ({})", tuple(&functions)).unwrap();
    Ok(output)
}

fn tuple(items: &[String]) -> String {
    if items.is_empty() {
        String::new()
    } else {
        format!("{},", items.join(", "))
    }
}
