use std::io::{self, BufRead, BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Deserialize;
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
        "file.read" => {
            let params: PathParams = decode_params(request)?;
            encode_result(crate::read_file(params.path))
        }
        "file.write" => {
            let params: WriteFileParams = decode_params(request)?;
            encode_result(crate::write_file(
                params.path,
                params.content,
                params.dir,
                params.files,
            ))
        }
        "config.read_directory" => {
            let params: ReadDirectoryParams = decode_params(request)?;
            encode_result(crate::read_directory_recursive(
                params.dir_path,
                params.base_path,
            ))
        }
        "path.is_binary" => {
            let params: PathParams = decode_params(request)?;
            encode_result(crate::is_binary_file(params.path))
        }
        "path.validate" => {
            let params: PathParams = decode_params(request)?;
            crate::validate_path(params.path)
        }
        "manifest.parse_instance" => {
            let params: PathParams = decode_params(request)?;
            encode_result(crate::composables::manifest::parse_minecraft_instance(
                params.path,
            ))
        }
        "manifest.compare" => {
            let params: CompareManifestParams = decode_params(request)?;
            encode_result(crate::composables::manifest::compare_manifests(
                params.old, params.new,
            ))
        }
        method => Err(format!("Unknown sidecar service method: {method}")),
    }
}

fn decode_params<T: for<'de> Deserialize<'de>>(request: &ServiceRequest) -> Result<T, String> {
    serde_json::from_value(request.params.clone()).map_err(|error| {
        format!(
            "Invalid parameters for sidecar method '{}': {error}",
            request.method
        )
    })
}

fn encode_result<T: serde::Serialize>(result: Result<T, String>) -> Result<Value, String> {
    let value = result?;
    serde_json::to_value(value)
        .map_err(|error| format!("Failed to encode sidecar service result: {error}"))
}

#[derive(Deserialize)]
struct PathParams {
    path: String,
}

#[derive(Deserialize)]
struct WriteFileParams {
    path: Option<String>,
    content: Option<String>,
    dir: Option<String>,
    files: Option<Vec<(String, String)>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadDirectoryParams {
    dir_path: String,
    base_path: String,
}

#[derive(Deserialize)]
struct CompareManifestParams {
    old: crate::composables::manifest::Manifest,
    new: crate::composables::manifest::Manifest,
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
