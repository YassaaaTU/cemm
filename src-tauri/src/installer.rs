use crate::composables::manifest::{
    ConfigFileWithContent as ConfigFile, Manifest, BINARY_CONTENT_PREFIX,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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

const MAX_ADDON_FILE_BYTES: u64 = 500 * 1024 * 1024;

fn is_trusted_addon_url(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|host| host == "forgecdn.net" || host.ends_with(".forgecdn.net"))
}

/// Addon files are executable content once Minecraft loads them. Only accept
/// encrypted CurseForge CDN URLs, and apply the same rule to redirects below.
fn validate_addon_download_url(addon_name: &str, url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|error| {
        format!(
            "Refusing to install addon '{}': cdn_download_url '{}' is invalid: {}",
            addon_name, url, error
        )
    })?;

    if is_trusted_addon_url(&parsed) {
        Ok(())
    } else {
        Err(format!(
            "Refusing to install addon '{}': cdn_download_url '{}' must use HTTPS on forgecdn.net",
            addon_name, url
        ))
    }
}

async fn download_and_save(
    client: &Client,
    url: &str,
    dest_path: &Path,
    max_bytes: u64,
) -> Result<(), String> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Failed to download {}: {}", url, error))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download {}: HTTP {}",
            url,
            response.status()
        ));
    }

    if let Some(len) = response.content_length() {
        if len > max_bytes {
            return Err(format!(
                "Refusing to download {}: reported size {} bytes exceeds the {} byte limit",
                url, len, max_bytes
            ));
        }
    }

    if let Some(parent) = dest_path.parent() {
        async_fs::create_dir_all(parent).await.map_err(|error| {
            format!("Failed to create directory {}: {}", parent.display(), error)
        })?;
    }

    let mut file = async_fs::File::create(dest_path)
        .await
        .map_err(|error| format!("Failed to create file {}: {}", dest_path.display(), error))?;
    let mut downloaded = 0u64;

    loop {
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(error) => {
                drop(file);
                let _ = async_fs::remove_file(dest_path).await;
                return Err(format!("Failed to read bytes from {}: {}", url, error));
            }
        };

        downloaded = match downloaded.checked_add(chunk.len() as u64) {
            Some(total) => total,
            None => {
                drop(file);
                let _ = async_fs::remove_file(dest_path).await;
                return Err(format!("Refusing to download {}: byte count overflow", url));
            }
        };
        if downloaded > max_bytes {
            drop(file);
            let _ = async_fs::remove_file(dest_path).await;
            return Err(format!(
                "Refusing to save {}: downloaded {} bytes exceeds the {} byte limit",
                url, downloaded, max_bytes
            ));
        }

        if let Err(error) = file.write_all(&chunk).await {
            drop(file);
            let _ = async_fs::remove_file(dest_path).await;
            return Err(format!(
                "Failed to write file {}: {}",
                dest_path.display(),
                error
            ));
        }
    }

    file.flush()
        .await
        .map_err(|error| format!("Failed to flush file {}: {}", dest_path.display(), error))?;

    Ok(())
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

/// Options for install_update function. Only ever deserialized (received from
/// the frontend as a command argument) — never serialized back out.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstallOptions {
    /// Old manifest for cleanup of removed/updated addons
    pub old_manifest: Option<Manifest>,
    /// Whether to perform cleanup of old files (default: true when old_manifest provided)
    #[serde(default)]
    pub cleanup_old: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgress {
    pub progress: f64,
    pub message: String,
}

pub type InstallProgressCallback = Arc<dyn Fn(InstallProgress) + Send + Sync>;

const INSTALL_TRANSACTION_DIR: &str = ".cemm-transaction";
pub(crate) const INSTALLED_MANIFEST_FILE: &str = "cemm-manifest.json";
const INSTALL_JOURNAL_FILE: &str = "journal.json";
const INSTALL_COMMITTED_FILE: &str = "committed";

fn targets_reserved_install_path(relative_path: &str) -> bool {
    Path::new(relative_path)
        .components()
        .find_map(|component| match component {
            std::path::Component::Normal(value) => value.to_str(),
            std::path::Component::CurDir => None,
            _ => None,
        })
        .is_some_and(|component| {
            component.eq_ignore_ascii_case(INSTALL_TRANSACTION_DIR)
                || component.eq_ignore_ascii_case(INSTALLED_MANIFEST_FILE)
        })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstallTransactionEntry {
    relative_path: String,
    had_original: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstallTransactionJournal {
    entries: Vec<InstallTransactionEntry>,
}

async fn remove_file_if_present(path: &Path) -> Result<(), String> {
    match async_fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Failed to remove {}: {error}", path.display())),
    }
}

async fn rollback_install_transaction(
    modpack_path: &Path,
    transaction_dir: &Path,
    journal: &InstallTransactionJournal,
) -> Result<(), String> {
    let backup_dir = transaction_dir.join("backup");

    for (index, entry) in journal.entries.iter().enumerate().rev() {
        let final_path = validate_path_within_base(modpack_path, &entry.relative_path)?;
        let backup_path = backup_dir.join(index.to_string());

        if backup_path.exists() {
            remove_file_if_present(&final_path).await?;
            if let Some(parent) = final_path.parent() {
                async_fs::create_dir_all(parent).await.map_err(|error| {
                    format!(
                        "Failed to recreate rollback directory {}: {error}",
                        parent.display()
                    )
                })?;
            }
            async_fs::rename(&backup_path, &final_path)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to restore {} from transaction backup: {error}",
                        final_path.display()
                    )
                })?;
        } else if !entry.had_original {
            remove_file_if_present(&final_path).await?;
        }
    }

    async_fs::remove_dir_all(transaction_dir)
        .await
        .map_err(|error| {
            format!(
                "Files were restored, but transaction cleanup failed at {}: {error}",
                transaction_dir.display()
            )
        })
}

async fn recover_interrupted_install(modpack_path: &Path) -> Result<(), String> {
    let transaction_dir = modpack_path.join(INSTALL_TRANSACTION_DIR);
    if !transaction_dir.exists() {
        return Ok(());
    }

    if transaction_dir.join(INSTALL_COMMITTED_FILE).exists() {
        return async_fs::remove_dir_all(&transaction_dir)
            .await
            .map_err(|error| {
                format!(
                    "Failed to clean committed install transaction {}: {error}",
                    transaction_dir.display()
                )
            });
    }

    let journal_path = transaction_dir.join(INSTALL_JOURNAL_FILE);
    if !journal_path.exists() {
        return async_fs::remove_dir_all(&transaction_dir)
            .await
            .map_err(|error| {
                format!(
                    "Failed to clean incomplete install staging {}: {error}",
                    transaction_dir.display()
                )
            });
    }

    let journal_bytes = async_fs::read(&journal_path).await.map_err(|error| {
        format!(
            "Failed to read interrupted install journal {}: {error}",
            journal_path.display()
        )
    })?;
    let journal: InstallTransactionJournal =
        serde_json::from_slice(&journal_bytes).map_err(|error| {
            format!(
                "Interrupted install journal is invalid at {}: {error}",
                journal_path.display()
            )
        })?;

    rollback_install_transaction(modpack_path, &transaction_dir, &journal).await
}

async fn finalize_install_transaction<F>(
    modpack_path: &Path,
    transaction_dir: &Path,
    cleanup_paths: Vec<PathBuf>,
    staged_moves: Vec<(PathBuf, PathBuf)>,
    before_promotion: F,
) -> Result<(), String>
where
    F: Fn(usize, &Path) -> Result<(), String>,
{
    let mut relative_paths = Vec::new();
    let mut seen = HashSet::new();

    for final_path in cleanup_paths
        .iter()
        .chain(staged_moves.iter().map(|(_, final_path)| final_path))
    {
        let relative = final_path.strip_prefix(modpack_path).map_err(|_| {
            format!(
                "Install destination escaped the modpack directory: {}",
                final_path.display()
            )
        })?;
        let relative = relative.to_str().ok_or_else(|| {
            format!(
                "Install destination is not valid Unicode: {}",
                final_path.display()
            )
        })?;
        let validated = validate_path_within_base(modpack_path, relative)?;
        let key = validated.to_string_lossy().to_string();
        if seen.insert(key) {
            relative_paths.push(relative.to_string());
        }
    }

    let entries = relative_paths
        .into_iter()
        .map(|relative_path| {
            let final_path = modpack_path.join(&relative_path);
            InstallTransactionEntry {
                relative_path,
                had_original: final_path.exists(),
            }
        })
        .collect::<Vec<_>>();
    let journal = InstallTransactionJournal { entries };
    let journal_path = transaction_dir.join(INSTALL_JOURNAL_FILE);
    let journal_temp_path = transaction_dir.join("journal.tmp");
    let journal_bytes = serde_json::to_vec_pretty(&journal)
        .map_err(|error| format!("Failed to encode install journal: {error}"))?;

    async_fs::create_dir_all(transaction_dir)
        .await
        .map_err(|error| {
            format!(
                "Failed to create install transaction directory {}: {error}",
                transaction_dir.display()
            )
        })?;
    async_fs::write(&journal_temp_path, journal_bytes)
        .await
        .map_err(|error| format!("Failed to write install journal: {error}"))?;
    async_fs::rename(&journal_temp_path, &journal_path)
        .await
        .map_err(|error| format!("Failed to publish install journal: {error}"))?;

    let mutation_result = async {
        let backup_dir = transaction_dir.join("backup");
        async_fs::create_dir_all(&backup_dir)
            .await
            .map_err(|error| format!("Failed to create install backup directory: {error}"))?;

        for (index, entry) in journal.entries.iter().enumerate() {
            if !entry.had_original {
                continue;
            }
            let final_path = validate_path_within_base(modpack_path, &entry.relative_path)?;
            let backup_path = backup_dir.join(index.to_string());
            async_fs::rename(&final_path, &backup_path)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to back up {} before install: {error}",
                        final_path.display()
                    )
                })?;
        }

        for (index, (staged_path, final_path)) in staged_moves.iter().enumerate() {
            before_promotion(index, final_path)?;
            if let Some(parent) = final_path.parent() {
                async_fs::create_dir_all(parent).await.map_err(|error| {
                    format!(
                        "Failed to create install directory {}: {error}",
                        parent.display()
                    )
                })?;
            }
            async_fs::rename(staged_path, final_path)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to promote {} into place: {error}",
                        final_path.display()
                    )
                })?;
        }

        async_fs::write(transaction_dir.join(INSTALL_COMMITTED_FILE), b"committed")
            .await
            .map_err(|error| format!("Failed to mark install transaction committed: {error}"))?;
        Ok::<(), String>(())
    }
    .await;

    if let Err(error) = mutation_result {
        return match rollback_install_transaction(modpack_path, transaction_dir, &journal).await {
            Ok(()) => Err(format!("{error}. The previous installation was restored.")),
            Err(rollback_error) => Err(format!(
                "{error}. Automatic rollback also failed: {rollback_error}"
            )),
        };
    }

    // The committed marker makes a leftover directory safe: recovery will only
    // clean it, never restore backups over the successful installation.
    let _ = async_fs::remove_dir_all(transaction_dir).await;
    Ok(())
}

// Helper to emit progress
fn emit_progress(callback: &InstallProgressCallback, progress: usize, total: usize, msg: &str) {
    let safe_progress = if total > 0 {
        progress.min(total)
    } else {
        progress
    };
    callback(InstallProgress {
        progress: if total > 0 {
            (safe_progress as f64) / (total as f64) * 100.0
        } else {
            100.0
        },
        message: msg.to_string(),
    });
}
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
/// The old manifest's entries for the same category, by position.
fn old_addons_for(old_manifest: &Manifest, index: usize) -> &[crate::composables::manifest::Addon] {
    match index {
        0 => &old_manifest.mods,
        1 => &old_manifest.resourcepacks,
        2 => &old_manifest.shaderpacks,
        _ => &old_manifest.datapacks,
    }
}
/// The single predicate deciding whether an addon's file must be fetched.
///
/// Both passes call this. They previously inlined the decision with
/// different shapes -- the counting pass nested two `if let`s and counted
/// nothing when a diff existed without an old manifest, while the download
/// pass matched both at once and fetched everything in that same state.
/// They agreed only because `diff` is built as `Some` exclusively when
/// `old_manifest` is `Some`, an invariant established over a hundred lines
/// earlier. Expressing it once removes the trap rather than documenting it.
fn needs_download(
    addon: &crate::composables::manifest::Addon,
    destination: &Path,
    diff: Option<&UpdateDiff>,
    old_addons: Option<&[crate::composables::manifest::Addon]>,
) -> bool {
    match (diff, old_addons) {
        (Some(diff), Some(old_addons)) => {
            should_download_addon(addon, old_addons, diff, destination)
        }
        // No baseline to compare against means a fresh install.
        _ => true,
    }
}

/// Unified install service operation that handles all installation scenarios.
pub async fn install_update_with_progress(
    modpack_path: String,
    manifest: Manifest,
    config_files: Vec<ConfigFile>,
    options: Option<InstallOptions>,
    progress_callback: InstallProgressCallback,
) -> Result<(), String> {
    let options = options.unwrap_or_default();
    // Addon files can legitimately be large (resource/shader packs run to hundreds
    // of MB), so the timeout is generous — its job is only to bound a stalled
    // connection to something finite, not to cap normal downloads.
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if is_trusted_addon_url(attempt.url()) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
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

    let modpack_path_buf = PathBuf::from(&modpack_path);
    recover_interrupted_install(&modpack_path_buf).await?;
    let transaction_dir = modpack_path_buf.join(INSTALL_TRANSACTION_DIR);
    let staging_dir = transaction_dir.join("staging");

    // Config destinations and binary payloads are also attacker-controlled.
    // Resolve and decode every one before addon cleanup or promotion begins so
    // a malformed final config cannot fail only after destructive work.
    let mut prepared_configs = Vec::with_capacity(config_files.len());
    for config in config_files {
        if targets_reserved_install_path(&config.relative_path) {
            return Err(format!(
                "Config file path uses a CEMM-reserved install path: {}",
                config.relative_path
            ));
        }
        let dest = validate_path_within_base(&modpack_path_buf, &config.relative_path)?;
        let bytes = if config.content.starts_with(BINARY_CONTENT_PREFIX) {
            let base64_content = config
                .content
                .strip_prefix(BINARY_CONTENT_PREFIX)
                .unwrap_or(&config.content);
            use base64::engine::general_purpose::STANDARD;
            use base64::Engine;
            STANDARD.decode(base64_content).map_err(|error| {
                format!(
                    "Failed to decode binary config file {}: {error}",
                    dest.display()
                )
            })?
        } else {
            config.content.into_bytes()
        };
        prepared_configs.push((dest, bytes));
    }

    // `cemm-manifest.json` records the pack's *state*, not the payload of the
    // update that produced it. For a full update the two are the same document.
    // For a config-only one they are deliberately not: its addon arrays are
    // empty by construction (see `buildUpdateManifest` in useAdminApi.ts), so
    // writing it verbatim erased the addon baseline. The next full update then
    // diffed against nothing and reported every addon already installed as new.
    let installed_manifest = if is_config_only {
        let baseline = options.old_manifest.as_ref();
        let carry = |select: fn(&Manifest) -> &Vec<crate::composables::manifest::Addon>| {
            baseline.map(select).cloned().unwrap_or_default()
        };
        let snapshot = Manifest {
            // A state snapshot is neither kind of update, and calling it "config"
            // would describe the file's provenance rather than its contents.
            update_type: None,
            mods: carry(|manifest| &manifest.mods),
            resourcepacks: carry(|manifest| &manifest.resourcepacks),
            shaderpacks: carry(|manifest| &manifest.shaderpacks),
            datapacks: carry(|manifest| &manifest.datapacks),
            custom_datapacks: baseline
                .map(|baseline| baseline.custom_datapacks.clone())
                .unwrap_or_default(),
            config_files: manifest.config_files.clone(),
        };
        serde_json::to_vec_pretty(&snapshot)
    } else {
        serde_json::to_vec_pretty(&manifest)
    }
    .map_err(|error| format!("Failed to encode installed manifest: {error}"))?;

    // Calculate diff once for both cleanup and selective downloads.
    // Skipped entirely for config-only updates so the cleanup step below can never run.
    let diff = if is_config_only {
        None
    } else if let Some(ref old_manifest) = options.old_manifest {
        Some(calculate_update_diff(old_manifest, &manifest)?)
    } else {
        None
    };

    // Every payload is staged in the transaction directory before live files
    // are touched. If the process stops, the journal is recovered next time.
    let mut staged_moves: Vec<(PathBuf, PathBuf)> = Vec::new();

    let mut current = 0usize;

    // The four categories, each with the directory it installs into and the
    // noun used in progress messages. Named once here rather than spelled out
    // in every loop: the counting pass and the download pass below used to
    // repeat the same block four times each, and the two passes had drifted
    // into encoding the same decision differently.
    let categories: [(&Vec<crate::composables::manifest::Addon>, &str, &str); 4] = [
        (&manifest.mods, "mods", "mod"),
        (&manifest.resourcepacks, "resourcepacks", "resourcepack"),
        (&manifest.shaderpacks, "shaderpacks", "shaderpack"),
        (&manifest.datapacks, "datapacks", "datapack"),
    ];

    // Everything the progress bar will step through: the addons that actually
    // need fetching, every config being staged, and the installed manifest.
    //
    // The denominator used to count addons alone on a full update, so the
    // config loop and the manifest stepped `current` past the total and
    // `emit_progress` clamped them all to 100%. The bar sat full through the
    // slowest, most alarming part of an install -- staging and finalisation --
    // and a full update carrying only configs jumped straight to 100% before
    // anything had been written.
    let files_to_download = if is_config_only {
        prepared_configs.len() + 1
    } else {
        let mut count = prepared_configs.len() + 1;
        for (index, (addons, directory, _)) in categories.iter().enumerate() {
            let old_addons = options
                .old_manifest
                .as_ref()
                .map(|old| old_addons_for(old, index));
            for addon in addons.iter() {
                if addon.disabled == Some(true) {
                    continue;
                }
                let destination = Path::new(&modpack_path)
                    .join(directory)
                    .join(&addon.file_name_on_disk);
                if needs_download(addon, &destination, diff.as_ref(), old_addons) {
                    count += 1;
                }
            }
        }
        count
    };

    // Install addons -- entirely skipped for config-only updates so no addon in
    // mods/resourcepacks/shaderpacks/datapacks is ever downloaded or removed.
    if !is_config_only {
        for (index, (addons, directory, noun)) in categories.iter().enumerate() {
            let old_addons = options
                .old_manifest
                .as_ref()
                .map(|old| old_addons_for(old, index));
            for addon in addons.iter() {
                if addon.disabled == Some(true) {
                    continue;
                }
                let final_dest = Path::new(&modpack_path)
                    .join(directory)
                    .join(&addon.file_name_on_disk);

                if !needs_download(addon, &final_dest, diff.as_ref(), old_addons) {
                    log::info!("Skipping unchanged {noun}: {}", addon.addon_name);
                    continue;
                }

                let staged_dest = staging_dir.join(directory).join(&addon.file_name_on_disk);
                download_and_save(
                    &client,
                    &addon.cdn_download_url,
                    &staged_dest,
                    MAX_ADDON_FILE_BYTES,
                )
                .await?;
                staged_moves.push((staged_dest, final_dest));
                current += 1;
                emit_progress(
                    &progress_callback,
                    current,
                    files_to_download,
                    &format!("Downloaded {noun}: {}", addon.addon_name),
                );
            }
        }
    }

    // Configs and the installed manifest participate in the same transaction as
    // add-ons. Nothing is written directly into the live instance yet.
    for (index, (dest, bytes)) in prepared_configs.into_iter().enumerate() {
        let staged_path = staging_dir.join("configs").join(index.to_string());
        if let Some(parent) = staged_path.parent() {
            async_fs::create_dir_all(parent).await.map_err(|error| {
                format!(
                    "Failed to create config staging directory {}: {error}",
                    parent.display()
                )
            })?;
        }
        async_fs::write(&staged_path, bytes)
            .await
            .map_err(|error| {
                format!(
                    "Failed to stage config file for {}: {error}",
                    dest.display()
                )
            })?;
        staged_moves.push((staged_path, dest.clone()));

        current += 1;
        emit_progress(
            &progress_callback,
            current,
            files_to_download,
            &format!("Prepared config: {}", dest.display()),
        );
    }

    let manifest_path = validate_path_within_base(&modpack_path_buf, INSTALLED_MANIFEST_FILE)?;
    let staged_manifest_path = staging_dir.join(INSTALLED_MANIFEST_FILE);
    if let Some(parent) = staged_manifest_path.parent() {
        async_fs::create_dir_all(parent)
            .await
            .map_err(|error| format!("Failed to create manifest staging directory: {error}"))?;
    }
    async_fs::write(&staged_manifest_path, installed_manifest)
        .await
        .map_err(|error| format!("Failed to stage installed manifest: {error}"))?;
    staged_moves.push((staged_manifest_path, manifest_path));
    // The manifest is the `+ 1` in the denominator above.
    current += 1;

    let cleanup_paths = if options.cleanup_old {
        if let (Some(old_manifest), Some(ref diff)) = (options.old_manifest.as_ref(), &diff) {
            collect_old_file_paths(&modpack_path_buf, old_manifest, diff)?
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    emit_progress(
        &progress_callback,
        current,
        files_to_download,
        "Finalizing installed files...",
    );
    finalize_install_transaction(
        &modpack_path_buf,
        &transaction_dir,
        cleanup_paths,
        staged_moves,
        |_, _| Ok(()),
    )
    .await?;

    emit_progress(
        &progress_callback,
        files_to_download,
        files_to_download,
        "Installation complete!",
    );
    Ok(())
}

/// Represents the difference between two manifest versions during an update.
///
/// Serialized straight to the frontend over `manifest.diff`, so the preview a
/// user approves and the deletions `collect_old_file_paths` performs are the
/// same values, not two computations that happen to agree.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "generated/")]
pub struct UpdateDiff {
    /// Display only, and deliberately so: names are not an identity. Two
    /// CurseForge projects can carry the same `addon_name` -- across categories
    /// (a mod and a resourcepack), or within one after a fork rename -- so
    /// `collect_old_file_paths` reads `removed_addon_ids` instead.
    pub removed_addons: Vec<String>,
    /// Project IDs to delete from the pack. Drives `collect_old_file_paths`.
    ///
    /// This used to be the name list above, matched per category against every
    /// old addon in that category. A kept addon whose name collided with a
    /// removed one in *another* category matched too: the transaction backed
    /// its file up, deleted it, and never restored it, while the installed
    /// manifest still listed it and the preview called it untouched. Project
    /// IDs are unique across all four categories, which is why
    /// `updated_addon_ids` was already keyed on them.
    #[ts(type = "number[]")]
    pub removed_addon_ids: Vec<u64>,
    /// Project IDs whose file must be replaced. Also drives
    /// `collect_old_file_paths`, which removes both `X.jar` and
    /// `X.jar.disabled` — so an addon disabled in the old manifest still
    /// belongs here when its version changed, and is deliberately not filtered.
    #[ts(type = "number[]")]
    pub updated_addon_ids: Vec<u64>,
    /// Display only; nothing in the installer reads it. Disabled addons are
    /// excluded because an addon that arrives disabled is not being added to
    /// the pack the user runs.
    pub new_addons: Vec<String>,
}

/// The one place any caller asks "what changes between these manifests".
///
/// The preview the user approves, the admin-side comparison and the installer's
/// own cleanup all resolve here. They used to be three implementations across
/// two languages, and they had drifted: the TypeScript preview skipped addons
/// disabled in the old manifest when detecting version changes, while the
/// installer did not. An addon disabled in the old manifest and updated in the
/// new one was therefore absent from the preview but present in the deletion
/// list -- the dialog said nothing and the files went away.
///
/// `old` is `None` for a first install, where every enabled addon is new.
pub fn update_diff(old: Option<&Manifest>, new: &Manifest) -> Result<UpdateDiff, String> {
    // Checked before the `old` case so a config-only first install is still
    // reported as touching no addons.
    if is_config_only_update(new) {
        return Ok(UpdateDiff {
            removed_addons: Vec::new(),
            removed_addon_ids: Vec::new(),
            updated_addon_ids: Vec::new(),
            new_addons: Vec::new(),
        });
    }

    match old {
        Some(old) => calculate_update_diff(old, new),
        None => Ok(UpdateDiff {
            removed_addons: Vec::new(),
            removed_addon_ids: Vec::new(),
            updated_addon_ids: Vec::new(),
            new_addons: [
                &new.mods,
                &new.resourcepacks,
                &new.shaderpacks,
                &new.datapacks,
            ]
            .into_iter()
            .flatten()
            .filter(|addon| addon.disabled != Some(true))
            .map(|addon| addon.addon_name.clone())
            .collect(),
        }),
    }
}

pub fn calculate_update_diff(
    old_manifest: &Manifest,
    new_manifest: &Manifest,
) -> Result<UpdateDiff, String> {
    let mut diff = UpdateDiff {
        removed_addons: Vec::new(),
        removed_addon_ids: Vec::new(),
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
                diff.removed_addon_ids.push(old_addon.addon_project_id);
            } else if let Some(new_addon) = maybe_new {
                if new_addon.disabled == Some(true) {
                    diff.removed_addons.push(old_addon.addon_name.clone());
                    diff.removed_addon_ids.push(old_addon.addon_project_id);
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

        // Find new addons. Display only, so a disabled arrival is not listed.
        for new_addon in new_addons {
            if new_addon.disabled == Some(true) {
                continue;
            }
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

fn collect_old_file_paths(
    modpack_path: &Path,
    old_manifest: &Manifest,
    diff: &UpdateDiff,
) -> Result<Vec<PathBuf>, String> {
    fn collect_category_files(
        modpack_path: &Path,
        category_dir: &str,
        old_addons: &[crate::composables::manifest::Addon],
        diff: &UpdateDiff,
    ) -> Result<Vec<PathBuf>, String> {
        let mut paths = Vec::new();
        for old_addon in old_addons {
            let is_removed = diff.removed_addon_ids.contains(&old_addon.addon_project_id);
            let is_updated = diff.updated_addon_ids.contains(&old_addon.addon_project_id);
            if !is_removed && !is_updated {
                continue;
            }

            validate_addon_file_name(&old_addon.addon_name, &old_addon.file_name_on_disk)?;
            for filename in [
                old_addon.file_name_on_disk.clone(),
                format!("{}.disabled", old_addon.file_name_on_disk),
            ] {
                let relative_path = Path::new(category_dir).join(filename);
                let relative_path = relative_path.to_str().ok_or_else(|| {
                    format!(
                        "Old addon path is not valid Unicode for '{}'",
                        old_addon.addon_name
                    )
                })?;
                let path = validate_path_within_base(modpack_path, relative_path)?;
                if path.exists() {
                    paths.push(path);
                }
            }
        }
        Ok(paths)
    }

    let mut paths = Vec::new();
    paths.extend(collect_category_files(
        modpack_path,
        "mods",
        &old_manifest.mods,
        diff,
    )?);
    paths.extend(collect_category_files(
        modpack_path,
        "resourcepacks",
        &old_manifest.resourcepacks,
        diff,
    )?);
    paths.extend(collect_category_files(
        modpack_path,
        "shaderpacks",
        &old_manifest.shaderpacks,
        diff,
    )?);
    paths.extend(collect_category_files(
        modpack_path,
        "datapacks",
        &old_manifest.datapacks,
        diff,
    )?);
    Ok(paths)
}

#[cfg(test)]
async fn remove_old_files(
    modpack_path: &str,
    old_manifest: &Manifest,
    diff: &UpdateDiff,
) -> Result<(), String> {
    for path in collect_old_file_paths(Path::new(modpack_path), old_manifest, diff)? {
        async_fs::remove_file(&path)
            .await
            .map_err(|error| format!("Failed to remove {}: {error}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composables::manifest::Addon;
    use tokio::io::AsyncReadExt;

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
            custom_datapacks: Vec::new(),
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

    /// Both the counting pass and the download pass call this, so progress
    /// denominators and actual downloads cannot disagree. They used to inline
    /// the decision separately, and in the state below -- a diff with no old
    /// manifest -- the counter contributed nothing while the downloader fetched
    /// everything.
    #[test]
    fn a_missing_baseline_means_download_regardless_of_the_diff() {
        let addon = make_addon(1, "Sodium", "sodium-1.jar");
        let diff = UpdateDiff {
            removed_addons: Vec::new(),
            removed_addon_ids: Vec::new(),
            updated_addon_ids: Vec::new(),
            new_addons: Vec::new(),
        };

        assert!(needs_download(
            &addon,
            Path::new("nowhere/sodium-1.jar"),
            Some(&diff),
            None
        ));
        assert!(needs_download(
            &addon,
            Path::new("nowhere/sodium-1.jar"),
            None,
            None
        ));
    }

    #[test]
    fn an_unchanged_addon_already_on_disk_is_not_downloaded_again() {
        let temp = tempfile::tempdir().expect("temp dir");
        let present = temp.path().join("sodium-1.jar");
        std::fs::write(&present, b"jar").expect("write");

        let addon = make_addon(1, "Sodium", "sodium-1.jar");
        let old = vec![make_addon(1, "Sodium", "sodium-1.jar")];
        let diff = UpdateDiff {
            removed_addons: Vec::new(),
            removed_addon_ids: Vec::new(),
            updated_addon_ids: Vec::new(),
            new_addons: Vec::new(),
        };

        assert!(!needs_download(&addon, &present, Some(&diff), Some(&old)));

        // ... but a missing file is always refetched, even when unchanged.
        let absent = temp.path().join("gone.jar");
        assert!(needs_download(&addon, &absent, Some(&diff), Some(&old)));
    }

    /// The divergence that motivated collapsing three diff implementations into
    /// one. The TypeScript preview skipped addons disabled in the old manifest
    /// when detecting version changes; this function never did. So an addon
    /// disabled in the old manifest and updated in the new one was absent from
    /// the dialog the user approved, yet present in `updated_addon_ids`, which
    /// `collect_old_file_paths` reads to build the deletion list.
    #[test]
    fn an_addon_disabled_in_the_old_manifest_still_counts_as_updated() {
        let mut was_disabled = make_addon(1, "Sodium", "sodium-1.jar");
        was_disabled.disabled = Some(true);

        let diff = update_diff(
            Some(&make_manifest(None, vec![was_disabled])),
            &make_manifest(None, vec![make_addon(1, "Sodium", "sodium-2.jar")]),
        )
        .expect("diff should succeed");

        assert_eq!(diff.updated_addon_ids, vec![1]);
        assert!(diff.removed_addons.is_empty());
    }

    /// `collect_old_file_paths` removes both `X.jar` and `X.jar.disabled` for
    /// anything in `updated_addon_ids`, which is what makes the case above a
    /// deletion the preview has to disclose.
    #[test]
    fn an_updated_addon_sweeps_both_its_enabled_and_disabled_files() {
        let temp = tempfile::tempdir().expect("temp dir");
        let mods = temp.path().join("mods");
        std::fs::create_dir_all(&mods).expect("mods dir");
        std::fs::write(mods.join("sodium-1.jar.disabled"), b"old").expect("write");

        let old = make_manifest(None, vec![make_addon(1, "Sodium", "sodium-1.jar")]);
        let diff = UpdateDiff {
            removed_addons: Vec::new(),
            removed_addon_ids: Vec::new(),
            updated_addon_ids: vec![1],
            new_addons: Vec::new(),
        };

        let paths = collect_old_file_paths(temp.path(), &old, &diff).expect("collect");

        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("sodium-1.jar.disabled"));
    }

    /// Names are not an identity, and CurseForge does not treat them as one:
    /// a mod and a resourcepack can both be called "Sodium". Removal used to be
    /// matched by name against every category, so dropping the mod also swept
    /// the resourcepack -- backed up, deleted, never restored, and still listed
    /// in the manifest left on disk.
    #[test]
    fn a_removal_never_sweeps_a_same_named_addon_from_another_category() {
        let temp = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(temp.path().join("mods")).expect("mods dir");
        std::fs::create_dir_all(temp.path().join("resourcepacks")).expect("resourcepacks dir");
        std::fs::write(temp.path().join("mods/sodium-1.jar"), b"mod").expect("write mod");
        std::fs::write(
            temp.path().join("resourcepacks/sodium-pack.zip"),
            b"resourcepack",
        )
        .expect("write resourcepack");

        let kept = {
            let mut addon = make_addon(99, "Sodium", "sodium-pack.zip");
            addon.mod_folder_path = "resourcepacks".to_string();
            addon
        };

        let mut old = make_manifest(None, vec![make_addon(1, "Sodium", "sodium-1.jar")]);
        old.resourcepacks = vec![kept.clone()];
        let mut new = make_manifest(None, Vec::new());
        new.resourcepacks = vec![kept];

        let diff = calculate_update_diff(&old, &new).expect("diff");
        assert_eq!(diff.removed_addon_ids, vec![1]);

        let paths = collect_old_file_paths(temp.path(), &old, &diff).expect("collect");

        assert_eq!(
            paths.len(),
            1,
            "only the removed mod may be swept, not the resourcepack sharing its name: {paths:?}"
        );
        assert!(paths[0].ends_with("sodium-1.jar"));
        assert!(
            temp.path().join("resourcepacks/sodium-pack.zip").exists(),
            "the kept resourcepack must still be on disk"
        );
    }

    #[test]
    fn a_disabled_arrival_is_not_announced_as_new() {
        let mut arrives_disabled = make_addon(2, "Lithium", "lithium-1.jar");
        arrives_disabled.disabled = Some(true);

        let diff = update_diff(
            Some(&make_manifest(
                None,
                vec![make_addon(1, "Sodium", "sodium-1.jar")],
            )),
            &make_manifest(
                None,
                vec![make_addon(1, "Sodium", "sodium-1.jar"), arrives_disabled],
            ),
        )
        .expect("diff should succeed");

        assert!(diff.new_addons.is_empty());
    }

    #[test]
    fn a_first_install_reports_every_enabled_addon_as_new() {
        let mut hidden = make_addon(9, "Hidden", "hidden-1.jar");
        hidden.disabled = Some(true);
        let mut incoming = three_installed_addons();
        incoming.push(hidden);

        let diff = update_diff(None, &make_manifest(None, incoming)).expect("diff should succeed");

        assert_eq!(diff.new_addons, vec!["Sodium", "Lithium", "Iris"]);
        assert!(diff.removed_addons.is_empty());
        assert!(diff.updated_addon_ids.is_empty());
    }

    #[test]
    fn a_config_only_update_reports_no_addon_changes_even_on_first_install() {
        let diff = update_diff(
            None,
            &make_manifest(Some("config"), three_installed_addons()),
        )
        .expect("diff should succeed");

        assert!(diff.new_addons.is_empty());
        assert!(diff.removed_addons.is_empty());
        assert!(diff.updated_addon_ids.is_empty());
    }

    #[test]
    fn a_renamed_addon_is_an_update_not_a_remove_and_add() {
        let diff = update_diff(
            Some(&make_manifest(
                None,
                vec![make_addon(1, "JEI", "jei-1.jar")],
            )),
            &make_manifest(None, vec![make_addon(1, "Just Enough Items", "jei-2.jar")]),
        )
        .expect("diff should succeed");

        assert_eq!(diff.updated_addon_ids, vec![1]);
        assert!(diff.removed_addons.is_empty());
        assert!(diff.new_addons.is_empty());
    }

    /// A rename that changes nothing else is invisible, and has to be: the file
    /// on disk is the same file, so there is nothing to download and nothing to
    /// delete. Asserted here because this used to be `compare_manifests`'
    /// coverage, and that second implementation is gone.
    #[test]
    fn a_rename_with_no_version_change_is_not_a_change_at_all() {
        let diff = update_diff(
            Some(&make_manifest(
                None,
                vec![make_addon(1, "JEI", "jei-1.jar")],
            )),
            &make_manifest(None, vec![make_addon(1, "Just Enough Items", "jei-1.jar")]),
        )
        .expect("diff should succeed");

        assert!(diff.new_addons.is_empty());
        assert!(diff.removed_addons.is_empty());
        assert!(diff.removed_addon_ids.is_empty());
        assert!(diff.updated_addon_ids.is_empty());
    }

    /// The plain cases, kept alongside the renaming ones so the whole identity
    /// rule reads in one place.
    #[test]
    fn a_new_project_is_added_and_a_dropped_one_is_removed() {
        let old = make_manifest(
            None,
            vec![
                make_addon(1, "Sodium", "sodium-1.jar"),
                make_addon(2, "Lithium", "lithium-1.jar"),
            ],
        );
        let new = make_manifest(
            None,
            vec![
                make_addon(1, "Sodium", "sodium-1.jar"),
                make_addon(3, "Iris", "iris-1.jar"),
            ],
        );

        let diff = update_diff(Some(&old), &new).expect("diff should succeed");

        assert_eq!(diff.new_addons, vec!["Iris".to_string()]);
        assert_eq!(diff.removed_addons, vec!["Lithium".to_string()]);
        assert_eq!(diff.removed_addon_ids, vec![2]);
        assert!(diff.updated_addon_ids.is_empty());
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
    fn rejects_untrusted_or_unencrypted_download_urls() {
        for bad in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "ftp://example.com/x",
            "http://edge.forgecdn.net/files/mod.jar",
            "https://example.com/mod.jar",
            "https://evilforgecdn.net/mod.jar",
            "",
        ] {
            assert!(
                validate_addon_download_url("Evil Addon", bad).is_err(),
                "expected '{bad}' to be rejected as a download URL"
            );
        }
    }

    #[test]
    fn accepts_curseforge_cdn_hosts_over_https() {
        for good in [
            "https://edge.forgecdn.net/files/mod.jar",
            "https://mediafilez.forgecdn.net/files/mod.jar",
            "https://forgecdn.net/files/mod.jar",
        ] {
            assert!(
                validate_addon_download_url("Good Addon", good).is_ok(),
                "expected '{good}' to be accepted as a download URL"
            );
        }
    }

    #[tokio::test]
    async fn streaming_download_removes_partial_file_when_limit_is_exceeded() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have an address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("request should connect");
            let mut request = [0u8; 1024];
            let _ = stream
                .read(&mut request)
                .await
                .expect("request should be readable");
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n6\r\nabcdef\r\n0\r\n\r\n",
                )
                .await
                .expect("response should be writable");
        });

        let temp = tempfile::tempdir().expect("temp dir should be available");
        let destination = temp.path().join("oversized.jar");
        let client = Client::builder().build().expect("client should build");
        let result = download_and_save(
            &client,
            &format!("http://{address}/oversized.jar"),
            &destination,
            5,
        )
        .await;

        server.await.expect("test server should finish");
        assert!(result.is_err(), "oversized response should be rejected");
        assert!(
            !destination.exists(),
            "partial oversized response should be removed"
        );
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

    #[tokio::test]
    async fn service_install_writes_config_and_records_installed_manifest() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let manifest = make_manifest(Some("config"), Vec::new());
        let progress_events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured_events = Arc::clone(&progress_events);
        let progress: InstallProgressCallback = Arc::new(move |event| {
            captured_events
                .lock()
                .expect("progress lock should not be poisoned")
                .push(event);
        });

        install_update_with_progress(
            temp.path().to_string_lossy().into_owned(),
            manifest.clone(),
            vec![ConfigFile {
                filename: "settings.toml".to_string(),
                relative_path: "config/settings.toml".to_string(),
                content: "enabled = true".to_string(),
                is_binary: None,
            }],
            None,
            progress,
        )
        .await
        .expect("config-only installation should succeed");

        assert_eq!(
            async_fs::read_to_string(temp.path().join("config/settings.toml"))
                .await
                .expect("installed config should be readable"),
            "enabled = true"
        );
        let installed_manifest: Manifest = serde_json::from_slice(
            &async_fs::read(temp.path().join("cemm-manifest.json"))
                .await
                .expect("installed manifest should be readable"),
        )
        .expect("installed manifest should be valid JSON");
        // The recorded baseline is a state snapshot, so it drops the incoming
        // manifest's "config" discriminator; there is no prior install here for
        // it to carry addons over from.
        assert_eq!(installed_manifest, make_manifest(None, Vec::new()));

        let events = progress_events
            .lock()
            .expect("progress lock should not be poisoned");
        assert_eq!(events.last().map(|event| event.progress), Some(100.0));
        assert_eq!(
            events.last().map(|event| event.message.as_str()),
            Some("Installation complete!")
        );
    }

    /// A config-only update leaves every addon alone, so the baseline it records
    /// has to keep describing them. Writing the update's own (empty by
    /// construction) addon arrays instead made the next full update diff against
    /// nothing: every already-installed addon came back as "new".
    #[tokio::test]
    async fn config_only_install_keeps_the_addon_baseline() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let baseline = make_manifest(Some("full"), three_installed_addons());

        let progress: InstallProgressCallback = Arc::new(|_| {});
        install_update_with_progress(
            temp.path().to_string_lossy().into_owned(),
            make_manifest(Some("config"), Vec::new()),
            vec![ConfigFile {
                filename: "settings.toml".to_string(),
                relative_path: "config/settings.toml".to_string(),
                content: "enabled = true".to_string(),
                is_binary: None,
            }],
            Some(InstallOptions {
                old_manifest: Some(baseline.clone()),
                cleanup_old: true,
            }),
            progress,
        )
        .await
        .expect("config-only installation should succeed");

        let installed_manifest: Manifest = serde_json::from_slice(
            &async_fs::read(temp.path().join("cemm-manifest.json"))
                .await
                .expect("installed manifest should be readable"),
        )
        .expect("installed manifest should be valid JSON");

        assert_eq!(installed_manifest.mods, baseline.mods);
        assert_eq!(installed_manifest.resourcepacks, baseline.resourcepacks);
        assert_eq!(installed_manifest.shaderpacks, baseline.shaderpacks);
        assert_eq!(installed_manifest.datapacks, baseline.datapacks);
        assert_eq!(installed_manifest.update_type, None);

        // And the addons it names still diff as unchanged against the same set.
        let diff =
            calculate_update_diff(&installed_manifest, &baseline).expect("diff should compute");
        assert!(diff.new_addons.is_empty(), "got: {:?}", diff.new_addons);
        assert!(diff.removed_addons.is_empty());
        assert!(diff.updated_addon_ids.is_empty());
    }

    #[tokio::test]
    async fn malformed_config_is_rejected_before_existing_addon_cleanup() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let mods_dir = temp.path().join("mods");
        async_fs::create_dir_all(&mods_dir)
            .await
            .expect("failed to create mods dir");
        let existing_addon = mods_dir.join("old.jar");
        async_fs::write(&existing_addon, b"existing jar")
            .await
            .expect("failed to seed existing addon");

        let old_manifest = make_manifest(Some("full"), vec![make_addon(1, "Old", "old.jar")]);
        let new_manifest = make_manifest(Some("full"), Vec::new());
        let result = install_update_with_progress(
            temp.path().to_string_lossy().into_owned(),
            new_manifest,
            vec![ConfigFile {
                filename: "broken.bin".to_string(),
                relative_path: "config/broken.bin".to_string(),
                content: format!("{BINARY_CONTENT_PREFIX}%%%invalid%%%"),
                is_binary: None,
            }],
            Some(InstallOptions {
                old_manifest: Some(old_manifest),
                cleanup_old: true,
            }),
            Arc::new(|_| {}),
        )
        .await;

        let error = result.expect_err("invalid base64 should reject the install");
        assert!(
            error.contains("Failed to decode binary config file"),
            "unexpected error: {error}"
        );
        assert!(
            existing_addon.exists(),
            "existing addon must remain when prevalidation fails"
        );
        assert!(
            !temp.path().join("cemm-manifest.json").exists(),
            "failed installation must not record a new manifest"
        );
    }

    #[tokio::test]
    async fn finalization_failure_restores_every_original_file() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let transaction_dir = temp.path().join(INSTALL_TRANSACTION_DIR);
        let staging_dir = transaction_dir.join("staging");
        async_fs::create_dir_all(staging_dir.join("mods"))
            .await
            .expect("failed to create addon staging");
        async_fs::create_dir_all(staging_dir.join("configs"))
            .await
            .expect("failed to create config staging");
        async_fs::create_dir_all(temp.path().join("mods"))
            .await
            .expect("failed to create mods dir");
        async_fs::create_dir_all(temp.path().join("config"))
            .await
            .expect("failed to create config dir");

        let old_addon = temp.path().join("mods/old.jar");
        let new_addon = temp.path().join("mods/new.jar");
        let config = temp.path().join("config/settings.toml");
        let manifest = temp.path().join("cemm-manifest.json");
        async_fs::write(&old_addon, b"old addon")
            .await
            .expect("failed to seed old addon");
        async_fs::write(&config, b"old config")
            .await
            .expect("failed to seed old config");
        async_fs::write(&manifest, b"old manifest")
            .await
            .expect("failed to seed old manifest");

        let staged_addon = staging_dir.join("mods/new.jar");
        let staged_config = staging_dir.join("configs/0");
        let staged_manifest = staging_dir.join("cemm-manifest.json");
        async_fs::write(&staged_addon, b"new addon")
            .await
            .expect("failed to stage addon");
        async_fs::write(&staged_config, b"new config")
            .await
            .expect("failed to stage config");
        async_fs::write(&staged_manifest, b"new manifest")
            .await
            .expect("failed to stage manifest");

        let result = finalize_install_transaction(
            temp.path(),
            &transaction_dir,
            vec![old_addon.clone()],
            vec![
                (staged_addon, new_addon.clone()),
                (staged_config, config.clone()),
                (staged_manifest, manifest.clone()),
            ],
            |index, _| {
                if index == 2 {
                    Err("simulated manifest promotion failure".to_string())
                } else {
                    Ok(())
                }
            },
        )
        .await;

        let error = result.expect_err("injected finalization failure should propagate");
        assert!(error.contains("previous installation was restored"));
        assert_eq!(async_fs::read(&old_addon).await.unwrap(), b"old addon");
        assert!(!new_addon.exists(), "new addon should be rolled back");
        assert_eq!(async_fs::read(&config).await.unwrap(), b"old config");
        assert_eq!(async_fs::read(&manifest).await.unwrap(), b"old manifest");
        assert!(!transaction_dir.exists());
    }

    #[tokio::test]
    async fn interrupted_transaction_is_recovered_on_the_next_install() {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let transaction_dir = temp.path().join(INSTALL_TRANSACTION_DIR);
        let backup_dir = transaction_dir.join("backup");
        async_fs::create_dir_all(&backup_dir)
            .await
            .expect("failed to create backup dir");
        async_fs::create_dir_all(temp.path().join("mods"))
            .await
            .expect("failed to create mods dir");
        async_fs::create_dir_all(temp.path().join("config"))
            .await
            .expect("failed to create config dir");

        async_fs::write(temp.path().join("config/settings.toml"), b"new config")
            .await
            .expect("failed to seed promoted config");
        async_fs::write(temp.path().join("mods/new.jar"), b"new addon")
            .await
            .expect("failed to seed promoted addon");
        async_fs::write(backup_dir.join("0"), b"old config")
            .await
            .expect("failed to seed config backup");
        async_fs::write(backup_dir.join("2"), b"old addon")
            .await
            .expect("failed to seed addon backup");

        let journal = InstallTransactionJournal {
            entries: vec![
                InstallTransactionEntry {
                    relative_path: "config/settings.toml".to_string(),
                    had_original: true,
                },
                InstallTransactionEntry {
                    relative_path: "mods/new.jar".to_string(),
                    had_original: false,
                },
                InstallTransactionEntry {
                    relative_path: "mods/old.jar".to_string(),
                    had_original: true,
                },
            ],
        };
        async_fs::write(
            transaction_dir.join(INSTALL_JOURNAL_FILE),
            serde_json::to_vec_pretty(&journal).unwrap(),
        )
        .await
        .expect("failed to seed journal");

        recover_interrupted_install(temp.path())
            .await
            .expect("recovery should succeed");

        assert_eq!(
            async_fs::read(temp.path().join("config/settings.toml"))
                .await
                .unwrap(),
            b"old config"
        );
        assert!(!temp.path().join("mods/new.jar").exists());
        assert_eq!(
            async_fs::read(temp.path().join("mods/old.jar"))
                .await
                .unwrap(),
            b"old addon"
        );
        assert!(!transaction_dir.exists());
    }

    #[tokio::test]
    async fn config_cannot_target_reserved_transaction_or_manifest_paths() {
        for relative_path in [
            ".cemm-transaction/journal.json",
            "./.cemm-transaction/journal.json",
            "././.CEMM-TRANSACTION/backup/0",
            ".CEMM-TRANSACTION/backup/0",
            "cemm-manifest.json",
            "./cemm-manifest.json",
        ] {
            let temp = tempfile::tempdir().expect("failed to create temp dir");
            let result = install_update_with_progress(
                temp.path().to_string_lossy().into_owned(),
                make_manifest(Some("config"), Vec::new()),
                vec![ConfigFile {
                    filename: "reserved".to_string(),
                    relative_path: relative_path.to_string(),
                    content: "blocked".to_string(),
                    is_binary: None,
                }],
                None,
                Arc::new(|_| {}),
            )
            .await;
            assert!(result
                .expect_err("reserved path should be rejected")
                .contains("CEMM-reserved install path"));
        }
    }
}
