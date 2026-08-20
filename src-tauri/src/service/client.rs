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
    supervisor: Arc<ServiceSupervisor>,
}

struct ServiceSupervisor {
    executable: PathBuf,
    cache_dir: Option<PathBuf>,
    event_sink: EventSink,
    inner: Mutex<Option<Arc<ClientInner>>>,
    shutting_down: AtomicBool,
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
        let supervisor = Arc::new(ServiceSupervisor {
            executable: executable.to_path_buf(),
            cache_dir,
            event_sink,
            inner: Mutex::new(None),
            shutting_down: AtomicBool::new(false),
        });
        supervisor.start_if_needed()?;
        Ok(Self { supervisor })
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        let inner = self.supervisor.start_if_needed()?;
        call_inner(&inner, method, params).await
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

    /// Explicitly replace the child process. In-flight requests are failed and
    /// never retried because a mutation may have completed before its response
    /// was lost.
    pub fn restart(&self) -> Result<(), String> {
        self.supervisor.restart()
    }

    pub fn shutdown(&self) {
        self.supervisor.shutdown();
    }
}

impl ServiceSupervisor {
    fn start_if_needed(&self) -> Result<Arc<ClientInner>, String> {
        if self.shutting_down.load(Ordering::Acquire) {
            return Err("Local CEMM service is shutting down".to_string());
        }

        let mut slot = self
            .inner
            .lock()
            .map_err(|_| "Local service supervisor lock was poisoned".to_string())?;
        if let Some(inner) = slot.as_ref() {
            if !inner.closed.load(Ordering::Acquire) {
                return Ok(Arc::clone(inner));
            }
        }

        if let Some(stopped) = slot.take() {
            stopped.stop();
        }
        let inner = spawn_child(
            &self.executable,
            self.cache_dir.clone(),
            Arc::clone(&self.event_sink),
        )?;
        *slot = Some(Arc::clone(&inner));
        Ok(inner)
    }

    fn restart(&self) -> Result<(), String> {
        if self.shutting_down.load(Ordering::Acquire) {
            return Err("Local CEMM service is shutting down".to_string());
        }
        let mut slot = self
            .inner
            .lock()
            .map_err(|_| "Local service supervisor lock was poisoned".to_string())?;
        if let Some(inner) = slot.take() {
            inner.stop();
        }
        let inner = spawn_child(
            &self.executable,
            self.cache_dir.clone(),
            Arc::clone(&self.event_sink),
        )?;
        *slot = Some(inner);
        Ok(())
    }

    fn shutdown(&self) {
        self.shutting_down.store(true, Ordering::Release);
        if let Ok(mut slot) = self.inner.lock() {
            if let Some(inner) = slot.take() {
                inner.stop();
            }
        }
    }
}

impl Drop for ServiceSupervisor {
    fn drop(&mut self) {
        self.shutting_down.store(true, Ordering::Release);
        if let Ok(slot) = self.inner.get_mut() {
            if let Some(inner) = slot.take() {
                inner.stop();
            }
        }
    }
}

fn spawn_child(
    executable: &Path,
    cache_dir: Option<PathBuf>,
    event_sink: EventSink,
) -> Result<Arc<ClientInner>, String> {
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
    let stdin = match child.stdin.take() {
        Some(stdin) => stdin,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Local CEMM service did not expose stdin".to_string());
        }
    };
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Local CEMM service did not expose stdout".to_string());
        }
    };

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
        Ok(Ok(version)) if version == SERVICE_PROTOCOL_VERSION => Ok(inner),
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

async fn call_inner(
    inner: &Arc<ClientInner>,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    if inner.closed.load(Ordering::Acquire) {
        return Err("Local CEMM service is not running".to_string());
    }

    let id = inner.next_id.fetch_add(1, Ordering::Relaxed);
    let request = ServiceRequest {
        id,
        method: method.to_string(),
        params,
    };
    let encoded = serde_json::to_string(&request)
        .map_err(|error| format!("Failed to encode local service request: {error}"))?;
    let (sender, receiver) = oneshot::channel();

    inner
        .pending
        .lock()
        .map_err(|_| "Local service response lock was poisoned".to_string())?
        .insert(id, sender);

    let write_result = inner
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
        remove_pending(inner, id);
        inner.stop();
        return Err(error);
    }

    match tokio::time::timeout(request_timeout(method), receiver).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err("Local CEMM service stopped before responding".to_string()),
        Err(_) => {
            remove_pending(inner, id);
            inner.stop();
            Err(format!(
                "Local CEMM service method '{method}' exceeded its {:?} deadline; the child was stopped and the operation result is unknown",
                request_timeout(method)
            ))
        }
    }
}

fn request_timeout(method: &str) -> Duration {
    match method {
        "ping" => Duration::from_secs(10),
        "file.read"
        | "file.write"
        | "config.read_directory"
        | "path.is_binary"
        | "path.validate"
        | "manifest.parse_instance"
        | "manifest.compare"
        | "library.scan" => Duration::from_secs(120),
        "library.cache_icons" => Duration::from_secs(10 * 60),
        "github.upload_update" | "github.download_manifest" | "github.download_config_files" => {
            Duration::from_secs(30 * 60)
        }
        "install.apply_update" => Duration::from_secs(2 * 60 * 60),
        _ => Duration::from_secs(5 * 60),
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

    fn reap_exited_child(&self) {
        if let Ok(mut child) = self.child.lock() {
            if let Some(mut child) = child.take() {
                let _ = child.wait();
            }
        }
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
                    fail_protocol(
                        inner,
                        &format!("Local CEMM service sent invalid JSON: {error}"),
                    );
                    return;
                }
            },
            Err(error) => {
                fail_protocol(
                    inner,
                    &format!("Failed to read local service output: {error}"),
                );
                return;
            }
        };

        match message {
            ServiceMessage::Response { id, result } => {
                if !complete_pending(inner, id, Ok(result)) {
                    fail_protocol(inner, "Local CEMM service returned an unknown request ID");
                    return;
                }
            }
            ServiceMessage::Error { id, error } => {
                if !complete_pending(inner, id, Err(error)) {
                    fail_protocol(inner, "Local CEMM service returned an unknown request ID");
                    return;
                }
            }
            ServiceMessage::Event { id, name, payload } => {
                if !has_pending(inner, id) {
                    fail_protocol(
                        inner,
                        "Local CEMM service emitted an event for an unknown request ID",
                    );
                    return;
                }
                event_sink(ServiceEvent {
                    request_id: id,
                    name,
                    payload,
                });
            }
            ServiceMessage::Ready { .. } => {
                fail_protocol(inner, "Local CEMM service sent a duplicate ready message");
                return;
            }
        }
    }

    inner.closed.store(true, Ordering::Release);
    fail_pending(inner, "Local CEMM service exited");
    inner.reap_exited_child();
}

fn fail_protocol(inner: &ClientInner, message: &str) {
    inner.closed.store(true, Ordering::Release);
    fail_pending(inner, message);
    inner.stop();
}

fn has_pending(inner: &ClientInner, id: u64) -> bool {
    inner
        .pending
        .lock()
        .map(|pending| pending.contains_key(&id))
        .unwrap_or(false)
}

fn complete_pending(inner: &ClientInner, id: u64, result: Result<Value, String>) -> bool {
    if let Ok(mut pending) = inner.pending.lock() {
        if let Some(sender) = pending.remove(&id) {
            let _ = sender.send(result);
            return true;
        }
    }
    false
}

fn remove_pending(inner: &ClientInner, id: u64) {
    if let Ok(mut pending) = inner.pending.lock() {
        pending.remove(&id);
    }
}

fn fail_pending(inner: &ClientInner, message: &str) {
    if let Ok(mut pending) = inner.pending.lock() {
        for (_, sender) in pending.drain() {
            let _ = sender.send(Err(message.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_service_method_has_a_finite_deadline() {
        let local = request_timeout("file.read");
        let network = request_timeout("github.upload_update");
        let install = request_timeout("install.apply_update");

        assert!(local < network);
        assert!(network < install);
        assert!(install <= Duration::from_secs(2 * 60 * 60));
    }
}
