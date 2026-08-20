use std::collections::HashMap;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::oneshot;

use super::protocol::{ServiceEvent, ServiceMessage, ServiceRequest, SERVICE_PROTOCOL_VERSION};

type PendingResponse = oneshot::Sender<Result<Value, String>>;
type EventSink = Arc<dyn Fn(ServiceEvent) + Send + Sync>;

#[derive(Clone)]
pub struct ServiceClient {
    inner: Arc<ClientInner>,
}

struct ClientInner {
    writer: Mutex<Option<BufWriter<ChildStdin>>>,
    child: Mutex<Option<Child>>,
    pending: Mutex<HashMap<u64, PendingResponse>>,
    next_id: AtomicU64,
    closed: AtomicBool,
}

impl ServiceClient {
    pub fn spawn(
        executable: &Path,
        cache_dir: Option<PathBuf>,
        event_sink: EventSink,
    ) -> Result<Self, String> {
        let mut command = Command::new(executable);
        command
            .arg("--cemm-sidecar-service")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        if let Some(cache_dir) = cache_dir {
            command.env("CEMM_SERVICE_CACHE_DIR", cache_dir);
        }

        let mut child = command
            .spawn()
            .map_err(|error| format!("Failed to start local CEMM service: {error}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Local CEMM service did not expose stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Local CEMM service did not expose stdout".to_string())?;

        let inner = Arc::new(ClientInner {
            writer: Mutex::new(Some(BufWriter::new(stdin))),
            child: Mutex::new(Some(child)),
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            closed: AtomicBool::new(false),
        });

        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let reader_inner = Arc::clone(&inner);
        std::thread::Builder::new()
            .name("cemm-sidecar-reader".to_string())
            .spawn(move || {
                read_messages(stdout, &reader_inner, &event_sink, ready_tx);
            })
            .map_err(|error| format!("Failed to start local service reader: {error}"))?;

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(version)) if version == SERVICE_PROTOCOL_VERSION => Ok(Self { inner }),
            Ok(Ok(version)) => {
                inner.stop();
                Err(format!(
                    "Local CEMM service protocol mismatch: host {}, service {}",
                    SERVICE_PROTOCOL_VERSION, version
                ))
            }
            Ok(Err(error)) => {
                inner.stop();
                Err(error)
            }
            Err(_) => {
                inner.stop();
                Err("Local CEMM service did not become ready within 5 seconds".to_string())
            }
        }
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        if self.inner.closed.load(Ordering::Acquire) {
            return Err("Local CEMM service is not running".to_string());
        }

        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let request = ServiceRequest {
            id,
            method: method.to_string(),
            params,
        };
        let encoded = serde_json::to_string(&request)
            .map_err(|error| format!("Failed to encode local service request: {error}"))?;
        let (sender, receiver) = oneshot::channel();

        self.inner
            .pending
            .lock()
            .map_err(|_| "Local service response lock was poisoned".to_string())?
            .insert(id, sender);

        let write_result = self
            .inner
            .writer
            .lock()
            .map_err(|_| "Local service request lock was poisoned".to_string())?
            .as_mut()
            .ok_or_else(|| "Local CEMM service input is closed".to_string())
            .and_then(|writer| {
                writer
                    .write_all(encoded.as_bytes())
                    .and_then(|_| writer.write_all(b"\n"))
                    .and_then(|_| writer.flush())
                    .map_err(|error| format!("Failed to send local service request: {error}"))
            });

        if let Err(error) = write_result {
            if let Ok(mut pending) = self.inner.pending.lock() {
                pending.remove(&id);
            }
            return Err(error);
        }

        receiver
            .await
            .map_err(|_| "Local CEMM service stopped before responding".to_string())?
    }

    pub async fn call_typed<T: DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<T, String> {
        let value = self.call(method, params).await?;
        serde_json::from_value(value).map_err(|error| {
            format!("Local service returned an invalid {method} response: {error}")
        })
    }
}

impl ClientInner {
    fn stop(&self) {
        self.closed.store(true, Ordering::Release);
        if let Ok(mut writer) = self.writer.lock() {
            writer.take();
        }
        if let Ok(mut child) = self.child.lock() {
            if let Some(mut child) = child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        fail_pending(self, "Local CEMM service stopped");
    }
}

impl Drop for ClientInner {
    fn drop(&mut self) {
        self.stop();
    }
}

fn read_messages(
    stdout: std::process::ChildStdout,
    inner: &Arc<ClientInner>,
    event_sink: &EventSink,
    ready_tx: mpsc::SyncSender<Result<u32, String>>,
) {
    let mut lines = BufReader::new(stdout).lines();
    let ready = match lines.next() {
        Some(Ok(line)) => match serde_json::from_str::<ServiceMessage>(&line) {
            Ok(ServiceMessage::Ready { protocol_version }) => Ok(protocol_version),
            Ok(_) => Err("Local CEMM service sent data before its ready message".to_string()),
            Err(error) => Err(format!(
                "Local CEMM service sent an invalid ready message: {error}"
            )),
        },
        Some(Err(error)) => Err(format!(
            "Failed to read local service ready message: {error}"
        )),
        None => Err("Local CEMM service exited before becoming ready".to_string()),
    };

    if ready_tx.send(ready.clone()).is_err() || ready.is_err() {
        inner.closed.store(true, Ordering::Release);
        fail_pending(inner, "Local CEMM service failed during startup");
        return;
    }

    for line in lines {
        let message = match line {
            Ok(line) => match serde_json::from_str::<ServiceMessage>(&line) {
                Ok(message) => message,
                Err(error) => {
                    inner.closed.store(true, Ordering::Release);
                    fail_pending(
                        inner,
                        &format!("Local CEMM service sent invalid JSON: {error}"),
                    );
                    return;
                }
            },
            Err(error) => {
                inner.closed.store(true, Ordering::Release);
                fail_pending(
                    inner,
                    &format!("Failed to read local service output: {error}"),
                );
                return;
            }
        };

        match message {
            ServiceMessage::Response { id, result } => complete_pending(inner, id, Ok(result)),
            ServiceMessage::Error { id, error } => complete_pending(inner, id, Err(error)),
            ServiceMessage::Event { id, name, payload } => event_sink(ServiceEvent {
                request_id: id,
                name,
                payload,
            }),
            ServiceMessage::Ready { .. } => {
                inner.closed.store(true, Ordering::Release);
                fail_pending(inner, "Local CEMM service sent a duplicate ready message");
                return;
            }
        }
    }

    inner.closed.store(true, Ordering::Release);
    fail_pending(inner, "Local CEMM service exited");
}

fn complete_pending(inner: &ClientInner, id: u64, result: Result<Value, String>) {
    if let Ok(mut pending) = inner.pending.lock() {
        if let Some(sender) = pending.remove(&id) {
            let _ = sender.send(result);
        }
    }
}

fn fail_pending(inner: &ClientInner, message: &str) {
    if let Ok(mut pending) = inner.pending.lock() {
        for (_, sender) in pending.drain() {
            let _ = sender.send(Err(message.to_string()));
        }
    }
}
