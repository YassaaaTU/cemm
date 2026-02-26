use std::fs;
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};

// Helper function to normalize Windows extended paths
fn normalize_path(path: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        // Remove Windows extended path prefix \\?\
        if path.starts_with(r"\\?\") {
            return path[4..].to_string();
        }
    }
    path.to_string()
}

mod composables {
    pub mod github;
    pub mod manifest;
}

mod updater;
pub use updater::{check_for_updates, download_updater_file, install_updater_file};

pub use composables::github::{download_update, download_manifest, download_config_files, upload_update};
pub use composables::manifest::{
    compare_manifests, open_curseforge_url, open_url, parse_minecraft_instance, Addon, Manifest,
    UpdateInfo,
};
mod installer;
pub use installer::{install_update, ConfigFile as InstallerConfigFile, InstallOptions};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_keyring::init())
        .invoke_handler(tauri::generate_handler![
            select_directory,
            select_file,
            select_save_file,
            read_file,
            write_file,
            parse_minecraft_instance,
            compare_manifests,
            open_curseforge_url,
            open_url,
            upload_update,
            download_update,
            download_manifest,
            download_config_files,
            install_update,
            check_for_updates,
            download_updater_file,
            install_updater_file,
            get_app_data_dir,
            select_multiple_files,
            select_config_directory,
            read_directory_recursive,
            is_binary_file,
            validate_path
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Secure storage setup
            app.handle()
                .plugin(tauri_plugin_keyring::init())
                .expect("failed to setup keyring plugin");



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
        async_directory_dialog(app)
    ).await {
        Ok(Ok(path)) => {
            log::info!("select_directory: dialog succeeded: {}", path);
            Ok(path)
        }
        Ok(Err(e)) => {
            log::error!("select_directory: dialog error: {}", e);
            Err(format!("Dialog error: {}. Try using manual path input instead.", e))
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
                },
                None => Err("No directory selected".to_string()),
            };
            let _ = tx.send(result);
        });
    });
    
    match rx.await {
        Ok(Ok(path)) => {
            log::info!("Tauri directory dialog returned: {}", path);
            Ok(path)
        },
        Ok(Err(e)) => {
            log::warn!("Tauri directory dialog error: {}", e);
            Err(e)
        },
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
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        async_file_dialog(app)
    ).await {
        Ok(Ok(path)) => {
            log::info!("select_file: dialog succeeded: {}", path);
            Ok(path)
        }
        Ok(Err(e)) => {
            log::error!("select_file: dialog error: {}", e);
            Err(format!("Dialog error: {}. Try using manual path input instead.", e))
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
                },
                None => Err("No file selected".to_string()),
            };
            let _ = tx.send(result);
        });
    });
    
    match rx.await {
        Ok(Ok(path)) => {
            log::info!("Tauri file dialog returned: {}", path);
            Ok(path)
        },
        Ok(Err(e)) => {
            log::warn!("Tauri file dialog error: {}", e);
            Err(e)
        },
        Err(_) => {
            log::error!("Tauri file dialog channel error");
            Err("Dialog communication error".to_string())
        }
    }
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    log::info!("read_file: attempting to read {path}");
    
    // Check if file exists first
    if !std::path::Path::new(&path).exists() {
        log::error!("read_file: file does not exist: {path}");
        return Err(format!("File does not exist: {}", path));
    }
    
    match fs::read_to_string(&path) {
        Ok(content) => {
            log::info!("read_file: successfully read {path}, content length: {}", content.len());
            Ok(content)
        }
        Err(e) => {
            log::error!("read_file: failed to read {path}: {e}");
            
            // Check if this might be a binary file
            if e.to_string().contains("invalid utf-8") || e.to_string().contains("stream did not contain valid UTF-8") {
                log::warn!("read_file: file appears to be binary, attempting to read as base64: {path}");
                
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
                        Err(format!("Failed to read file as text or binary: {}", read_err))
                    }
                }
            } else {
                Err(e.to_string())
            }
        }
    }
}

#[tauri::command]
fn write_file(path: Option<String>, content: Option<String>, dir: Option<String>, files: Option<Vec<(String, String)>>) -> Result<(), String> {
    use std::path::Path;
    // Batch mode
    if let (Some(dir), Some(files)) = (dir, files) {
        for (filename, content) in files {
            let file_path = Path::new(&dir).join(&filename);
            if let Some(parent) = file_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return Err(format!("Failed to create directory {}: {}", parent.display(), e));
                }
            }
            if let Err(e) = std::fs::write(&file_path, content) {
                return Err(format!("Failed to write file {}: {}", file_path.display(), e));
            }
        }
        return Ok(());
    }
    // Single file mode
    if let (Some(path), Some(content)) = (path, content) {
        if let Some(parent) = Path::new(&path).parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return Err(format!("Failed to create directory {}: {}", parent.display(), e));
            }
        }
        return std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write file {}: {}", path, e));
    }
    Err("Invalid arguments: must provide either (path, content) or (dir, files)".to_string())
}

#[tauri::command]
fn select_save_file(app: tauri::AppHandle) -> Result<String, String> {
    use std::sync::mpsc;
    log::info!("select_save_file: dialog opened");
    let (tx, rx) = mpsc::channel();
    let dialog = app.dialog().clone();
    FileDialogBuilder::new(dialog)
        .set_title("Save Manifest As...")
        .add_filter("Manifest JSON", &["json"])
        .set_file_name("cemm-manifest.json")
        .save_file(move |file| {
            let _ = tx.send(file);
        });
    match rx.recv() {
        Ok(Some(path)) => {
            let normalized_path = normalize_path(&path.to_string());
            log::info!("select_save_file: selected {}", normalized_path);
            Ok(normalized_path)
        }
        Ok(None) => {
            log::info!("select_save_file: no file selected");
            Err("No file selected".to_string())
        }
        Err(e) => {
            log::error!("select_save_file: error receiving dialog result: {e}");
            Err("No file selected".to_string())
        }
    }
}

#[tauri::command]
fn get_app_data_dir(app: tauri::AppHandle) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let normalized_path = normalize_path(&app_data_dir.to_string_lossy());
    Ok(normalized_path)
}

#[tauri::command]
fn select_multiple_files(window: tauri::Window) -> Result<Vec<String>, String> {
    let dialog = window
        .dialog()
        .file()
        .add_filter("Config Files", &[
            "cfg", "txt", "json", "json5", "toml", "properties", "conf",
            "yaml", "yml", "ini", "xml", "js", "ts", "groovy", "kts",
            "mcmeta", "snbt", "nbt", "dat", "emotecraft"
        ])
        .add_filter("All Files", &["*"]);
    
    match dialog.blocking_pick_files() {
        Some(files) => Ok(files.into_iter().map(|f| normalize_path(&f.to_string())).collect()),
        None => Ok(Vec::new()),
    }
}

#[tauri::command]
fn select_config_directory(window: tauri::Window) -> Result<String, String> {
    let dialog = window
        .dialog()
        .file()
        .set_title("Select Config Directory (config/, kubejs/, etc.)");
    
    match dialog.blocking_pick_folder() {
        Some(folder) => Ok(normalize_path(&folder.to_string())),
        None => Err("No directory selected".to_string()),
    }
}

#[tauri::command]
fn read_directory_recursive(dir_path: String, base_path: String) -> Result<Vec<ConfigFileWithContent>, String> {
    use std::path::Path;
    
    let mut config_files = Vec::new();
    let dir = Path::new(&dir_path);
    let base = Path::new(&base_path);
    
    fn collect_files(
        dir: &Path, 
        base: &Path, 
        config_files: &mut Vec<ConfigFileWithContent>
    ) -> Result<(), String> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?;
            
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();
            
            if path.is_file() {
                // Check if file has a config-related extension
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if matches!(ext_str.as_str(), 
                        "cfg" | "txt" | "json" | "json5" | "toml" | "properties" | 
                        "conf" | "yaml" | "yml" | "ini" | "xml" | "js" | "ts" | 
                        "groovy" | "kts" | "mcmeta" | "snbt" | "nbt" | "dat" |
                        "emotecraft" // Added support for .emotecraft files
                    ) {
                        // Try reading as text first, fallback to binary for files like .emotecraft
                        let content = match std::fs::read_to_string(&path) {
                            Ok(text_content) => text_content,
                            Err(_) => {
                                // File is likely binary, read as bytes and encode as base64
                                match std::fs::read(&path) {
                                    Ok(bytes) => {
                                        use base64::engine::general_purpose::STANDARD;
                                        use base64::Engine;
                                        let encoded = STANDARD.encode(&bytes);
                                        format!("data:application/octet-stream;base64,{}", encoded)
                                    }
                                    Err(e) => return Err(format!("Failed to read file {}: {}", path.display(), e))
                                }
                            }
                        };
                        
                        // Calculate relative path from base directory
                        let relative_path = path.strip_prefix(base)
                            .map_err(|_| format!("Failed to make path relative: {}", path.display()))?
                            .to_string_lossy()
                            .replace('\\', "/"); // Normalize path separators
                        let filename = path.file_name()
                            .ok_or_else(|| format!("Failed to get filename from path: {}", path.display()))?
                            .to_string_lossy()
                            .to_string();
                        
                        // Check if this is a binary file based on content or extension
                        let is_binary = content.starts_with("data:application/octet-stream;base64,") 
                            || ext_str == "emotecraft";
                        
                        config_files.push(ConfigFileWithContent {
                            filename,
                            relative_path,
                            content,
                            is_binary: Some(is_binary),
                        });
                    }
                }
            } else if path.is_dir() {
                // Recursively process subdirectories
                collect_files(&path, base, config_files)?;
            }
        }
        Ok(())
    }
    
    collect_files(dir, base, &mut config_files)?;
    Ok(config_files)
}

#[tauri::command]
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

#[tauri::command]
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
        
        // Check permissions
        let can_read = path_obj.metadata().map(|m| !m.permissions().readonly()).unwrap_or(false);
        result.insert("can_read".to_string(), serde_json::Value::Bool(can_read));
        
        // For directories, check if it looks like a modpack
        if is_dir {
            let has_minecraft_instance = path_obj.join("minecraftinstance.json").exists();
            let has_mods_folder = path_obj.join("mods").exists();
            let has_config_folder = path_obj.join("config").exists();
            
            result.insert("has_minecraft_instance".to_string(), serde_json::Value::Bool(has_minecraft_instance));
            result.insert("has_mods_folder".to_string(), serde_json::Value::Bool(has_mods_folder));
            result.insert("has_config_folder".to_string(), serde_json::Value::Bool(has_config_folder));
            
            let is_likely_modpack = has_minecraft_instance || (has_mods_folder && has_config_folder);
            result.insert("is_likely_modpack".to_string(), serde_json::Value::Bool(is_likely_modpack));
        }
        
        // For files, check if it's a valid config file type
        if is_file {
            if let Some(extension) = path_obj.extension() {
                let ext_str = extension.to_string_lossy().to_lowercase();
                let valid_extensions = vec!["json", "toml", "cfg", "txt", "properties", "emotecraft"];
                let is_valid_config = valid_extensions.contains(&ext_str.as_str());
                result.insert("is_valid_config".to_string(), serde_json::Value::Bool(is_valid_config));
                result.insert("extension".to_string(), serde_json::Value::String(ext_str));
            }
        }
        
        // Get absolute path
        if let Ok(absolute_path) = path_obj.canonicalize() {
            let normalized_path = normalize_path(&absolute_path.to_string_lossy());
            result.insert("absolute_path".to_string(), serde_json::Value::String(normalized_path));
        }
    }
    
    result.insert("original_path".to_string(), serde_json::Value::String(path));
    
    Ok(serde_json::Value::Object(result))
}

use crate::composables::github::ConfigFileWithContent;
