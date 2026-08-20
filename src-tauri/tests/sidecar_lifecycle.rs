use std::path::Path;
use std::sync::Arc;

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
        .call("ping", serde_json::Value::Null)
        .await
        .expect("first child should answer ping");
    assert_eq!(first["protocolVersion"], 1);

    client.restart().expect("sidecar restart should succeed");
    let second = client
        .call("ping", serde_json::Value::Null)
        .await
        .expect("replacement child should answer ping");
    assert_eq!(second["protocolVersion"], 1);
}

#[tokio::test]
async fn shutdown_is_terminal_and_rejects_later_requests() {
    let client = spawn_client();
    client.shutdown();

    let error = client
        .call("ping", serde_json::Value::Null)
        .await
        .expect_err("a shut down supervisor must not respawn");
    assert!(error.contains("shutting down"));
}
