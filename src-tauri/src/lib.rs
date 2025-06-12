use std::fs;
use std::path::PathBuf;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_stronghold::Builder::with_argon2(&PathBuf::from("cemm-stronghold-salt"))
            .build())
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
        ])
        .setup(|app| {
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
fn write_file(path: String, content: String) -> Result<(), String> {
    log::info!("write_file: attempting to write {path}");
    match fs::write(&path, content) {
        Ok(_) => {
            log::info!("write_file: successfully wrote {path}");
            Ok(())
        }
        Err(e) => {
            log::error!("write_file: failed to write {path}: {e}");
            Err(e.to_string())
        }
    }
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
