use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use cemm_lib::service::protocol::Method;
use cemm_lib::service::ServiceClient;

fn spawn_client() -> ServiceClient {
    ServiceClient::spawn(
        Path::new(env!("CARGO_BIN_EXE_cemm")),
        None,
        Arc::new(|_| {}),
    )
    .expect("compiled CEMM binary should start in sidecar mode")
}

#[tokio::test]
async fn client_can_restart_the_real_sidecar_without_retrying_a_request() {
    let client = spawn_client();
    let first = client
        .call(Method::Ping, serde_json::Value::Null)
        .await
        .expect("first child should answer ping");
    assert_eq!(first["protocolVersion"], 1);

    client.restart().expect("sidecar restart should succeed");
    let second = client
        .call(Method::Ping, serde_json::Value::Null)
        .await
        .expect("replacement child should answer ping");
    assert_eq!(second["protocolVersion"], 1);
}

#[tokio::test]
async fn shutdown_is_terminal_and_rejects_later_requests() {
    let client = spawn_client();
    client.shutdown();

    let error = client
        .call(Method::Ping, serde_json::Value::Null)
        .await
        .expect_err("a shut down supervisor must not respawn");
    assert!(error.contains("shutting down"));
}

#[test]
fn compiled_binary_speaks_protocol_and_exits_when_stdin_closes() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cemm"))
        .arg("--cemm-sidecar-service")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("compiled CEMM binary should start");
    let mut stdin = child.stdin.take().expect("sidecar stdin should be piped");
    let stdout = child.stdout.take().expect("sidecar stdout should be piped");
    let (line_tx, line_rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if line_tx.send(line).is_err() {
                break;
            }
        }
    });

    let read_message = || {
        let line = line_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("sidecar should respond within five seconds")
            .expect("sidecar output should be readable");
        serde_json::from_str::<serde_json::Value>(&line)
            .expect("sidecar output should be newline-delimited JSON")
    };

    let ready = read_message();
    assert_eq!(ready["type"], "ready");
    assert_eq!(ready["protocol_version"], 1);

    stdin
        .write_all(b"{\"id\":1,\"method\":\"ping\",\"params\":{}}\n")
        .expect("ping should be writable");
    stdin.flush().expect("ping should flush");
    let ping = read_message();
    assert_eq!(ping["type"], "response");
    assert_eq!(ping["id"], 1);
    assert_eq!(ping["result"]["protocolVersion"], 1);

    stdin
        .write_all(b"{\"id\":2,\"method\":\"not-a-real-method\",\"params\":{}}\n")
        .expect("unknown method should be writable");
    stdin.flush().expect("unknown method should flush");
    let error = read_message();
    assert_eq!(error["type"], "error");
    assert_eq!(error["id"], 2);
    assert!(error["error"]
        .as_str()
        .is_some_and(|message| message.contains("Unknown sidecar service method")));

    drop(stdin);
    let deadline = Instant::now() + Duration::from_secs(5);
    let exit_status = loop {
        if let Some(status) = child.try_wait().expect("sidecar status should be readable") {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "sidecar did not exit after its inherited stdin closed"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(exit_status.success());
    reader.join().expect("sidecar reader should finish");
}
