// Test-only endpoint control; the executable still uses its one production
// SleetRuntime connection and application-owned RUST-OMS-008 recovery loop.
struct TlsEndpoint {
    child: Child,
    port: u16,
}

impl TlsEndpoint {
    fn start(
        directory: &std::path::Path,
        backend: &str,
        port: u16,
        identity: &str,
        epoch: &str,
    ) -> Self {
        let mut child = Command::new("python3")
            .arg(tls_script())
            .arg(directory)
            .args(["--backend", backend.rsplit(':').next().unwrap()])
            .args([
                "--port",
                &port.to_string(),
                "--identity",
                identity,
                "--epoch",
                epoch,
            ])
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let port = line
            .trim()
            .strip_prefix("READY ")
            .expect("TLS listener readiness")
            .parse()
            .unwrap();
        Self { child, port }
    }

    fn url(&self) -> String {
        format!("wss://localhost:{}", self.port)
    }
}

impl Drop for TlsEndpoint {
    fn drop(&mut self) {
        let _ = self.child.kill();
        self.child.wait().unwrap();
        assert!(!std::path::Path::new(&format!("/proc/{}", self.child.id())).exists());
    }
}

fn tls_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/tls_endpoint.py")
}

// Passive capture only, not a WebSocket transport. TLS and HTTP/OWP forwarding
// are exclusively Python ssl and the unmodified pinned client/server.
fn captured_texts(path: &std::path::Path) -> Vec<String> {
    let wire = std::fs::read(path).unwrap();
    let end = wire
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .unwrap()
        + 4;
    let header = String::from_utf8_lossy(&wire[..end]).to_ascii_lowercase();
    assert!(header.contains("sec-websocket-protocol: owp"), "{header}");
    let mut data = &wire[end..];
    let mut texts = Vec::new();
    while !data.is_empty() {
        assert!(data.len() >= 2);
        let opcode = data[0] & 15;
        let masked = data[1] & 128 != 0;
        let mut size = usize::from(data[1] & 127);
        data = &data[2..];
        if size == 126 {
            size = usize::from(u16::from_be_bytes(data[..2].try_into().unwrap()));
            data = &data[2..];
        } else if size == 127 {
            size = u64::from_be_bytes(data[..8].try_into().unwrap())
                .try_into()
                .unwrap();
            data = &data[8..];
        }
        let mask = if masked {
            let mask: [u8; 4] = data[..4].try_into().unwrap();
            data = &data[4..];
            mask
        } else {
            [0; 4]
        };
        let payload: Vec<_> = data[..size]
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ mask[i % 4])
            .collect();
        if opcode == 1 {
            texts.push(String::from_utf8(payload).unwrap());
        }
        data = &data[size..];
    }
    texts
}

pub fn tls_failure_worker_probe(url: &str) {
    let before = application_workers(std::process::id());
    let error = SleetRuntime::connect(
        RuntimeConfig::new(url, TOPIC, VERSION)
            .with_connect_timeout(Duration::from_secs(2))
            .with_initial_connect_retry(8, Duration::from_millis(50), Duration::from_millis(50)),
        ServiceCodec,
    )
    .err()
    .expect("TLS must fail");
    assert_eq!(
        application_workers(std::process::id()),
        before,
        "failed connect must join before returning"
    );
    assert_eq!(connections(url), 0);
    eprintln!("TLS_FAILURE_WORKER_JOINED: {error}");
}

fn negative_tls(directory: &std::path::Path, url: &str, expected: &str) {
    let mut app =
        Application::start_trusted(url, &["--reconnect-attempts", "8"], "4", Some(directory));
    app.finish(false);
    assert!(app.output.contains(expected), "{}", app.output);
    for forbidden in [
        "CONNECTED",
        "SUBSCRIBED",
        "PUBLISHED",
        "RECONNECT_ATTEMPT",
        "RECONNECT_UNAVAILABLE",
    ] {
        assert!(!app.output.contains(forbidden), "{}", app.output);
    }
    let probe = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "tls_failure_worker_probe", "--nocapture"])
        .env("AMS_GRA_TLS_FAILURE_URL", url)
        .env("SSL_CERT_FILE", directory.join("ca.pem"))
        .env("SSL_CERT_DIR", directory.join("empty-roots"))
        .output()
        .unwrap();
    assert!(
        probe.status.success(),
        "{}",
        String::from_utf8_lossy(&probe.stderr)
    );
    let evidence = String::from_utf8_lossy(&probe.stderr);
    assert!(evidence.contains("TLS_FAILURE_WORKER_JOINED"), "{evidence}");
    assert!(evidence.contains(expected), "{evidence}");
    eprintln!(
        "RUST-OMS-009 {VERSION} TLS negative {expected}; initial nonzero recovery/runtime retry budgets terminal; {evidence}"
    );
}

pub fn wss() {
    let (mut server, backend) = start_sleet_with_topics(&TOPICS);
    connect(&backend).close().unwrap();
    pause(&mut server);
    let log = restart(&mut server);
    connect(&backend).close().unwrap();
    let directory = server.directory.join("tls");
    assert!(
        Command::new("python3")
            .arg(tls_script())
            .arg(&directory)
            .arg("--generate")
            .status()
            .unwrap()
            .success()
    );
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let files = fixtures();
    let bodies: [Value; 4] = files
        .each_ref()
        .map(|file| serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap());
    let publications: Vec<_> = FLAGS
        .iter()
        .zip(&files)
        .flat_map(|(flag, file)| [*flag, file.to_str().unwrap()])
        .collect();

    // One valid authenticated session, then a fresh valid authenticated session.
    // Independent WS clients observe the unmodified broker, not proxy synthesis.
    let endpoint = TlsEndpoint::start(&directory, &backend, 0, "valid", "positive-1");
    let port = endpoint.port;
    let url = endpoint.url();
    let mut observer = executor.block_on(raw_connect(&backend));
    executor.block_on(async {
        for (i, (name, topic)) in MESSAGES.iter().zip(TOPICS).enumerate() {
            observer
                .subscribe(&format!("initial-{i}"), name, topic, None)
                .await
                .unwrap();
        }
    });
    let mut args = publications.clone();
    args.extend(["--reconnect-attempts", "8", "--reconnect-delay-ms", "1000"]);
    let mut app = Application::start_trusted(&url, &args, "8", Some(&directory));
    app.until("RECEIVED DLZ");
    for (i, name) in MESSAGES.iter().enumerate() {
        let observed = executor.block_on(async {
            tokio::time::timeout(WAIT, observer.recv())
                .await
                .unwrap()
                .unwrap()
        });
        assert_eq!(observed.sid, format!("initial-{i}"));
        assert_eq!(
            serde_json::from_str::<Value>(&observed.payload).unwrap(),
            json!({*name: bodies[i]})
        );
        eprintln!(
            "RUST-OMS-009 {VERSION} initial WSS publication independently observed {name}: {}",
            observed.payload
        );
    }
    assert_eq!(app.subscription_ids().len(), 4);
    let old = application_workers(app.child.id());
    assert_eq!(old.len(), 1);
    assert_eq!(application_connections(app.child.id(), &url), 1);
    round_trip(&executor, &backend, &mut app, &distinguish(&bodies, 1), 1);
    drop(endpoint);
    app.until("CLOSED runtime; worker joined; session=1");
    assert!(application_workers(app.child.id()).is_empty());
    assert_eq!(application_connections(app.child.id(), &url), 0);
    let endpoint = TlsEndpoint::start(&directory, &backend, port, "valid", "positive-2");
    app.until("RECONNECT_COMPLETE session=2");
    let fresh = application_workers(app.child.id());
    assert_eq!(fresh.len(), 1);
    assert!(old.is_disjoint(&fresh));
    assert_eq!(application_connections(app.child.id(), &url), 1);
    round_trip(&executor, &backend, &mut app, &distinguish(&bodies, 2), 2);
    app.finish(true);
    app.assert_cleanup();
    broker_barrier(&log, "dispatch UNSUB", 4);
    let restored = app
        .output
        .split("RECONNECT_COMPLETE session=2")
        .nth(1)
        .unwrap();
    assert_eq!(restored.matches("RECEIVED ").count(), 4);
    for name in MESSAGES {
        assert_eq!(restored.matches(&format!("RECEIVED {name} ")).count(), 1);
        assert_eq!(app.output.matches(&format!("RECEIVED {name} ")).count(), 3);
    }
    assert_eq!(app.output.matches("PUBLISHED ").count(), 4);
    drop(endpoint);
    for (epoch, pubs, unsubs) in [("positive-1", 4, 0), ("positive-2", 0, 4)] {
        let client = captured_texts(&directory.join(format!("{epoch}-1-client.bin")));
        let reply = captured_texts(&directory.join(format!("{epoch}-1-server.bin")));
        for (op, count) in [
            ("INIT ", 1),
            ("SUB ", 4),
            ("PUB ", pubs),
            ("UNSUB ", unsubs),
        ] {
            assert_eq!(
                client.iter().filter(|text| text.starts_with(op)).count(),
                count,
                "{epoch} {op}"
            );
        }
        assert_eq!(
            reply
                .iter()
                .filter(|text| text.starts_with("INFO "))
                .count(),
            1
        );
    }
    eprintln!(
        "RUST-OMS-009 {VERSION} verified WSS four-message exchange and recovery; worker {old:?} -> {fresh:?}; fresh INIT/INFO, four SUB, four UNSUB, no PUB replay: PASSED"
    );

    for (identity, expected) in [
        ("untrusted", "UnknownIssuer"),
        ("hostname", "certificate not valid for name"),
        ("expired", "certificate expired"),
        ("future", "certificate not valid yet"),
        ("chain", "UnknownIssuer"),
    ] {
        let endpoint = TlsEndpoint::start(&directory, &backend, 0, identity, identity);
        negative_tls(&directory, &endpoint.url(), expected);
        drop(endpoint);
        assert_eq!(
            std::fs::read_to_string(directory.join(format!("{identity}-handshakes.txt")))
                .unwrap()
                .lines()
                .count(),
            2,
            "one CLI handshake plus one runtime retry-policy probe, never eight retries"
        );
        assert!(
            !directory.join(format!("{identity}-1-client.bin")).exists(),
            "no authenticated application bytes"
        );
    }
    negative_tls(
        &directory,
        &backend.replacen("ws://", "wss://", 1),
        "transport error:",
    );
    let mut unsupported =
        Application::start_trusted("https://localhost", &[], "1", Some(&directory));
    unsupported.finish(false);
    assert!(unsupported.output.contains("--url must be"));
    assert!(!unsupported.output.contains("CONNECTED"));

    // Identity changes are terminal on the first recovery handshake, despite
    // seven attempts remaining. The old session has already joined.
    for (identity, expected) in [
        ("untrusted", "UnknownIssuer"),
        ("hostname", "certificate not valid for name"),
    ] {
        let endpoint = TlsEndpoint::start(
            &directory,
            &backend,
            0,
            "valid",
            &format!("{identity}-before"),
        );
        let port = endpoint.port;
        let url = endpoint.url();
        let mut app = Application::start_trusted(
            &url,
            &["--reconnect-attempts", "8", "--reconnect-delay-ms", "1000"],
            "8",
            Some(&directory),
        );
        app.until("SUBSCRIBED");
        round_trip(&executor, &backend, &mut app, &distinguish(&bodies, 1), 1);
        drop(endpoint);
        app.until("CLOSED runtime; worker joined; session=1");
        assert!(application_workers(app.child.id()).is_empty());
        assert_eq!(application_connections(app.child.id(), &url), 0);
        let endpoint = TlsEndpoint::start(
            &directory,
            &backend,
            port,
            identity,
            &format!("{identity}-after"),
        );
        app.finish(false);
        assert!(
            app.output.contains("terminal connect/INIT failure"),
            "{}",
            app.output
        );
        assert!(app.output.contains(expected), "{}", app.output);
        assert_eq!(app.output.matches("RECONNECT_ATTEMPT").count(), 1);
        assert!(!app.output.contains("RECONNECT_UNAVAILABLE"));
        assert!(!app.output.contains("CONNECTED session=2"));
        assert!(
            !directory
                .join(format!("{identity}-after-1-client.bin"))
                .exists()
        );
        drop(endpoint);
        eprintln!(
            "RUST-OMS-009 {VERSION} recovery identity {identity}: terminal {expected}; no INIT/publication/new session: PASSED"
        );
        assert_eq!(
            std::fs::read_to_string(directory.join(format!("{identity}-after-handshakes.txt")))
                .unwrap()
                .lines()
                .count(),
            1
        );
    }
    server.shutdown();
    eprintln!(
        "RUST-OMS-009 {VERSION} verified WSS interoperability, TLS negatives, recovery and cleanup: PASSED"
    );
}
