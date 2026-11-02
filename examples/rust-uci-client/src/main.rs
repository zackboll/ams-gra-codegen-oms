mod cli;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

use ams_gra_oms_runtime_rust::{OmsJsonCodec, RuntimeConfig, RuntimeEvent, SleetRuntime};
use generated::{
    model as m,
    service_api::{function_log as log, function_rdma as rdma},
    service_codec::ServiceCodec,
};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const SCHEMA: &str = env!("AMS_GRA_SCHEMA_VERSION_BUILD");

// Actual generated values cross this channel. Handlers only clone/send; all
// printing and blocking runtime operations belong to the application thread.
enum Received {
    Log(Box<m::LogMT>),
    Rdma(Box<m::RDMAInitializeSetupMT>),
}

fn load<P>(path: &Path) -> Result<P, String>
where
    ServiceCodec: OmsJsonCodec<P>,
{
    let bytes = std::fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let body = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid input JSON {}: {error}", path.display()))?;
    // The generated decoder validates through checked model constructors;
    // these models have no separate Validate trait. Never deserialize them
    // with Serde or bypass the generated codec.
    ServiceCodec
        .decode_payload(&body)
        .map_err(|error| format!("generated codec rejected {}: {error}", path.display()))
}

fn validate_schema(actual: &str) -> Result<(), String> {
    if actual != SCHEMA {
        return Err(format!(
            "MessageHeader.SchemaVersion {actual:?} does not match build-selected {SCHEMA}"
        ));
    }
    Ok(())
}

fn print_received(value: Received) {
    match value {
        Received::Log(value) => println!(
            "RECEIVED {} system={} severity={:?} label={:?}",
            log::exchange_receive::MESSAGE_NAME,
            value.messageheader.systemid.uuid.as_str(),
            value.messagedata.severity,
            value.messagedata.label.as_str(),
        ),
        Received::Rdma(value) => println!(
            "RECEIVED {} setup={} provider={:?} endpoints={}",
            rdma::exchange_receive::MESSAGE_NAME,
            value.messagedata.rdma_initializesetupid.uuid.as_str(),
            value.messagedata.rdma_provider,
            value.messagedata.endpoint.as_slice().len(),
        ),
    }
}

fn report_event(event: RuntimeEvent) -> String {
    match event {
        RuntimeEvent::ServerError { error, details } => format!(
            "ServerError {error}: {details:?}; non-verbose OWP cannot attribute this to a specific operation"
        ),
        RuntimeEvent::SubscriptionDecodeError {
            sid,
            message_name,
            error,
        } => format!(
            "SubscriptionDecodeError sid={sid} message={message_name}: {error}; no typed delivery"
        ),
        RuntimeEvent::ProtocolError { error } => format!("ProtocolError: {error}"),
        RuntimeEvent::ConnectionClosed { error } => format!("ConnectionClosed: {error}"),
        other => format!("runtime event: {other:?}"),
    }
}

fn run(options: cli::Options) -> Result<(), String> {
    // Validate both selected files before connecting or publishing either one.
    let log: Option<m::LogMT> = options.log.as_deref().map(load).transpose()?;
    let rdma: Option<m::RDMAInitializeSetupMT> = options.rdma.as_deref().map(load).transpose()?;
    if let Some(value) = &log {
        validate_schema(value.messageheader.schemaversion.as_str())?;
    }
    if let Some(value) = &rdma {
        validate_schema(value.messageheader.schemaversion.as_str())?;
    }
    let mut runtime: SleetRuntime<ServiceCodec> = SleetRuntime::connect(
        RuntimeConfig::new(&options.url, &options.service_id, SCHEMA),
        ServiceCodec,
    )
    .map_err(|error| format!("connect/INIT: {error}"))?;
    println!(
        "CONNECTED service={} UCI={} INIT.schema={SCHEMA}",
        options.service_id,
        env!("AMS_GRA_UCI_RELEASE_BUILD")
    );
    let (sender, receiver) = mpsc::channel();
    let logs = sender.clone();
    let log_sub = log::exchange_receive::subscribe(&mut runtime, move |value| {
        // Receiver outlives the runtime. A failed send during unwinding needs
        // no runtime call; Drop still signals and joins the worker.
        let _ = logs.send(Received::Log(Box::new(value.clone())));
    })
    .map_err(|error| format!("subscribe Log: {error}"))?;
    let rdma_sub = rdma::exchange_receive::subscribe(&mut runtime, move |value| {
        let _ = sender.send(Received::Rdma(Box::new(value.clone())));
    })
    .map_err(|error| format!("subscribe RDMA_InitializeSetup: {error}"))?;
    println!(
        "SUBSCRIBED Log sid={} RDMA_InitializeSetup sid={} (frames written)",
        log_sub.id(),
        rdma_sub.id()
    );

    // The closure retains runtime ownership here, so publication/receive errors
    // still go through both unsubscribe attempts and explicit close below.
    let result = (|| {
        if let Some(value) = &log {
            log::exchange_publish::publish(&mut runtime, value)
                .map_err(|error| format!("publish Log: {error}"))?;
            println!("PUBLISHED Log: frame written; broker acceptance not acknowledged");
        }
        if let Some(value) = &rdma {
            rdma::exchange_publish::publish(&mut runtime, value)
                .map_err(|error| format!("publish RDMA_InitializeSetup: {error}"))?;
            println!(
                "PUBLISHED RDMA_InitializeSetup: frame written; broker acceptance not acknowledged"
            );
        }
        let deadline = Instant::now() + options.duration;
        while Instant::now() < deadline {
            // Finite batches prevent message/event floods from starving the
            // other queue or the execution deadline. Any runtime event fails
            // this demo visibly; it does not introduce a recovery policy.
            for _ in 0..64 {
                if let Some(event) = runtime.try_recv_event() {
                    return Err(report_event(event));
                }
                let Ok(value) = receiver.try_recv() else {
                    break;
                };
                print_received(value);
                if Instant::now() >= deadline {
                    break;
                }
            }
            if let Some(event) = runtime.recv_event_timeout(
                Duration::from_millis(20).min(deadline.saturating_duration_since(Instant::now())),
            ) {
                return Err(report_event(event));
            }
            if !runtime.is_running() {
                return Err("runtime worker stopped before duration elapsed".into());
            }
        }
        Ok(())
    })();
    let mut errors: Vec<String> = result.err().into_iter().collect();
    for (name, subscription) in [("Log", log_sub), ("RDMA_InitializeSetup", rdma_sub)] {
        match subscription.unsubscribe() {
            Ok(()) => println!("UNSUBSCRIBED {name}: frame written; handler removed"),
            Err(error) => errors.push(format!("unsubscribe {name}: {error}")),
        }
    }
    // Drain a bounded final batch before close consumes the runtime. OWP has
    // no non-verbose acceptance barrier: a quiet queue is not an ACK.
    for _ in 0..64 {
        let Some(event) = runtime.try_recv_event() else {
            break;
        };
        errors.push(report_event(event));
    }
    if runtime.try_recv_event().is_some() {
        errors.push("additional runtime events beyond bounded shutdown report".into());
    }
    match runtime.close() {
        Ok(()) => println!("CLOSED runtime; worker joined"),
        Err(error) => errors.push(format!("shutdown: {error}")),
    }
    if errors.is_empty() {
        println!(
            "COMPLETE bounded execution; no runtime errors observed (not a broker acceptance guarantee)"
        );
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn main() -> std::process::ExitCode {
    let result = cli::Options::parse(std::env::args().skip(1)).and_then(|options| match options {
        Some(options) => run(options),
        None => {
            print!("{}", cli::HELP);
            Ok(())
        }
    });
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ERROR: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_version_must_match_build() {
        assert!(validate_schema(SCHEMA).is_ok());
        assert!(
            validate_schema("002.0.0")
                .unwrap_err()
                .contains("build-selected")
        );
    }

    #[test]
    fn server_errors_never_claim_publication_acceptance() {
        let message = report_event(RuntimeEvent::ServerError {
            error: "Illegal-State".into(),
            details: Some("denied".into()),
        });
        assert!(message.contains("ServerError Illegal-State"));
        assert!(message.contains("cannot attribute"));
    }
}
