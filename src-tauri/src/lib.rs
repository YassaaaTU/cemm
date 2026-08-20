use std::fs;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};

pub mod service;
mod service_commands;

// Helper function to normalize Windows extended paths
fn normalize_path(path: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        // Remove Windows extended path prefix \\?\
        if let Some(stripped) = path.strip_prefix(r"\\?\") {
            return stripped.to_string();
        }
    }
    path.to_string()
}

mod composables {
    pub mod github;
    pub mod instances;
    pub mod manifest;
}

pub use composables::github::{download_config_files, download_manifest, upload_update};
pub use composables::instances::{
    cache_pack_icons, scan_pack_library, CachedIcon, PackGroup, PackLibrary, PackSummary,
};
pub use composables::manifest::{
    compare_manifests, open_curseforge_url, open_url, parse_minecraft_instance, Addon, Manifest,
    UpdateInfo,
};
mod installer;
pub use installer::{install_update, ConfigFile as InstallerConfigFile, InstallOptions};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_keyring::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            select_directory,
            select_file,
            select_save_file,
            service_commands::read_file,
            service_commands::write_file,
            service_commands::parse_minecraft_instance,
            service_commands::compare_manifests,
            open_curseforge_url,
            open_url,
            upload_update,
            download_manifest,
            download_config_files,
            install_update,
            select_multiple_files,
            service_commands::read_directory_recursive,
            service_commands::is_binary_file,
            service_commands::validate_path,
            scan_pack_library,
            cache_pack_icons
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            let event_sink = Arc::new(move |event: service::ServiceEvent| {
                let _ = app_handle.emit(&event.name, event.payload);
            });
            let cache_dir = app.path().app_cache_dir().ok();
            let executable = std::env::current_exe().map_err(|error| {
                std::io::Error::other(format!(
                    "Failed to locate CEMM executable for local service: {error}"
                ))
            })?;
            let service = service::ServiceClient::spawn(&executable, cache_dir, event_sink)
                .map_err(std::io::Error::other)?;
            app.manage(service);

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Process plugin for restart functionality
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_process::init())
                .expect("failed to setup process plugin");

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Opens a native directory selection dialog.
///
/// This function uses Tauri's built-in dialog API which provides native file dialogs
/// on all platforms (Windows, macOS, and Linux). On Linux, the dialog requires either
/// GTK or Zenity to be installed (typically available on most desktop environments).
///
/// If the native dialog fails or times out, users can manually input the path
/// through the UI's text input field as a fallback option.
///
/// # Arguments
/// * `app` - Tauri application handle for accessing the dialog API
///
/// # Returns
/// * `Ok(String)` - The selected directory path
/// * `Err(String)` - Error message with suggestion to use manual path input
#[tauri::command]
async fn select_directory(app: tauri::AppHandle) -> Result<String, String> {
    log::info!("select_directory: attempting to open dialog");

    // Use Tauri's built-in dialog with timeout
    // Note: On Linux, this requires GTK or Zenity. If unavailable, users should
    // use the manual path input option in the UI.
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        async_directory_dialog(app),
    )
    .await
    {
        Ok(Ok(path)) => {
            log::info!("select_directory: dialog succeeded: {}", path);
            Ok(path)
        }
        Ok(Err(e)) => {
            log::error!("select_directory: dialog error: {}", e);
            Err(format!(
                "Dialog error: {}. Try using manual path input instead.",
                e
            ))
        }
        Err(_) => {
            log::error!("select_directory: dialog timeout");
            Err("Dialog timeout - the system dialog may be unresponsive. Please try manual path input.".to_string())
        }
    }
}

// Async wrapper for Tauri dialog
async fn async_directory_dialog(app: tauri::AppHandle) -> Result<String, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    log::info!("Starting Tauri directory dialog");

    // Run the dialog in a separate task to prevent blocking
    tokio::task::spawn_blocking(move || {
        let dialog = app.dialog().clone();
        FileDialogBuilder::new(dialog).pick_folder(move |folder| {
            let result = match folder {
                Some(path) => {
                    let path_str = path.to_string();
                    let normalized_path = normalize_path(&path_str);
                    Ok(normalized_path)
                }
                None => Err("No directory selected".to_string()),
            };
            let _ = tx.send(result);
        });
    });

    match rx.await {
        Ok(Ok(path)) => {
            log::info!("Tauri directory dialog returned: {}", path);
            Ok(path)
        }
        Ok(Err(e)) => {
            log::warn!("Tauri directory dialog error: {}", e);
            Err(e)
        }
        Err(_) => {
            log::error!("Tauri directory dialog channel error");
            Err("Dialog communication error".to_string())
        }
    }
}

#[tauri::command]
async fn select_file(app: tauri::AppHandle) -> Result<String, String> {
    log::info!("select_file: attempting to open dialog");

    // Use Tauri's built-in dialog with timeout
    match tokio::time::timeout(std::time::Duration::from_secs(30), async_file_dialog(app)).await {
        Ok(Ok(path)) => {
            log::info!("select_file: dialog succeeded: {}", path);
            Ok(path)
        }
        Ok(Err(e)) => {
            log::error!("select_file: dialog error: {}", e);
            Err(format!(
                "Dialog error: {}. Try using manual path input instead.",
                e
            ))
        }
        Err(_) => {
            log::error!("select_file: dialog timeout");
            Err("Dialog timeout - the system dialog may be unresponsive. Please try manual path input.".to_string())
        }
    }
}

async fn async_file_dialog(app: tauri::AppHandle) -> Result<String, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    log::info!("Starting Tauri file dialog");

    // Run the dialog in a separate task to prevent blocking
    tokio::task::spawn_blocking(move || {
        let dialog = app.dialog().clone();
        FileDialogBuilder::new(dialog).pick_file(move |file| {
            let result = match file {
                Some(path) => {
                    let path_str = path.to_string();
                    let normalized_path = normalize_path(&path_str);
                    Ok(normalized_path)
                }
                None => Err("No file selected".to_string()),
            };
            let _ = tx.send(result);
        });
    });

    match rx.await {
        Ok(Ok(path)) => {
            log::info!("Tauri file dialog returned: {}", path);
            Ok(path)
        }
        Ok(Err(e)) => {
            log::warn!("Tauri file dialog error: {}", e);
            Err(e)
        }
        Err(_) => {
            log::error!("Tauri file dialog channel error");
            Err("Dialog communication error".to_string())
        }
    }
}

fn read_file(path: String) -> Result<String, String> {
    log::info!("read_file: attempting to read {path}");

    // Check if file exists first
    if !std::path::Path::new(&path).exists() {
        log::error!("read_file: file does not exist: {path}");
        return Err(format!("File does not exist: {}", path));
    }

    match fs::read_to_string(&path) {
        Ok(content) => {
            log::info!(
                "read_file: successfully read {path}, content length: {}",
                content.len()
            );
            Ok(content)
        }
        Err(e) => {
            log::error!("read_file: failed to read {path}: {e}");

            // Check if this might be a binary file
            if e.to_string().contains("invalid utf-8")
                || e.to_string().contains("stream did not contain valid UTF-8")
            {
                log::warn!(
                    "read_file: file appears to be binary, attempting to read as base64: {path}"
                );

                // For binary files like .emotecraft, read as bytes and encode as base64
                match fs::read(&path) {
                    Ok(bytes) => {
                        use base64::engine::general_purpose::STANDARD;
                        use base64::Engine;
                        let encoded = STANDARD.encode(&bytes);
                        log::info!("read_file: successfully read binary file as base64: {path}");
                        Ok(format!("data:application/octet-stream;base64,{}", encoded))
                    }
                    Err(read_err) => {
                        log::error!("read_file: failed to read binary file: {path}: {read_err}");
                        Err(format!(
                            "Failed to read file as text or binary: {}",
                            read_err
                        ))
                    }
                }
            } else {
                Err(e.to_string())
            }
        }
    }
}

fn write_file(
    path: Option<String>,
    content: Option<String>,
    dir: Option<String>,
    files: Option<Vec<(String, String)>>,
) -> Result<(), String> {
    use std::path::Path;
    // Batch mode. `write_file` has no notion of a base directory in single-file mode
    // (callers pass a fully-resolved path there), but batch mode always represents
    // "write these relative paths under this directory" — so it is the one place this
    // generic command can and must enforce that the result stays inside `dir`.
    if let (Some(dir), Some(files)) = (dir, files) {
        for (filename, _) in &files {
            installer::validate_path_within_base(Path::new(&dir), filename)?;
        }
        for (filename, content) in files {
            let file_path = Path::new(&dir).join(&filename);
            if let Some(parent) = file_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return Err(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        e
                    ));
                }
            }
            if let Err(e) = std::fs::write(&file_path, content) {
                return Err(format!(
                    "Failed to write file {}: {}",
                    file_path.display(),
                    e
                ));
            }
        }
        return Ok(());
    }
    // Single file mode
    if let (Some(path), Some(content)) = (path, content) {
        if let Some(parent) = Path::new(&path).parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return Err(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ));
            }
        }
        return std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write file {}: {}", path, e));
    }
    Err("Invalid arguments: must provide either (path, content) or (dir, files)".to_string())
}

// select_save_file/select_multiple_files/select_config_directory previously ran
// as sync commands that blocked the IPC handler thread waiting on the dialog
// (via a std::sync::mpsc::recv() here, and blocking_pick_* below) — the plugin's
// own docs mark that unsafe outside an async command, since on Windows the sync
// dispatch path runs inline on the event-loop thread and the dialog itself needs
// that same thread to pump messages (F-P1-6, self-deadlock hazard). All three
// now match select_directory/select_file's already-correct async pattern.
#[tauri::command]
async fn select_save_file(app: tauri::AppHandle) -> Result<String, String> {
    log::info!("select_save_file: dialog opened");
    let dialog = app.dialog().clone();
    let file = FileDialogBuilder::new(dialog)
        .set_title("Save Manifest As...")
        .add_filter("Manifest JSON", &["json"])
        .set_file_name("cemm-manifest.json")
        .blocking_save_file();

    match file {
        Some(path) => {
            let normalized_path = normalize_path(&path.to_string());
            log::info!("select_save_file: selected {}", normalized_path);
            Ok(normalized_path)
        }
        None => {
            log::info!("select_save_file: no file selected");
            Err("No file selected".to_string())
        }
    }
}

#[tauri::command]
async fn select_multiple_files(window: tauri::Window) -> Result<Vec<String>, String> {
    let dialog = window
        .dialog()
        .file()
        .add_filter(
            "Config Files",
            &[
                "cfg",
                "txt",
                "json",
                "json5",
                "toml",
                "properties",
                "conf",
                "yaml",
                "yml",
                "ini",
                "xml",
                "js",
                "ts",
                "groovy",
                "kts",
                "mcmeta",
                "snbt",
                "nbt",
                "dat",
                "emotecraft",
            ],
        )
        .add_filter("All Files", &["*"]);

    match dialog.blocking_pick_files() {
        Some(files) => Ok(files
            .into_iter()
            .map(|f| normalize_path(&f.to_string()))
            .collect()),
        None => Ok(Vec::new()),
    }
}

fn read_directory_recursive(
    dir_path: String,
    base_path: String,
) -> Result<Vec<ConfigFileWithContent>, String> {
    read_directory_recursive_with_limits(
        std::path::Path::new(&dir_path),
        std::path::Path::new(&base_path),
        ScanLimits::production(),
    )
}

#[derive(Clone, Copy, Debug)]
struct ScanLimits {
    max_depth: usize,
    max_files: usize,
    max_file_bytes: u64,
    max_total_bytes: u64,
}

impl ScanLimits {
    const fn production() -> Self {
        Self {
            max_depth: 64,
            max_files: 10_000,
            max_file_bytes: 128 * 1024 * 1024,
            max_total_bytes: 1024 * 1024 * 1024,
        }
    }
}

fn is_supported_config_extension(extension: &str) -> bool {
    matches!(
        extension,
        "cfg"
            | "txt"
            | "json"
            | "json5"
            | "toml"
            | "properties"
            | "conf"
            | "yaml"
            | "yml"
            | "ini"
            | "xml"
            | "js"
            | "ts"
            | "groovy"
            | "kts"
            | "mcmeta"
            | "snbt"
            | "nbt"
            | "dat"
            | "emotecraft"
    )
}

fn read_limited<R: std::io::Read>(reader: R, limit: u64) -> Result<Vec<u8>, std::io::Error> {
    use std::io::Read;

    let mut bytes = Vec::new();
    reader
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn split_root_path(
    path: &std::path::Path,
) -> Result<(std::path::PathBuf, Vec<std::ffi::OsString>), String> {
    use std::path::Component;

    if path.as_os_str().is_empty() {
        return Err("Config import directory path must not be empty".to_string());
    }

    let mut components = path.components();
    let anchor = if path.is_absolute() {
        let mut anchor = std::path::PathBuf::new();
        loop {
            match components.next() {
                Some(Component::Prefix(prefix)) => anchor.push(prefix.as_os_str()),
                Some(Component::RootDir) => {
                    anchor.push(std::path::MAIN_SEPARATOR.to_string());
                    break anchor;
                }
                _ => {
                    return Err(format!(
                        "Unsupported absolute config import root: {}",
                        path.display()
                    ));
                }
            }
        }
    } else {
        std::path::PathBuf::from(".")
    };

    let mut names = Vec::new();
    for component in components {
        match component {
            Component::Normal(name) => names.push(name.to_os_string()),
            Component::CurDir if names.is_empty() => {}
            _ => {
                return Err(format!(
                    "Unsupported config import root component in {}",
                    path.display()
                ));
            }
        }
    }

    Ok((anchor, names))
}

fn read_directory_recursive_with_limits(
    dir: &std::path::Path,
    base: &std::path::Path,
    limits: ScanLimits,
) -> Result<Vec<ConfigFileWithContent>, String> {
    use cap_fs_ext::{OpenOptionsFollowExt, OpenOptionsMaybeDirExt};
    use cap_primitives::fs::{
        open, open_ambient_dir, open_dir_nofollow, read_base_dir, stat, FollowSymlinks, OpenOptions,
    };

    fn open_root_directory(path: &std::path::Path) -> Result<fs::File, String> {
        let (anchor, components) = split_root_path(path)?;

        let mut directory = open_ambient_dir(&anchor, cap_primitives::ambient_authority())
            .map_err(|e| {
                format!(
                    "Failed to open filesystem anchor {}: {}",
                    anchor.display(),
                    e
                )
            })?;
        for name in components {
            directory =
                open_dir_nofollow(&directory, std::path::Path::new(&name)).map_err(|e| {
                    format!(
                        "Failed to open root directory component without following links in {}: {}",
                        path.display(),
                        e
                    )
                })?;
        }

        Ok(directory)
    }

    let root = open_root_directory(dir)?;
    if !root
        .metadata()
        .map_err(|e| {
            format!(
                "Failed to inspect opened directory {}: {}",
                dir.display(),
                e
            )
        })?
        .is_dir()
    {
        return Err(format!("Path is not a directory: {}", dir.display()));
    }
    let mut config_files = Vec::new();
    let mut total_bytes = 0_u64;
    let mut directories = vec![(root, dir.to_path_buf(), 0_usize)];

    while let Some((current_dir, current_path, depth)) = directories.pop() {
        let entries = read_base_dir(&current_dir)
            .map_err(|e| format!("Failed to read directory {}: {}", current_path.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                format!("Failed to read entry in {}: {}", current_path.display(), e)
            })?;
            let name = entry.file_name();
            let path = current_path.join(&name);
            let metadata = stat(
                &current_dir,
                std::path::Path::new(&name),
                FollowSymlinks::No,
            )
            .map_err(|e| format!("Failed to inspect path {}: {}", path.display(), e))?;

            if metadata.file_type().is_symlink() {
                log::warn!("Skipping symlink during config import: {}", path.display());
                continue;
            }

            if metadata.is_dir() {
                let child_depth = depth + 1;
                if child_depth > limits.max_depth {
                    return Err(format!(
                        "Config import depth limit ({}) exceeded at {} (depth {})",
                        limits.max_depth,
                        path.display(),
                        child_depth
                    ));
                }
                let child_dir = open_dir_nofollow(&current_dir, std::path::Path::new(&name))
                    .map_err(|e| {
                        format!(
                            "Failed to open directory without following links {}: {}",
                            path.display(),
                            e
                        )
                    })?;
                if !child_dir
                    .metadata()
                    .map_err(|e| {
                        format!(
                            "Failed to inspect opened directory {}: {}",
                            path.display(),
                            e
                        )
                    })?
                    .is_dir()
                {
                    return Err(format!("Path is not a directory: {}", path.display()));
                }
                directories.push((child_dir, path, child_depth));
                continue;
            }

            if !metadata.is_file() {
                continue;
            }

            let Some(extension) = path.extension() else {
                continue;
            };
            let extension = extension.to_string_lossy().to_lowercase();
            if !is_supported_config_extension(&extension) {
                continue;
            }

            let mut options = OpenOptions::new();
            options
                .read(true)
                .follow(FollowSymlinks::No)
                .maybe_dir(false);
            let mut file =
                open(&current_dir, std::path::Path::new(&name), &options).map_err(|e| {
                    format!(
                        "Failed to open file without following links {}: {}",
                        path.display(),
                        e
                    )
                })?;
            let opened_metadata = file
                .metadata()
                .map_err(|e| format!("Failed to inspect opened file {}: {}", path.display(), e))?;
            if !opened_metadata.is_file() {
                return Err(format!("Path is not a regular file: {}", path.display()));
            }

            if config_files.len() >= limits.max_files {
                return Err(format!(
                    "Config import file limit ({}) exceeded at {}",
                    limits.max_files,
                    path.display()
                ));
            }

            let file_bytes = opened_metadata.len();
            if file_bytes > limits.max_file_bytes {
                return Err(format!(
                    "Config import per-file size limit ({} bytes) exceeded at {} ({} bytes)",
                    limits.max_file_bytes,
                    path.display(),
                    file_bytes
                ));
            }
            if file_bytes > limits.max_total_bytes.saturating_sub(total_bytes) {
                return Err(format!(
                    "Config import aggregate size limit ({} bytes) exceeded at {}",
                    limits.max_total_bytes,
                    path.display()
                ));
            }

            let remaining_bytes = limits.max_total_bytes - total_bytes;
            let read_limit = limits.max_file_bytes.min(remaining_bytes);
            let bytes = read_limited(&mut file, read_limit)
                .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;
            if bytes.len() as u64 > read_limit {
                let (limit_name, limit_value, remaining_context) =
                    if limits.max_file_bytes <= remaining_bytes {
                        ("per-file", limits.max_file_bytes, String::new())
                    } else {
                        (
                            "aggregate",
                            limits.max_total_bytes,
                            format!(" ({} bytes remained)", remaining_bytes),
                        )
                    };
                return Err(format!(
                    "Config import {limit_name} size limit ({limit_value} bytes) exceeded while reading {}{remaining_context}",
                    path.display(),
                ));
            }
            let raw_bytes = bytes.len() as u64;

            let content = match String::from_utf8(bytes) {
                Ok(text_content) => text_content,
                Err(error) => {
                    use base64::engine::general_purpose::STANDARD;
                    use base64::Engine;
                    format!(
                        "data:application/octet-stream;base64,{}",
                        STANDARD.encode(error.into_bytes())
                    )
                }
            };

            let relative_path = path
                .strip_prefix(base)
                .map_err(|_| format!("Failed to make path relative: {}", path.display()))?
                .to_string_lossy()
                .replace('\\', "/");
            let filename = path
                .file_name()
                .ok_or_else(|| format!("Failed to get filename from path: {}", path.display()))?
                .to_string_lossy()
                .to_string();
            let is_binary = content.starts_with("data:application/octet-stream;base64,")
                || extension == "emotecraft";

            total_bytes += raw_bytes;
            config_files.push(ConfigFileWithContent {
                filename,
                relative_path,
                content,
                is_binary: Some(is_binary),
            });
        }
    }

    Ok(config_files)
}

fn is_binary_file(path: String) -> Result<bool, String> {
    log::info!("is_binary_file: checking {path}");

    if !std::path::Path::new(&path).exists() {
        return Err(format!("File does not exist: {}", path));
    }

    // Read first 512 bytes to check for binary content
    match fs::read(&path) {
        Ok(bytes) => {
            let sample_size = std::cmp::min(512, bytes.len());
            let sample = &bytes[0..sample_size];

            // Check for null bytes (common indicator of binary files)
            let has_null_bytes = sample.contains(&0);

            // Check file extension for known binary types
            let path_lower = path.to_lowercase();
            let is_known_binary = path_lower.ends_with(".emotecraft")
                || path_lower.ends_with(".exe")
                || path_lower.ends_with(".dll")
                || path_lower.ends_with(".bin")
                || path_lower.ends_with(".dat")
                || path_lower.ends_with(".zip")
                || path_lower.ends_with(".jar");

            let is_binary = has_null_bytes || is_known_binary;
            log::info!("is_binary_file: {path} is binary: {is_binary}");
            Ok(is_binary)
        }
        Err(e) => {
            log::error!("is_binary_file: failed to read {path}: {e}");
            Err(e.to_string())
        }
    }
}

fn validate_path(path: String) -> Result<serde_json::Value, String> {
    log::info!("validate_path: checking path {}", path);

    let path_obj = std::path::Path::new(&path);
    let mut result = serde_json::Map::new();

    // Check if path exists
    let exists = path_obj.exists();
    result.insert("exists".to_string(), serde_json::Value::Bool(exists));

    if exists {
        let is_dir = path_obj.is_dir();
        let is_file = path_obj.is_file();

        result.insert("is_directory".to_string(), serde_json::Value::Bool(is_dir));
        result.insert("is_file".to_string(), serde_json::Value::Bool(is_file));

        // `!readonly()` reports whether the path is *writable*, not readable
        // (F-P3-12) — actually probing read access matters for is_dir, since a
        // directory can be listable-by-stat but not openable for reading.
        let can_read = if is_dir {
            fs::read_dir(path_obj).is_ok()
        } else {
            fs::File::open(path_obj).is_ok()
        };
        result.insert("can_read".to_string(), serde_json::Value::Bool(can_read));

        // For directories, check if it looks like a modpack
        if is_dir {
            let has_minecraft_instance = path_obj.join("minecraftinstance.json").exists();
            let has_mods_folder = path_obj.join("mods").exists();
            let has_config_folder = path_obj.join("config").exists();

            result.insert(
                "has_minecraft_instance".to_string(),
                serde_json::Value::Bool(has_minecraft_instance),
            );
            result.insert(
                "has_mods_folder".to_string(),
                serde_json::Value::Bool(has_mods_folder),
            );
            result.insert(
                "has_config_folder".to_string(),
                serde_json::Value::Bool(has_config_folder),
            );

            let is_likely_modpack =
                has_minecraft_instance || (has_mods_folder && has_config_folder);
            result.insert(
                "is_likely_modpack".to_string(),
                serde_json::Value::Bool(is_likely_modpack),
            );
        }

        // For files, check if it's a valid config file type
        if is_file {
            if let Some(extension) = path_obj.extension() {
                let ext_str = extension.to_string_lossy().to_lowercase();
                let valid_extensions = ["json", "toml", "cfg", "txt", "properties", "emotecraft"];
                let is_valid_config = valid_extensions.contains(&ext_str.as_str());
                result.insert(
                    "is_valid_config".to_string(),
                    serde_json::Value::Bool(is_valid_config),
                );
                result.insert("extension".to_string(), serde_json::Value::String(ext_str));
            }
        }

        // Get absolute path
        if let Ok(absolute_path) = path_obj.canonicalize() {
            let normalized_path = normalize_path(&absolute_path.to_string_lossy());
            result.insert(
                "absolute_path".to_string(),
                serde_json::Value::String(normalized_path),
            );
        }
    }

    result.insert("original_path".to_string(), serde_json::Value::String(path));

    Ok(serde_json::Value::Object(result))
}

use crate::composables::github::ConfigFileWithContent;

#[cfg(test)]
mod tests {
    use super::*;

    fn test_limits() -> ScanLimits {
        ScanLimits {
            max_depth: 8,
            max_files: 8,
            max_file_bytes: 64,
            max_total_bytes: 128,
        }
    }

    fn write_test_file(dir: &std::path::Path, relative_path: &str, contents: &[u8]) {
        let path = dir.join(relative_path);
        fs::create_dir_all(path.parent().expect("test file must have a parent"))
            .expect("failed to create test directory");
        fs::write(path, contents).expect("failed to write test file");
    }

    #[test]
    fn directory_import_preserves_nested_relative_paths() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        write_test_file(temp.path(), "config/inner/settings.toml", b"enabled = true");

        let files = read_directory_recursive_with_limits(temp.path(), temp.path(), test_limits())
            .expect("nested config file should import");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "settings.toml");
        assert_eq!(files[0].relative_path, "config/inner/settings.toml");
        assert_eq!(files[0].content, "enabled = true");
    }

    #[test]
    fn directory_import_ignores_unsupported_files_when_enforcing_limits() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        write_test_file(temp.path(), "ignored.bin", &[0; 100]);
        write_test_file(temp.path(), "settings.toml", b"enabled = true");
        let limits = ScanLimits {
            max_file_bytes: 16,
            max_total_bytes: 16,
            ..test_limits()
        };

        let files = read_directory_recursive_with_limits(temp.path(), temp.path(), limits)
            .expect("unsupported file must not consume import limits");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "settings.toml");
    }

    #[test]
    fn directory_import_rejects_excessive_depth_before_reading() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        write_test_file(
            temp.path(),
            "one/two/three/settings.toml",
            b"enabled = true",
        );
        let limits = ScanLimits {
            max_depth: 2,
            ..test_limits()
        };

        let error = read_directory_recursive_with_limits(temp.path(), temp.path(), limits)
            .expect_err("third nested directory must exceed the depth limit");

        assert!(error.contains("depth limit (2)"));
        assert!(error.contains("three"));
    }

    #[test]
    fn directory_import_rejects_excessive_matching_file_count() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        write_test_file(temp.path(), "one.toml", b"one");
        write_test_file(temp.path(), "two.toml", b"two");
        let limits = ScanLimits {
            max_files: 1,
            ..test_limits()
        };

        let error = read_directory_recursive_with_limits(temp.path(), temp.path(), limits)
            .expect_err("second supported file must exceed the count limit");

        assert!(error.contains("file limit (1)"));
    }

    #[test]
    fn directory_import_rejects_file_larger_than_limit() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        write_test_file(temp.path(), "oversized.toml", &[b'x'; 9]);
        let limits = ScanLimits {
            max_file_bytes: 8,
            ..test_limits()
        };

        let error = read_directory_recursive_with_limits(temp.path(), temp.path(), limits)
            .expect_err("oversized file must be rejected before reading");

        assert!(error.contains("per-file size limit (8 bytes)"));
        assert!(error.contains("oversized.toml"));
    }

    #[test]
    fn directory_import_rejects_aggregate_size_before_reading_overflow_file() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        write_test_file(temp.path(), "one.toml", b"12345678");
        write_test_file(temp.path(), "two.toml", b"abcdefgh");
        let limits = ScanLimits {
            max_total_bytes: 12,
            ..test_limits()
        };

        let error = read_directory_recursive_with_limits(temp.path(), temp.path(), limits)
            .expect_err("second file must exceed aggregate limit");

        assert!(error.contains("aggregate size limit (12 bytes)"));
    }

    #[test]
    fn limited_read_stops_at_limit_plus_one_byte() {
        let bytes = read_limited(std::io::Cursor::new(vec![b'x'; 10]), 8)
            .expect("bounded in-memory read should succeed");

        assert_eq!(bytes.len(), 9);
    }

    #[test]
    fn root_path_rejects_empty_input() {
        let error = split_root_path(std::path::Path::new(""))
            .expect_err("empty config import path must be rejected");

        assert_eq!(error, "Config import directory path must not be empty");
    }

    #[test]
    fn root_path_accepts_leading_current_directory_components() {
        let (anchor, components) = split_root_path(std::path::Path::new("././configs"))
            .expect("leading current-directory components should be accepted");

        assert_eq!(anchor, std::path::PathBuf::from("."));
        assert_eq!(components, vec![std::ffi::OsString::from("configs")]);
        assert!(split_root_path(std::path::Path::new(".")).is_ok());
        assert!(split_root_path(std::path::Path::new("configs/../other")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn directory_import_skips_symlink_loop() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("failed to create temp dir");
        write_test_file(temp.path(), "config/settings.toml", b"enabled = true");
        symlink(temp.path(), temp.path().join("config/loop"))
            .expect("failed to create test symlink loop");

        let files = read_directory_recursive_with_limits(temp.path(), temp.path(), test_limits())
            .expect("symlink must be skipped rather than followed");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "config/settings.toml");
    }

    #[cfg(unix)]
    #[test]
    fn directory_import_rejects_intermediate_root_symlink() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let outside = temp.path().join("outside");
        write_test_file(&outside, "config/settings.toml", b"enabled = true");
        symlink(&outside, temp.path().join("linked-parent"))
            .expect("failed to create intermediate symlink");
        let root = temp.path().join("linked-parent/config");

        let error = read_directory_recursive_with_limits(&root, &root, test_limits())
            .expect_err("intermediate root symlink must not be followed");

        assert!(error.contains("root directory component"));
    }

    #[cfg(windows)]
    #[test]
    fn directory_import_skips_symlink_when_windows_privileges_allow_it() {
        use cap_fs_ext::{OpenOptionsFollowExt, OpenOptionsMaybeDirExt};
        use cap_primitives::fs::{open, open_ambient_dir, FollowSymlinks, OpenOptions};
        use std::os::windows::fs::{symlink_dir, symlink_file};

        let temp = tempfile::tempdir().expect("failed to create temp dir");
        write_test_file(temp.path(), "config/settings.toml", b"enabled = true");
        if let Err(error) = symlink_dir(temp.path(), temp.path().join("config/loop")) {
            eprintln!("Windows symlink probe skipped because a link could not be created: {error}");
            return;
        }

        let files = read_directory_recursive_with_limits(temp.path(), temp.path(), test_limits())
            .expect("symlink must be skipped rather than followed");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "config/settings.toml");

        symlink_file(
            temp.path().join("config/settings.toml"),
            temp.path().join("replacement.toml"),
        )
        .expect("failed to create test file symlink");
        let directory = open_ambient_dir(temp.path(), cap_primitives::ambient_authority())
            .expect("failed to open test directory");
        let mut options = OpenOptions::new();
        options
            .read(true)
            .follow(FollowSymlinks::No)
            .maybe_dir(false);

        assert!(open(
            &directory,
            std::path::Path::new("replacement.toml"),
            &options
        )
        .is_err());

        let outside = temp.path().join("outside");
        write_test_file(&outside, "config/settings.toml", b"enabled = true");
        symlink_dir(&outside, temp.path().join("linked-parent"))
            .expect("failed to create intermediate test symlink");
        let root = temp.path().join("linked-parent/config");

        let error = read_directory_recursive_with_limits(&root, &root, test_limits())
            .expect_err("intermediate root symlink must not be followed");

        assert!(error.contains("root directory component"));
    }

    #[test]
    fn write_file_batch_mode_rejects_traversal_and_absolute_paths() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let dir = temp.path().to_str().unwrap().to_string();

        for bad_path in ["../evil.bat", "/etc/evil", "a/../../evil", "~/evil"] {
            let result = write_file(
                None,
                None,
                Some(dir.clone()),
                Some(vec![(bad_path.to_string(), "attacker content".to_string())]),
            );
            assert!(result.is_err(), "expected '{bad_path}' to be rejected");
        }

        // Nothing from the rejected batch should exist outside the temp base.
        assert!(!temp.path().parent().unwrap().join("evil.bat").exists());
    }

    #[test]
    fn write_file_batch_mode_allows_nested_relative_paths() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let dir = temp.path().to_str().unwrap().to_string();

        let result = write_file(
            None,
            None,
            Some(dir),
            Some(vec![(
                "config/sub/file.toml".to_string(),
                "value = 1".to_string(),
            )]),
        );

        assert!(result.is_ok());
        assert!(temp
            .path()
            .join("config")
            .join("sub")
            .join("file.toml")
            .exists());
    }

    #[test]
    fn write_file_batch_mode_rejects_whole_batch_if_any_entry_is_unsafe() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let dir = temp.path().to_str().unwrap().to_string();

        let result = write_file(
            None,
            None,
            Some(dir),
            Some(vec![
                ("safe.toml".to_string(), "value = 1".to_string()),
                ("../evil.bat".to_string(), "attacker content".to_string()),
            ]),
        );

        assert!(result.is_err());
        // The safe entry must not have been written either — validation runs
        // as a pass over the whole batch before any write is attempted.
        assert!(!temp.path().join("safe.toml").exists());
    }
}
