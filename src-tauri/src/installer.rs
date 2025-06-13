use crate::composables::manifest::Manifest;
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;
use tauri::command;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;
use sha2::{Digest, Sha256};
use tauri::Window;
use tauri::Emitter;

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigFile {
    pub path: String,
    pub content: String,
}

#[command]
pub async fn install_update(
    modpack_path: String,
    manifest: Manifest,
    config_files: Vec<ConfigFile>,
) -> Result<(), String> {
    let client = Client::new();

    // --- Advanced Features: Progress Reporting, Integrity Check, Rollback ---

    // Progress state
    let total_files = manifest.mods.len()
        + manifest.resourcepacks.len()
        + manifest.shaderpacks.len()
        + manifest.datapacks.len()
        + config_files.len();
    let mut installed_paths: Vec<std::path::PathBuf> = Vec::new();

    // Helper to emit progress (if window is available)
    fn emit_progress(window: Option<&Window>, progress: usize, total: usize, msg: &str) {
        if let Some(w) = window {
            let _ = Emitter::emit(w, "install-progress", Some(serde_json::json!({
                "progress": (progress as f64) / (total as f64) * 100.0,
                "message": msg
            })));
        }
    }

    // Helper to extract filename from URL
    fn extract_filename_from_url(url: &str) -> Option<String> {
        url.split('/').last()
            .and_then(|name| {
                if name.contains('.') && !name.is_empty() {
                    Some(name.to_string())
                } else {
                    None
                }
            })
    }    // Helper to download, save, and check integrity
    async fn download_save_check(
        client: &Client,
        url: &str,
        dest_path: &Path,
        expected_sha256: Option<&str>,
    ) -> Result<(), String> {
        
        let resp = client.get(url).send().await.map_err(|e| {
            let err_msg = format!("Failed to download {}: {}", url, e);
            err_msg
        })?;
        
        if !resp.status().is_success() {
            let err_msg = format!("Failed to download {}: HTTP {}", url, resp.status());
            return Err(err_msg);
        }
        
        let bytes = resp.bytes().await.map_err(|e| {
            let err_msg = format!("Failed to read bytes from {}: {}", url, e);
            err_msg
        })?;
        
        if let Some(parent) = dest_path.parent() {
            async_fs::create_dir_all(parent).await.map_err(|e| {
                let err_msg = format!("Failed to create directory {}: {}", parent.display(), e);
                err_msg
            })?;
        }
        
        let mut file = async_fs::File::create(dest_path).await.map_err(|e| {
            let err_msg = format!("Failed to create file {}: {}", dest_path.display(), e);
            err_msg
        })?;
        
        file.write_all(&bytes).await.map_err(|e| {
            let err_msg = format!("Failed to write file {}: {}", dest_path.display(), e);
            err_msg
        })?;
        
        // Integrity check (optional)
        if let Some(expected) = expected_sha256 {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let actual = format!("{:x}", hasher.finalize());
            if actual != expected {
                return Err(format!("SHA256 mismatch for {}: expected {}, got {}", dest_path.display(), expected, actual));
            }
        }
        Ok(())
    }    // Wrap everything in a rollback guard
    let result: Result<(), String> = async {
        let window = None; // Optionally pass a tauri::Window for progress events
        let mut current = 0usize;
        // Mods
        for addon in &manifest.mods {
            let filename = extract_filename_from_url(&addon.cdn_download_url)
                .unwrap_or_else(|| format!("{}-{}.jar", addon.addon_name.replace(" ", "_"), addon.version));
            let dest = Path::new(&modpack_path).join("mods").join(filename);
            download_save_check(&client, &addon.cdn_download_url, &dest, None).await?;
            installed_paths.push(dest.clone());
            current += 1;
            emit_progress(window.as_ref(), current, total_files, &format!("Installed mod: {}", addon.addon_name));
        }
        // Resourcepacks
        for addon in &manifest.resourcepacks {
            let filename = extract_filename_from_url(&addon.cdn_download_url)
                .unwrap_or_else(|| format!("{}-{}.zip", addon.addon_name.replace(" ", "_"), addon.version));
            let dest = Path::new(&modpack_path).join("resourcepacks").join(filename);
            download_save_check(&client, &addon.cdn_download_url, &dest, None).await?;
            installed_paths.push(dest.clone());
            current += 1;
            emit_progress(window.as_ref(), current, total_files, &format!("Installed resourcepack: {}", addon.addon_name));
        }
        // Shaderpacks
        for addon in &manifest.shaderpacks {
            let filename = extract_filename_from_url(&addon.cdn_download_url)
                .unwrap_or_else(|| format!("{}-{}.zip", addon.addon_name.replace(" ", "_"), addon.version));
            let dest = Path::new(&modpack_path).join("shaderpacks").join(filename);
            download_save_check(&client, &addon.cdn_download_url, &dest, None).await?;
            installed_paths.push(dest.clone());
            current += 1;
            emit_progress(window.as_ref(), current, total_files, &format!("Installed shaderpack: {}", addon.addon_name));
        }
        // Datapacks
        for addon in &manifest.datapacks {
            let filename = extract_filename_from_url(&addon.cdn_download_url)
                .unwrap_or_else(|| format!("{}-{}.zip", addon.addon_name.replace(" ", "_"), addon.version));
            let dest = Path::new(&modpack_path).join("datapacks").join(filename);
            download_save_check(&client, &addon.cdn_download_url, &dest, None).await?;
            installed_paths.push(dest.clone());
            current += 1;
            emit_progress(window.as_ref(), current, total_files, &format!("Installed datapack: {}", addon.addon_name));
        }
        // Config files
        for config in config_files {
            let dest = Path::new(&modpack_path).join(&config.path);
            if let Some(parent) = dest.parent() {
                async_fs::create_dir_all(parent).await.map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
            }
            async_fs::write(&dest, config.content).await.map_err(|e| format!("Failed to write config file {}: {}", dest.display(), e))?;
            installed_paths.push(dest.clone());
            current += 1;
            emit_progress(window.as_ref(), current, total_files, &format!("Installed config: {}", dest.display()));
        }
        Ok(())
    }.await;
    // Rollback on error
    if let Err(e) = result {
        for path in installed_paths {
            let _ = async_fs::remove_file(&path).await;
        }
        return Err(format!("Install failed and rolled back: {}", e));
    }
    Ok(())
}
