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

// Helper function to extract filename from URL (moved to top level)
fn extract_filename_from_url(url: &str) -> Option<String> {
    url.split('/').last()
        .and_then(|name| {
            if name.contains('.') && !name.is_empty() {
                Some(name.to_string())
            } else {
                None
            }
        })
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct UpdateDiff {
    pub removed_addons: Vec<String>, // addon names to remove
    pub updated_addons: Vec<(String, String)>, // (old_version, new_version) pairs
    pub new_addons: Vec<String>, // completely new addon names
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

#[command]
pub async fn install_update_with_cleanup(
    modpack_path: String,
    old_manifest: Option<Manifest>,
    new_manifest: Manifest,
    config_files: Vec<ConfigFile>,
) -> Result<(), String> {
    // If no old manifest, fall back to regular install
    if old_manifest.is_none() {
        return install_update(modpack_path, new_manifest, config_files).await;
    }    let old_manifest = old_manifest.unwrap();
    let _client = Client::new(); // Prefix with _ to indicate intentionally unused

    // Step 1: Calculate what needs to be removed/updated
    let diff = calculate_update_diff(&old_manifest, &new_manifest)?;
    
    // Step 2: Remove old files first
    remove_old_files(&modpack_path, &old_manifest, &diff).await?;
    
    // Step 3: Install new/updated files
    install_update(modpack_path, new_manifest, config_files).await
}

fn calculate_update_diff(old_manifest: &Manifest, new_manifest: &Manifest) -> Result<UpdateDiff, String> {
    let mut diff = UpdateDiff {
        removed_addons: Vec::new(),
        updated_addons: Vec::new(),
        new_addons: Vec::new(),
    };

    // Helper to compare addon lists
    fn process_addon_category<F>(
        old_addons: &[crate::composables::manifest::Addon],
        new_addons: &[crate::composables::manifest::Addon],
        diff: &mut UpdateDiff,
        _extract_filename: F,
    ) where F: Fn(&str) -> Option<String> {
        // Find removed addons (in old but not in new)
        for old_addon in old_addons {
            if !new_addons.iter().any(|new_addon| new_addon.addon_project_id == old_addon.addon_project_id) {
                diff.removed_addons.push(old_addon.addon_name.clone());
            }
        }

        // Find updated addons (same project ID, different version)
        for old_addon in old_addons {
            if let Some(new_addon) = new_addons.iter().find(|a| a.addon_project_id == old_addon.addon_project_id) {
                if old_addon.version != new_addon.version {
                    diff.updated_addons.push((old_addon.version.clone(), new_addon.version.clone()));
                }
            }
        }

        // Find new addons (in new but not old)
        for new_addon in new_addons {
            if !old_addons.iter().any(|old_addon| old_addon.addon_project_id == new_addon.addon_project_id) {
                diff.new_addons.push(new_addon.addon_name.clone());
            }
        }
    }

    // Helper to extract filename from URL
    let extract_filename = |url: &str| -> Option<String> {
        url.split('/').last()
            .and_then(|name| {
                if name.contains('.') && !name.is_empty() {
                    Some(name.to_string())
                } else {
                    None
                }
            })
    };

    // Process each addon category
    process_addon_category(&old_manifest.mods, &new_manifest.mods, &mut diff, &extract_filename);
    process_addon_category(&old_manifest.resourcepacks, &new_manifest.resourcepacks, &mut diff, &extract_filename);
    process_addon_category(&old_manifest.shaderpacks, &new_manifest.shaderpacks, &mut diff, &extract_filename);
    process_addon_category(&old_manifest.datapacks, &new_manifest.datapacks, &mut diff, &extract_filename);

    Ok(diff)
}

async fn remove_old_files(modpack_path: &str, old_manifest: &Manifest, diff: &UpdateDiff) -> Result<(), String> {
    // Helper to remove files from a category directory
    async fn remove_category_files(
        modpack_path: &str,
        category_dir: &str,
        old_addons: &[crate::composables::manifest::Addon],
        diff: &UpdateDiff,
    ) -> Result<(), String> {
        use tokio_stream::{StreamExt, wrappers::ReadDirStream};
        
        let category_path = Path::new(modpack_path).join(category_dir);
        
        if !category_path.exists() {
            return Ok(()); // Directory doesn't exist, nothing to remove
        }

        // Read directory contents
        let dir_entries = async_fs::read_dir(&category_path).await
            .map_err(|e| format!("Failed to read directory {}: {}", category_path.display(), e))?;

        let mut dir_stream = ReadDirStream::new(dir_entries);

        while let Some(entry_result) = dir_stream.next().await {
            let entry = entry_result
                .map_err(|e| format!("Failed to read directory entry: {}", e))?;
            
            let file_path = entry.path();
            let file_name = file_path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            // Check if this file belongs to a removed addon
            for removed_addon in &diff.removed_addons {
                if let Some(old_addon) = old_addons.iter().find(|a| &a.addon_name == removed_addon) {
                    let expected_filename = extract_filename_from_url(&old_addon.cdn_download_url)
                        .unwrap_or_else(|| format!("{}-{}", old_addon.addon_name.replace(" ", "_"), old_addon.version));
                    
                    if file_name == expected_filename || file_name.contains(&old_addon.addon_name.replace(" ", "_")) {
                        async_fs::remove_file(&file_path).await
                            .map_err(|e| format!("Failed to remove file {}: {}", file_path.display(), e))?;
                        break; // File removed, no need to check other conditions
                    }
                }
            }

            // Check if this file belongs to an updated addon (remove old version)
            for old_addon in old_addons {
                // Check if there's a corresponding new version of this addon
                if diff.updated_addons.iter().any(|(old_ver, _)| old_ver == &old_addon.version) {
                    let expected_filename = extract_filename_from_url(&old_addon.cdn_download_url)
                        .unwrap_or_else(|| format!("{}-{}", old_addon.addon_name.replace(" ", "_"), old_addon.version));
                    
                    if file_name == expected_filename || 
                       (file_name.contains(&old_addon.addon_name.replace(" ", "_")) && file_name.contains(&old_addon.version)) {
                        async_fs::remove_file(&file_path).await
                            .map_err(|e| format!("Failed to remove old version {}: {}", file_path.display(), e))?;
                        break; // File removed, no need to check other conditions
                    }
                }
            }
        }

        Ok(())
    }

    // Remove old files from each category
    remove_category_files(modpack_path, "mods", &old_manifest.mods, diff).await?;
    remove_category_files(modpack_path, "resourcepacks", &old_manifest.resourcepacks, diff).await?;
    remove_category_files(modpack_path, "shaderpacks", &old_manifest.shaderpacks, diff).await?;
    remove_category_files(modpack_path, "datapacks", &old_manifest.datapacks, diff).await?;

    Ok(())
}
