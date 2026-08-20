use std::sync::Arc;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::composables::manifest::Manifest;

/// Encodes downloaded config-file bytes for storage in `ConfigFileWithContent.content`.
/// Valid UTF-8 is kept as plain text; anything else is base64-wrapped in the same
/// `data:application/octet-stream;base64,` form `installer::install_update` already
/// decodes, so binary files round-trip byte-for-byte instead of being corrupted by
/// lossy UTF-8 decoding (F-P1-4).
fn encode_downloaded_content(bytes: Vec<u8>) -> String {
    match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(e) => {
            use base64::engine::general_purpose::STANDARD;
            use base64::Engine;
            format!(
                "data:application/octet-stream;base64,{}",
                STANDARD.encode(e.into_bytes())
            )
        }
    }
}

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

/// Progress event payload for upload operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadProgress {
    pub progress: u8,
    pub message: String,
}

pub type UploadProgressCallback = Arc<dyn Fn(UploadProgress) + Send + Sync>;

/// Reports upload progress without coupling the GitHub service to Tauri.
fn emit_progress(callback: &UploadProgressCallback, progress: u8, message: &str) {
    callback(UploadProgress {
        progress,
        message: message.to_string(),
    });
}

#[derive(Deserialize)]
struct GitHubErrorResponse {
    message: Option<String>,
}

/// Produces a credential-safe error for a failed GitHub API response.
///
/// GitHub's JSON `message` is useful to the caller, while raw response bodies
/// are intentionally not returned: they may be non-JSON proxy output and are
/// not needed to diagnose the upload stage or HTTP failure.
fn format_github_api_error(operation: &str, status: reqwest::StatusCode, body: &str) -> String {
    let detail = if body.trim().is_empty() {
        "GitHub returned no error details".to_string()
    } else if let Ok(error) = serde_json::from_str::<GitHubErrorResponse>(body) {
        error
            .message
            .filter(|message| !message.trim().is_empty())
            .unwrap_or_else(|| "GitHub returned an error response without a message".to_string())
    } else {
        "GitHub returned a non-JSON error response".to_string()
    };

    format!("GitHub {operation} failed ({status}): {detail}")
}

/// Reads a GitHub response once, preserving API failure context before decoding
/// the successful response body into its requested shape.
async fn decode_github_response<T>(
    operation: &str,
    response: reqwest::Response,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Failed to read GitHub response for {operation}: {error}"))?;

    if !status.is_success() {
        return Err(format_github_api_error(operation, status, &body));
    }

    serde_json::from_str(&body)
        .map_err(|error| format!("Failed to decode GitHub response for {operation}: {error}"))
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

/// Splits and validates a GitHub "owner/repo" string, mirroring the frontend's
/// own check (`GitHubSettings.vue`'s `/^[a-zA-Z0-9._-]+\/[a-zA-Z0-9._-]+$/`).
/// `UserPanel.vue` writes `githubRepo` directly on input with no such check, so
/// the backend cannot trust that value has ever been validated (F-P2-20) — every
/// caller here interpolates the result straight into a GitHub API URL, and a
/// value like `owner/repo/contents/x?ref=` would otherwise inject extra path
/// and query segments.
fn parse_and_validate_repo(repo: &str) -> Result<(&str, &str), String> {
    let mut parts = repo.splitn(2, '/');
    let owner = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or("Invalid repo format")?;
    let repo_name = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or("Invalid repo format")?;

    let is_valid_segment = |s: &str| {
        s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    };

    if !is_valid_segment(owner) || !is_valid_segment(repo_name) {
        return Err(format!(
            "Invalid repository format: '{}'. Expected 'owner/repo'.",
            repo
        ));
    }

    Ok((owner, repo_name))
}

pub async fn upload_update_with_progress(
    repo: String,
    token: String,
    uuid: String,
    modpack_key: Option<String>,
    manifest: Manifest,
    config_files: Vec<ConfigFileWithContent>,
    progress_callback: UploadProgressCallback,
) -> Result<(), String> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use reqwest::Client;
    use serde_json::json;

    emit_progress(&progress_callback, 5, "Preparing upload...");

    let uuid = normalize_update_uuid_arg(uuid)?;

    let (owner, repo_name) = parse_and_validate_repo(&repo)?;
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let user_agent = "cemm-app-tauri";

    // Step 1: Get the current commit SHA of main branch
    emit_progress(&progress_callback, 10, "Getting branch reference...");
    let refs_url = format!("https://api.github.com/repos/{owner}/{repo_name}/git/refs/heads/main");
    let refs_response = client
        .get(&refs_url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", user_agent)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let refs_json: serde_json::Value =
        decode_github_response("get main branch reference", refs_response).await?;
    let base_commit_sha = refs_json["object"]["sha"]
        .as_str()
        .ok_or("Could not find main branch SHA")?;

    // Step 2: Get the base tree SHA
    emit_progress(&progress_callback, 15, "Getting tree structure...");
    let commit_url =
        format!("https://api.github.com/repos/{owner}/{repo_name}/git/commits/{base_commit_sha}");
    let commit_response = client
        .get(&commit_url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", user_agent)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let commit_json: serde_json::Value =
        decode_github_response("get base commit", commit_response).await?;
    let base_tree_sha = commit_json["tree"]["sha"]
        .as_str()
        .ok_or("Could not find base tree SHA")?;

    // Step 3: Create blobs for all files
    emit_progress(&progress_callback, 20, "Uploading manifest...");
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

    let manifest_blob_json: serde_json::Value =
        decode_github_response("create manifest blob", manifest_blob_response).await?;
    let manifest_blob_sha = manifest_blob_json["sha"]
        .as_str()
        .ok_or("Could not get manifest blob SHA")?;

    // Create blobs for config files with progress
    let total_config_files = config_files.len();
    let mut config_blob_shas = Vec::new();
    for (index, file) in config_files.iter().enumerate() {
        // Calculate progress: 20-70% for config files
        let progress = 20 + ((index + 1) as f32 / total_config_files as f32 * 50.0) as u8;
        emit_progress(
            &progress_callback,
            progress,
            &format!(
                "Uploading config file {}/{}...",
                index + 1,
                total_config_files
            ),
        );

        // Check if content is already base64-encoded (binary files)
        let (content, encoding) = if file
            .content
            .starts_with("data:application/octet-stream;base64,")
        {
            // Already base64-encoded binary content, extract the base64 part
            let base64_content = file
                .content
                .strip_prefix("data:application/octet-stream;base64,")
                .unwrap_or(&file.content);
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

        let config_blob_json: serde_json::Value =
            decode_github_response("create config file blob", config_blob_response).await?;
        let config_blob_sha = config_blob_json["sha"]
            .as_str()
            .ok_or("Could not get config blob SHA")?
            .to_string();
        config_blob_shas.push(config_blob_sha);
    }

    let update_base_path = primary_update_base_path(modpack_key.as_deref(), &uuid);

    // Step 4: Create a new tree with all files
    emit_progress(&progress_callback, 75, "Creating file tree...");
    // Note: This will automatically overwrite any existing files at the same paths
    // because Git tree creation replaces the entire directory structure
    let mut tree_items = vec![json!({
        "path": format!("{}/cemm-manifest.json", update_base_path),
        "mode": "100644",
        "type": "blob",
        "sha": manifest_blob_sha
    })];

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

    let tree_json: serde_json::Value =
        decode_github_response("create file tree", tree_response).await?;
    let new_tree_sha = tree_json["sha"]
        .as_str()
        .ok_or("Could not get new tree SHA")?;

    // Step 5: Create a commit
    emit_progress(&progress_callback, 85, "Creating commit...");
    let config_count = config_files.len();
    let commit_message = if config_count > 0 {
        format!(
            "Upload update {} (manifest + {} config files)",
            uuid, config_count
        )
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

    let new_commit_json: serde_json::Value =
        decode_github_response("create commit", commit_response).await?;
    let new_commit_sha = new_commit_json["sha"]
        .as_str()
        .ok_or("Could not get new commit SHA")?;

    // Step 6: Update the main branch reference
    emit_progress(&progress_callback, 95, "Finalizing...");
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

    let _: serde_json::Value =
        decode_github_response("update main branch reference", update_ref_response).await?;

    emit_progress(&progress_callback, 100, "Upload complete");
    Ok(())
}

pub async fn download_manifest(
    repo: String,
    uuid: String,
    modpack_key: Option<String>,
) -> Result<Manifest, String> {
    use reqwest::Client;
    use serde_json::Value;

    let uuid = normalize_update_uuid_arg(uuid)?;

    // Debug logging
    eprintln!(
        "download_manifest called with repo: '{}', uuid: '{}'",
        repo, uuid
    );

    let (owner, repo_name) = parse_and_validate_repo(&repo)?;
    let base_paths = update_base_path_candidates(modpack_key.as_deref(), &uuid);
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let user_agent = "cemm-app-tauri";
    let mut last_error = String::new();

    for base_path in base_paths {
        let api_base =
            format!("https://api.github.com/repos/{owner}/{repo_name}/contents/{base_path}");
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

pub async fn download_config_files(
    repo: String,
    uuid: String,
    modpack_key: Option<String>,
    manifest: Manifest,
) -> Result<Vec<ConfigFileWithContent>, String> {
    use reqwest::Client;

    let uuid = normalize_update_uuid_arg(uuid)?;

    let (owner, repo_name) = parse_and_validate_repo(&repo)?;
    // Config files can be sizeable binaries (e.g. resource-pack-adjacent assets),
    // so this gets more headroom than the plain API-listing client above.
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let user_agent = "cemm-app-tauri";
    let base_paths = update_base_path_candidates(modpack_key.as_deref(), &uuid);

    eprintln!(
        "Downloading {} config files from manifest",
        manifest.config_files.len()
    );

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

            // .text() performs lossy UTF-8 decoding, which silently corrupts binary
            // config files (e.g. .emotecraft) by replacing invalid byte sequences
            // with U+FFFD (F-P1-4). Reading raw bytes and only decoding as UTF-8
            // when that round-trips cleanly keeps both text and binary files intact.
            let bytes = content_res.bytes().await.map_err(|e| e.to_string())?;
            downloaded_content = Some(encode_downloaded_content(bytes.to_vec()));
            break;
        }

        let content = downloaded_content.ok_or_else(|| {
            if last_error.is_empty() {
                format!(
                    "Failed to download config file {}",
                    config_file.relative_path
                )
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

    eprintln!(
        "Successfully downloaded {} config files",
        config_files.len()
    );
    Ok(config_files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_json_error_retains_the_api_message_and_status() {
        let error = format_github_api_error(
            "create commit",
            reqwest::StatusCode::UNAUTHORIZED,
            r#"{"message":"Bad credentials"}"#,
        );

        assert_eq!(
            error,
            "GitHub create commit failed (401 Unauthorized): Bad credentials"
        );
    }

    #[test]
    fn github_non_json_error_reports_stage_and_status_without_echoing_body() {
        let error = format_github_api_error(
            "create file tree",
            reqwest::StatusCode::BAD_GATEWAY,
            "upstream service unavailable",
        );

        assert_eq!(
            error,
            "GitHub create file tree failed (502 Bad Gateway): GitHub returned a non-JSON error response"
        );
        assert!(!error.contains("upstream service unavailable"));
    }

    #[test]
    fn github_empty_error_reports_stage_and_status() {
        let error = format_github_api_error(
            "update main branch reference",
            reqwest::StatusCode::CONFLICT,
            "  \n",
        );

        assert_eq!(
            error,
            "GitHub update main branch reference failed (409 Conflict): GitHub returned no error details"
        );
    }

    #[test]
    fn parse_and_validate_repo_accepts_well_formed_values() {
        assert_eq!(
            parse_and_validate_repo("YassaaaTU/cemm").unwrap(),
            ("YassaaaTU", "cemm")
        );
        assert_eq!(
            parse_and_validate_repo("some.org/repo_name-2").unwrap(),
            ("some.org", "repo_name-2")
        );
    }

    #[test]
    fn parse_and_validate_repo_rejects_injection_attempts() {
        for bad in [
            "owner/repo/contents/x?ref=",
            "owner/../repo",
            "owner",
            "owner/",
            "/repo",
            "owner/repo?evil=1",
            "owner/repo#fragment",
            "",
        ] {
            assert!(
                parse_and_validate_repo(bad).is_err(),
                "expected '{bad}' to be rejected"
            );
        }
    }

    /// Mirrors the decode step in installer.rs's config-file install loop, so this
    /// test exercises the same encode/decode pair the real upload -> download ->
    /// install pipeline uses, without needing a live GitHub download.
    fn decode_installed_content(content: &str) -> Vec<u8> {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;

        match content.strip_prefix("data:application/octet-stream;base64,") {
            Some(base64_content) => STANDARD.decode(base64_content).expect("valid base64"),
            None => content.as_bytes().to_vec(),
        }
    }

    #[test]
    fn valid_utf8_bytes_are_kept_as_plain_text() {
        let content = encode_downloaded_content(b"key = \"value\"\n".to_vec());
        assert_eq!(content, "key = \"value\"\n");
        assert!(!content.starts_with("data:application/octet-stream;base64,"));
    }

    #[test]
    fn non_utf8_bytes_round_trip_through_the_data_uri_encoding() {
        // The exact byte sequence the audit calls out: invalid as UTF-8, and
        // previously mangled into three U+FFFD replacement characters by .text().
        let original: Vec<u8> = vec![0xFF, 0xFE, 0x00, b'r', b'e', b's', b't'];

        let content = encode_downloaded_content(original.clone());
        assert!(content.starts_with("data:application/octet-stream;base64,"));

        let round_tripped = decode_installed_content(&content);
        assert_eq!(
            round_tripped, original,
            "binary content must round-trip byte-for-byte"
        );
    }

    #[test]
    fn empty_bytes_round_trip_as_empty_text() {
        let content = encode_downloaded_content(Vec::new());
        assert_eq!(content, "");
    }
}
