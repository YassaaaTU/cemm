use crate::composables::manifest::Manifest;
use reqwest::Client;
use serde::Deserialize;
use std::path::{Path, PathBuf};
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
    pub filename: String,
    pub relative_path: String,
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
    // Helper to download, save, and check integrity
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
        let mut current = 0usize;        // Mods
        for addon in &manifest.mods {
            // Skip disabled addons
            if addon.disabled == Some(true) {
                continue;
            }
            let filename = &addon.file_name_on_disk;
            let dest = Path::new(&modpack_path).join("mods").join(filename);
            download_save_check(&client, &addon.cdn_download_url, &dest, None).await?;
            installed_paths.push(dest.clone());
            current += 1;
            emit_progress(window.as_ref(), current, total_files, &format!("Installed mod: {}", addon.addon_name));
        }        // Resourcepacks
        for addon in &manifest.resourcepacks {
            // Skip disabled addons
            if addon.disabled == Some(true) {
                continue;
            }
            let filename = &addon.file_name_on_disk;
            let dest = Path::new(&modpack_path).join("resourcepacks").join(filename);
            download_save_check(&client, &addon.cdn_download_url, &dest, None).await?;
            installed_paths.push(dest.clone());
            current += 1;
            emit_progress(window.as_ref(), current, total_files, &format!("Installed resourcepack: {}", addon.addon_name));
        }        // Shaderpacks
        for addon in &manifest.shaderpacks {
            // Skip disabled addons
            if addon.disabled == Some(true) {
                continue;
            }
            let filename = &addon.file_name_on_disk;
            let dest = Path::new(&modpack_path).join("shaderpacks").join(filename);
            download_save_check(&client, &addon.cdn_download_url, &dest, None).await?;
            installed_paths.push(dest.clone());
            current += 1;
            emit_progress(window.as_ref(), current, total_files, &format!("Installed shaderpack: {}", addon.addon_name));
        }        // Datapacks
        for addon in &manifest.datapacks {
            // Skip disabled addons
            if addon.disabled == Some(true) {
                continue;
            }
            let filename = &addon.file_name_on_disk;
            let dest = Path::new(&modpack_path).join("datapacks").join(filename);
            download_save_check(&client, &addon.cdn_download_url, &dest, None).await?;
            installed_paths.push(dest.clone());
            current += 1;
            emit_progress(window.as_ref(), current, total_files, &format!("Installed datapack: {}", addon.addon_name));
        }
        // Config files
        for config in config_files {
            let dest = Path::new(&modpack_path).join(&config.relative_path);
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
        // Find removed addons (in old but not in new, or disabled in new)
        for old_addon in old_addons {
            let maybe_new = new_addons.iter().find(|new_addon| new_addon.addon_project_id == old_addon.addon_project_id);
            if maybe_new.is_none() {
                diff.removed_addons.push(old_addon.addon_name.clone());
            } else if let Some(new_addon) = maybe_new {
                if new_addon.disabled == Some(true) {
                    diff.removed_addons.push(old_addon.addon_name.clone());
                }
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
                .unwrap_or("");            // Check if this file belongs to a removed addon
            for removed_addon in &diff.removed_addons {
                if let Some(old_addon) = old_addons.iter().find(|a| &a.addon_name == removed_addon) {
                    let exact_filename = &old_addon.file_name_on_disk;
                    let disabled_filename = format!("{}.disabled", exact_filename);
                    
                    // Remove if matches exact filename or .disabled variant
                    if file_name == exact_filename || file_name == disabled_filename {
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

#[command]
pub async fn install_update_optimized(
    modpack_path: String,
    old_manifest: Option<Manifest>,
    new_manifest: Manifest,
    config_files: Vec<ConfigFile>,
) -> Result<(), String> {
    // Check if this is a config-only update
    if new_manifest.update_type.as_ref() == Some(&"config".to_string()) {
        // Config-only update: just install config files, don't touch addons
        println!("🔧 Config-only update: {} config files", config_files.len());
        
        // Helper to emit progress for config-only updates
        let emit_progress = |current: usize, total: usize, msg: &str| {
            let progress = if total > 0 { current as f64 / total as f64 * 100.0 } else { 100.0 };
            println!("Progress: {:.1}% - {}", progress, msg);
        };
        
        emit_progress(1, 10, "Installing config files only...");
        
        // Install config files
        for (i, config_file) in config_files.iter().enumerate() {
            emit_progress(i + 1, config_files.len(), &format!("Installing config file: {}", config_file.relative_path));

            let file_path = format!("{}/{}", modpack_path, config_file.relative_path);
            let parent_dir = std::path::Path::new(&file_path).parent()
                .ok_or("Invalid config file path")?;
            std::fs::create_dir_all(parent_dir)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
            std::fs::write(&file_path, &config_file.content)
                .map_err(|e| format!("Failed to write config file {}: {}", config_file.relative_path, e))?;
        }
        
        emit_progress(config_files.len(), config_files.len(), "Config-only update complete!");
        return Ok(());
    }

    // If no old manifest, fall back to regular install (all addons are "new")
    if old_manifest.is_none() {
        return install_update(modpack_path, new_manifest, config_files).await;
    }

    let old_manifest = old_manifest.unwrap();
    let client = Client::new();

    // Step 1: Calculate what needs to be changed
    let diff = calculate_detailed_update_diff(&old_manifest, &new_manifest)?;
    
    // Step 2: Remove old files first
    remove_old_files(&modpack_path, &old_manifest, &calculate_update_diff(&old_manifest, &new_manifest)?).await?;
    
    // Step 3: Only download and install changed addons (optimized!)
    install_changed_addons(&client, &modpack_path, &new_manifest, &diff, config_files).await
}

#[derive(Debug, Clone)]
pub struct DetailedUpdateDiff {
    pub addons_to_remove: Vec<crate::composables::manifest::Addon>, // Full addon info for removal
    pub addons_to_update: Vec<crate::composables::manifest::Addon>, // New versions to download
    pub addons_to_add: Vec<crate::composables::manifest::Addon>, // Completely new addons
    pub addons_unchanged: Vec<crate::composables::manifest::Addon>, // Skip these entirely
}

fn calculate_detailed_update_diff(old_manifest: &Manifest, new_manifest: &Manifest) -> Result<DetailedUpdateDiff, String> {
    let mut diff = DetailedUpdateDiff {
        addons_to_remove: Vec::new(),
        addons_to_update: Vec::new(),
        addons_to_add: Vec::new(),
        addons_unchanged: Vec::new(),
    };

    // Helper to process each addon category with detailed diff
    fn process_category_detailed(
        old_addons: &[crate::composables::manifest::Addon],
        new_addons: &[crate::composables::manifest::Addon],
        diff: &mut DetailedUpdateDiff,
    ) {
        // Process each old addon
        for old_addon in old_addons {
            if let Some(new_addon) = new_addons.iter().find(|a| a.addon_project_id == old_addon.addon_project_id) {
                if new_addon.disabled == Some(true) {
                    // Old addon exists but new one is disabled = removal
                    diff.addons_to_remove.push(old_addon.clone());
                } else if old_addon.version != new_addon.version {
                    // Same addon, different version = update (download new)
                    diff.addons_to_update.push(new_addon.clone());
                } else {
                    // Same addon, same version = unchanged (skip download)
                    diff.addons_unchanged.push(new_addon.clone());
                }
            } else {
                // Old addon not in new manifest = removal
                diff.addons_to_remove.push(old_addon.clone());
            }
        }

        // Process each new addon to find completely new ones
        for new_addon in new_addons {
            if new_addon.disabled == Some(true) {
                continue; // Skip disabled addons
            }
            
            if !old_addons.iter().any(|old_addon| old_addon.addon_project_id == new_addon.addon_project_id) {
                // New addon not in old manifest = addition (download)
                diff.addons_to_add.push(new_addon.clone());
            }
        }
    }

    // Process all categories
    process_category_detailed(&old_manifest.mods, &new_manifest.mods, &mut diff);
    process_category_detailed(&old_manifest.resourcepacks, &new_manifest.resourcepacks, &mut diff);
    process_category_detailed(&old_manifest.shaderpacks, &new_manifest.shaderpacks, &mut diff);
    process_category_detailed(&old_manifest.datapacks, &new_manifest.datapacks, &mut diff);

    Ok(diff)
}

async fn install_changed_addons(
    client: &Client,
    modpack_path: &str,
    _new_manifest: &Manifest,
    diff: &DetailedUpdateDiff,
    config_files: Vec<ConfigFile>,
) -> Result<(), String> {
    // Calculate total work: only addons that need downloading + config files
    let total_downloads = diff.addons_to_add.len() + diff.addons_to_update.len() + config_files.len();
    let mut current_progress = 0;
    let mut installed_paths: Vec<std::path::PathBuf> = Vec::new();

    println!("🚀 Optimized install: {} new, {} updated, {} unchanged (skipped), {} config files", 
        diff.addons_to_add.len(), 
        diff.addons_to_update.len(), 
        diff.addons_unchanged.len(),
        config_files.len()
    );    // Helper to emit progress 
    let emit_progress = |current: usize, total: usize, msg: &str| {
        // Note: In real implementation, you'd emit to window here
        let progress = if total > 0 { current as f64 / total as f64 * 100.0 } else { 100.0 };
        println!("Progress: {:.1}% - {}", progress, msg);
    };    // Helper async function to download and install a single addon
    async fn download_addon_helper(
        client: &Client,
        addon: crate::composables::manifest::Addon,
        category: String,
        modpack_path: String,
    ) -> Result<PathBuf, String> {
        let filename = &addon.file_name_on_disk;
        let dest = Path::new(&modpack_path).join(&category).join(filename);
        
        // Download the addon
        let resp = client.get(&addon.cdn_download_url).send().await.map_err(|e| {
            format!("Failed to download {}: {}", addon.addon_name, e)
        })?;
        
        if !resp.status().is_success() {
            return Err(format!("Failed to download {}: HTTP {}", addon.addon_name, resp.status()));
        }
        
        let bytes = resp.bytes().await.map_err(|e| {
            format!("Failed to read bytes for {}: {}", addon.addon_name, e)
        })?;
        
        // Create directory if needed
        if let Some(parent) = dest.parent() {
            async_fs::create_dir_all(parent).await.map_err(|e| {
                format!("Failed to create directory {}: {}", parent.display(), e)
            })?;
        }
        
        // Write file
        async_fs::write(&dest, bytes).await.map_err(|e| {
            format!("Failed to write {}: {}", dest.display(), e)
        })?;
        
        Ok(dest)
    }// Install new addons
    for addon in &diff.addons_to_add {
        if addon.disabled == Some(true) { continue; }        // Determine category properly by checking which list in new_manifest contains this addon
        let category = determine_addon_category(_new_manifest, addon);
        
        match download_addon_helper(client, addon.clone(), category.to_string(), modpack_path.to_string()).await {
            Ok(path) => {
                installed_paths.push(path);
                current_progress += 1;
                emit_progress(current_progress, total_downloads, &format!("Downloaded new: {}", addon.addon_name));
            }
            Err(e) => {
                // Rollback on error
                for path in &installed_paths {
                    let _ = async_fs::remove_file(path).await;
                }
                return Err(format!("Failed to install new addon: {}", e));
            }
        }
    }

    // Install updated addons
    for addon in &diff.addons_to_update {
        if addon.disabled == Some(true) { continue; }        let category = determine_addon_category(_new_manifest, addon);
        
        match download_addon_helper(client, addon.clone(), category.to_string(), modpack_path.to_string()).await {
            Ok(path) => {
                installed_paths.push(path);
                current_progress += 1;
                emit_progress(current_progress, total_downloads, &format!("Updated: {}", addon.addon_name));
            }
            Err(e) => {
                // Rollback on error
                for path in &installed_paths {
                    let _ = async_fs::remove_file(path).await;
                }
                return Err(format!("Failed to update addon: {}", e));
            }
        }
    }

    // Install config files
    for config in config_files {
        let dest = Path::new(modpack_path).join(&config.relative_path);
        if let Some(parent) = dest.parent() {
            async_fs::create_dir_all(parent).await.map_err(|e| {
                format!("Failed to create directory {}: {}", parent.display(), e)
            })?;
        }
        
        async_fs::write(&dest, config.content).await.map_err(|e| {
            format!("Failed to write config file {}: {}", dest.display(), e)
        })?;
        
        installed_paths.push(dest.clone());
        current_progress += 1;
        emit_progress(current_progress, total_downloads, &format!("Installed config: {}", dest.display()));
    }

    emit_progress(total_downloads, total_downloads, "Optimized installation complete!");
    println!("✅ Optimized install complete! Skipped {} unchanged addons.", diff.addons_unchanged.len());
    
    Ok(())
}

fn determine_addon_category(manifest: &Manifest, addon: &crate::composables::manifest::Addon) -> String {
    if manifest.mods.iter().any(|a| a.addon_project_id == addon.addon_project_id) {
        "mods".to_string()
    } else if manifest.resourcepacks.iter().any(|a| a.addon_project_id == addon.addon_project_id) {
        "resourcepacks".to_string()
    } else if manifest.shaderpacks.iter().any(|a| a.addon_project_id == addon.addon_project_id) {
        "shaderpacks".to_string()
    } else {
        "datapacks".to_string()
    }
}
