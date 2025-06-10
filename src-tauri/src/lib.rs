use std::fs;
use tauri_plugin_dialog::{FileDialogBuilder};

#[tauri::command]
pub async fn select_directory(app: tauri::AppHandle) -> Result<String, String> {
    let file_path = FileDialogBuilder::new()
        .pick_folder(&app)
        .await;
    
    match file_path {
        Some(path) => Ok(path.to_string_lossy().to_string()),
        None => Err("No directory selected".to_string()),
    }
}

#[tauri::command]
pub async fn select_file(app: tauri::AppHandle) -> Result<String, String> {
    let file_path = FileDialogBuilder::new()
        .add_filter("JSON files", &["json"])
        .pick_file(&app)
        .await;
    
    match file_path {
        Some(path) => Ok(path.to_string_lossy().to_string()),
        None => Err("No file selected".to_string()),
    }
}

#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![
      select_directory,
      select_file,
      read_file,
      write_file
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
