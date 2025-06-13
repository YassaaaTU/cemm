use std::fs;
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};

mod composables {
    pub mod github;
    pub mod manifest;
}

pub use composables::github::{download_update, upload_update};
pub use composables::manifest::{
    compare_manifests, open_curseforge_url, open_url, parse_minecraft_instance, Addon, Manifest,
    UpdateInfo,
};
mod installer;
pub use installer::{install_update, ConfigFile as InstallerConfigFile};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
            install_update,
        ])
        .setup(|app| {
            // Initialize Stronghold with app-specific data directory
            let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
            let stronghold_path = app_data_dir.join("cemm-stronghold-salt");

            app.handle()
                .plugin(tauri_plugin_stronghold::Builder::with_argon2(&stronghold_path).build())
                .map_err(|e| e.to_string())?;

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
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
