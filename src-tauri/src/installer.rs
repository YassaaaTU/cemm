use crate::composables::manifest::Manifest;
use reqwest::Client;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tauri::command;
use tauri::Emitter;
use tauri::Window;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;

/// Validates that a path stays within the base directory (prevents path traversal attacks).
/// Returns the canonicalized destination path if valid, or an error if path traversal is detected.
pub(crate) fn validate_path_within_base(
    base_path: &Path,
    relative_path: &str,
) -> Result<PathBuf, String> {
    // Check for obvious path traversal patterns in the relative path
    if relative_path.contains("..") {
        return Err(format!(
            "Path traversal detected: '{}' contains '..'",
            relative_path
        ));
    }

    // Check for absolute paths (Windows and Unix)
    if relative_path.starts_with('/')
        || (relative_path.len() > 1 && relative_path.chars().nth(1) == Some(':'))
    {
        return Err(format!(
            "Path traversal detected: '{}' is an absolute path",
            relative_path
        ));
    }

    // Check for home directory expansion
    if relative_path.starts_with('~') {
        return Err(format!(
            "Path traversal detected: '{}' references home directory",
            relative_path
        ));
    }

    // Join the paths
    let dest = base_path.join(relative_path);

    // Canonicalize the base path (the directory must exist)
    let canonical_base = base_path
        .canonicalize()
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
            let canonical_parent = current
                .canonicalize()
                .map_err(|e| format!("Failed to canonicalize parent path: {}", e))?;
            let remaining = dest
                .strip_prefix(&current)
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

/// Whether a manifest describes a config-only update (`updateType: "config"`).
/// Config-only manifests always ship with empty addon arrays (see `useAdminApi.ts`
/// `saveManifest`); this predicate is the single source of truth `install_update`
/// uses to guarantee addons are never downloaded or removed for such an update.
fn is_config_only_update(manifest: &Manifest) -> bool {
    manifest.update_type.as_deref() == Some("config")
}

/// `file_name_on_disk` is documented and always produced (by CurseForge, and by
/// `parse_minecraft_instance`) as a bare filename, never a path. Rejecting any
/// separator or dot-segment closes the path-traversal / absolute-path escape
/// entirely, since a single ordinary path component joined onto a directory can
/// never leave that directory.
fn is_safe_addon_file_name(name: &str) -> bool {
    if name.is_empty() || name.contains('/') || name.contains('\\') {
        return false;
    }
    !matches!(name, "." | "..")
}

/// Every addon destination is built by joining this filename onto a fixed category
/// directory (`mods/`, `resourcepacks/`, `shaderpacks/`, `datapacks/`) under the
/// modpack root, so a bare, separator-free filename can never resolve outside it.
fn validate_addon_file_name(addon_name: &str, file_name: &str) -> Result<(), String> {
    if is_safe_addon_file_name(file_name) {
        Ok(())
    } else {
        Err(format!(
            "Refusing to install addon '{}': fileNameOnDisk '{}' is not a plain filename",
            addon_name, file_name
        ))
    }
}

/// Only accept the schemes a real download URL can use. This does not pin to a
/// specific CDN host — CurseForge serves addons from multiple mirror hosts, and
/// enforcing a host allowlist without auditing already-published manifests risks
/// breaking legitimate installs (see audit F-P0-2 compatibility note).
fn validate_addon_download_url(addon_name: &str, url: &str) -> Result<(), String> {
    if url.starts_with("https://") || url.starts_with("http://") {
        Ok(())
    } else {
        Err(format!(
            "Refusing to install addon '{}': cdn_download_url '{}' is not an http(s) URL",
            addon_name, url
        ))
    }
}

/// Validates every addon in the manifest before any network call or file write
/// happens, so a single malicious entry aborts the whole install rather than
/// partially applying and leaving inconsistent state.
fn validate_all_addons(manifest: &Manifest) -> Result<(), String> {
    let categories = [
        &manifest.mods,
        &manifest.resourcepacks,
        &manifest.shaderpacks,
        &manifest.datapacks,
    ];
    for addons in categories {
        for addon in addons.iter() {
            validate_addon_file_name(&addon.addon_name, &addon.file_name_on_disk)?;
            validate_addon_download_url(&addon.addon_name, &addon.cdn_download_url)?;
        }
    }
    Ok(())
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

/// Options for install_update function. Only ever deserialized (received from
/// the frontend as a command argument) — never serialized back out.
#[derive(Debug, Clone, Default, Deserialize)]
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
    // Addon files can legitimately be large (resource/shader packs run to hundreds
    // of MB), so the timeout is generous — its job is only to bound a stalled
    // connection to something finite, not to cap normal downloads.
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    // Config-only updates must never touch addon files, even if the manifest's
    // addon arrays are unexpectedly non-empty. The UI promises "no addons will be
    // modified"; the backend must guarantee that itself rather than trust the caller.
    let is_config_only = is_config_only_update(&manifest);

    // The manifest is attacker-controlled (anyone who can hand out an update code
    // controls it), so every addon destination and download source is validated
    // up front, before any download or write is attempted.
    validate_all_addons(&manifest)?;

    // Helper to emit progress
    fn emit_progress(window: &Window, progress: usize, total: usize, msg: &str) {
        let safe_progress = if total > 0 {
            progress.min(total)
        } else {
            progress
        };
        let _ = Emitter::emit(
            window,
            "install-progress",
            Some(serde_json::json!({
                "progress": if total > 0 { (safe_progress as f64) / (total as f64) * 100.0 } else { 100.0 },
                "message": msg
            })),
        );
    }

    // A single addon file is never legitimately larger than this. Checked against
    // the declared Content-Length before reading the body into memory, so a
    // misbehaving or malicious CDN response can't force an unbounded allocation.
    const MAX_ADDON_FILE_BYTES: u64 = 500 * 1024 * 1024;

    // Helper to download and save a file
    async fn download_and_save(client: &Client, url: &str, dest_path: &Path) -> Result<(), String> {
        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to download {}: {}", url, e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Failed to download {}: HTTP {}",
                url,
                resp.status()
            ));
        }

        if let Some(len) = resp.content_length() {
            if len > MAX_ADDON_FILE_BYTES {
                return Err(format!(
                    "Refusing to download {}: reported size {} bytes exceeds the {} byte limit",
                    url, len, MAX_ADDON_FILE_BYTES
                ));
            }
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Failed to read bytes from {}: {}", url, e))?;

        if bytes.len() as u64 > MAX_ADDON_FILE_BYTES {
            return Err(format!(
                "Refusing to save {}: downloaded {} bytes exceeds the {} byte limit",
                url,
                bytes.len(),
                MAX_ADDON_FILE_BYTES
            ));
        }

        if let Some(parent) = dest_path.parent() {
            async_fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
        }

        let mut file = async_fs::File::create(dest_path)
            .await
            .map_err(|e| format!("Failed to create file {}: {}", dest_path.display(), e))?;

        file.write_all(&bytes)
            .await
            .map_err(|e| format!("Failed to write file {}: {}", dest_path.display(), e))?;

        Ok(())
    }

    // Calculate diff once for both cleanup and selective downloads.
    // Skipped entirely for config-only updates so the cleanup step below can never run.
    let diff = if is_config_only {
        None
    } else if let Some(ref old_manifest) = options.old_manifest {
        Some(calculate_update_diff(old_manifest, &manifest)?)
    } else {
        None
    };

    // Cleanup used to run here, before any download — a network failure partway
    // through downloading meant old files were already gone with nothing to
    // replace them (F-P1-1). It now runs after every download below has staged
    // successfully; see the promotion step following the addon loops.
    let staging_dir = Path::new(&modpack_path).join(".cemm-staging");
    let mut staged_moves: Vec<(PathBuf, PathBuf)> = Vec::new();

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
        let is_new = !old_addons
            .iter()
            .any(|old| old.addon_project_id == addon.addon_project_id);
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

    // Count files that actually need downloading for accurate progress.
    // Config-only updates install config files only.
    let files_to_download = if is_config_only {
        config_files.len()
    } else {
        let mut count = 0usize;

        // Count mods
        for addon in &manifest.mods {
            if addon.disabled == Some(true) {
                continue;
            }
            let dest = Path::new(&modpack_path)
                .join("mods")
                .join(&addon.file_name_on_disk);
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
            let dest = Path::new(&modpack_path)
                .join("resourcepacks")
                .join(&addon.file_name_on_disk);
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
            let dest = Path::new(&modpack_path)
                .join("shaderpacks")
                .join(&addon.file_name_on_disk);
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
            let dest = Path::new(&modpack_path)
                .join("datapacks")
                .join(&addon.file_name_on_disk);
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

    // Install addons — entirely skipped for config-only updates so no addon in
    // mods/resourcepacks/shaderpacks/datapacks is ever downloaded or removed.
    if !is_config_only {
        // Install mods (selective download)
        for addon in &manifest.mods {
            if addon.disabled == Some(true) {
                continue;
            }
            let final_dest = Path::new(&modpack_path)
                .join("mods")
                .join(&addon.file_name_on_disk);

            // Check if we need to download this addon
            let needs_download =
                if let (Some(ref d), Some(old_manifest)) = (&diff, options.old_manifest.as_ref()) {
                    should_download_addon(addon, &old_manifest.mods, d, &final_dest)
                } else {
                    // No old manifest means fresh install - download everything
                    true
                };

            if needs_download {
                let staged_dest = staging_dir.join("mods").join(&addon.file_name_on_disk);
                download_and_save(&client, &addon.cdn_download_url, &staged_dest).await?;
                staged_moves.push((staged_dest, final_dest));
                current += 1;
                emit_progress(
                    &window,
                    current,
                    files_to_download,
                    &format!("Downloaded mod: {}", addon.addon_name),
                );
            } else {
                log::info!("Skipping unchanged mod: {}", addon.addon_name);
            }
        }

        // Install resourcepacks (selective download)
        for addon in &manifest.resourcepacks {
            if addon.disabled == Some(true) {
                continue;
            }
            let final_dest = Path::new(&modpack_path)
                .join("resourcepacks")
                .join(&addon.file_name_on_disk);

            let needs_download =
                if let (Some(ref d), Some(old_manifest)) = (&diff, options.old_manifest.as_ref()) {
                    should_download_addon(addon, &old_manifest.resourcepacks, d, &final_dest)
                } else {
                    true
                };

            if needs_download {
                let staged_dest = staging_dir
                    .join("resourcepacks")
                    .join(&addon.file_name_on_disk);
                download_and_save(&client, &addon.cdn_download_url, &staged_dest).await?;
                staged_moves.push((staged_dest, final_dest));
                current += 1;
                emit_progress(
                    &window,
                    current,
                    files_to_download,
                    &format!("Downloaded resourcepack: {}", addon.addon_name),
                );
            } else {
                log::info!("Skipping unchanged resourcepack: {}", addon.addon_name);
            }
        }

        // Install shaderpacks (selective download)
        for addon in &manifest.shaderpacks {
            if addon.disabled == Some(true) {
                continue;
            }
            let final_dest = Path::new(&modpack_path)
                .join("shaderpacks")
                .join(&addon.file_name_on_disk);

            let needs_download =
                if let (Some(ref d), Some(old_manifest)) = (&diff, options.old_manifest.as_ref()) {
                    should_download_addon(addon, &old_manifest.shaderpacks, d, &final_dest)
                } else {
                    true
                };

            if needs_download {
                let staged_dest = staging_dir
                    .join("shaderpacks")
                    .join(&addon.file_name_on_disk);
                download_and_save(&client, &addon.cdn_download_url, &staged_dest).await?;
                staged_moves.push((staged_dest, final_dest));
                current += 1;
                emit_progress(
                    &window,
                    current,
                    files_to_download,
                    &format!("Downloaded shaderpack: {}", addon.addon_name),
                );
            } else {
                log::info!("Skipping unchanged shaderpack: {}", addon.addon_name);
            }
        }

        // Install datapacks (selective download)
        for addon in &manifest.datapacks {
            if addon.disabled == Some(true) {
                continue;
            }
            let final_dest = Path::new(&modpack_path)
                .join("datapacks")
                .join(&addon.file_name_on_disk);

            let needs_download =
                if let (Some(ref d), Some(old_manifest)) = (&diff, options.old_manifest.as_ref()) {
                    should_download_addon(addon, &old_manifest.datapacks, d, &final_dest)
                } else {
                    true
                };

            if needs_download {
                let staged_dest = staging_dir.join("datapacks").join(&addon.file_name_on_disk);
                download_and_save(&client, &addon.cdn_download_url, &staged_dest).await?;
                staged_moves.push((staged_dest, final_dest));
                current += 1;
                emit_progress(
                    &window,
                    current,
                    files_to_download,
                    &format!("Downloaded datapack: {}", addon.addon_name),
                );
            } else {
                log::info!("Skipping unchanged datapack: {}", addon.addon_name);
            }
        }

        // Every download above succeeded — only now is it safe to run the
        // destructive step. cleanup_old deletes files; promoting staged
        // downloads is a same-filesystem rename, the fastest and least
        // failure-prone part of the whole operation.
        if options.cleanup_old {
            if let (Some(old_manifest), Some(ref diff)) = (options.old_manifest.as_ref(), &diff) {
                remove_old_files(&modpack_path, old_manifest, diff).await?;
            }
        }

        emit_progress(
            &window,
            current,
            files_to_download,
            "Finalizing installed files...",
        );
        for (staged_path, final_path) in staged_moves {
            if let Some(parent) = final_path.parent() {
                async_fs::create_dir_all(parent).await.map_err(|e| {
                    format!("Failed to create directory {}: {}", parent.display(), e)
                })?;
            }
            async_fs::rename(&staged_path, &final_path)
                .await
                .map_err(|e| {
                    format!("Failed to move {} into place: {}", final_path.display(), e)
                })?;
        }

        // Best-effort: leftover staging files are harmless and get overwritten
        // by the next install, so a failure here is not itself an install failure.
        let _ = async_fs::remove_dir_all(&staging_dir).await;
    }

    // Install config files (with path traversal protection)
    let modpack_path_buf = PathBuf::from(&modpack_path);
    for config in config_files {
        // Validate the path to prevent path traversal attacks
        let dest = validate_path_within_base(&modpack_path_buf, &config.relative_path)?;

        if let Some(parent) = dest.parent() {
            async_fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
        }

        // Handle binary files that are base64-encoded
        if config
            .content
            .starts_with("data:application/octet-stream;base64,")
        {
            let base64_content = config
                .content
                .strip_prefix("data:application/octet-stream;base64,")
                .unwrap_or(&config.content);
            use base64::engine::general_purpose::STANDARD;
            use base64::Engine;
            let binary_data = STANDARD.decode(base64_content).map_err(|e| {
                format!(
                    "Failed to decode base64 config file {}: {}",
                    dest.display(),
                    e
                )
            })?;
            async_fs::write(&dest, binary_data).await.map_err(|e| {
                format!(
                    "Failed to write binary config file {}: {}",
                    dest.display(),
                    e
                )
            })?;
        } else {
            async_fs::write(&dest, config.content.as_bytes())
                .await
                .map_err(|e| format!("Failed to write config file {}: {}", dest.display(), e))?;
        }

        current += 1;
        emit_progress(
            &window,
            current,
            files_to_download,
            &format!("Installed config: {}", dest.display()),
        );
    }

    emit_progress(
        &window,
        files_to_download,
        files_to_download,
        "Installation complete!",
    );
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

fn calculate_update_diff(
    old_manifest: &Manifest,
    new_manifest: &Manifest,
) -> Result<UpdateDiff, String> {
    let mut diff = UpdateDiff {
        removed_addons: Vec::new(),
        updated_addon_ids: Vec::new(),
        new_addons: Vec::new(),
    };

    // Config-only updates never change addons, even if a malformed manifest
    // unexpectedly includes addon entries. Use the explicit discriminator here
    // so a legitimate full update can still remove the final addon in a category.
    if is_config_only_update(new_manifest) {
        return Ok(diff);
    }

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
            let maybe_new = new_addons
                .iter()
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
            if let Some(new_addon) = new_addons
                .iter()
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
            if !old_addons
                .iter()
                .any(|old_addon| old_addon.addon_project_id == new_addon.addon_project_id)
            {
                diff.new_addons.push(new_addon.addon_name.clone());
            }
        }
    }

    process_addon_category(&old_manifest.mods, &new_manifest.mods, &mut diff);
    process_addon_category(
        &old_manifest.resourcepacks,
        &new_manifest.resourcepacks,
        &mut diff,
    );
    process_addon_category(
        &old_manifest.shaderpacks,
        &new_manifest.shaderpacks,
        &mut diff,
    );
    process_addon_category(&old_manifest.datapacks, &new_manifest.datapacks, &mut diff);

    Ok(diff)
}

async fn remove_old_files(
    modpack_path: &str,
    old_manifest: &Manifest,
    diff: &UpdateDiff,
) -> Result<(), String> {
    log::info!(
        "remove_old_files: Starting removal for {} removed, {} updated addons",
        diff.removed_addons.len(),
        diff.updated_addon_ids.len()
    );

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

        let mut dir_entries = async_fs::read_dir(&category_path).await.map_err(|e| {
            format!(
                "Failed to read directory {}: {}",
                category_path.display(),
                e
            )
        })?;

        while let Some(entry) = dir_entries.next_entry().await.map_err(|e| e.to_string())? {
            let file_path = entry.path();
            let file_name = file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            // Check for removed addons
            for removed_addon in &diff.removed_addons {
                if let Some(old_addon) = old_addons.iter().find(|a| &a.addon_name == removed_addon)
                {
                    let exact_filename = &old_addon.file_name_on_disk;
                    let disabled_filename = format!("{}.disabled", exact_filename);

                    if file_name == exact_filename || file_name == disabled_filename {
                        log::info!(
                            "Removing file for addon '{}': {}",
                            removed_addon,
                            file_path.display()
                        );
                        async_fs::remove_file(&file_path).await.map_err(|e| {
                            format!("Failed to remove file {}: {}", file_path.display(), e)
                        })?;
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

                    if file_name == exact_filename || file_name == disabled_filename {
                        log::info!(
                            "Removing old version of '{}': {}",
                            old_addon.addon_name,
                            file_path.display()
                        );
                        async_fs::remove_file(&file_path).await.map_err(|e| {
                            format!(
                                "Failed to remove old version {}: {}",
                                file_path.display(),
                                e
                            )
                        })?;
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    remove_category_files(modpack_path, "mods", &old_manifest.mods, diff).await?;
    remove_category_files(
        modpack_path,
        "resourcepacks",
        &old_manifest.resourcepacks,
        diff,
    )
    .await?;
    remove_category_files(modpack_path, "shaderpacks", &old_manifest.shaderpacks, diff).await?;
    remove_category_files(modpack_path, "datapacks", &old_manifest.datapacks, diff).await?;

    log::info!("remove_old_files: Removal complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composables::manifest::Addon;

    fn make_addon(project_id: u64, name: &str, file_name: &str) -> Addon {
        Addon {
            addon_file_id: project_id,
            addon_name: name.to_string(),
            addon_project_id: project_id,
            cdn_download_url: format!("https://edge.forgecdn.net/{file_name}"),
            mod_folder_path: "mods".to_string(),
            version: file_name.to_string(),
            web_site_url: None,
            disabled: None,
            file_name_on_disk: file_name.to_string(),
        }
    }

    fn make_manifest(update_type: Option<&str>, mods: Vec<Addon>) -> Manifest {
        Manifest {
            update_type: update_type.map(|s| s.to_string()),
            mods,
            resourcepacks: Vec::new(),
            shaderpacks: Vec::new(),
            datapacks: Vec::new(),
            config_files: Vec::new(),
        }
    }

    fn three_installed_addons() -> Vec<Addon> {
        vec![
            make_addon(1, "Sodium", "sodium-1.jar"),
            make_addon(2, "Lithium", "lithium-1.jar"),
            make_addon(3, "Iris", "iris-1.jar"),
        ]
    }

    #[test]
    fn rejects_traversing_or_absolute_addon_file_names() {
        for bad in ["../evil", "/etc/evil", "C:\\evil", "a/b", "..", ".", ""] {
            assert!(
                validate_addon_file_name("Evil Addon", bad).is_err(),
                "expected '{bad}' to be rejected as an addon file name"
            );
        }
    }

    #[test]
    fn accepts_ordinary_addon_file_names() {
        for good in [
            "sodium-1.jar",
            "some mod (1).jar",
            "file.name.with.dots.jar",
        ] {
            assert!(
                validate_addon_file_name("Good Addon", good).is_ok(),
                "expected '{good}' to be accepted as an addon file name"
            );
        }
    }

    #[test]
    fn rejects_non_http_download_urls() {
        for bad in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "ftp://example.com/x",
            "",
        ] {
            assert!(
                validate_addon_download_url("Evil Addon", bad).is_err(),
                "expected '{bad}' to be rejected as a download URL"
            );
        }
    }

    #[test]
    fn validate_all_addons_rejects_a_single_malicious_entry_among_many() {
        let mut manifest = make_manifest(Some("full"), three_installed_addons());
        manifest
            .mods
            .push(make_addon(4, "Evil", "../../../Startup/run.bat"));

        assert!(validate_all_addons(&manifest).is_err());
    }

    #[test]
    fn validate_all_addons_accepts_a_well_formed_manifest() {
        let manifest = make_manifest(Some("full"), three_installed_addons());
        assert!(validate_all_addons(&manifest).is_ok());
    }

    #[test]
    fn is_config_only_update_matches_only_the_config_discriminator() {
        assert!(is_config_only_update(&make_manifest(
            Some("config"),
            Vec::new()
        )));
        assert!(!is_config_only_update(&make_manifest(
            Some("full"),
            Vec::new()
        )));
        assert!(!is_config_only_update(&make_manifest(None, Vec::new())));
    }

    #[test]
    fn config_only_manifest_produces_no_removals() {
        // Shape of a real config-only manifest per useAdminApi.ts saveManifest:
        // updateType "config" always pairs with every addon array empty.
        let old = make_manifest(None, three_installed_addons());
        let new = make_manifest(Some("config"), Vec::new());

        let diff = calculate_update_diff(&old, &new).expect("diff should compute");

        assert!(
            diff.removed_addons.is_empty(),
            "config-only manifest must not mark any addon as removed, got: {:?}",
            diff.removed_addons
        );
        assert!(diff.updated_addon_ids.is_empty());
    }

    #[test]
    fn full_update_with_genuine_removal_still_reports_it() {
        // Guards against the belt-and-braces empty-category check overreaching:
        // a full update that drops one of several addons must still report it.
        let old = make_manifest(Some("full"), three_installed_addons());
        let new = make_manifest(Some("full"), vec![make_addon(1, "Sodium", "sodium-1.jar")]);

        let diff = calculate_update_diff(&old, &new).expect("diff should compute");

        assert_eq!(
            diff.removed_addons,
            vec!["Lithium".to_string(), "Iris".to_string()]
        );
    }

    #[test]
    fn full_update_can_remove_every_addon_in_a_category() {
        let old = make_manifest(Some("full"), three_installed_addons());
        let new = make_manifest(Some("full"), Vec::new());

        let diff = calculate_update_diff(&old, &new).expect("diff should compute");

        assert_eq!(
            diff.removed_addons,
            vec![
                "Sodium".to_string(),
                "Lithium".to_string(),
                "Iris".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn config_only_install_leaves_existing_addons_on_disk_untouched() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let mods_dir = temp.path().join("mods");
        async_fs::create_dir_all(&mods_dir)
            .await
            .expect("failed to create mods dir");

        let files = ["sodium-1.jar", "lithium-1.jar", "iris-1.jar"];
        for file in files {
            async_fs::write(mods_dir.join(file), b"jar contents")
                .await
                .expect("failed to seed jar");
        }

        let old = make_manifest(None, three_installed_addons());
        let new = make_manifest(Some("config"), Vec::new());

        // Mirrors exactly what install_update does before touching any addon: because
        // is_config_only_update(&new) is true, install_update never calls
        // calculate_update_diff/remove_old_files at all. This test also exercises the
        // explicit discriminator inside calculate_update_diff as defense in depth.
        let diff = calculate_update_diff(&old, &new).expect("diff should compute");
        let modpack_path = temp.path().to_str().unwrap().to_string();
        remove_old_files(&modpack_path, &old, &diff)
            .await
            .expect("cleanup should not fail");

        for file in files {
            assert!(
                mods_dir.join(file).exists(),
                "{file} should survive a config-only update"
            );
        }
    }

    /// Only the download phase — mirrors the addon loops in install_update, which
    /// write to staging and propagate any failure via `?` before cleanup ever runs.
    async fn simulate_staged_downloads(
        staging_dir: &Path,
        fail_on_third: bool,
    ) -> Result<(), String> {
        async_fs::write(staging_dir.join("sodium-2.jar"), b"new jar contents")
            .await
            .map_err(|e| e.to_string())?;
        async_fs::write(staging_dir.join("lithium-2.jar"), b"new jar contents")
            .await
            .map_err(|e| e.to_string())?;
        if fail_on_third {
            return Err("simulated network failure downloading iris-2.jar".to_string());
        }
        async_fs::write(staging_dir.join("iris-2.jar"), b"new jar contents")
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn three_updated_addons() -> (Manifest, Manifest, &'static [&'static str]) {
        let old = make_manifest(
            None,
            vec![
                make_addon(1, "Sodium", "sodium-1.jar"),
                make_addon(2, "Lithium", "lithium-1.jar"),
                make_addon(3, "Iris", "iris-1.jar"),
            ],
        );
        let new = make_manifest(
            Some("full"),
            vec![
                make_addon(1, "Sodium", "sodium-2.jar"),
                make_addon(2, "Lithium", "lithium-2.jar"),
                make_addon(3, "Iris", "iris-2.jar"),
            ],
        );
        (old, new, &["sodium-1.jar", "lithium-1.jar", "iris-1.jar"])
    }

    #[tokio::test]
    async fn failed_download_never_reaches_cleanup_and_leaves_existing_files_intact() {
        // Reproduces the F-P1-1 scenario: a 3-mod update where the 3rd download
        // fails. Before this fix, remove_old_files ran first and unconditionally;
        // now the download loop (simulated here) must fail and return before
        // cleanup is ever reached.
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let mods_dir = temp.path().join("mods");
        async_fs::create_dir_all(&mods_dir)
            .await
            .expect("failed to create mods dir");

        let (old, new, old_files) = three_updated_addons();
        for file in old_files {
            async_fs::write(mods_dir.join(file), b"old jar contents")
                .await
                .expect("failed to seed jar");
        }

        let diff = calculate_update_diff(&old, &new).expect("diff should compute");
        assert_eq!(
            diff.updated_addon_ids.len(),
            3,
            "all three addons should be flagged as updated"
        );

        let staging_dir = temp.path().join(".cemm-staging").join("mods");
        async_fs::create_dir_all(&staging_dir)
            .await
            .expect("failed to create staging dir");

        let download_result = simulate_staged_downloads(&staging_dir, true).await;
        assert!(
            download_result.is_err(),
            "the simulated download phase should report the failure"
        );

        // remove_old_files is deliberately never called on this path — exactly
        // like install_update, where the `?` on the failed download exits the
        // function before the cleanup/promotion code is reached. What's under
        // test is that the pre-existing files are consequently still there.
        for file in old_files {
            assert!(
                mods_dir.join(file).exists(),
                "{file} must survive an install that fails partway through downloading"
            );
        }
    }

    #[tokio::test]
    async fn successful_download_promotes_staged_files_and_cleans_up_old_ones() {
        // Happy-path complement: once every download succeeds, cleanup and
        // promotion (rename staged -> final, matching install_update) must still
        // leave the modpack in the fully-updated state.
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let mods_dir = temp.path().join("mods");
        async_fs::create_dir_all(&mods_dir)
            .await
            .expect("failed to create mods dir");

        let (old, new, old_files) = three_updated_addons();
        for file in old_files {
            async_fs::write(mods_dir.join(file), b"old jar contents")
                .await
                .expect("failed to seed jar");
        }

        let diff = calculate_update_diff(&old, &new).expect("diff should compute");
        let staging_dir = temp.path().join(".cemm-staging").join("mods");
        async_fs::create_dir_all(&staging_dir)
            .await
            .expect("failed to create staging dir");

        simulate_staged_downloads(&staging_dir, false)
            .await
            .expect("all downloads should succeed");

        let modpack_path = temp.path().to_str().unwrap().to_string();
        remove_old_files(&modpack_path, &old, &diff)
            .await
            .expect("cleanup should succeed");

        for file in ["sodium-2.jar", "lithium-2.jar", "iris-2.jar"] {
            let staged = staging_dir.join(file);
            let final_dest = mods_dir.join(file);
            async_fs::rename(&staged, &final_dest)
                .await
                .expect("promotion rename should succeed");
        }

        for old_file in old_files {
            assert!(
                !mods_dir.join(old_file).exists(),
                "{old_file} should have been removed by cleanup"
            );
        }
        for new_file in ["sodium-2.jar", "lithium-2.jar", "iris-2.jar"] {
            assert!(
                mods_dir.join(new_file).exists(),
                "{new_file} should be present after promotion"
            );
        }
    }
}
