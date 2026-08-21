use std::io::{self, BufRead, BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use serde_json::Value;

use super::protocol::{Method, ServiceMessage, ServiceRequest, SERVICE_PROTOCOL_VERSION};

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
    let method = Method::from_wire(&request.method)
        .ok_or_else(|| format!("Unknown sidecar service method: {}", request.method))?;

    match method {
        Method::Ping => Ok(serde_json::json!({
            "protocolVersion": SERVICE_PROTOCOL_VERSION,
            "cacheDirectoryConfigured": context.cache_dir.is_some()
        })),
        Method::FileRead => {
            let params: PathParams = decode_params(request)?;
            encode_result(crate::read_file(params.path))
        }
        Method::FileWrite => {
            let params: WriteFileParams = decode_params(request)?;
            encode_result(crate::write_file(
                params.path,
                params.content,
                params.dir,
                params.files,
            ))
        }
        Method::ConfigReadDirectory => {
            let params: ReadDirectoryParams = decode_params(request)?;
            encode_result(crate::read_directory_recursive(
                params.dir_path,
                params.base_path,
            ))
        }
        Method::PathIsBinary => {
            let params: PathParams = decode_params(request)?;
            encode_result(crate::is_binary_file(params.path))
        }
        Method::PathValidate => {
            let params: PathParams = decode_params(request)?;
            crate::validate_path(params.path)
        }
        Method::ManifestParseInstance => {
            let params: PathParams = decode_params(request)?;
            encode_result(crate::composables::manifest::parse_minecraft_instance(
                params.path,
            ))
        }
        Method::ManifestDiff => {
            let params: DiffManifestParams = decode_params(request)?;
            encode_result(crate::installer::update_diff(
                params.old.as_ref(),
                &params.new,
            ))
        }
        Method::GithubUploadUpdate => {
            let params: UploadUpdateParams = decode_params(request)?;
            let events = Arc::clone(event_callback);
            let operation_id = params.operation_id.clone();
            let progress_callback = Arc::new(
                move |progress: crate::composables::github::UploadProgress| {
                    events(
                        "upload_progress",
                        serde_json::json!({
                            "operationId": operation_id,
                            "progress": progress.progress,
                            "message": progress.message
                        }),
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
        Method::GithubDownloadManifest => {
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
        Method::GithubDownloadConfigFiles => {
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
        Method::LibraryScan => {
            let params: ScanLibraryParams = decode_params(request)?;
            let icon_cache = context.cache_dir.as_ref().map(|dir| dir.join("pack-icons"));
            encode_result(crate::composables::instances::scan_library(
                params.instances_dir,
                icon_cache.as_deref(),
            ))
        }
        Method::LibraryCacheIcons => {
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
        Method::InstallApplyUpdate => {
            let params: InstallUpdateParams = decode_params(request)?;
            let events = Arc::clone(event_callback);
            let operation_id = params.operation_id.clone();
            let progress_callback = Arc::new(move |progress: crate::installer::InstallProgress| {
                events(
                    "install-progress",
                    serde_json::json!({
                        "operationId": operation_id,
                        "progress": progress.progress,
                        "message": progress.message
                    }),
                );
            });
            encode_result(
                crate::installer::install_update_with_progress(
                    params.modpack_path,
                    params.manifest,
                    params.config_files,
                    params.options,
                    progress_callback,
                )
                .await,
            )
        }
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
struct DiffManifestParams {
    /// Absent on a first install, where every enabled addon is new.
    #[serde(default)]
    old: Option<crate::composables::manifest::Manifest>,
    new: crate::composables::manifest::Manifest,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadUpdateParams {
    operation_id: String,
    repo: String,
    token: String,
    uuid: String,
    modpack_key: Option<String>,
    manifest: crate::composables::manifest::Manifest,
    config_files: Vec<crate::composables::manifest::ConfigFileWithContent>,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallUpdateParams {
    operation_id: String,
    modpack_path: String,
    manifest: crate::composables::manifest::Manifest,
    config_files: Vec<crate::composables::manifest::ConfigFileWithContent>,
    options: Option<crate::installer::InstallOptions>,
}

/// Where the sidecar's `log::` calls go.
///
/// `tauri_plugin_log` is installed by the host process and never by this one,
/// so until now every `log::` call in domain code ran into the no-op default
/// logger: the process that does all the file and network work had no
/// diagnostics at all, which is why the code that most needed them reached for
/// `eprintln!` directly and shipped request URLs to anyone watching stderr.
///
/// stderr only, and deliberately so -- stdout carries the protocol, and a stray
/// line on it is read by the host as a malformed message.
struct StderrLogger;

/// Debug while developing, warnings only in a shipped build: a background
/// service should be quiet until something is wrong.
const fn sidecar_log_level() -> log::LevelFilter {
    if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Warn
    }
}

impl log::Log for StderrLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= sidecar_log_level()
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        eprintln!(
            "[cemm-sidecar] {:<5} {}: {}",
            record.level(),
            record.target(),
            record.args()
        );
    }

    fn flush(&self) {}
}

static SIDECAR_LOGGER: StderrLogger = StderrLogger;

fn install_sidecar_logger() {
    if log::set_logger(&SIDECAR_LOGGER).is_ok() {
        log::set_max_level(sidecar_log_level());
    }
}

pub fn run_stdio_service() -> i32 {
    install_sidecar_logger();

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            log::error!("Failed to start sidecar runtime: {error}");
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
        log::error!("{error}");
        return 1;
    }

    for line in io::stdin().lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                log::error!("Failed to read sidecar request: {error}");
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
            log::error!("{error}");
            return 1;
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_manifest(update_type: &str) -> Value {
        serde_json::json!({
            "updateType": update_type,
            "mods": [],
            "resourcepacks": [],
            "shaderpacks": [],
            "datapacks": [],
            "config_files": []
        })
    }

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

    #[tokio::test]
    async fn publish_contract_accepts_frontend_payload_shape_before_domain_validation() {
        let request = ServiceRequest {
            id: 3,
            method: "github.upload_update".to_string(),
            params: serde_json::json!({
                "operationId": "publish-test",
                "repo": "invalid-repository",
                "token": "test-token",
                "uuid": "test-update",
                "modpackKey": "test-pack",
                "manifest": empty_manifest("full"),
                "configFiles": []
            }),
        };

        let events = Arc::new(Mutex::new(Vec::new()));
        let captured_events = Arc::clone(&events);
        let event_callback: EventCallback = Arc::new(move |name, payload| {
            captured_events
                .lock()
                .expect("event lock should not be poisoned")
                .push((name.to_string(), payload));
        });
        let error = dispatch(&request, &context(), &event_callback)
            .await
            .expect_err("invalid repository should fail before network access");
        assert!(
            error.contains("Invalid repo format"),
            "frontend payload should decode before domain validation: {error}"
        );
        assert!(events.lock().unwrap().iter().any(|(name, payload)| {
            name == "upload_progress" && payload["operationId"] == "publish-test"
        }));
    }

    #[tokio::test]
    async fn install_contract_writes_only_to_temp_instance_and_forwards_progress() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let request = ServiceRequest {
            id: 4,
            method: "install.apply_update".to_string(),
            params: serde_json::json!({
                "operationId": "install-test",
                "modpackPath": temp.path().to_string_lossy(),
                "manifest": empty_manifest("config"),
                "configFiles": [{
                    "filename": "service.toml",
                    "relative_path": "config/service.toml",
                    "content": "sidecar = true"
                }],
                "options": null
            }),
        };
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured_events = Arc::clone(&events);
        let event_callback: EventCallback = Arc::new(move |name, payload| {
            captured_events
                .lock()
                .expect("event lock should not be poisoned")
                .push((name.to_string(), payload));
        });

        dispatch(&request, &context(), &event_callback)
            .await
            .expect("frontend install payload should complete through dispatch");

        assert_eq!(
            tokio::fs::read_to_string(temp.path().join("config/service.toml"))
                .await
                .expect("installed config should be readable"),
            "sidecar = true"
        );
        assert!(temp.path().join("cemm-manifest.json").exists());
        let events = events.lock().expect("event lock should not be poisoned");
        assert!(events.iter().any(|(name, payload)| {
            name == "install-progress"
                && payload["operationId"] == "install-test"
                && payload["progress"] == serde_json::json!(100.0)
        }));
    }
}
