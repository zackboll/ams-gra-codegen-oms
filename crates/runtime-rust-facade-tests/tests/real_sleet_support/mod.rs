struct Server {
    child: Child,
    directory: PathBuf,
}

impl Server {
    fn shutdown(&mut self) {
        self.child.kill().expect("stop qualification broker");
        self.child.wait().expect("reap qualification broker");
        std::fs::remove_dir_all(&self.directory).expect("remove broker configuration");
        assert!(!self.directory.exists());
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[allow(dead_code)] // Combined-service consumers use start_sleet_with_topics.
fn start_sleet() -> (Server, String) {
    start_sleet_with_topics(&[TOPIC])
}

fn start_sleet_with_topics(topics: &[&str]) -> (Server, String) {
    let binary = std::env::var_os("AMS_GRA_SLEET_BIN").expect("pinned Sleet binary required");
    let directory =
        std::env::temp_dir().join(format!("real-uci-{TOPIC}-{VERSION}-{}", std::process::id()));
    let services = directory.join("services");
    std::fs::create_dir_all(&services).unwrap();
    std::fs::write(services.join("service.toml"), format!(
        "service_id = \"{TOPIC}\"\nservice_uuid = \"550e8400-e29b-41d4-a716-446655440102\"\nallowed_topics = {topics:?}\n"
    )).unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let config = directory.join("server.toml");
    std::fs::write(&config, format!(
        "bind_addr = \"127.0.0.1:{port}\"\nserver_id = \"real-uci\"\nsystem_label = \"Rust generated UCI\"\nschema_path = \"{ROOT}\"\nschema_version = \"{VERSION}\"\nservices_dir = \"{}\"\n", services.display()
    )).unwrap();
    let child = Command::new(binary)
        .arg(config)
        .env("RUST_LOG", "warn")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    (
        Server { child, directory },
        format!("ws://127.0.0.1:{port}"),
    )
}

fn connect(url: &str) -> SleetRuntime<ServiceCodec> {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match SleetRuntime::connect(
            RuntimeConfig::new(url, TOPIC, VERSION).with_connect_timeout(WAIT),
            ServiceCodec,
        ) {
            Ok(runtime) => return runtime,
            Err(_) if Instant::now() < deadline => std::thread::sleep(QUIET),
            Err(error) => panic!("Sleet connect/INIT: {error}"),
        }
    }
}

async fn raw_connect(url: &str) -> CalClient {
    tokio::time::timeout(
        WAIT,
        CalClient::connect_with_options(
            url,
            TOPIC,
            InitOptions {
                schema: VERSION.to_owned(),
                ..InitOptions::default()
            },
        ),
    )
    .await
    .unwrap()
    .unwrap()
}

async fn raw_publish(
    client: &mut CalClient,
    topic: &str,
    document: &Value,
) -> Result<(), ClientError> {
    tokio::time::timeout(WAIT, client.publish(topic, document))
        .await
        .expect("PUB response timeout")
}
