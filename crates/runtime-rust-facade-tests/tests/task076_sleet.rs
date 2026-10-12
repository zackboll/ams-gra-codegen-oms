//! Research-only raw OWP differential. Requested execution never silently skips.
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const WAIT: Duration = Duration::from_secs(15);
struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn probe(
    url: &str,
    version: &str,
    name: &str,
    raw: &str,
    control: &str,
) -> Result<Value, String> {
    let deadline = tokio::time::Instant::now() + WAIT;
    let (mut ws, _) = loop {
        let mut request = url.into_client_request().map_err(|e| e.to_string())?;
        request
            .headers_mut()
            .insert("Sec-WebSocket-Protocol", "owp".parse().unwrap());
        match connect_async(request).await {
            Ok(v) => break v,
            Err(_) if tokio::time::Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(e) => return Err(format!("connect: {e}")),
        }
    };
    ws.send(Message::Text(format!("INIT {{\"versions\":[\"1.0\"],\"schema\":\"{version}\",\"service_id\":\"task076\",\"verbose\":true}}").into())).await.map_err(|e| e.to_string())?;
    let mut info = false;
    while !info {
        let frame = ws
            .next()
            .await
            .ok_or("INIT closed")?
            .map_err(|e| e.to_string())?;
        if let Message::Text(t) = frame {
            if t.starts_with("-ERR") {
                return Ok(json!({"stage":"broker-configuration", "details":t.as_str()}));
            }
            info = t.starts_with("INFO ");
        }
    }
    // Bare subscription follows pinned loader's name identity, not a rewrite of input.
    ws.send(Message::Text(format!("SUB c {name} task076").into()))
        .await
        .map_err(|e| e.to_string())?;
    loop {
        let frame = ws
            .next()
            .await
            .ok_or("SUB closed")?
            .map_err(|e| e.to_string())?;
        if let Message::Text(t) = frame {
            if t.starts_with("-ERR") {
                return Ok(json!({"stage":"broker-authorization", "details":t.as_str()}));
            }
            if t.starts_with("+OK") {
                break;
            }
        }
    }
    // Matched positive control confirms active subscription and authorization.
    ws.send(Message::Text(format!("PUB task076 {control}").into()))
        .await
        .map_err(|e| e.to_string())?;
    let mut delivered = false;
    let mut acknowledged = false;
    while !delivered || !acknowledged {
        let frame = ws
            .next()
            .await
            .ok_or("control closed")?
            .map_err(|e| e.to_string())?;
        if let Message::Text(t) = frame {
            if t.starts_with("-ERR") {
                return Ok(json!({"stage":"positive-control-rejection", "details":t.as_str()}));
            }
            if t.starts_with("MSG ") {
                let payload = t.splitn(3, ' ').nth(2).ok_or("control MSG shape")?;
                assert_eq!(
                    serde_json::from_str::<Value>(payload).unwrap(),
                    serde_json::from_str::<Value>(control).unwrap()
                );
                delivered = true;
            }
            acknowledged |= t.starts_with("+OK");
        }
    }
    ws.send(Message::Text(format!("PUB task076 {raw}").into()))
        .await
        .map_err(|e| e.to_string())?;
    let mut payload = None;
    acknowledged = false;
    while payload.is_none() || !acknowledged {
        let frame = ws
            .next()
            .await
            .ok_or("case closed")?
            .map_err(|e| e.to_string())?;
        if let Message::Text(t) = frame {
            if t.starts_with("-ERR") {
                let stage = if t.contains("schema validation failed") {
                    "broker-message-validation"
                } else if t.contains("invalid JSON") {
                    "broker-json-syntax"
                } else {
                    "broker-configuration-or-authorization"
                };
                return Ok(json!({"stage":stage, "details":t.as_str(), "control_delivered":true}));
            }
            if t.starts_with("MSG ") {
                let text = t.splitn(3, ' ').nth(2).ok_or("MSG shape")?;
                let value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
                // Semantic identity comparison; duplicates separately retain raw digest.
                assert_eq!(value, serde_json::from_str::<Value>(raw).unwrap());
                payload = Some(value);
            }
            acknowledged |= t.starts_with("+OK");
        }
    }
    ws.close(None).await.map_err(|e| e.to_string())?;
    Ok(json!({"stage":"delivered", "payload":payload, "control_delivered":true}))
}

#[test]
#[ignore = "requires explicit pinned broker campaign; wrapper requires execution marker"]
fn task076_broker() {
    let binary = std::env::var_os("AMS_GRA_SLEET_BIN").expect("missing pinned broker");
    let campaign = std::env::var_os("TASK076_CAMPAIGN").expect("missing campaign");
    let scratch = PathBuf::from(std::env::var_os("TASK076_SCRATCH").expect("missing scratch"));
    let campaign: Value = serde_json::from_slice(&std::fs::read(campaign).unwrap()).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut output = serde_json::Map::new();
    for (group, bundle) in campaign["bundles"].as_object().unwrap() {
        let dir = scratch.join(format!("server-{group}"));
        std::fs::create_dir_all(dir.join("services")).unwrap();
        std::fs::write(dir.join("services/task076.toml"), "service_id = \"task076\"\nservice_uuid = \"550e8400-e29b-41d4-a716-446655440076\"\nallowed_topics = [\"task076\"]\n").unwrap();
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        // Task074 overlay intentionally has no version; do not patch it for Sleet.
        let version = if group.starts_with("2.5") {
            "002.5.0"
        } else if group.starts_with("2.6") {
            "002.6.0"
        } else {
            "000.1.0"
        };
        let schema = PathBuf::from(bundle["root"].as_str().unwrap())
            .join(bundle["manifest"]["root"].as_str().unwrap());
        std::fs::write(dir.join("server.toml"),format!("bind_addr = \"127.0.0.1:{port}\"\nserver_id = \"task076\"\nsystem_label = \"Task076\"\nschema_path = \"{}\"\nschema_version = \"{version}\"\nservices_dir = \"{}\"\n", schema.display(), dir.join("services").display())).unwrap();
        let log = std::fs::File::create(dir.join("server.log")).unwrap();
        let mut server = Server(
            Command::new(&binary)
                .arg(dir.join("server.toml"))
                .env("RUST_LOG", "info")
                .env("NO_COLOR", "1")
                .stdout(Stdio::from(log.try_clone().unwrap()))
                .stderr(Stdio::from(log))
                .spawn()
                .unwrap(),
        );
        let rows: Vec<_> = campaign["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["bundle"] == *group)
            .collect();
        for row in rows {
            let id = row["id"].as_str().unwrap();
            let raw = std::fs::read_to_string(row["raw_path"].as_str().unwrap()).unwrap();
            let name = row["message_qname"]
                .as_str()
                .unwrap()
                .rsplit('}')
                .next()
                .unwrap();
            let control_id = if group.starts_with("2.") {
                format!("uci-{}-known-add", &group[..3])
            } else if name == "ConcreteMessage" || name == "Blocked" {
                "concrete-implicit".to_owned()
            } else {
                "known".to_owned()
            };
            // Matched declared-concrete positive control for blocked substitution.
            let control_row = campaign["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["id"] == control_id)
                .unwrap();
            let mut control =
                std::fs::read_to_string(control_row["raw_path"].as_str().unwrap()).unwrap();
            if name == "Blocked" {
                control = control.replace("\"ConcreteMessage\"", "\"Blocked\"");
            }
            let result = runtime.block_on(async {
                tokio::time::timeout(
                    Duration::from_secs(35),
                    probe(
                        &format!("ws://127.0.0.1:{port}"),
                        version,
                        name,
                        &raw,
                        &control,
                    ),
                )
                .await
            });
            let observation = match result {
                Ok(Ok(value)) => value,
                Ok(Err(e)) => {
                    if let Some(status) = server.0.try_wait().unwrap() {
                        let log = std::fs::read_to_string(dir.join("server.log")).unwrap();
                        // Exact startup error retained; never payload rejection.
                        let detail = log
                            .lines()
                            .find(|s| s.contains("ERROR"))
                            .unwrap_or(&log)
                            .to_owned();
                        let detail = detail
                            .split("ERROR")
                            .nth(1)
                            .unwrap_or(&detail)
                            .trim()
                            .to_owned();
                        json!({"stage":"server-startup-failure", "exit":status.code(),"details":detail})
                    } else {
                        json!({"stage":"infrastructure-inconclusive","details":e})
                    }
                }
                Err(_) => json!({"stage":"timeout-inconclusive"}),
            };
            output.insert(id.to_owned(), observation);
        }
    }
    std::fs::write(
        scratch.join("broker.json"),
        serde_json::to_vec_pretty(&Value::Object(output)).unwrap(),
    )
    .unwrap();
    eprintln!("TASK076 BROKER: EXECUTED");
}
