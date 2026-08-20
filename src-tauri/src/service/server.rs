use std::io::{self, BufRead, BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use super::protocol::{ServiceMessage, ServiceRequest, SERVICE_PROTOCOL_VERSION};

#[derive(Debug, Clone)]
pub struct ServiceContext {
    pub cache_dir: Option<PathBuf>,
}

impl ServiceContext {
    fn from_environment() -> Self {
        Self {
            cache_dir: std::env::var_os("CEMM_SERVICE_CACHE_DIR").map(PathBuf::from),
        }
    }
}

type SharedOutput = Arc<Mutex<BufWriter<io::Stdout>>>;

fn write_message(output: &SharedOutput, message: &ServiceMessage) -> Result<(), String> {
    let encoded = serde_json::to_string(message)
        .map_err(|error| format!("Failed to encode sidecar message: {error}"))?;
    let mut writer = output
        .lock()
        .map_err(|_| "Sidecar output lock was poisoned".to_string())?;
    writer
        .write_all(encoded.as_bytes())
        .and_then(|_| writer.write_all(b"\n"))
        .and_then(|_| writer.flush())
        .map_err(|error| format!("Failed to write sidecar message: {error}"))
}

async fn dispatch(request: &ServiceRequest, context: &ServiceContext) -> Result<Value, String> {
    match request.method.as_str() {
        "ping" => Ok(serde_json::json!({
            "protocolVersion": SERVICE_PROTOCOL_VERSION,
            "cacheDirectoryConfigured": context.cache_dir.is_some()
        })),
        method => Err(format!("Unknown sidecar service method: {method}")),
    }
}

pub fn run_stdio_service() -> i32 {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("Failed to start sidecar runtime: {error}");
            return 1;
        }
    };

    let context = ServiceContext::from_environment();
    let output = Arc::new(Mutex::new(BufWriter::new(io::stdout())));
    if let Err(error) = write_message(
        &output,
        &ServiceMessage::Ready {
            protocol_version: SERVICE_PROTOCOL_VERSION,
        },
    ) {
        eprintln!("{error}");
        return 1;
    }

    for line in io::stdin().lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("Failed to read sidecar request: {error}");
                return 1;
            }
        };

        let request = match serde_json::from_str::<ServiceRequest>(&line) {
            Ok(request) => request,
            Err(error) => {
                let _ = write_message(
                    &output,
                    &ServiceMessage::Error {
                        id: 0,
                        error: format!("Malformed sidecar request: {error}"),
                    },
                );
                continue;
            }
        };

        let response = match runtime.block_on(dispatch(&request, &context)) {
            Ok(result) => ServiceMessage::Response {
                id: request.id,
                result,
            },
            Err(error) => ServiceMessage::Error {
                id: request.id,
                error,
            },
        };

        if let Err(error) = write_message(&output, &response) {
            eprintln!("{error}");
            return 1;
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> ServiceContext {
        ServiceContext { cache_dir: None }
    }

    #[tokio::test]
    async fn ping_reports_the_protocol_version() {
        let request = ServiceRequest {
            id: 1,
            method: "ping".to_string(),
            params: Value::Null,
        };

        let result = dispatch(&request, &context())
            .await
            .expect("ping should succeed");
        assert_eq!(result["protocolVersion"], SERVICE_PROTOCOL_VERSION);
    }

    #[tokio::test]
    async fn unknown_methods_fail_without_panicking() {
        let request = ServiceRequest {
            id: 2,
            method: "not-a-real-method".to_string(),
            params: Value::Null,
        };

        let error = dispatch(&request, &context())
            .await
            .expect_err("unknown method should fail");
        assert!(error.contains("Unknown sidecar service method"));
    }
}
