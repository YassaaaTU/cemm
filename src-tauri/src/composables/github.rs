use std::sync::Arc;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::composables::manifest::{ConfigFileWithContent, Manifest, BINARY_CONTENT_PREFIX};

const MAX_REMOTE_CONFIG_FILES: usize = 1_000;
const MAX_REMOTE_CONFIG_FILE_BYTES: usize = 128 * 1024 * 1024;
const MAX_REMOTE_CONFIG_TOTAL_BYTES: usize = 512 * 1024 * 1024;
const MAX_REMOTE_MANIFEST_BYTES: usize = 8 * 1024 * 1024;

/// The manifest's name inside an update folder. Published under this name by
/// `upload_update_with_progress` and fetched under it by `download_manifest`,
/// so the two cannot drift.
const MANIFEST_FILE_NAME: &str = "cemm-manifest.json";

async fn read_response_limited(
    mut response: reqwest::Response,
    max_bytes: usize,
    description: &str,
) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes as u64)
    {
        return Err(format!(
            "{description} exceeds the {max_bytes}-byte download limit"
        ));
    }

    let mut bytes = Vec::with_capacity(
        response
            .content_length()
            .unwrap_or_default()
            .min(max_bytes as u64) as usize,
    );
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("Failed to read {description}: {error}"))?
    {
        let next_len = bytes
            .len()
            .checked_add(chunk.len())
            .ok_or_else(|| format!("{description} size overflowed"))?;
        if next_len > max_bytes {
            return Err(format!(
                "{description} exceeds the {max_bytes}-byte download limit"
            ));
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok(bytes)
}

fn validate_config_repo_relative_path(path: &str) -> Result<(), String> {
    if !is_safe_repo_relative_path(path)
        || path.contains(['\\', '?', '#'])
        || path
            .split('/')
            .any(|component| component.is_empty() || component == ".")
    {
        return Err(format!("Invalid config file repository path: {path}"));
    }
    Ok(())
}

fn checked_config_download_total(current: usize, next: usize) -> Result<usize, String> {
    let total = current
        .checked_add(next)
        .ok_or_else(|| "Downloaded config data size overflowed".to_string())?;
    if total > MAX_REMOTE_CONFIG_TOTAL_BYTES {
        return Err(format!(
            "Downloaded config data exceeds the {}-byte total limit",
            MAX_REMOTE_CONFIG_TOTAL_BYTES
        ));
    }
    Ok(total)
}

fn validate_remote_config_manifest(manifest: &Manifest) -> Result<(), String> {
    if manifest.config_files.len() > MAX_REMOTE_CONFIG_FILES {
        return Err(format!(
            "Manifest contains {} config files; the download limit is {}",
            manifest.config_files.len(),
            MAX_REMOTE_CONFIG_FILES
        ));
    }
    for config_file in &manifest.config_files {
        validate_config_repo_relative_path(&config_file.relative_path)?;
    }
    Ok(())
}

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
            format!("{BINARY_CONTENT_PREFIX}{}", STANDARD.encode(e.into_bytes()))
        }
    }
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

/// The branch CEMM publishes to, and therefore the branch it downloads from.
/// `upload_update_with_progress` fast-forwards `refs/heads/main` and nothing
/// else, so an update only ever exists here.
const PUBLISH_BRANCH: &str = "main";

/// Where a published file lives, as a URL that can be fetched directly.
///
/// The download path used to ask the contents API for every file -- one
/// `GET /repos/{owner}/{repo}/contents/...` per config file, plus a directory
/// listing for the manifest -- and then follow the `download_url` it reported,
/// which pointed here anyway. Those API calls are unauthenticated on the user
/// side by design, and GitHub allows 60 of them per hour per IP: a pack with
/// more config files than that exhausted the quota partway through its own
/// install, failed on a 403, and kept failing for the rest of the hour. Worse
/// for a household or a LAN party, who share the quota.
///
/// Raw fetches do not count against that limit, and the manifest already tells
/// us every path, so the listing was never needed. This makes an install cost
/// zero REST calls.
///
/// Segments are pushed rather than interpolated so spaces and other characters
/// legal in a config path are percent-encoded. Callers validate the shape first
/// (`validate_config_repo_relative_path`), which is what keeps a path from
/// climbing out of the update folder.
fn raw_update_file_url(
    owner: &str,
    repo_name: &str,
    base_path: &str,
    relative_path: &str,
) -> Result<String, String> {
    let mut url = url::Url::parse("https://raw.githubusercontent.com/")
        .map_err(|error| format!("Could not build a download URL: {error}"))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| "Could not build a download URL".to_string())?;
        segments.push(owner);
        segments.push(repo_name);
        segments.push(PUBLISH_BRANCH);
        for segment in base_path
            .split('/')
            .chain(relative_path.split('/'))
            .filter(|segment| !segment.is_empty())
        {
            segments.push(segment);
        }
    }
    Ok(url.to_string())
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
    let refs_url =
        format!("https://api.github.com/repos/{owner}/{repo_name}/git/refs/heads/{PUBLISH_BRANCH}");
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
        let (content, encoding) = if file.content.starts_with(BINARY_CONTENT_PREFIX) {
            // Already base64-encoded binary content, extract the base64 part
            let base64_content = file
                .content
                .strip_prefix(BINARY_CONTENT_PREFIX)
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
        "path": format!("{update_base_path}/{MANIFEST_FILE_NAME}"),
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

    let uuid = normalize_update_uuid_arg(uuid)?;

    log::debug!("download_manifest: repo '{repo}', update '{uuid}'");

    let (owner, repo_name) = parse_and_validate_repo(&repo)?;
    let base_paths = update_base_path_candidates(modpack_key.as_deref(), &uuid);
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let user_agent = "cemm-app-tauri";
    let mut last_error = String::new();

    for base_path in base_paths {
        // Straight to the file. The listing this used to do existed only to
        // find a name we already know, and cost an API call per candidate.
        let manifest_url = raw_update_file_url(owner, repo_name, &base_path, MANIFEST_FILE_NAME)?;
        log::debug!("download_manifest: trying {manifest_url}");

        let manifest_res = client
            .get(&manifest_url)
            .header("User-Agent", user_agent)
            .send()
            .await
            .map_err(|e| {
                log::warn!("download_manifest: request to {manifest_url} failed: {e}");
                e.to_string()
            })?;

        if !manifest_res.status().is_success() {
            last_error = format!(
                "Failed to download {MANIFEST_FILE_NAME} (status {})",
                manifest_res.status()
            );
            continue;
        }

        let manifest_json =
            read_response_limited(manifest_res, MAX_REMOTE_MANIFEST_BYTES, "CEMM manifest").await?;

        let manifest: Manifest = serde_json::from_slice(&manifest_json).map_err(|e| {
            log::warn!("download_manifest: {MANIFEST_FILE_NAME} did not parse: {e}");
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

    validate_remote_config_manifest(&manifest)?;

    eprintln!(
        "Downloading {} config files from manifest",
        manifest.config_files.len()
    );

    // Download config files based on manifest list
    let mut config_files = Vec::with_capacity(manifest.config_files.len());
    let mut total_downloaded_bytes = 0usize;
    for config_file in manifest.config_files {
        let mut downloaded_content: Option<Vec<u8>> = None;
        let mut last_error = String::new();

        for base_path in &base_paths {
            let file_url =
                raw_update_file_url(owner, repo_name, base_path, &config_file.relative_path)?;
            log::debug!("download_config_files: fetching {file_url}");

            let content_res = client
                .get(&file_url)
                .header("User-Agent", user_agent)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !content_res.status().is_success() {
                last_error = format!(
                    "Failed to download config file {} (status {})",
                    config_file.relative_path,
                    content_res.status()
                );
                continue;
            }

            // .text() performs lossy UTF-8 decoding, which silently corrupts binary
            // config files (e.g. .emotecraft) by replacing invalid byte sequences
            // with U+FFFD (F-P1-4). Reading raw bytes and only decoding as UTF-8
            // when that round-trips cleanly keeps both text and binary files intact.
            let bytes = read_response_limited(
                content_res,
                MAX_REMOTE_CONFIG_FILE_BYTES,
                &format!("config file {}", config_file.relative_path),
            )
            .await?;
            total_downloaded_bytes =
                checked_config_download_total(total_downloaded_bytes, bytes.len())?;
            downloaded_content = Some(bytes);
            break;
        }

        let content = downloaded_content
            .map(encode_downloaded_content)
            .ok_or_else(|| {
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

    /// An install's file URLs are now built here rather than read out of a
    /// contents-API response, so this is the shape the whole download path
    /// depends on.
    #[test]
    fn raw_urls_point_at_the_published_branch_and_encode_their_segments() {
        assert_eq!(
            raw_update_file_url("YassaaaTU", "cemm", "my-pack/abc123", "cemm-manifest.json")
                .expect("url"),
            "https://raw.githubusercontent.com/YassaaaTU/cemm/main/my-pack/abc123/cemm-manifest.json"
        );

        // Config paths legitimately contain spaces; a raw interpolation would
        // have produced an invalid URL where the contents API tolerated one.
        assert_eq!(
            raw_update_file_url("owner", "repo", "pack/id", "config/Some Mod/settings.json")
                .expect("url"),
            "https://raw.githubusercontent.com/owner/repo/main/pack/id/config/Some%20Mod/settings.json"
        );
    }

    /// The paths that reach the URL builder have been through this first, and
    /// it is what stops one climbing out of its update folder or smuggling a
    /// query string onto the request.
    #[test]
    fn config_paths_that_could_escape_the_update_folder_are_refused() {
        for bad in [
            "../../../etc/passwd",
            "/etc/passwd",
            "C:/Windows/System32/drivers/etc/hosts",
            r"config\windows\path.json",
            "config/settings.json?ref=other",
            "config/settings.json#fragment",
            "config//settings.json",
            "config/./settings.json",
        ] {
            assert!(
                validate_config_repo_relative_path(bad).is_err(),
                "{bad} should not be accepted as a config path"
            );
        }

        assert!(validate_config_repo_relative_path("config/mod/settings.json").is_ok());
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

        match content.strip_prefix(BINARY_CONTENT_PREFIX) {
            Some(base64_content) => STANDARD.decode(base64_content).expect("valid base64"),
            None => content.as_bytes().to_vec(),
        }
    }

    #[test]
    fn valid_utf8_bytes_are_kept_as_plain_text() {
        let content = encode_downloaded_content(b"key = \"value\"\n".to_vec());
        assert_eq!(content, "key = \"value\"\n");
        assert!(!content.starts_with(BINARY_CONTENT_PREFIX));
    }

    #[test]
    fn non_utf8_bytes_round_trip_through_the_data_uri_encoding() {
        // The exact byte sequence the audit calls out: invalid as UTF-8, and
        // previously mangled into three U+FFFD replacement characters by .text().
        let original: Vec<u8> = vec![0xFF, 0xFE, 0x00, b'r', b'e', b's', b't'];

        let content = encode_downloaded_content(original.clone());
        assert!(content.starts_with(BINARY_CONTENT_PREFIX));

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

    #[test]
    fn config_repository_paths_are_validated_before_download() {
        assert!(validate_config_repo_relative_path("config/client/settings.toml").is_ok());

        for bad in [
            "",
            "../outside.toml",
            "/absolute.toml",
            "C:/absolute.toml",
            "./config.toml",
            "config//settings.toml",
            "config\\settings.toml",
            "config/settings.toml?raw=1",
            "config/settings.toml#fragment",
        ] {
            assert!(
                validate_config_repo_relative_path(bad).is_err(),
                "expected '{bad}' to be rejected"
            );
        }
    }

    #[test]
    fn config_download_total_is_bounded_and_overflow_safe() {
        assert_eq!(
            checked_config_download_total(MAX_REMOTE_CONFIG_TOTAL_BYTES - 1, 1).unwrap(),
            MAX_REMOTE_CONFIG_TOTAL_BYTES
        );
        assert!(checked_config_download_total(MAX_REMOTE_CONFIG_TOTAL_BYTES, 1).is_err());
        assert!(checked_config_download_total(usize::MAX, 1).is_err());
    }

    #[test]
    fn config_manifest_file_count_is_bounded() {
        let config_file = crate::composables::manifest::ConfigFile {
            filename: "settings.toml".to_string(),
            relative_path: "config/settings.toml".to_string(),
        };
        let manifest = Manifest {
            update_type: Some("config".to_string()),
            mods: Vec::new(),
            resourcepacks: Vec::new(),
            shaderpacks: Vec::new(),
            datapacks: Vec::new(),
            config_files: vec![config_file; MAX_REMOTE_CONFIG_FILES + 1],
        };

        assert!(validate_remote_config_manifest(&manifest).is_err());
    }
}
