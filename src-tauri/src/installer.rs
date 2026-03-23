use crate::composables::manifest::Manifest;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::command;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;
use tauri::Window;
use tauri::Emitter;

/// Validates that a path stays within the base directory (prevents path traversal attacks).
/// Returns the canonicalized destination path if valid, or an error if path traversal is detected.
fn validate_path_within_base(base_path: &Path, relative_path: &str) -> Result<PathBuf, String> {
    // Check for obvious path traversal patterns in the relative path
    if relative_path.contains("..") {
        return Err(format!("Path traversal detected: '{}' contains '..'", relative_path));
    }
    
    // Check for absolute paths (Windows and Unix)
    if relative_path.starts_with('/') ||
       (relative_path.len() > 1 && relative_path.chars().nth(1) == Some(':')) {
        return Err(format!("Path traversal detected: '{}' is an absolute path", relative_path));
    }
    
    // Check for home directory expansion
    if relative_path.starts_with('~') {
        return Err(format!("Path traversal detected: '{}' references home directory", relative_path));
    }
    
    // Join the paths
    let dest = base_path.join(relative_path);
    
    // Canonicalize the base path (the directory must exist)
    let canonical_base = base_path.canonicalize()
        .map_err(|e| format!("Failed to canonicalize base path: {}", e))?;
    
    // For the destination, we need to handle the case where parent directories don't exist yet
    // We canonicalize the parent if it exists, or check the path components
    let dest_for_check = if dest.exists() {
        dest.clone()
    } else {
        // Find the first existing parent and canonicalize that
        let mut current = dest.clone();
        while !current.exists() && current.parent().is_some() {
            current = current.parent().unwrap().to_path_buf();
        }
        if current.exists() {
            // Rebuild the path from the canonicalized parent
            let canonical_parent = current.canonicalize()
                .map_err(|e| format!("Failed to canonicalize parent path: {}", e))?;
            let remaining = dest.strip_prefix(&current)
                .map_err(|e| format!("Failed to strip prefix: {}", e))?;
            canonical_parent.join(remaining)
        } else {
            // No parent exists, use the base path
            canonical_base.clone()
        }
    };
    
    // Try to canonicalize the destination (or its calculation)
    let canonical_dest = if dest.exists() {
        dest.canonicalize()
            .map_err(|e| format!("Failed to canonicalize destination path: {}", e))?
    } else {
        dest_for_check
    };
    
    // Verify the destination is within the base directory
    if !canonical_dest.starts_with(&canonical_base) {
        return Err(format!(
            "Path traversal detected: '{}' resolves outside the modpack directory",
            relative_path
        ));
    }
    
    Ok(dest)
}

/// Configuration file with content for installation operations.
///
/// This struct is mirrored in multiple locations across the codebase:
/// - Rust: src-tauri/src/installer.rs (this file)
/// - Rust: src-tauri/src/composables/github.rs (ConfigFileWithContent struct)
/// - TypeScript: app/types/index.ts (ConfigFile and ConfigFileWithContent interfaces)
///
/// When modifying this struct, ensure all definitions remain consistent.
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigFile {
    pub filename: String,
    pub relative_path: String,
    pub content: String,
}

/// Options for install_update function
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstallOptions {
    /// Old manifest for cleanup of removed/updated addons
    pub old_manifest: Option<Manifest>,
    /// Whether to perform cleanup of old files (default: true when old_manifest provided)
    #[serde(default)]
    pub cleanup_old: bool,
}

/// Unified install function that handles all installation scenarios
#[command]
pub async fn install_update(
    window: Window,
    modpack_path: String,
    manifest: Manifest,
    config_files: Vec<ConfigFile>,
    options: Option<InstallOptions>,
) -> Result<(), String> {
    let options = options.unwrap_or_default();
    let client = Client::new();

    // Helper to emit progress
    fn emit_progress(window: &Window, progress: usize, total: usize, msg: &str) {
        let _ = Emitter::emit(window, "install-progress", Some(serde_json::json!({
            "progress": if total > 0 { (progress as f64) / (total as f64) * 100.0 } else { 100.0 },
            "message": msg
        })));
    }

    // Helper to download and save a file
    async fn download_and_save(
        client: &Client,
        url: &str,
        dest_path: &Path,
    ) -> Result<(), String> {
        let resp = client.get(url).send().await
            .map_err(|e| format!("Failed to download {}: {}", url, e))?;
        
        if !resp.status().is_success() {
            return Err(format!("Failed to download {}: HTTP {}", url, resp.status()));
        }
        
        let bytes = resp.bytes().await
            .map_err(|e| format!("Failed to read bytes from {}: {}", url, e))?;
        
        if let Some(parent) = dest_path.parent() {
            async_fs::create_dir_all(parent).await
                .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
        }
        
        let mut file = async_fs::File::create(dest_path).await
            .map_err(|e| format!("Failed to create file {}: {}", dest_path.display(), e))?;
        
        file.write_all(&bytes).await
            .map_err(|e| format!("Failed to write file {}: {}", dest_path.display(), e))?;
        
        Ok(())
    }

    // Calculate diff once for both cleanup and selective downloads
    let diff = if let Some(ref old_manifest) = options.old_manifest {
        Some(calculate_update_diff(old_manifest, &manifest)?)
    } else {
        None
    };

    // Step 1: Cleanup old files if requested and diff was calculated
    if options.cleanup_old {
        if let (Some(ref old_manifest), Some(ref diff)) = (options.old_manifest.as_ref(), &diff) {
            remove_old_files(&modpack_path, old_manifest, diff).await?;
        }
    }

    // Step 2: Install only changed/new addons and all config files
    let mut installed_paths: Vec<std::path::PathBuf> = Vec::new();
    let mut current = 0usize;

    /// Determines if an addon needs to be downloaded during an update.
    /// Returns true if the addon is:
    /// - New (not in old manifest)
    /// - Updated (same project_id, different version)
    /// - File doesn't exist on disk (safety fallback)
    fn should_download_addon(
        addon: &crate::composables::manifest::Addon,
        old_addons: &[crate::composables::manifest::Addon],
        diff: &UpdateDiff,
        dest_path: &Path,
    ) -> bool {
        // Check if this is a new addon
        let is_new = !old_addons.iter().any(|old| old.addon_project_id == addon.addon_project_id);
        if is_new {
            return true;
        }

        // Check if this addon was updated (version changed)
        let is_updated = diff.updated_addon_ids.contains(&addon.addon_project_id);
        if is_updated {
            return true;
        }

        // Safety fallback: download if file doesn't exist
        !dest_path.exists()
    }

    // Count files that actually need downloading for accurate progress
    let files_to_download = {
        let mut count = 0usize;
        
        // Count mods
        for addon in &manifest.mods {
            if addon.disabled == Some(true) {
                continue;
            }
            let dest = Path::new(&modpack_path).join("mods").join(&addon.file_name_on_disk);
            if let Some(ref d) = diff {
                if let Some(ref old_manifest) = options.old_manifest {
                    if should_download_addon(addon, &old_manifest.mods, d, &dest) {
                        count += 1;
                    }
                }
            } else {
                // No diff means fresh install - download everything
                count += 1;
            }
        }
        
        // Count resourcepacks
        for addon in &manifest.resourcepacks {
            if addon.disabled == Some(true) {
                continue;
            }
            let dest = Path::new(&modpack_path).join("resourcepacks").join(&addon.file_name_on_disk);
            if let Some(ref d) = diff {
                if let Some(ref old_manifest) = options.old_manifest {
                    if should_download_addon(addon, &old_manifest.resourcepacks, d, &dest) {
                        count += 1;
                    }
                }
            } else {
                count += 1;
            }
        }
        
        // Count shaderpacks
        for addon in &manifest.shaderpacks {
            if addon.disabled == Some(true) {
                continue;
            }
            let dest = Path::new(&modpack_path).join("shaderpacks").join(&addon.file_name_on_disk);
            if let Some(ref d) = diff {
                if let Some(ref old_manifest) = options.old_manifest {
                    if should_download_addon(addon, &old_manifest.shaderpacks, d, &dest) {
                        count += 1;
                    }
                }
            } else {
                count += 1;
            }
        }
        
        // Count datapacks
        for addon in &manifest.datapacks {
            if addon.disabled == Some(true) {
                continue;
            }
            let dest = Path::new(&modpack_path).join("datapacks").join(&addon.file_name_on_disk);
            if let Some(ref d) = diff {
                if let Some(ref old_manifest) = options.old_manifest {
                    if should_download_addon(addon, &old_manifest.datapacks, d, &dest) {
                        count += 1;
                    }
                }
            } else {
                count += 1;
            }
        }
        
        // Config files are always installed
        count + config_files.len()
    };

    // Install mods (selective download)
    for addon in &manifest.mods {
        if addon.disabled == Some(true) {
            continue;
        }
        let dest = Path::new(&modpack_path).join("mods").join(&addon.file_name_on_disk);
        
        // Check if we need to download this addon
        let needs_download = if let (Some(ref d), Some(ref old_manifest)) = (&diff, options.old_manifest.as_ref()) {
            should_download_addon(addon, &old_manifest.mods, d, &dest)
        } else {
            // No old manifest means fresh install - download everything
            true
        };
        
        if needs_download {
            download_and_save(&client, &addon.cdn_download_url, &dest).await?;
            emit_progress(&window, current + 1, files_to_download, &format!("Installed mod: {}", addon.addon_name));
        } else {
            log::info!("Skipping unchanged mod: {}", addon.addon_name);
        }
        installed_paths.push(dest);
        current += 1;
    }

    // Install resourcepacks (selective download)
    for addon in &manifest.resourcepacks {
        if addon.disabled == Some(true) {
            continue;
        }
        let dest = Path::new(&modpack_path).join("resourcepacks").join(&addon.file_name_on_disk);
        
        let needs_download = if let (Some(ref d), Some(ref old_manifest)) = (&diff, options.old_manifest.as_ref()) {
            should_download_addon(addon, &old_manifest.resourcepacks, d, &dest)
        } else {
            true
        };
        
        if needs_download {
            download_and_save(&client, &addon.cdn_download_url, &dest).await?;
            emit_progress(&window, current + 1, files_to_download, &format!("Installed resourcepack: {}", addon.addon_name));
        } else {
            log::info!("Skipping unchanged resourcepack: {}", addon.addon_name);
        }
        installed_paths.push(dest);
        current += 1;
    }

    // Install shaderpacks (selective download)
    for addon in &manifest.shaderpacks {
        if addon.disabled == Some(true) {
            continue;
        }
        let dest = Path::new(&modpack_path).join("shaderpacks").join(&addon.file_name_on_disk);
        
        let needs_download = if let (Some(ref d), Some(ref old_manifest)) = (&diff, options.old_manifest.as_ref()) {
            should_download_addon(addon, &old_manifest.shaderpacks, d, &dest)
        } else {
            true
        };
        
        if needs_download {
            download_and_save(&client, &addon.cdn_download_url, &dest).await?;
            emit_progress(&window, current + 1, files_to_download, &format!("Installed shaderpack: {}", addon.addon_name));
        } else {
            log::info!("Skipping unchanged shaderpack: {}", addon.addon_name);
        }
        installed_paths.push(dest);
        current += 1;
    }

    // Install datapacks (selective download)
    for addon in &manifest.datapacks {
        if addon.disabled == Some(true) {
            continue;
        }
        let dest = Path::new(&modpack_path).join("datapacks").join(&addon.file_name_on_disk);
        
        let needs_download = if let (Some(ref d), Some(ref old_manifest)) = (&diff, options.old_manifest.as_ref()) {
            should_download_addon(addon, &old_manifest.datapacks, d, &dest)
        } else {
            true
        };
        
        if needs_download {
            download_and_save(&client, &addon.cdn_download_url, &dest).await?;
            emit_progress(&window, current + 1, files_to_download, &format!("Installed datapack: {}", addon.addon_name));
        } else {
            log::info!("Skipping unchanged datapack: {}", addon.addon_name);
        }
        installed_paths.push(dest);
        current += 1;
    }

    // Install config files (with path traversal protection)
    let modpack_path_buf = PathBuf::from(&modpack_path);
    for config in config_files {
        // Validate the path to prevent path traversal attacks
        let dest = validate_path_within_base(&modpack_path_buf, &config.relative_path)?;
        
        if let Some(parent) = dest.parent() {
            async_fs::create_dir_all(parent).await
                .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
        }
        
        // Handle binary files that are base64-encoded
        if config.content.starts_with("data:application/octet-stream;base64,") {
            let base64_content = config.content.strip_prefix("data:application/octet-stream;base64,")
                .unwrap_or(&config.content);
            use base64::engine::general_purpose::STANDARD;
            use base64::Engine;
            let binary_data = STANDARD.decode(base64_content)
                .map_err(|e| format!("Failed to decode base64 config file {}: {}", dest.display(), e))?;
            async_fs::write(&dest, binary_data).await
                .map_err(|e| format!("Failed to write binary config file {}: {}", dest.display(), e))?;
        } else {
            async_fs::write(&dest, config.content.as_bytes()).await
                .map_err(|e| format!("Failed to write config file {}: {}", dest.display(), e))?;
        }
        
        installed_paths.push(dest.clone());
        current += 1;
        emit_progress(&window, current, files_to_download, &format!("Installed config: {}", dest.display()));
    }

    emit_progress(&window, files_to_download, files_to_download, "Installation complete!");
    Ok(())
}

/// Represents the difference between two manifest versions during an update.
///
/// This struct is mirrored in:
/// - Rust: src-tauri/src/installer.rs (this file)
/// - TypeScript: app/types/index.ts (UpdateDiff interface)
///
/// When modifying this struct, ensure all definitions remain consistent.
#[derive(Debug, Clone)]
pub struct UpdateDiff {
    pub removed_addons: Vec<String>,
    /// Project IDs of addons that were updated (matched by project_id, not version)
    pub updated_addon_ids: Vec<u64>,
    pub new_addons: Vec<String>,
}

fn calculate_update_diff(old_manifest: &Manifest, new_manifest: &Manifest) -> Result<UpdateDiff, String> {
    let mut diff = UpdateDiff {
        removed_addons: Vec::new(),
        updated_addon_ids: Vec::new(),
        new_addons: Vec::new(),
    };

    fn process_addon_category(
        old_addons: &[crate::composables::manifest::Addon],
        new_addons: &[crate::composables::manifest::Addon],
        diff: &mut UpdateDiff,
    ) {
        // Find removed addons
        for old_addon in old_addons {
            if old_addon.disabled.unwrap_or(false) {
                continue;
            }
            let maybe_new = new_addons.iter()
                .find(|new_addon| new_addon.addon_project_id == old_addon.addon_project_id);
            
            if maybe_new.is_none() {
                diff.removed_addons.push(old_addon.addon_name.clone());
            } else if let Some(new_addon) = maybe_new {
                if new_addon.disabled == Some(true) {
                    diff.removed_addons.push(old_addon.addon_name.clone());
                }
            }
        }

        // Find updated addons (match by project_id, not version string)
        for old_addon in old_addons {
            if let Some(new_addon) = new_addons.iter()
                .find(|a| a.addon_project_id == old_addon.addon_project_id)
            {
                if old_addon.version != new_addon.version {
                    // Store project_id for reliable matching during removal
                    diff.updated_addon_ids.push(old_addon.addon_project_id);
                }
            }
        }

        // Find new addons
        for new_addon in new_addons {
            if !old_addons.iter().any(|old_addon| old_addon.addon_project_id == new_addon.addon_project_id) {
                diff.new_addons.push(new_addon.addon_name.clone());
            }
        }
    }

    process_addon_category(&old_manifest.mods, &new_manifest.mods, &mut diff);
    process_addon_category(&old_manifest.resourcepacks, &new_manifest.resourcepacks, &mut diff);
    process_addon_category(&old_manifest.shaderpacks, &new_manifest.shaderpacks, &mut diff);
    process_addon_category(&old_manifest.datapacks, &new_manifest.datapacks, &mut diff);

    Ok(diff)
}

async fn remove_old_files(modpack_path: &str, old_manifest: &Manifest, diff: &UpdateDiff) -> Result<(), String> {
    log::info!("remove_old_files: Starting removal for {} removed, {} updated addons",
        diff.removed_addons.len(), diff.updated_addon_ids.len());

    async fn remove_category_files(
        modpack_path: &str,
        category_dir: &str,
        old_addons: &[crate::composables::manifest::Addon],
        diff: &UpdateDiff,
    ) -> Result<(), String> {
        let category_path = Path::new(modpack_path).join(category_dir);
        
        if !category_path.exists() {
            return Ok(());
        }

        let mut dir_entries = async_fs::read_dir(&category_path).await
            .map_err(|e| format!("Failed to read directory {}: {}", category_path.display(), e))?;

        while let Some(entry) = dir_entries.next_entry().await.map_err(|e| e.to_string())? {
            let file_path = entry.path();
            let file_name = file_path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            // Check for removed addons
            for removed_addon in &diff.removed_addons {
                if let Some(old_addon) = old_addons.iter().find(|a| &a.addon_name == removed_addon) {
                    let exact_filename = &old_addon.file_name_on_disk;
                    let disabled_filename = format!("{}.disabled", exact_filename);
                    
                    if file_name == exact_filename || file_name == &disabled_filename {
                        log::info!("Removing file for addon '{}': {}", removed_addon, file_path.display());
                        async_fs::remove_file(&file_path).await
                            .map_err(|e| format!("Failed to remove file {}: {}", file_path.display(), e))?;
                        break;
                    }
                }
            }

            // Check for updated addons (match by project_id for reliable identification)
            for old_addon in old_addons {
                // Check if this addon has an update by matching project_id
                let is_updated = diff.updated_addon_ids.contains(&old_addon.addon_project_id);
                if is_updated {
                    // Use exact filename matching for safety
                    let exact_filename = &old_addon.file_name_on_disk;
                    let disabled_filename = format!("{}.disabled", exact_filename);
                    
                    if file_name == exact_filename || file_name == &disabled_filename {
                        log::info!("Removing old version of '{}': {}", old_addon.addon_name, file_path.display());
                        async_fs::remove_file(&file_path).await
                            .map_err(|e| format!("Failed to remove old version {}: {}", file_path.display(), e))?;
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    remove_category_files(modpack_path, "mods", &old_manifest.mods, diff).await?;
    remove_category_files(modpack_path, "resourcepacks", &old_manifest.resourcepacks, diff).await?;
    remove_category_files(modpack_path, "shaderpacks", &old_manifest.shaderpacks, diff).await?;
    remove_category_files(modpack_path, "datapacks", &old_manifest.datapacks, diff).await?;

    log::info!("remove_old_files: Removal complete");
    Ok(())
}
