// Included in real_sleet's support module; uses the same pinned server binary,
// isolated configuration, independent fixtures and actual executable.

fn pause(server: &mut Server) {
    server.child.kill().unwrap();
    server.child.wait().unwrap();
}

fn restart(server: &mut Server) -> PathBuf {
    let log = server.directory.join("restart.log");
    server.child = Command::new(std::env::var_os("AMS_GRA_SLEET_BIN").unwrap())
        .arg(server.directory.join("server.toml"))
        .env("RUST_LOG", "sleet=debug")
        .env("NO_COLOR", "1")
        .stdout(Stdio::from(std::fs::File::create(&log).unwrap()))
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    log
}

fn broker_barrier(log: &PathBuf, marker: &str, count: usize) {
    let deadline = Instant::now() + WAIT;
    loop {
        let text = std::fs::read_to_string(log).unwrap();
        if text.matches(marker).count() >= count {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "missing broker evidence {marker}: {text}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn application_workers(pid: u32) -> BTreeSet<String> {
    std::fs::read_dir(format!("/proc/{pid}/task"))
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            std::fs::read_to_string(entry.path().join("comm"))
                .unwrap()
                .starts_with("ams-gra-oms-lac")
                .then(|| entry.file_name().to_string_lossy().into_owned())
        })
        .collect()
}

fn fixtures() -> [PathBuf; 4] {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let suffix = env!("AMS_GRA_UCI_RELEASE_BUILD").replace('.', "");
    [
        root.join(format!("service-generate/task093-real-log{suffix}.json")),
        root.join(format!("service-generate/task092-real-rdma{suffix}.json")),
        root.join(if suffix == "25" {
            "service-generate/task092-real-survivability25.json"
        } else {
            "task105/survivability26.json"
        }),
        root.join(format!("task101/uci{suffix}-full-dlz.json")),
    ]
}

fn distinguish(bodies: &[Value; 4], round: u8) -> [Value; 4] {
    let mut values = bodies.clone();
    values[0]["MessageData"]["Label"] = json!(format!("Reconnect{round}"));
    for (family, field) in [
        (1, "RDMA_InitializeSetupID"),
        (2, "SurvivabilityRiskLevelID"),
        (3, "DLZ_ID"),
    ] {
        values[family]["MessageData"][field]["UUID"] =
            json!(format!("{round}{family}345678-1234-1234-8234-123456789abc"));
    }
    values
}

fn round_trip(
    executor: &tokio::runtime::Runtime,
    url: &str,
    app: &mut Application,
    bodies: &[Value; 4],
    epoch: u8,
) {
    let mut witness = executor.block_on(raw_connect(url));
    // Verbose independent SUB has an acceptance response; it does not stand
    // in for the application's four non-verbose subscriptions.
    executor.block_on(async {
        for (index, (name, topic)) in MESSAGES.iter().zip(TOPICS).enumerate() {
            witness
                .subscribe(&format!("independent-{index}"), name, topic, None)
                .await
                .unwrap();
        }
    });
    for (family, ((name, topic), body)) in MESSAGES.iter().zip(TOPICS).zip(bodies).enumerate() {
        let document = json!({(*name): body});
        executor
            .block_on(raw_publish(&mut witness, topic, &document))
            .unwrap();
        let observed = executor.block_on(async {
            tokio::time::timeout(WAIT, witness.recv())
                .await
                .unwrap()
                .unwrap()
        });
        assert_eq!(observed.sid, format!("independent-{family}"));
        assert_eq!(
            serde_json::from_str::<Value>(&observed.payload).unwrap(),
            document
        );
        let marker = if family == 0 {
            format!("label=\"Reconnect{epoch}\"")
        } else {
            format!("{epoch}{family}345678-1234-1234-8234-123456789abc")
        };
        app.until(&marker);
        let line = app
            .output
            .lines()
            .find(|line| line.starts_with(&format!("RECEIVED {name} ")) && line.contains(&marker))
            .unwrap();
        assert!(
            line.starts_with(&format!("RECEIVED {name} "))
                && line.ends_with(&format!("session={epoch}")),
            "{line}"
        );
        eprintln!(
            "RUST-OMS-008 {VERSION} session={epoch} independent OWP MSG {}: {}",
            observed.sid, observed.payload
        );
    }
    drop(witness);
}

pub fn reconnection() {
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
    let (mut server, url) = start_sleet_with_topics(&TOPICS);
    connect(&url).close().unwrap();
    let mut args = publications.clone();
    args.extend(["--reconnect-attempts", "8", "--reconnect-delay-ms", "500"]);
    let mut app = Application::start_for(&url, &args, "8");
    app.until("RECEIVED DLZ"); // Initial CLI self-echo: four effective subscriptions.
    let old_ids = app.subscription_ids();
    let old_workers = application_workers(app.child.id());
    assert_eq!(old_workers.len(), 1);
    assert_eq!(application_connections(app.child.id(), &url), 1);
    round_trip(&executor, &url, &mut app, &distinguish(&bodies, 1), 1);
    pause(&mut server);
    app.until("CLOSED runtime; worker joined; session=1");
    // Observe the policy's finite delay, not a sleep added to hide teardown.
    assert!(application_workers(app.child.id()).is_empty());
    assert_eq!(application_connections(app.child.id(), &url), 0);
    app.until("RECONNECT_ATTEMPT 1/8");
    let log = restart(&mut server);
    app.until("RECONNECT_COMPLETE session=2");
    broker_barrier(&log, "subscription registered", 4);
    let fresh_workers = application_workers(app.child.id());
    assert_eq!(fresh_workers.len(), 1);
    assert!(old_workers.is_disjoint(&fresh_workers));
    assert_eq!(application_connections(app.child.id(), &url), 1);
    assert!(app.output.contains("SUBSCRIBED session=2"));
    // Same textual SIDs are valid: they belong to different connections.
    assert_eq!(old_ids, vec!["sub-1", "sub-2", "sub-3", "sub-4"]);
    round_trip(&executor, &url, &mut app, &distinguish(&bodies, 2), 2);
    app.finish(true);
    app.assert_cleanup();
    broker_barrier(&log, "dispatch UNSUB", 4);
    assert_eq!(app.output.matches("PUBLISHED ").count(), 4);
    let restored = app
        .output
        .split("RECONNECT_COMPLETE session=2")
        .nth(1)
        .unwrap();
    assert_eq!(
        restored.matches("RECEIVED ").count(),
        4,
        "no CLI replay or stale delivery: {restored}"
    );
    for name in MESSAGES {
        assert_eq!(restored.matches(&format!("RECEIVED {name} ")).count(), 1);
    }
    let broker = std::fs::read_to_string(&log).unwrap();
    assert_eq!(
        broker.matches("dispatch PUB").count(),
        4,
        "no automatic publication replay: {broker}"
    );
    assert!(
        app.output.find("worker joined; session=1").unwrap()
            < app.output.find("CONNECTED session=2").unwrap()
    );
    eprintln!(
        "RUST-OMS-008 {VERSION} fresh worker IDs {old_workers:?} -> {fresh_workers:?}; zero workers/connections between sessions; fresh SUB and four UNSUB broker evidence; no publication replay"
    );
    server.shutdown();
    drop(server);

    // Every control first establishes a real session; initial failure is not recovery.
    for control in [
        "outage", "schema", "denied", "deadline", "timeout", "budget",
    ] {
        let (mut server, url) = start_sleet_with_topics(&TOPICS);
        connect(&url).close().unwrap();
        let delay = if control == "deadline" { "2000" } else { "500" };
        let budget = if control == "outage" {
            "2"
        } else if control == "budget" {
            "1"
        } else {
            "8"
        };
        let seconds = if matches!(control, "deadline" | "timeout") {
            "2"
        } else {
            "6"
        };
        let mut app = Application::start_for(
            &url,
            &[
                "--reconnect-attempts",
                budget,
                "--reconnect-delay-ms",
                delay,
            ],
            seconds,
        );
        app.until("SUBSCRIBED");
        let started = Instant::now();
        if control == "deadline" {
            // Place the outage near the end of the two-second receive period.
            std::thread::sleep(Duration::from_millis(1500));
        }
        pause(&mut server);
        app.until("CLOSED runtime; worker joined; session=1");
        // A listening socket that never completes the upgrade exercises the
        // remaining-deadline connect timeout without modifying Sleet.
        let stalled = (control == "timeout")
            .then(|| TcpListener::bind(url.strip_prefix("ws://").unwrap()).unwrap());
        if control == "schema" {
            let path = server.directory.join("server.toml");
            let (other_version, variable) = if VERSION == "002.5.0" {
                ("002.6.0", "AMS_GRA_UCI_2_6_ROOT")
            } else {
                ("002.5.0", "AMS_GRA_UCI_2_5_ROOT")
            };
            let other_root = std::env::var(variable)
                .expect("both authentic roots required for incompatible-schema restart");
            let config = std::fs::read_to_string(&path)
                .unwrap()
                .replace(
                    &format!("schema_version = \"{VERSION}\""),
                    &format!("schema_version = \"{other_version}\""),
                )
                .replace(
                    &format!("schema_path = \"{ROOT}\""),
                    &format!("schema_path = \"{other_root}\""),
                );
            std::fs::write(path, config).unwrap();
            restart(&mut server);
        } else if control == "denied" {
            let path = server.directory.join("services/service.toml");
            let config = std::fs::read_to_string(&path).unwrap().replace(
                &format!("allowed_topics = {TOPICS:?}"),
                "allowed_topics = []",
            );
            std::fs::write(path, config).unwrap();
            restart(&mut server);
        } else if control == "budget" {
            restart(&mut server);
            app.until("RECONNECT_COMPLETE session=2");
            pause(&mut server);
        }
        app.finish(false);
        assert!(app.output.contains("ConnectionClosed"));
        assert!(
            app.output
                .contains("CLOSED runtime; worker joined; session=1")
        );
        match control {
            "schema" => {
                assert!(app.output.contains("terminal connect/INIT failure"));
                assert_eq!(app.output.matches("RECONNECT_ATTEMPT").count(), 1);
            }
            "denied" => {
                assert!(app.output.contains("ServerError"));
                assert_eq!(app.output.matches("RECONNECT_ATTEMPT").count(), 1);
            }
            "deadline" => {
                assert!(app.output.contains("deadline expired while unavailable"));
                assert!(!app.output.contains("RECONNECT_ATTEMPT"));
                assert!(
                    started.elapsed() < Duration::from_millis(2750),
                    "deadline extended"
                );
            }
            "timeout" => {
                assert!(app.output.contains("connect/INIT timed out"));
                assert!(app.output.contains("deadline expired while unavailable"));
                assert_eq!(app.output.matches("RECONNECT_ATTEMPT").count(), 1);
                assert!(
                    started.elapsed() < Duration::from_millis(2750),
                    "connect timeout extended deadline"
                );
            }
            _ => {
                assert!(app.output.contains("attempt budget exhausted"));
                assert_eq!(
                    app.output.matches("RECONNECT_ATTEMPT").count(),
                    budget.parse::<usize>().unwrap()
                );
            }
        }
        eprintln!("RUST-OMS-008 {VERSION} negative control={control}: PASSED");
        drop(stalled);
        drop(server);
    }
    let offline = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://{}", offline.local_addr().unwrap());
    drop(offline);
    let mut initial = Application::start(&url, &["--reconnect-attempts", "8"]);
    initial.finish(false);
    assert!(initial.output.contains("connect/INIT"));
    assert!(!initial.output.contains("RECONNECT_ATTEMPT"));
    assert!(!initial.output.contains("CONNECTED"));
    eprintln!("RUST-OMS-008 {VERSION} bounded reconnection and terminal controls: PASSED");
}
