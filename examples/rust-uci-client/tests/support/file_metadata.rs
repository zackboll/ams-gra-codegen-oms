// Independently authored OMS bodies and expectations; no generated encoder oracle.
const FILE_TOPIC: &str = "file.metadata";
const FIVE_TOPICS: [&str; 5] = [LOG, RDMA, RISK, DLZ, FILE_TOPIC];

fn metadata(round: u8) -> Value {
    json!({
        "SecurityInformation": {"Classification":"U", "OwnerProducer":[{"GovernmentIdentifier":"USA"}]},
        "MessageHeader": {
            "SystemID":{"UUID":"550e8400-e29b-41d4-a716-446655440000"},
            "Timestamp":"2026-01-01T00:00:00Z", "SchemaVersion":VERSION, "Mode":"LIVE"
        },
        "MessageData": {
            "FileMetadataID":{"UUID":format!("{round}5345678-1234-1234-8234-123456789abc")},
            "FileDescription":{"FileType":["OTHER"], "FileFormat":{"NonMIME":{"Key":"META", "SystemName":"Independent011"}}},
            "FileName":format!("Independent_011_{round}.txt"),
            "FileSource":{"SystemID":[{"UUID":"550e8400-e29b-41d4-a716-446655440030"}]},
            "CreationSource":"OTHER", "UntrustedModification":false,
            "Timestamp":"2026-02-03T04:05:06Z",
            "SecurityInformation":{"Classification":"U", "OwnerProducer":[{"GovernmentIdentifier":"USA"}]}
        }
    })
}

fn metadata_receipt(app: &mut Application, round: u8, epoch: u8) {
    let marker = format!("filename=\"Independent_011_{round}.txt\"");
    app.until(&marker);
    let line = app
        .output
        .lines()
        .find(|line| line.contains(&marker))
        .unwrap();
    assert_eq!(
        line,
        format!(
            "RECEIVED FileMetadata id={round}5345678-1234-1234-8234-123456789abc {marker} creation-source=OTHER timestamp=2026-02-03T04:05:06Z sha-2-hash-present=false session={epoch}"
        )
    );
}

fn metadata_wire(executor: &tokio::runtime::Runtime, observer: &mut CalClient, round: u8) {
    let observed = executor.block_on(async {
        tokio::time::timeout(WAIT, observer.recv())
            .await
            .unwrap()
            .unwrap()
    });
    assert_eq!(observed.sid, "metadata-wire");
    assert_eq!(
        serde_json::from_str::<Value>(&observed.payload).unwrap(),
        json!({"FileMetadata":metadata(round)})
    );
    eprintln!("RUST-OMS-011 {VERSION} independently witnessed FileMetadata OWP MSG round={round}");
}

fn five_positive(tls: bool) {
    let (mut server, backend) = start_sleet_with_topics(&FIVE_TOPICS);
    connect(&backend).close().unwrap();
    pause(&mut server);
    let log = restart(&mut server);
    connect(&backend).close().unwrap();
    let directory = server.directory.join("tls-011");
    let mut endpoint = if tls {
        assert!(
            Command::new("python3")
                .arg(tls_script())
                .arg(&directory)
                .arg("--generate")
                .status()
                .unwrap()
                .success()
        );
        Some(TlsEndpoint::start(
            &directory, &backend, 0, "valid", "five-1",
        ))
    } else {
        None
    };
    let url = endpoint
        .as_ref()
        .map_or_else(|| backend.clone(), TlsEndpoint::url);
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut observer = executor.block_on(raw_connect(&backend));
    executor
        .block_on(observer.subscribe("metadata-wire", "FileMetadata", FILE_TOPIC, None))
        .unwrap();
    let file = server.directory.join("independent-file-metadata.json");
    std::fs::write(&file, serde_json::to_vec_pretty(&metadata(0)).unwrap()).unwrap();
    let args = [
        "--enable-file-metadata",
        "--publish-file-metadata",
        file.to_str().unwrap(),
        "--reconnect-attempts",
        "1",
        "--reconnect-delay-ms",
        "1000",
    ];
    let mut app = Application::start_trusted(&url, &args, "8", tls.then_some(directory.as_path()));
    metadata_wire(&executor, &mut observer, 0);
    metadata_receipt(&mut app, 0, 1);
    let old_ids = app.subscription_ids();
    assert_eq!(old_ids.len(), 5);
    assert_eq!(old_ids.iter().collect::<BTreeSet<_>>().len(), 5);
    let old = application_workers(app.child.id());
    assert_eq!(old.len(), 1);
    assert_eq!(application_connections(app.child.id(), &url), 1);
    let bodies: [Value; 4] = fixtures()
        .each_ref()
        .map(|file| serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap());
    round_trip(&executor, &backend, &mut app, &distinguish(&bodies, 1), 1);
    executor
        .block_on(raw_publish(
            &mut observer,
            FILE_TOPIC,
            &json!({"FileMetadata":metadata(1)}),
        ))
        .unwrap();
    metadata_wire(&executor, &mut observer, 1);
    metadata_receipt(&mut app, 1, 1);
    let port = endpoint.as_ref().map(|endpoint| endpoint.port);
    if tls {
        drop(endpoint.take());
    } else {
        pause(&mut server);
    }
    app.until("CLOSED runtime; worker joined; session=1");
    assert!(application_workers(app.child.id()).is_empty());
    assert_eq!(application_connections(app.child.id(), &url), 0);
    let restored_log = if tls {
        endpoint = Some(TlsEndpoint::start(
            &directory,
            &backend,
            port.unwrap(),
            "valid",
            "five-2",
        ));
        log.clone()
    } else {
        restart(&mut server)
    };
    app.until("RECONNECT_COMPLETE session=2");
    broker_barrier(&restored_log, "subscription registered", 5);
    let fresh = application_workers(app.child.id());
    assert_eq!(fresh.len(), 1);
    assert!(old.is_disjoint(&fresh));
    assert_eq!(application_connections(app.child.id(), &url), 1);
    let restored_ids: Vec<_> = app
        .output
        .lines()
        .find(|line| line.starts_with("SUBSCRIBED session=2"))
        .unwrap()
        .split_whitespace()
        .filter_map(|word| word.strip_prefix("sid="))
        .collect();
    assert_eq!(restored_ids.len(), 5);
    // SIDs are connection-local and may repeat; fresh handles/registrations
    // are witnessed by the replacement worker and broker/TLS frames.
    assert_eq!(restored_ids.iter().collect::<BTreeSet<_>>().len(), 5);
    if !tls {
        observer = executor.block_on(raw_connect(&backend));
        executor
            .block_on(observer.subscribe("metadata-wire", "FileMetadata", FILE_TOPIC, None))
            .unwrap();
    }
    round_trip(&executor, &backend, &mut app, &distinguish(&bodies, 2), 2);
    executor
        .block_on(raw_publish(
            &mut observer,
            FILE_TOPIC,
            &json!({"FileMetadata":metadata(2)}),
        ))
        .unwrap();
    metadata_wire(&executor, &mut observer, 2);
    metadata_receipt(&mut app, 2, 2);
    app.finish(true);
    app.assert_cleanup();
    assert!(
        app.output
            .contains("UNSUBSCRIBED FileMetadata: frame written; handler removed; session=2")
    );
    assert_eq!(
        app.output.matches("PUBLISHED ").count(),
        1,
        "no publication replay"
    );
    assert_eq!(app.output.matches("RECEIVED FileMetadata ").count(), 3);
    let restored = app
        .output
        .split("RECONNECT_COMPLETE session=2")
        .nth(1)
        .unwrap();
    assert_eq!(
        restored.matches("RECEIVED ").count(),
        5,
        "no stale typed delivery"
    );
    assert!(
        !restored.contains("Independent_011_0.txt") && !restored.contains("Independent_011_1.txt")
    );
    broker_barrier(&restored_log, "dispatch UNSUB", 5);
    if tls {
        drop(endpoint.take());
        for (epoch, pubs, unsubs) in [("five-1", 1, 0), ("five-2", 0, 5)] {
            let client = captured_texts(&directory.join(format!("{epoch}-1-client.bin")));
            let reply = captured_texts(&directory.join(format!("{epoch}-1-server.bin")));
            for (op, count) in [
                ("INIT ", 1),
                ("SUB ", 5),
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
            assert_eq!(
                client
                    .iter()
                    .filter(|text| text.starts_with("SUB ") && text.contains(FILE_TOPIC))
                    .count(),
                1
            );
        }
    }
    drop(observer);
    server.shutdown();
    eprintln!(
        "RUST-OMS-011 {VERSION} five-message {} publication/reception/recovery/unsubscribe/cleanup: PASSED",
        if tls { "verified WSS" } else { "WS" }
    );
}

fn metadata_negatives() {
    // A listening socket witnesses that invalid CLI/publication inputs never connect.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let directory =
        std::env::temp_dir().join(format!("rust-oms-011-negative-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join("body.json");
    let log_file = fixtures()[0].clone();
    for (body, enabled, expected) in [
        (
            serde_json::to_vec(&metadata(0)).unwrap(),
            false,
            "requires --enable-file-metadata",
        ),
        (b"{broken".to_vec(), true, "invalid input JSON"),
        (
            {
                let mut body = metadata(0);
                body["MessageData"]["FileName"] = json!("../not-a-filename");
                serde_json::to_vec(&body).unwrap()
            },
            true,
            "generated codec rejected",
        ),
    ] {
        std::fs::write(&file, body).unwrap();
        let mut args = vec![
            "--publish-log",
            log_file.to_str().unwrap(),
            "--publish-file-metadata",
            file.to_str().unwrap(),
        ];
        if enabled {
            args.push("--enable-file-metadata");
        }
        let mut app = Application::start(&url, &args);
        app.finish(false);
        assert!(app.output.contains(expected), "{}", app.output);
        assert!(!app.output.contains("CONNECTED"));
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
    }
    // Valid 32 octets remain accepted locally. Pinned Sleet's character-count
    // defect rejects the 64 hex digits asynchronously, with no callback/MSG.
    let (mut server, url) = start_sleet_with_topics(&FIVE_TOPICS);
    connect(&url).close().unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut observer = executor.block_on(raw_connect(&url));
    executor
        .block_on(observer.subscribe("metadata-wire", "FileMetadata", FILE_TOPIC, None))
        .unwrap();
    let mut hash = metadata(0);
    hash["MessageData"]["SHA_2_Hash"] =
        json!("000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F");
    std::fs::write(&file, serde_json::to_vec(&hash).unwrap()).unwrap();
    let mut app = Application::start(
        &url,
        &[
            "--enable-file-metadata",
            "--publish-file-metadata",
            file.to_str().unwrap(),
            "--reconnect-attempts",
            "8",
        ],
    );
    app.finish(false);
    for expected in [
        "VALIDATED FileMetadata",
        "PUBLISHED FileMetadata: frame written; broker acceptance not acknowledged",
        "ServerError",
        "length 64 != 32",
        "cannot attribute",
        "CLOSED runtime; worker joined",
    ] {
        assert!(app.output.contains(expected), "{}", app.output);
    }
    assert!(
        !app.output.contains("RECEIVED FileMetadata") && !app.output.contains("RECONNECT_ATTEMPT")
    );
    assert!(
        executor
            .block_on(async { tokio::time::timeout(QUIET, observer.recv()).await })
            .is_err()
    );
    drop(observer);
    server.shutdown();
    // Only the fifth topic is unauthorized; the earlier four remain supported.
    let (mut server, url) = start_sleet_with_topics(&TOPICS);
    connect(&url).close().unwrap();
    let mut denied = Application::start(
        &url,
        &["--enable-file-metadata", "--reconnect-attempts", "8"],
    );
    denied.finish(false);
    assert!(denied.output.contains("ServerError"));
    assert!(!denied.output.contains("RECONNECT_ATTEMPT"));
    assert!(denied.output.contains("CLOSED runtime; worker joined"));
    server.shutdown();
    // FileMetadata is schema-valid and the topic is authorized, but the
    // broker's per-topic binding deliberately does not support that family.
    let (mut server, url) = start_sleet_with_topics(&FIVE_TOPICS);
    connect(&url).close().unwrap();
    pause(&mut server);
    let service = server.directory.join("services/service.toml");
    let mut config = std::fs::read_to_string(&service).unwrap();
    config.push_str(
        "\n[[topic_bindings]]\ntopic = \"file.metadata\"\nallowed_messages = [\"Log\"]\n",
    );
    std::fs::write(service, config).unwrap();
    restart(&mut server);
    connect(&url).close().unwrap();
    let mut unsupported = Application::start(
        &url,
        &["--enable-file-metadata", "--reconnect-attempts", "8"],
    );
    unsupported.finish(false);
    assert!(unsupported.output.contains("ServerError"));
    assert!(
        unsupported
            .output
            .contains("message type FileMetadata not allowed on topic file.metadata")
    );
    assert!(!unsupported.output.contains("RECONNECT_ATTEMPT"));
    assert!(!unsupported.output.contains("RECEIVED FileMetadata"));
    assert!(unsupported.output.contains("CLOSED runtime; worker joined"));
    server.shutdown();
    std::fs::remove_dir_all(directory).unwrap();
    eprintln!(
        "RUST-OMS-011 {VERSION} preconnect validation/optional-hash broker rejection/fifth-topic terminal policy: PASSED"
    );
}

pub fn file_metadata_acceptance() {
    five_positive(false);
    five_positive(true);
    metadata_negatives();
    five_budget();
}

fn five_budget() {
    let (mut server, url) = start_sleet_with_topics(&FIVE_TOPICS);
    connect(&url).close().unwrap();
    let mut app = Application::start_for(
        &url,
        &[
            "--enable-file-metadata",
            "--reconnect-attempts",
            "1",
            "--reconnect-delay-ms",
            "1000",
        ],
        "8",
    );
    app.until("SUBSCRIBED");
    pause(&mut server);
    app.until("CLOSED runtime; worker joined; session=1");
    assert!(application_workers(app.child.id()).is_empty());
    assert_eq!(application_connections(app.child.id(), &url), 0);
    let log = restart(&mut server);
    app.until("RECONNECT_COMPLETE session=2");
    broker_barrier(&log, "subscription registered", 5);
    assert_eq!(application_workers(app.child.id()).len(), 1);
    assert_eq!(application_connections(app.child.id(), &url), 1);
    pause(&mut server);
    app.finish(false);
    assert_eq!(app.output.matches("RECONNECT_ATTEMPT").count(), 1);
    assert!(app.output.contains("attempt budget exhausted (1/1)"));
    assert!(
        app.output
            .contains("CLOSED runtime; worker joined; session=2")
    );
    assert!(!app.output.contains("CONNECTED session=3"));
    eprintln!(
        "RUST-OMS-011 {VERSION} five-message process-wide budget remains spent after recovery: PASSED"
    );
}
