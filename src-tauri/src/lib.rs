use std::fs;
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};

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
pub use installer::{install_update, install_update_with_cleanup, install_update_optimized, ConfigFile as InstallerConfigFile};

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
            install_update_with_cleanup,
            install_update_optimized,
            check_for_updates,
            download_updater_file,
            install_updater_file,
            get_app_data_dir,
            select_multiple_files,
            select_config_directory,
            read_directory_recursive
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

#[tauri::command]
fn select_directory(app: tauri::AppHandle) -> Result<String, String> {
    use std::sync::mpsc;
    log::info!("select_directory: dialog opened");
    let (tx, rx) = mpsc::channel();
    let dialog = app.dialog().clone();
    FileDialogBuilder::new(dialog).pick_folder(move |folder| {
        let _ = tx.send(folder);
    });
    match rx.recv() {
        Ok(Some(path)) => {
            log::info!("select_directory: selected {path}");
            Ok(path.to_string())
        }
        Ok(None) => {
            log::info!("select_directory: no directory selected");
            Err("No directory selected".to_string())
        }
        Err(e) => {
            log::error!("select_directory: error receiving dialog result: {e}");
            Err("No directory selected".to_string())
        }
    }
}

#[tauri::command]
fn select_file(app: tauri::AppHandle) -> Result<String, String> {
    use std::sync::mpsc;
    log::info!("select_file: dialog opened");
    let (tx, rx) = mpsc::channel();
    let dialog = app.dialog().clone();
    FileDialogBuilder::new(dialog).pick_file(move |file| {
        let _ = tx.send(file);
    });
    match rx.recv() {
        Ok(Some(path)) => {
            log::info!("select_file: selected {path}");
            Ok(path.to_string())
        }
        Ok(None) => {
            log::info!("select_file: no file selected");
            Err("No file selected".to_string())
        }
        Err(e) => {
            log::error!("select_file: error receiving dialog result: {e}");
            Err("No file selected".to_string())
        }
    }
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    log::info!("read_file: attempting to read {path}");
    match fs::read_to_string(&path) {
        Ok(content) => {
            log::info!("read_file: successfully read {path}");
            Ok(content)
        }
        Err(e) => {
            log::error!("read_file: failed to read {path}: {e}");
            Err(e.to_string())
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
        .set_file_name("manifest.json")
        .save_file(move |file| {
            let _ = tx.send(file);
        });
    match rx.recv() {
        Ok(Some(path)) => {
            log::info!("select_save_file: selected {path}");
            Ok(path.to_string())
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
    Ok(app_data_dir.to_string_lossy().to_string())
}

#[tauri::command]
fn select_multiple_files(window: tauri::Window) -> Result<Vec<String>, String> {
    let dialog = window
        .dialog()
        .file()
        .add_filter("Config Files", &[
            "cfg", "txt", "json", "json5", "toml", "properties", "conf",
            "yaml", "yml", "ini", "xml", "js", "ts", "groovy", "kts",
            "mcmeta", "snbt", "nbt", "dat"
        ])
        .add_filter("All Files", &["*"]);
    
    match dialog.blocking_pick_files() {
        Some(files) => Ok(files.into_iter().map(|f| f.to_string()).collect()),
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
        Some(folder) => Ok(folder.to_string()),
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
                        "groovy" | "kts" | "mcmeta" | "snbt" | "nbt" | "dat"
                    ) {
                        let content = std::fs::read_to_string(&path)
                            .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;
                        
                        // Calculate relative path from base directory
                        let relative_path = path.strip_prefix(base)
                            .map_err(|_| format!("Failed to make path relative: {}", path.display()))?
                            .to_string_lossy()
                            .replace('\\', "/"); // Normalize path separators
                        
                        let filename = path.file_name()
                            .ok_or_else(|| format!("Failed to get filename from path: {}", path.display()))?
                            .to_string_lossy()
                            .to_string();
                        
                        config_files.push(ConfigFileWithContent {
                            filename,
                            relative_path,
                            content,
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

use crate::composables::github::ConfigFileWithContent;
