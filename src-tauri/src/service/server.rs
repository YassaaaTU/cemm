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
type EventCallback = Arc<dyn Fn(&str, Value) + Send + Sync>;

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

async fn dispatch(
    request: &ServiceRequest,
    context: &ServiceContext,
    event_callback: &EventCallback,
) -> Result<Value, String> {
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
        "github.upload_update" => {
            let params: UploadUpdateParams = decode_params(request)?;
            let events = Arc::clone(event_callback);
            let progress_callback = Arc::new(
                move |progress: crate::composables::github::UploadProgress| {
                    events(
                        "upload_progress",
                        serde_json::to_value(progress).unwrap_or(Value::Null),
                    );
                },
            );
            encode_result(
                crate::composables::github::upload_update_with_progress(
                    params.repo,
                    params.token,
                    params.uuid,
                    params.modpack_key,
                    params.manifest,
                    params.config_files,
                    progress_callback,
                )
                .await,
            )
        }
        "github.download_manifest" => {
            let params: DownloadManifestParams = decode_params(request)?;
            encode_result(
                crate::composables::github::download_manifest(
                    params.repo,
                    params.uuid,
                    params.modpack_key,
                )
                .await,
            )
        }
        "github.download_config_files" => {
            let params: DownloadConfigFilesParams = decode_params(request)?;
            encode_result(
                crate::composables::github::download_config_files(
                    params.repo,
                    params.uuid,
                    params.modpack_key,
                    params.manifest,
                )
                .await,
            )
        }
        "library.scan" => {
            let params: ScanLibraryParams = decode_params(request)?;
            let icon_cache = context.cache_dir.as_ref().map(|dir| dir.join("pack-icons"));
            encode_result(crate::composables::instances::scan_library(
                params.instances_dir,
                icon_cache.as_deref(),
            ))
        }
        "library.cache_icons" => {
            let params: CacheIconsParams = decode_params(request)?;
            let cache_dir = context
                .cache_dir
                .as_ref()
                .ok_or_else(|| "Local service cache directory is not configured".to_string())?
                .join("pack-icons");
            encode_result(
                crate::composables::instances::cache_pack_icons_in(cache_dir, params.urls).await,
            )
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadUpdateParams {
    repo: String,
    token: String,
    uuid: String,
    modpack_key: Option<String>,
    manifest: crate::composables::manifest::Manifest,
    config_files: Vec<crate::composables::github::ConfigFileWithContent>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadManifestParams {
    repo: String,
    uuid: String,
    modpack_key: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadConfigFilesParams {
    repo: String,
    uuid: String,
    modpack_key: Option<String>,
    manifest: crate::composables::manifest::Manifest,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScanLibraryParams {
    instances_dir: Option<String>,
}

#[derive(Deserialize)]
struct CacheIconsParams {
    urls: Vec<String>,
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

        let request_id = request.id;
        let event_output = Arc::clone(&output);
        let event_callback: EventCallback = Arc::new(move |name, payload| {
            let _ = write_message(
                &event_output,
                &ServiceMessage::Event {
                    id: request_id,
                    name: name.to_string(),
                    payload,
                },
            );
        });

        let response = match runtime.block_on(dispatch(&request, &context, &event_callback)) {
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

    fn no_events() -> EventCallback {
        Arc::new(|_, _| {})
    }

    #[tokio::test]
    async fn ping_reports_the_protocol_version() {
        let request = ServiceRequest {
            id: 1,
            method: "ping".to_string(),
            params: Value::Null,
        };

        let result = dispatch(&request, &context(), &no_events())
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

        let error = dispatch(&request, &context(), &no_events())
            .await
            .expect_err("unknown method should fail");
        assert!(error.contains("Unknown sidecar service method"));
    }
}
