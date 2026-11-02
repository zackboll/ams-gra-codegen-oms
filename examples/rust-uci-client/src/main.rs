mod cli;
mod session;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

use ams_gra_oms_runtime_rust::{OmsJsonCodec, RuntimeConfig, RuntimeEvent, SleetRuntime};
use generated::{
    model as m,
    service_api::{
        function_dlz as dlz, function_log as log, function_rdma as rdma,
        function_survivability as survivability,
    },
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
    Survivability(Box<m::SurvivabilityRiskLevelMT>),
    #[allow(clippy::upper_case_acronyms)] // Preserve the UCI global message identity.
    DLZ(Box<m::DLZMT>),
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

fn print_received(epoch: u8, value: Received) {
    match value {
        Received::Log(value) => println!(
            "RECEIVED {} system={} severity={:?} label={:?} session={epoch}",
            log::exchange_receive::MESSAGE_NAME,
            value.messageheader.systemid.uuid.as_str(),
            value.messagedata.severity,
            value.messagedata.label.as_str(),
        ),
        Received::Rdma(value) => println!(
            "RECEIVED {} setup={} provider={:?} endpoints={} session={epoch}",
            rdma::exchange_receive::MESSAGE_NAME,
            value.messagedata.rdma_initializesetupid.uuid.as_str(),
            value.messagedata.rdma_provider,
            value.messagedata.endpoint.as_slice().len(),
        ),
        Received::Survivability(value) => {
            let settings = value.messagedata.risksetting.as_slice();
            let levels: usize = settings.iter().map(|s| s.risklevel.as_slice().len()).sum();
            println!(
                "RECEIVED {} id={} settings={} risk-levels={levels} detail={settings:?} session={epoch}",
                survivability::exchange_receive::MESSAGE_NAME,
                value.messagedata.survivabilityrisklevelid.uuid.as_str(),
                settings.len(),
            );
        }
        Received::DLZ(value) => {
            let data = &value.messagedata.dlz_data;
            println!(
                "RECEIVED {} id={} type={:?} time-of-intercept={} range-poi-bits={:016x} loft-bits={:016x?} vector-bits={:08x?} session={epoch}",
                dlz::exchange_receive::MESSAGE_NAME,
                value.messagedata.dlz_id.uuid.as_str(),
                value.messagedata.dlz_type,
                data.timeofintercept.as_str(),
                data.rangeprobabilityofintercept.get().to_bits(),
                data.loftangle.as_ref().map(|v| v.get().to_bits()),
                data.steeringunitvector.as_ref().map(|v| [
                    v.los_unitvector.x.get().to_bits(),
                    v.los_unitvector.y.get().to_bits(),
                    v.los_unitvector.z.get().to_bits(),
                ]),
            );
        }
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
    // Decode every selected file before connecting or publishing any of them.
    let log: Option<m::LogMT> = options.log.as_deref().map(load).transpose()?;
    let rdma: Option<m::RDMAInitializeSetupMT> = options.rdma.as_deref().map(load).transpose()?;
    let survivability: Option<m::SurvivabilityRiskLevelMT> =
        options.survivability.as_deref().map(load).transpose()?;
    let dlz: Option<m::DLZMT> = options.dlz.as_deref().map(load).transpose()?;
    if let Some(value) = &log {
        validate_schema(value.messageheader.schemaversion.as_str())?;
    }
    if let Some(value) = &rdma {
        validate_schema(value.messageheader.schemaversion.as_str())?;
    }
    if let Some(value) = &survivability {
        validate_schema(value.messageheader.schemaversion.as_str())?;
    }
    if let Some(value) = &dlz {
        validate_schema(value.messageheader.schemaversion.as_str())?;
    }
    let mut session = session::Session::connect(
        &options,
        1,
        ams_gra_oms_runtime_rust::DEFAULT_CONNECT_TIMEOUT,
    )?;
    let result = (|| {
        if let Some(value) = &log {
            log::exchange_publish::publish(&mut session.runtime, value)
                .map_err(|error| format!("publish Log: {error}"))?;
            println!("PUBLISHED Log: frame written; broker acceptance not acknowledged");
        }
        if let Some(value) = &rdma {
            rdma::exchange_publish::publish(&mut session.runtime, value)
                .map_err(|error| format!("publish RDMA_InitializeSetup: {error}"))?;
            println!(
                "PUBLISHED RDMA_InitializeSetup: frame written; broker acceptance not acknowledged"
            );
        }
        if let Some(value) = &survivability {
            survivability::exchange_publish::publish(&mut session.runtime, value)
                .map_err(|error| format!("publish SurvivabilityRiskLevel: {error}"))?;
            println!(
                "PUBLISHED SurvivabilityRiskLevel: frame written; broker acceptance not acknowledged"
            );
        }
        if let Some(value) = &dlz {
            dlz::exchange_publish::publish(&mut session.runtime, value)
                .map_err(|error| format!("publish DLZ: {error}"))?;
            println!("PUBLISHED DLZ: frame written; broker acceptance not acknowledged");
        }
        Ok(())
    })();
    if let Err(error) = result {
        return match session.close(false) {
            Ok(()) => Err(error),
            Err(shutdown) => Err(format!("{error}\n{shutdown}")),
        };
    }
    // One deadline and one additional-attempt budget for the entire receive period.
    session::receive_sessions(session, &options, Instant::now() + options.duration)?;
    println!(
        "COMPLETE bounded execution; no runtime errors observed (not a broker acceptance guarantee)"
    );
    Ok(())
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
    fn one_codec_supports_all_four_payloads() {
        fn compatible<C>()
        where
            C: OmsJsonCodec<m::LogMT>
                + OmsJsonCodec<m::RDMAInitializeSetupMT>
                + OmsJsonCodec<m::SurvivabilityRiskLevelMT>
                + OmsJsonCodec<m::DLZMT>,
        {
        }
        compatible::<ServiceCodec>();
    }

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
