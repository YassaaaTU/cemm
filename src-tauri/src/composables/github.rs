use anyhow::Result;
use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Emitter};

use crate::composables::manifest::Manifest;

/// Configuration file with content for GitHub upload/download operations.
///
/// This struct is mirrored in multiple locations across the codebase:
/// - Rust: src-tauri/src/composables/github.rs (this file)
/// - Rust: src-tauri/src/installer.rs (ConfigFile struct)
/// - TypeScript: app/types/index.ts (ConfigFile and ConfigFileWithContent interfaces)
///
/// When modifying this struct, ensure all definitions remain consistent.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigFileWithContent {
    pub filename: String,
    pub relative_path: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_binary: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadResult {
    pub manifest: Manifest,
    pub config_files: Vec<ConfigFileWithContent>,
}

/// Progress event payload for upload operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadProgress {
    pub progress: u8,
    pub message: String,
}

/// Helper to emit progress events
fn emit_progress(app: &AppHandle, progress: u8, message: &str) {
    let _ = app.emit("upload_progress", UploadProgress {
        progress,
        message: message.to_string(),
    });
}

fn sanitize_modpack_key(name: &str) -> String {
    let lowered = name.trim().to_lowercase();
    let mut out = String::new();
    let mut last_dash = false;

    for c in lowered.chars() {
        let mapped = if c.is_ascii_alphanumeric() {
            Some(c)
        } else if c == ' ' || c == '_' || c == '-' {
            Some('-')
        } else {
            None
        };

        if let Some(ch) = mapped {
            if ch == '-' {
                if !last_dash {
                    out.push(ch);
                    last_dash = true;
                }
            } else {
                out.push(ch);
                last_dash = false;
            }
        }
    }

    out.trim_matches('-').to_string()
}

fn primary_update_base_path(modpack_key: Option<&str>, uuid: &str) -> String {
    if let Some(key) = modpack_key {
        let sanitized = sanitize_modpack_key(key);
        if !sanitized.is_empty() {
            return format!("{}/{}", sanitized, uuid);
        }
    }
    uuid.to_string()
}

/// Rejects path traversal and absolute-style paths for GitHub `contents/{path}`.
fn is_safe_repo_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains("..")
        && !path.starts_with('/')
        && !path.starts_with('\\')
        && !(path.len() >= 2 && path.as_bytes().get(1) == Some(&b':'))
}

fn push_unique_candidate(candidates: &mut Vec<String>, s: String) {
    if !s.is_empty() && !candidates.iter().any(|e| e == &s) {
        candidates.push(s);
    }
}

/// Paths to try for `GET .../contents/{path}` (folder listing for an update).
///
/// When the user pastes `modpack-folder/update-id`, we try that path first so a
/// mismatched modpack key from the local instance still resolves. When they paste
/// only the update id, we try `modpack_key/id` then `id` (legacy layout).
fn update_base_path_candidates(modpack_key: Option<&str>, uuid: &str) -> Vec<String> {
    let uuid = uuid.trim();
    let mut candidates: Vec<String> = Vec::new();

    let compound_safe = uuid.contains('/') && is_safe_repo_relative_path(uuid);
    if compound_safe {
        push_unique_candidate(&mut candidates, uuid.to_string());
    }

    let primary = primary_update_base_path(modpack_key, uuid);
    push_unique_candidate(&mut candidates, primary);

    if compound_safe {
        if let Some(base) = uuid.rsplit('/').next().filter(|s| !s.is_empty()) {
            let p = primary_update_base_path(modpack_key, base);
            push_unique_candidate(&mut candidates, p);
            push_unique_candidate(&mut candidates, base.to_string());
        }
    } else if !uuid.contains('/') {
        push_unique_candidate(&mut candidates, uuid.to_string());
    }

    candidates
}

fn normalize_update_uuid_arg(uuid: String) -> Result<String, String> {
    let normalized = uuid.trim().replace('\\', "/");
    if normalized.contains("..") {
        return Err("Invalid update path: path traversal is not allowed".to_string());
    }
    if normalized.is_empty() {
        return Err("Update UUID or path is empty".to_string());
    }
    Ok(normalized)
}

#[command]
pub async fn upload_update(
    app: AppHandle,
    repo: String,
    token: String,
    uuid: String,
    modpack_key: Option<String>,
    manifest: Manifest,
    config_files: Vec<ConfigFileWithContent>,
) -> Result<(), String> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use reqwest::Client;
    use serde_json::json;

    emit_progress(&app, 5, "Preparing upload...");

    let uuid = normalize_update_uuid_arg(uuid)?;

    // Parse repo as "owner/repo"
    let mut parts = repo.splitn(2, '/');
    let owner = parts.next().ok_or("Invalid repo format")?;
    let repo_name = parts.next().ok_or("Invalid repo format")?;
    let client = Client::new();
    let user_agent = "cemm-app-tauri";

    // Step 1: Get the current commit SHA of main branch
    emit_progress(&app, 10, "Getting branch reference...");
    let refs_url = format!("https://api.github.com/repos/{owner}/{repo_name}/git/refs/heads/main");
    let refs_response = client
        .get(&refs_url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", user_agent)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if !refs_response.status().is_success() {
        return Err(format!(
            "Failed to get main branch ref: {}",
            refs_response.text().await.unwrap_or_default()
        ));
    }

    let refs_json: serde_json::Value = refs_response.json().await.map_err(|e| e.to_string())?;
    let base_commit_sha = refs_json["object"]["sha"]
        .as_str()
        .ok_or("Could not find main branch SHA")?;

    // Step 2: Get the base tree SHA
    emit_progress(&app, 15, "Getting tree structure...");
    let commit_url = format!("https://api.github.com/repos/{owner}/{repo_name}/git/commits/{base_commit_sha}");
    let commit_response = client
        .get(&commit_url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", user_agent)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let commit_json: serde_json::Value = commit_response.json().await.map_err(|e| e.to_string())?;
    let base_tree_sha = commit_json["tree"]["sha"]
        .as_str()
        .ok_or("Could not find base tree SHA")?;

    // Step 3: Create blobs for all files
    emit_progress(&app, 20, "Uploading manifest...");
    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    
    // Create blob for manifest
    let manifest_blob_url = format!("https://api.github.com/repos/{owner}/{repo_name}/git/blobs");
    let manifest_blob_response = client
        .post(&manifest_blob_url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", user_agent)
        .json(&json!({
            "content": STANDARD.encode(manifest_json),
            "encoding": "base64"
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let manifest_blob_json: serde_json::Value = manifest_blob_response.json().await.map_err(|e| e.to_string())?;
    let manifest_blob_sha = manifest_blob_json["sha"]
        .as_str()
        .ok_or("Could not get manifest blob SHA")?;

    // Create blobs for config files with progress
    let total_config_files = config_files.len();
    let mut config_blob_shas = Vec::new();
    for (index, file) in config_files.iter().enumerate() {
        // Calculate progress: 20-70% for config files
        let progress = 20 + ((index + 1) as f32 / total_config_files as f32 * 50.0) as u8;
        emit_progress(&app, progress, &format!("Uploading config file {}/{}...", index + 1, total_config_files));

        // Check if content is already base64-encoded (binary files)
        let (content, encoding) = if file.content.starts_with("data:application/octet-stream;base64,") {
            // Already base64-encoded binary content, extract the base64 part
            let base64_content = file.content.strip_prefix("data:application/octet-stream;base64,").unwrap_or(&file.content);
            (base64_content.to_string(), "base64")
        } else {
            // Text content, encode as base64
            (STANDARD.encode(&file.content), "base64")
        };
        
        let config_blob_response = client
            .post(&manifest_blob_url) // Same URL for creating blobs
            .header("Authorization", format!("token {}", token))
            .header("User-Agent", user_agent)
            .json(&json!({
                "content": content,
                "encoding": encoding
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let config_blob_json: serde_json::Value = config_blob_response.json().await.map_err(|e| e.to_string())?;
        let config_blob_sha = config_blob_json["sha"]
            .as_str()
            .ok_or("Could not get config blob SHA")?
            .to_string();
        config_blob_shas.push(config_blob_sha);
    }

    let update_base_path = primary_update_base_path(modpack_key.as_deref(), &uuid);

    // Step 4: Create a new tree with all files
    emit_progress(&app, 75, "Creating file tree...");
    // Note: This will automatically overwrite any existing files at the same paths
    // because Git tree creation replaces the entire directory structure
    let mut tree_items = vec![
        json!({
            "path": format!("{}/cemm-manifest.json", update_base_path),
            "mode": "100644",
            "type": "blob",
            "sha": manifest_blob_sha
        })
    ];

    // Add config files to tree (will overwrite existing config files if same UUID)
    for (i, file) in config_files.iter().enumerate() {
        tree_items.push(json!({
            "path": format!("{}/{}", update_base_path, file.relative_path),
            "mode": "100644",
            "type": "blob",
            "sha": config_blob_shas[i]
        }));
    }

    let tree_url = format!("https://api.github.com/repos/{owner}/{repo_name}/git/trees");
    let tree_response = client
        .post(&tree_url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", user_agent)
        .json(&json!({
            "base_tree": base_tree_sha,
            "tree": tree_items
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let tree_json: serde_json::Value = tree_response.json().await.map_err(|e| e.to_string())?;
    let new_tree_sha = tree_json["sha"]
        .as_str()
        .ok_or("Could not get new tree SHA")?;

    // Step 5: Create a commit
    emit_progress(&app, 85, "Creating commit...");
    let config_count = config_files.len();
    let commit_message = if config_count > 0 {
        format!("Upload update {} (manifest + {} config files)", uuid, config_count)
    } else {
        format!("Upload update {} (manifest only)", uuid)
    };

    let commit_url = format!("https://api.github.com/repos/{owner}/{repo_name}/git/commits");
    let commit_response = client
        .post(&commit_url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", user_agent)
        .json(&json!({
            "message": commit_message,
            "tree": new_tree_sha,
            "parents": [base_commit_sha]
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let new_commit_json: serde_json::Value = commit_response.json().await.map_err(|e| e.to_string())?;
    let new_commit_sha = new_commit_json["sha"]
        .as_str()
        .ok_or("Could not get new commit SHA")?;

    // Step 6: Update the main branch reference
    emit_progress(&app, 95, "Finalizing...");
    let update_ref_response = client
        .patch(&refs_url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", user_agent)
        .json(&json!({
            "sha": new_commit_sha,
            "force": false
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !update_ref_response.status().is_success() {
        return Err(format!(
            "Failed to update main branch: {}",
            update_ref_response.text().await.unwrap_or_default()
        ));
    }

    emit_progress(&app, 100, "Upload complete");
    Ok(())
}

#[command]
pub async fn download_manifest(
    repo: String,
    uuid: String,
    modpack_key: Option<String>,
) -> Result<Manifest, String> {
    use reqwest::Client;
    use serde_json::Value;

    let uuid = normalize_update_uuid_arg(uuid)?;
    
    // Debug logging
    eprintln!("download_manifest called with repo: '{}', uuid: '{}'", repo, uuid);
    
    let mut parts = repo.splitn(2, '/');
    let owner = parts.next().ok_or("Invalid repo format")?;
    let repo_name = parts.next().ok_or("Invalid repo format")?;
    let base_paths = update_base_path_candidates(modpack_key.as_deref(), &uuid);
    let client = Client::new();
    let user_agent = "cemm-app-tauri";
    let mut last_error = String::new();

    for base_path in base_paths {
        let api_base = format!("https://api.github.com/repos/{owner}/{repo_name}/contents/{base_path}");
        eprintln!("Trying manifest path: {}", api_base);

        let list_res = client
            .get(&api_base)
            .header("User-Agent", user_agent)
            .send()
            .await
            .map_err(|e| {
                eprintln!("Request error: {}", e);
                e.to_string()
            })?;

        if !list_res.status().is_success() {
            last_error = format!(
                "Failed to list update files (status {}): {}",
                list_res.status(),
                list_res.text().await.unwrap_or_default()
            );
            continue;
        }

        let files: Vec<Value> = list_res.json().await.map_err(|e| {
            eprintln!("JSON parsing error: {}", e);
            e.to_string()
        })?;

        let manifest_file = match files.iter().find(|f| f["name"] == "cemm-manifest.json") {
            Some(file) => file,
            None => {
                last_error = "cemm-manifest.json not found".to_string();
                continue;
            }
        };

        let manifest_url = match manifest_file["download_url"].as_str() {
            Some(url) => url,
            None => {
                last_error = "No download_url for cemm-manifest.json".to_string();
                continue;
            }
        };

        let manifest_res = client
            .get(manifest_url)
            .header("User-Agent", user_agent)
            .send()
            .await
            .map_err(|e| {
                eprintln!("Manifest download error: {}", e);
                e.to_string()
            })?;

        if !manifest_res.status().is_success() {
            last_error = format!(
                "Failed to download cemm-manifest.json (status {}): {}",
                manifest_res.status(),
                manifest_res.text().await.unwrap_or_default()
            );
            continue;
        }

        let manifest_json = manifest_res.text().await.map_err(|e| {
            eprintln!("Failed to read manifest response text: {}", e);
            e.to_string()
        })?;

        let manifest: Manifest = serde_json::from_str(&manifest_json).map_err(|e| {
            eprintln!("Failed to parse manifest JSON: {}", e);
            e.to_string()
        })?;

        return Ok(manifest);
    }

    let hint = "\n\nIf GitHub returned 404: confirm the folder exists under the repo (often `modpack-folder/update-id`). You can paste that full path from the repo root in the update field, or pick a modpack folder whose name matches the folder used when the update was published.";

    Err(if last_error.is_empty() {
        format!("Failed to find manifest in update path.{hint}")
    } else {
        format!("{last_error}{hint}")
    })
}

#[command]
pub async fn download_config_files(
    repo: String,
    uuid: String,
    modpack_key: Option<String>,
    manifest: Manifest,
) -> Result<Vec<ConfigFileWithContent>, String> {
    use reqwest::Client;

    let uuid = normalize_update_uuid_arg(uuid)?;
    
    let mut parts = repo.splitn(2, '/');
    let owner = parts.next().ok_or("Invalid repo format")?;
    let repo_name = parts.next().ok_or("Invalid repo format")?;
    let client = Client::new();
    let user_agent = "cemm-app-tauri";
    let base_paths = update_base_path_candidates(modpack_key.as_deref(), &uuid);

    eprintln!("Downloading {} config files from manifest", manifest.config_files.len());

    // Download config files based on manifest list
    let mut config_files = Vec::new();
    for config_file in manifest.config_files {
        let mut downloaded_content: Option<String> = None;
        let mut last_error = String::new();

        for base_path in &base_paths {
            let file_url = format!(
                "https://api.github.com/repos/{owner}/{repo_name}/contents/{}/{}",
                base_path, config_file.relative_path
            );
            eprintln!("Downloading config file from: {}", file_url);

            let file_res = client
                .get(&file_url)
                .header("User-Agent", user_agent)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !file_res.status().is_success() {
                last_error = format!(
                    "Failed to list config file {} (status {}): {}",
                    config_file.relative_path,
                    file_res.status(),
                    file_res.text().await.unwrap_or_default()
                );
                continue;
            }

            let file_data: serde_json::Value = file_res.json().await.map_err(|e| e.to_string())?;
            let download_url = match file_data["download_url"].as_str() {
                Some(url) => url,
                None => {
                    last_error = "No download_url for config file".to_string();
                    continue;
                }
            };

            let content_res = client
                .get(download_url)
                .header("User-Agent", user_agent)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !content_res.status().is_success() {
                last_error = format!(
                    "Failed to download config file {} (status {}): {}",
                    config_file.relative_path,
                    content_res.status(),
                    content_res.text().await.unwrap_or_default()
                );
                continue;
            }

            downloaded_content = Some(content_res.text().await.map_err(|e| e.to_string())?);
            break;
        }

        let content = downloaded_content.ok_or_else(|| {
            if last_error.is_empty() {
                format!("Failed to download config file {}", config_file.relative_path)
            } else {
                last_error
            }
        })?;
        
        config_files.push(ConfigFileWithContent {
            filename: config_file.filename,
            relative_path: config_file.relative_path,
            content,
            is_binary: None, // Will be determined during installation
        });
    }
    
    eprintln!("Successfully downloaded {} config files", config_files.len());
    Ok(config_files)
}

#[command]
pub async fn download_update(
    repo: String,
    uuid: String,
    modpack_key: Option<String>,
) -> Result<DownloadResult, String> {
    let manifest = download_manifest(repo.clone(), uuid.clone(), modpack_key.clone()).await?;
    let config_files = download_config_files(repo, uuid, modpack_key, manifest.clone()).await?;
    
    Ok(DownloadResult {
        manifest,
        config_files,
    })
}
