use super::*;
use ams_gra_oms_runtime_rust::{DEFAULT_CONNECT_TIMEOUT, RuntimeError, SubscriptionHandle};
use sleet_client::ClientError;
use std::error::Error;
use std::io::{Error as IoError, ErrorKind};

pub(super) struct Session {
    pub runtime: SleetRuntime<ServiceCodec>,
    subscriptions: [SubscriptionHandle; 4],
    epoch: u8,
    receiver: mpsc::Receiver<(u8, Received)>,
}

impl Session {
    pub fn connect(options: &cli::Options, epoch: u8, timeout: Duration) -> Result<Self, String> {
        let runtime = SleetRuntime::connect(config(options, timeout), ServiceCodec)
            .map_err(|error| format!("connect/INIT: {error}"))?;
        Self::subscribe(runtime, options, epoch)
    }

    fn subscribe(
        mut runtime: SleetRuntime<ServiceCodec>,
        options: &cli::Options,
        epoch: u8,
    ) -> Result<Self, String> {
        println!(
            "CONNECTED session={epoch} service={} UCI={} INIT.schema={SCHEMA}",
            options.service_id,
            env!("AMS_GRA_UCI_RELEASE_BUILD")
        );
        // Each session owns its queue. Closing joins the only producer before
        // this receiver is dropped, so queued old values cannot cross epochs.
        let (sender, receiver) = mpsc::channel();
        let logs = sender.clone();
        let log_sub = log::exchange_receive::subscribe(&mut runtime, move |value| {
            let _ = logs.send((epoch, Received::Log(Box::new(value.clone()))));
        })
        .map_err(|error| format!("subscribe Log: {error}"))?;
        let rdmas = sender.clone();
        let rdma_sub = rdma::exchange_receive::subscribe(&mut runtime, move |value| {
            let _ = rdmas.send((epoch, Received::Rdma(Box::new(value.clone()))));
        })
        .map_err(|error| format!("subscribe RDMA_InitializeSetup: {error}"))?;
        let risks = sender.clone();
        let risk_sub = survivability::exchange_receive::subscribe(&mut runtime, move |value| {
            let _ = risks.send((epoch, Received::Survivability(Box::new(value.clone()))));
        })
        .map_err(|error| format!("subscribe SurvivabilityRiskLevel: {error}"))?;
        let dlz_sub = dlz::exchange_receive::subscribe(&mut runtime, move |value| {
            let _ = sender.send((epoch, Received::DLZ(Box::new(value.clone()))));
        })
        .map_err(|error| format!("subscribe DLZ: {error}"))?;
        println!(
            "SUBSCRIBED session={epoch} Log sid={} RDMA_InitializeSetup sid={} SurvivabilityRiskLevel sid={} DLZ sid={} (frames written)",
            log_sub.id(),
            rdma_sub.id(),
            risk_sub.id(),
            dlz_sub.id()
        );
        Ok(Self {
            runtime,
            subscriptions: [log_sub, rdma_sub, risk_sub, dlz_sub],
            epoch,
            receiver,
        })
    }

    // Only an observed ConnectionClosed can return a recoverable outcome.
    pub fn receive(&self, deadline: Instant) -> Result<Option<String>, String> {
        while Instant::now() < deadline {
            for _ in 0..64 {
                if let Some(event) = self.runtime.try_recv_event() {
                    return connection_outcome(event);
                }
                let Ok((epoch, value)) = self.receiver.try_recv() else {
                    break;
                };
                if epoch != self.epoch {
                    return Err("typed delivery has an unexpected session epoch".into());
                }
                print_received(epoch, value);
                if Instant::now() >= deadline {
                    break;
                }
            }
            if let Some(event) = self.runtime.recv_event_timeout(
                Duration::from_millis(20).min(deadline.saturating_duration_since(Instant::now())),
            ) {
                return connection_outcome(event);
            }
            if !self.runtime.is_running() {
                // The worker may have enqueued its terminal event immediately
                // after the timed receive. Do not mistake that race for a panic.
                return match self.runtime.try_recv_event() {
                    Some(event) => connection_outcome(event),
                    None => Err("runtime worker stopped before duration elapsed".into()),
                };
            }
        }
        Ok(None)
    }

    pub fn close(self, lost: bool) -> Result<(), String> {
        let Self {
            runtime,
            subscriptions,
            epoch,
            receiver,
        } = self;
        let mut errors = Vec::new();
        if lost {
            // ConnectionClosed is terminal: the worker exits and disconnects
            // its event sender. Preserve every remaining non-loss event before
            // close consumes the runtime; a quiet bounded poll is not enough.
            while let Some(event) = runtime.recv_event() {
                if !matches!(event, RuntimeEvent::ConnectionClosed { .. }) {
                    errors.push(report_event(event));
                }
            }
            drop(subscriptions);
        } else {
            for (name, subscription) in [
                "Log",
                "RDMA_InitializeSetup",
                "SurvivabilityRiskLevel",
                "DLZ",
            ]
            .into_iter()
            .zip(subscriptions)
            {
                match subscription.unsubscribe() {
                    Ok(()) => println!(
                        "UNSUBSCRIBED {name}: frame written; handler removed; session={epoch}"
                    ),
                    Err(error) => errors.push(format!("unsubscribe {name}: {error}")),
                }
            }
            for _ in 0..64 {
                let Some(event) = runtime.try_recv_event() else {
                    break;
                };
                errors.push(report_event(event));
            }
            if runtime.try_recv_event().is_some() {
                errors.push("additional runtime events beyond bounded shutdown report".into());
            }
        }
        match runtime.close() {
            Ok(()) => println!("CLOSED runtime; worker joined; session={epoch}"),
            Err(error) => errors.push(format!("shutdown session={epoch}: {error}")),
        }
        // All senders are gone after join. Dropping the entire queue discards
        // old-session messages without an unbounded application-side drain.
        drop(receiver);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }
}

fn connection_outcome(event: RuntimeEvent) -> Result<Option<String>, String> {
    if matches!(event, RuntimeEvent::ConnectionClosed { .. }) {
        Ok(Some(report_event(event)))
    } else {
        Err(report_event(event))
    }
}

fn config(options: &cli::Options, timeout: Duration) -> RuntimeConfig {
    // Runtime initial retry remains disabled, including on recovery attempts.
    RuntimeConfig::new(&options.url, &options.service_id, SCHEMA).with_connect_timeout(timeout)
}

fn transient(error: &RuntimeError) -> bool {
    match error {
        RuntimeError::ConnectTimeout => true,
        RuntimeError::Client(client) => matches!(client.as_ref(), ClientError::Transport(transport)
            if transport.source().and_then(|source| source.downcast_ref::<IoError>())
                .is_some_and(|io| matches!(io.kind(), ErrorKind::ConnectionRefused | ErrorKind::TimedOut))),
        _ => false,
    }
}

pub(super) fn receive_sessions(
    mut session: Session,
    options: &cli::Options,
    deadline: Instant,
) -> Result<(), String> {
    let mut attempts = 0;
    loop {
        let loss = match session.receive(deadline) {
            Ok(Some(loss)) => loss,
            outcome => {
                let mut errors: Vec<_> = outcome.err().into_iter().collect();
                if let Err(error) = session.close(false) {
                    errors.push(error);
                }
                return if errors.is_empty() {
                    Ok(())
                } else {
                    Err(errors.join("\n"))
                };
            }
        };
        let epoch = session.epoch;
        println!("DISCONNECTED session={epoch} {loss}");
        // Consuming the old owner and joining must precede any new connect.
        session
            .close(true)
            .map_err(|error| format!("{loss}\nrecovery aborted: {error}"))?;
        let failure = |detail: String| format!("{loss}\nrecovery failed: {detail}");
        let mut last_error = None;
        loop {
            if attempts >= options.reconnect_attempts {
                return Err(failure(format!(
                    "attempt budget exhausted ({attempts}/{}); last connect error: {last_error:?}",
                    options.reconnect_attempts
                )));
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining <= options.reconnect_delay {
                std::thread::sleep(remaining);
                return Err(failure(
                    "execution deadline expired while unavailable".into(),
                ));
            }
            std::thread::sleep(options.reconnect_delay);
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(failure(
                    "execution deadline expired while unavailable".into(),
                ));
            }
            attempts += 1;
            println!(
                "RECONNECT_ATTEMPT {attempts}/{} after-session={epoch}",
                options.reconnect_attempts
            );
            match SleetRuntime::connect(
                config(options, DEFAULT_CONNECT_TIMEOUT.min(remaining)),
                ServiceCodec,
            ) {
                Ok(runtime) => {
                    if Instant::now() >= deadline {
                        runtime
                            .close()
                            .map_err(|error| failure(format!("shutdown: {error}")))?;
                        return Err(failure(
                            "execution deadline expired during connect/INIT".into(),
                        ));
                    }
                    session = Session::subscribe(runtime, options, epoch + 1).map_err(|error| {
                        failure(format!("terminal subscription setup: {error}"))
                    })?;
                    println!("RECONNECT_COMPLETE session={}", epoch + 1);
                    break;
                }
                Err(error) if transient(&error) => {
                    eprintln!("RECONNECT_UNAVAILABLE attempt={attempts}: {error}");
                    last_error = Some(error.to_string());
                    if Instant::now() >= deadline {
                        return Err(failure(format!(
                            "execution deadline expired while unavailable: {error}"
                        )));
                    }
                }
                Err(error) => {
                    return Err(failure(format!("terminal connect/INIT failure: {error}")));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_loss_is_recoverable() {
        assert!(connection_outcome(RuntimeEvent::HandlerPanicked { sid: "old".into() }).is_err());
        assert!(
            connection_outcome(RuntimeEvent::UnknownSubscription {
                sid: "old".into(),
                payload: "{}".into()
            })
            .is_err()
        );
        assert!(
            connection_outcome(RuntimeEvent::ServerError {
                error: "denied".into(),
                details: None
            })
            .is_err()
        );
        assert!(
            connection_outcome(RuntimeEvent::ProtocolError {
                error: Box::new(ClientError::UnexpectedServerOp("bad frame".into())),
            })
            .is_err()
        );
        assert!(
            connection_outcome(RuntimeEvent::SubscriptionDecodeError {
                sid: "old".into(),
                message_name: "Log".into(),
                error: ams_gra_oms_runtime_rust::MessageDecodeError::NotAnObject,
            })
            .is_err()
        );
        assert!(
            connection_outcome(RuntimeEvent::ConnectionClosed {
                error: Box::new(ClientError::Transport(
                    IoError::from(ErrorKind::ConnectionReset).into()
                ))
            })
            .unwrap()
            .is_some()
        );
    }

    #[test]
    fn establishment_policy_is_typed_and_narrow() {
        assert!(transient(&RuntimeError::ConnectTimeout));
        for error in [
            RuntimeError::WorkerStopped,
            RuntimeError::WorkerPanicked,
            RuntimeError::InvalidConfig("Connection refused".into()),
            RuntimeError::from(ClientError::InvalidInput("timeout".into())),
        ] {
            assert!(!transient(&error));
        }
        for kind in [
            ErrorKind::ConnectionRefused,
            ErrorKind::TimedOut,
            ErrorKind::PermissionDenied,
            ErrorKind::InvalidData,
        ] {
            assert_eq!(
                transient(&RuntimeError::from(ClientError::Transport(
                    IoError::from(kind).into()
                ))),
                matches!(kind, ErrorKind::ConnectionRefused | ErrorKind::TimedOut)
            );
        }
    }
}
