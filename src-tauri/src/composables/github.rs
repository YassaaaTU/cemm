use std::sync::Arc;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::composables::manifest::{
    ConfigFile, ConfigFileWithContent, Manifest, BINARY_CONTENT_PREFIX,
};
use crate::composables::release::{
    self, is_commit_sha, is_sha256_hex, is_valid_lineage, object_path, parse_release,
    parse_update_code, Release, ServerRelease, RELEASE_FILE_NAME,
};

const MAX_REMOTE_CONFIG_FILES: usize = 1_000;
const MAX_REMOTE_CONFIG_FILE_BYTES: usize = 128 * 1024 * 1024;
const MAX_REMOTE_CONFIG_TOTAL_BYTES: usize = 512 * 1024 * 1024;
const MAX_REMOTE_MANIFEST_BYTES: usize = 8 * 1024 * 1024;
/// A release lists every managed file with its hash, so it runs larger than a
/// legacy manifest: about 250 bytes an entry, and a big pack has tens of
/// thousands of entries.
const MAX_REMOTE_RELEASE_BYTES: usize = 48 * 1024 * 1024;

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

/// Every file an update asks the player to download, config files and custom
/// data pack files alike.
///
/// One list rather than two loops, because the two are identical in every way
/// that matters here: both are attacker-controlled relative paths fetched from
/// the update folder and written into the instance. Only their meaning differs,
/// and meaning is the manifest's business, not the downloader's.
fn remote_files(manifest: &Manifest) -> impl Iterator<Item = &ConfigFile> {
    manifest.config_files.iter().chain(
        manifest
            .custom_datapacks
            .iter()
            .flat_map(|datapack| datapack.files.iter()),
    )
}

fn validate_remote_config_manifest(manifest: &Manifest) -> Result<(), String> {
    let count = remote_files(manifest).count();
    if count > MAX_REMOTE_CONFIG_FILES {
        return Err(format!(
            "Manifest lists {count} files to download; the limit is {MAX_REMOTE_CONFIG_FILES}"
        ));
    }
    for file in remote_files(manifest) {
        validate_config_repo_relative_path(&file.relative_path)?;
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
    GithubEndpoints::resolve().raw_file_url(
        owner,
        repo_name,
        PUBLISH_BRANCH,
        base_path,
        relative_path,
    )
}

/// The two GitHub hosts CEMM talks to.
///
/// A type rather than two string literals so the transport can be exercised
/// end to end against local test storage. Only a development build can be
/// pointed anywhere else: `cfg!(debug_assertions)` is false in every shipped
/// binary, where the override variables are never read and both hosts are
/// exactly what they have always been.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubEndpoints {
    pub api: String,
    pub raw: String,
}

impl GithubEndpoints {
    pub fn resolve() -> Self {
        let dev = |name: &str| {
            if !cfg!(debug_assertions) {
                return None;
            }
            std::env::var(name)
                .ok()
                .map(|value| value.trim_end_matches('/').to_string())
                .filter(|value| !value.is_empty())
        };
        Self {
            api: dev("CEMM_DEV_GITHUB_API").unwrap_or_else(|| "https://api.github.com".to_string()),
            raw: dev("CEMM_DEV_GITHUB_RAW")
                .unwrap_or_else(|| "https://raw.githubusercontent.com".to_string()),
        }
    }

    /// A directly fetchable URL for one file at one reference — a branch name,
    /// or a commit, which is what makes a release immutable to its readers.
    pub fn raw_file_url(
        &self,
        owner: &str,
        repo_name: &str,
        reference: &str,
        base_path: &str,
        relative_path: &str,
    ) -> Result<String, String> {
        let mut url = url::Url::parse(&format!("{}/", self.raw))
            .map_err(|error| format!("Could not build a download URL: {error}"))?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| "Could not build a download URL".to_string())?;
            segments.pop_if_empty();
            segments.push(owner);
            segments.push(repo_name);
            segments.push(reference);
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
    let api = GithubEndpoints::resolve().api;

    // A code names one update for good. The tree API would happily replace the
    // files of an existing one, and everyone holding the old code would then
    // install something its admin never showed them.
    let update_base_path = primary_update_base_path(modpack_key.as_deref(), &uuid);
    if update_path_exists(&client, &api, owner, repo_name, &token, &update_base_path).await? {
        return Err(format!(
            "An update already exists at '{update_base_path}'. Update codes are never reused; publish again to get a new one."
        ));
    }

    // Step 1: Get the current commit SHA of main branch
    emit_progress(&progress_callback, 10, "Getting branch reference...");
    let refs_url = format!("{api}/repos/{owner}/{repo_name}/git/refs/heads/{PUBLISH_BRANCH}");
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
    let commit_url = format!("{api}/repos/{owner}/{repo_name}/git/commits/{base_commit_sha}");
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
    let manifest_blob_url = format!("{api}/repos/{owner}/{repo_name}/git/blobs");
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

    let tree_url = format!("{api}/repos/{owner}/{repo_name}/git/trees");
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

    let commit_url = format!("{api}/repos/{owner}/{repo_name}/git/commits");
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

    let endpoints = GithubEndpoints::resolve();
    for base_path in base_paths {
        match fetch_legacy_manifest(
            &client,
            &endpoints,
            owner,
            repo_name,
            PUBLISH_BRANCH,
            &base_path,
        )
        .await?
        {
            Ok(manifest) => return Ok(manifest),
            Err(not_found) => last_error = not_found,
        }
    }
    let _ = user_agent;

    Err(manifest_not_found(&last_error))
}

const NOT_FOUND_HINT: &str = "\n\nIf GitHub returned 404: confirm the folder exists under the repo (often `modpack-folder/update-id`). You can paste that full path from the repo root in the update field, or pick a modpack folder whose name matches the folder used when the update was published.";

fn manifest_not_found(last_error: &str) -> String {
    if last_error.is_empty() {
        format!("Failed to find manifest in update path.{NOT_FOUND_HINT}")
    } else {
        format!("{last_error}{NOT_FOUND_HINT}")
    }
}

/// Fetches the legacy manifest for one candidate path at one reference.
///
/// The outer `Result` is a failure worth stopping for — the request itself, or
/// a manifest that is there but does not parse. The inner one is "not at this
/// path", which the caller answers by trying the next candidate.
async fn fetch_legacy_manifest(
    client: &reqwest::Client,
    endpoints: &GithubEndpoints,
    owner: &str,
    repo_name: &str,
    reference: &str,
    base_path: &str,
) -> Result<Result<Manifest, String>, String> {
    // Straight to the file. The listing this used to do existed only to find a
    // name we already know, and cost an API call per candidate.
    let manifest_url =
        endpoints.raw_file_url(owner, repo_name, reference, base_path, MANIFEST_FILE_NAME)?;
    log::debug!("download_manifest: trying {manifest_url}");

    let manifest_res = client
        .get(&manifest_url)
        .header("User-Agent", "cemm-app-tauri")
        .send()
        .await
        .map_err(|e| {
            log::warn!("download_manifest: request to {manifest_url} failed: {e}");
            e.to_string()
        })?;

    if !manifest_res.status().is_success() {
        return Ok(Err(format!(
            "Failed to download {MANIFEST_FILE_NAME} (status {})",
            manifest_res.status()
        )));
    }

    let manifest_json =
        read_response_limited(manifest_res, MAX_REMOTE_MANIFEST_BYTES, "CEMM manifest").await?;

    let manifest: Manifest = serde_json::from_slice(&manifest_json).map_err(|e| {
        log::warn!("download_manifest: {MANIFEST_FILE_NAME} did not parse: {e}");
        e.to_string()
    })?;

    Ok(Ok(manifest))
}

/// Whether anything is already published under an update's folder.
async fn update_path_exists(
    client: &reqwest::Client,
    api: &str,
    owner: &str,
    repo_name: &str,
    token: &str,
    base_path: &str,
) -> Result<bool, String> {
    let mut url = url::Url::parse(&format!("{api}/repos/{owner}/{repo_name}/contents"))
        .map_err(|error| format!("Could not build a GitHub URL: {error}"))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| "Could not build a GitHub URL".to_string())?;
        for segment in base_path.split('/').filter(|segment| !segment.is_empty()) {
            segments.push(segment);
        }
    }
    url.query_pairs_mut().append_pair("ref", PUBLISH_BRANCH);

    let response = client
        .get(url)
        .header("Authorization", format!("token {token}"))
        .header("User-Agent", "cemm-app-tauri")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    match response.status() {
        status if status.is_success() => Ok(true),
        reqwest::StatusCode::NOT_FOUND => Ok(false),
        status => {
            let body = response.text().await.unwrap_or_default();
            Err(format_github_api_error(
                "check for an existing update",
                status,
                &body,
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Releases
// ---------------------------------------------------------------------------

/// What an update code resolved to.
///
/// One fetch for both formats, so the player pastes a code without having to
/// know which kind it is and the screen branches on the answer.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "kind", rename_all = "lowercase")]
#[ts(export, export_to = "generated/")]
pub enum FetchedUpdate {
    Legacy {
        manifest: Manifest,
    },
    Release {
        release: Box<Release>,
        /// SHA-256 of the release document as published.
        digest: String,
        /// The commit the document was read at. Every payload is read at the
        /// same one, so a release cannot be assembled from two states of `main`.
        revision: String,
        /// The full, pinned code — worth showing back, since it is the one
        /// that names this exact content for good.
        code: String,
    },
}

/// Asks which commit the publish branch points at, once.
async fn resolve_branch_head(
    client: &reqwest::Client,
    endpoints: &GithubEndpoints,
    owner: &str,
    repo_name: &str,
) -> Result<String, String> {
    let url = format!(
        "{}/repos/{owner}/{repo_name}/commits/{PUBLISH_BRANCH}",
        endpoints.api
    );
    let unpinned = |detail: String| {
        format!(
            "Could not pin this update to a revision ({detail}). GitHub limits how often that can be asked without signing in. Wait a few minutes, or paste the full code including the part after '@'."
        )
    };
    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github.sha")
        .header("User-Agent", "cemm-app-tauri")
        .send()
        .await
        .map_err(|error| unpinned(error.to_string()))?;
    if !response.status().is_success() {
        return Err(unpinned(format!("status {}", response.status())));
    }
    let bytes = read_response_limited(response, 256, "branch revision").await?;
    let revision = String::from_utf8_lossy(&bytes).trim().to_lowercase();
    if !is_commit_sha(&revision) {
        return Err(unpinned("unexpected answer".to_string()));
    }
    Ok(revision)
}

/// Resolves an update code to a release or a legacy manifest.
pub async fn fetch_update(
    repo: String,
    code: String,
    modpack_key: Option<String>,
) -> Result<FetchedUpdate, String> {
    let code = parse_update_code(&code)?;
    let (owner, repo_name) = parse_and_validate_repo(&repo)?;
    let endpoints = GithubEndpoints::resolve();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let reference = code
        .revision
        .clone()
        .unwrap_or_else(|| PUBLISH_BRANCH.to_string());
    let mut last_error = String::new();

    for base_path in update_base_path_candidates(modpack_key.as_deref(), &code.path) {
        let release_url =
            endpoints.raw_file_url(owner, repo_name, &reference, &base_path, RELEASE_FILE_NAME)?;
        let response = client
            .get(&release_url)
            .header("User-Agent", "cemm-app-tauri")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if response.status().is_success() {
            let mut bytes =
                read_response_limited(response, MAX_REMOTE_RELEASE_BYTES, "CEMM release").await?;
            let revision = match code.revision.clone() {
                Some(revision) => revision,
                None => {
                    // A short code names a path on a moving branch. Pin it now
                    // and read the document again at that commit, so it and
                    // every payload after it come from one tree.
                    let head = resolve_branch_head(&client, &endpoints, owner, repo_name).await?;
                    let pinned_url = endpoints.raw_file_url(
                        owner,
                        repo_name,
                        &head,
                        &base_path,
                        RELEASE_FILE_NAME,
                    )?;
                    let pinned = client
                        .get(&pinned_url)
                        .header("User-Agent", "cemm-app-tauri")
                        .send()
                        .await
                        .map_err(|e| e.to_string())?;
                    if !pinned.status().is_success() {
                        return Err(format!(
                            "The release could not be read at its pinned revision (status {}). Try again in a moment.",
                            pinned.status()
                        ));
                    }
                    bytes = read_response_limited(pinned, MAX_REMOTE_RELEASE_BYTES, "CEMM release")
                        .await?;
                    head
                }
            };

            let release = parse_release(&bytes)?;
            if release.code_path() != base_path {
                return Err(format!(
                    "The release published at '{base_path}' describes '{}'. A release is only valid under its own code, so CEMM will not apply this one.",
                    release.code_path()
                ));
            }
            let code = format!("{}@{revision}", release.code_path());
            return Ok(FetchedUpdate::Release {
                digest: release::sha256_hex(&bytes),
                release: Box::new(release),
                revision,
                code,
            });
        }

        match fetch_legacy_manifest(
            &client, &endpoints, owner, repo_name, &reference, &base_path,
        )
        .await?
        {
            Ok(manifest) => return Ok(FetchedUpdate::Legacy { manifest }),
            Err(not_found) => last_error = not_found,
        }
    }

    Err(manifest_not_found(&last_error))
}

/// Downloads one hosted payload at the release's pinned revision.
///
/// The caller verifies the hash; this only refuses to read more than the
/// release says the object holds.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn download_object(
    client: &reqwest::Client,
    endpoints: &GithubEndpoints,
    repo: &str,
    revision: &str,
    lineage: &str,
    sha256: &str,
    size: u64,
    destination: &std::path::Path,
) -> Result<(), String> {
    let (owner, repo_name) = parse_and_validate_repo(repo)?;
    if !is_commit_sha(revision) {
        return Err("This release is not pinned to a revision, so its files cannot be fetched safely. Fetch the update again.".to_string());
    }
    if !is_valid_lineage(lineage) || !is_sha256_hex(sha256) {
        return Err("Refusing to fetch a release file with a malformed address".to_string());
    }
    let url = endpoints.raw_file_url(
        owner,
        repo_name,
        revision,
        "",
        &object_path(lineage, sha256),
    )?;
    crate::installer::download_and_save(client, &url, destination, size).await
}

/// Proof of one publication, for the admin to keep outside the profile.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "generated/")]
pub struct PublishReceipt {
    /// The pinned code to hand out: `lineage/release@commit`.
    pub code: String,
    pub repo: String,
    pub lineage: String,
    pub release_id: String,
    #[ts(type = "number")]
    pub sequence: u64,
    /// The commit that introduced the release.
    pub revision: String,
    pub release_digest: String,
    pub uploaded_objects: u32,
    pub reused_objects: u32,
    #[ts(type = "number")]
    pub published_unix: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub server_release: Option<ServerRelease>,
}

async fn github_send(
    operation: &str,
    request: reqwest::RequestBuilder,
    token: &str,
) -> Result<serde_json::Value, String> {
    let response = request
        .header("Authorization", format!("token {token}"))
        .header("User-Agent", "cemm-app-tauri")
        .send()
        .await
        .map_err(|error| format!("GitHub {operation} failed: {error}"))?;
    decode_github_response(operation, response).await
}

/// How many times one upload waits out a rate limit before the publish gives up.
const MAX_RATE_LIMIT_WAITS: u32 = 8;
/// The longest single wait. A primary limit can ask for most of an hour; this
/// keeps the progress line moving and the total bounded at roughly forty minutes.
const MAX_RATE_LIMIT_WAIT_SECS: u64 = 300;

/// Whether a failed response is GitHub asking for patience, and for how long.
///
/// A first release of a large pack is thousands of blob uploads in a row, which
/// is exactly what GitHub's secondary limits exist to slow down. Those arrive
/// as 429, or as 403 with a `retry-after` header or a message saying so; the
/// primary limit arrives as 403 with `x-ratelimit-remaining: 0` and the time it
/// resets. Anything else — a bad token, a missing repository — is a real
/// failure and is not waited on.
fn rate_limit_wait(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
    body: &str,
    attempt: u32,
    now_unix: u64,
) -> Option<u64> {
    let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    let retry_after = header("retry-after").and_then(|value| value.trim().parse::<u64>().ok());
    let exhausted = header("x-ratelimit-remaining").map(str::trim) == Some("0");
    let says_so = {
        let lowered = body.to_lowercase();
        lowered.contains("rate limit") || lowered.contains("abuse detection")
    };

    let limited = status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || (status == reqwest::StatusCode::FORBIDDEN
            && (retry_after.is_some() || exhausted || says_so));
    if !limited {
        return None;
    }

    let until_reset = header("x-ratelimit-reset")
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|_| exhausted)
        .map(|reset| reset.saturating_sub(now_unix) + 1);
    let backoff = 10u64.saturating_mul(1 << attempt.min(5));
    Some(
        retry_after
            .or(until_reset)
            .unwrap_or(backoff)
            .min(MAX_RATE_LIMIT_WAIT_SECS),
    )
}

/// Sends a request that is safe to repeat, waiting out rate limits.
///
/// Only for idempotent calls. Creating a blob is one: the same bytes always
/// produce the same object, so sending them twice changes nothing.
async fn github_send_patiently(
    operation: &str,
    build: impl Fn() -> reqwest::RequestBuilder,
    token: &str,
    on_wait: impl Fn(u64),
) -> Result<serde_json::Value, String> {
    let mut attempt = 0u32;
    loop {
        let response = build()
            .header("Authorization", format!("token {token}"))
            .header("User-Agent", "cemm-app-tauri")
            .send()
            .await
            .map_err(|error| format!("GitHub {operation} failed: {error}"))?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = response
            .text()
            .await
            .map_err(|error| format!("Failed to read GitHub response for {operation}: {error}"))?;

        if status.is_success() {
            return serde_json::from_str(&body).map_err(|error| {
                format!("Failed to decode GitHub response for {operation}: {error}")
            });
        }
        match rate_limit_wait(
            status,
            &headers,
            &body,
            attempt,
            crate::installer::unix_now(),
        ) {
            Some(wait) if attempt < MAX_RATE_LIMIT_WAITS => {
                log::warn!("github: {operation} rate limited, waiting {wait}s (attempt {attempt})");
                on_wait(wait);
                tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                attempt += 1;
            }
            Some(_) => {
                return Err(format!(
                    "GitHub kept limiting uploads ({}). Nothing was published. Wait an hour, then publish again.",
                    format_github_api_error(operation, status, &body)
                ));
            }
            None => return Err(format_github_api_error(operation, status, &body)),
        }
    }
}

/// Publishes a release: its hosted payloads, its document and the legacy
/// tombstone, as one commit.
///
/// Atomic the same way a legacy publish is — blobs and a tree are invisible
/// until the branch moves, and it moves once, without force. Either the whole
/// release appears or none of it does.
///
/// Payloads are read from `modpack_path` here, in the sidecar, and hashed again
/// as they are read. The release document was captured a moment ago by a
/// separate call; a file edited in between would otherwise be uploaded under a
/// hash it no longer has.
pub async fn publish_release(
    repo: String,
    token: String,
    modpack_path: String,
    release: Release,
    progress_callback: UploadProgressCallback,
) -> Result<PublishReceipt, String> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use serde_json::json;

    emit_progress(&progress_callback, 2, "Checking the release...");
    release::validate_release(&release)?;
    let (owner, repo_name) = parse_and_validate_repo(&repo)?;
    let api = GithubEndpoints::resolve().api;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;
    let release_dir = release.code_path();

    emit_progress(&progress_callback, 5, "Reading the repository...");
    let refs_url = format!("{api}/repos/{owner}/{repo_name}/git/refs/heads/{PUBLISH_BRANCH}");
    let refs = github_send("get main branch reference", client.get(&refs_url), &token).await?;
    let base_commit = refs["object"]["sha"]
        .as_str()
        .ok_or("Could not find main branch SHA")?
        .to_string();
    let commit = github_send(
        "get base commit",
        client.get(format!(
            "{api}/repos/{owner}/{repo_name}/git/commits/{base_commit}"
        )),
        &token,
    )
    .await?;
    let base_tree = commit["tree"]["sha"]
        .as_str()
        .ok_or("Could not find base tree SHA")?
        .to_string();

    // What is already there: the release folder must not be, and any object
    // that is does not need sending again.
    let tree = github_send(
        "list repository tree",
        client.get(format!(
            "{api}/repos/{owner}/{repo_name}/git/trees/{base_tree}?recursive=1"
        )),
        &token,
    )
    .await?;
    let listed: std::collections::HashSet<String> = tree["tree"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry["path"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let complete_listing = tree["truncated"].as_bool() != Some(true);
    let prefix = format!("{release_dir}/");
    let already_published = listed
        .iter()
        .any(|path| path == &release_dir || path.starts_with(&prefix))
        || (!complete_listing
            && update_path_exists(&client, &api, owner, repo_name, &token, &release_dir).await?);
    if already_published {
        return Err(format!(
            "Release '{}' of '{}' is already published. A release never changes once it has a code; give this one a new release id.",
            release.release_id, release.lineage
        ));
    }

    // ---- Payloads ----------------------------------------------------------------
    let root = std::path::Path::new(&modpack_path);
    let blobs_url = format!("{api}/repos/{owner}/{repo_name}/git/blobs");
    let mut wanted: Vec<&release::ReleaseFile> =
        release.files.iter().filter(|file| file.hosted).collect();
    wanted.sort_by(|a, b| a.sha256.cmp(&b.sha256));
    wanted.dedup_by(|a, b| a.sha256 == b.sha256);

    let mut tree_items = Vec::new();
    let mut uploaded = 0u32;
    let mut reused = 0u32;
    let total = wanted.len().max(1);
    for (index, file) in wanted.iter().enumerate() {
        let repository_path = object_path(&release.lineage, &file.sha256);
        if complete_listing && listed.contains(&repository_path) {
            reused += 1;
            continue;
        }
        let local = crate::installer::validate_path_within_base(root, &file.path)?;
        let bytes = tokio::fs::read(&local)
            .await
            .map_err(|error| format!("Failed to read {}: {error}", local.display()))?;
        if release::sha256_hex(&bytes) != file.sha256 {
            return Err(format!(
                "{} changed after the release was checked. Check the release again before publishing. Nothing was published.",
                file.path
            ));
        }
        emit_progress(
            &progress_callback,
            10 + ((index + 1) as f32 / total as f32 * 70.0) as u8,
            &format!(
                "Uploading {} ({}/{})...",
                file.path,
                index + 1,
                wanted.len()
            ),
        );
        let payload = json!({
            "content": STANDARD.encode(&bytes),
            "encoding": "base64"
        });
        let upload_progress = 10 + ((index + 1) as f32 / total as f32 * 70.0) as u8;
        let blob = github_send_patiently(
            "create release file blob",
            || client.post(&blobs_url).json(&payload),
            &token,
            |wait| {
                emit_progress(
                    &progress_callback,
                    upload_progress,
                    &format!(
                        "GitHub asked CEMM to slow down. Continuing in {wait}s ({}/{} uploaded)...",
                        index,
                        wanted.len()
                    ),
                );
            },
        )
        .await?;
        tree_items.push(json!({
            "path": repository_path,
            "mode": "100644",
            "type": "blob",
            "sha": blob["sha"].as_str().ok_or("Could not get release file blob SHA")?
        }));
        uploaded += 1;
    }

    // ---- The document, and what old clients will read instead of it ---------------
    emit_progress(&progress_callback, 85, "Uploading the release document...");
    let document = serde_json::to_vec_pretty(&release).map_err(|e| e.to_string())?;
    let tombstone = serde_json::to_vec_pretty(&release::legacy_tombstone(&release))
        .map_err(|e| e.to_string())?;
    for (name, bytes) in [
        (RELEASE_FILE_NAME, &document),
        (MANIFEST_FILE_NAME, &tombstone),
    ] {
        let blob = github_send(
            "create release document blob",
            client.post(&blobs_url).json(&json!({
                "content": STANDARD.encode(bytes),
                "encoding": "base64"
            })),
            &token,
        )
        .await?;
        tree_items.push(json!({
            "path": format!("{release_dir}/{name}"),
            "mode": "100644",
            "type": "blob",
            "sha": blob["sha"].as_str().ok_or("Could not get release document blob SHA")?
        }));
    }

    emit_progress(&progress_callback, 90, "Creating commit...");
    let new_tree = github_send(
        "create file tree",
        client
            .post(format!("{api}/repos/{owner}/{repo_name}/git/trees"))
            .json(&json!({ "base_tree": base_tree, "tree": tree_items })),
        &token,
    )
    .await?;
    let new_commit = github_send(
        "create commit",
        client
            .post(format!("{api}/repos/{owner}/{repo_name}/git/commits"))
            .json(&json!({
                "message": format!(
                    "Publish {} release {} ({} new file(s))",
                    release.lineage, release.release_id, uploaded
                ),
                "tree": new_tree["sha"].as_str().ok_or("Could not get new tree SHA")?,
                "parents": [base_commit]
            })),
        &token,
    )
    .await?;
    let revision = new_commit["sha"]
        .as_str()
        .ok_or("Could not get new commit SHA")?
        .to_lowercase();

    emit_progress(&progress_callback, 96, "Finalizing...");
    github_send(
        "update main branch reference",
        client
            .patch(&refs_url)
            .json(&json!({ "sha": revision, "force": false })),
        &token,
    )
    .await
    .map_err(|error| {
        format!("{error}. Nothing was published; the repository is unchanged. Try again.")
    })?;

    emit_progress(&progress_callback, 100, "Release published");
    Ok(PublishReceipt {
        code: format!("{release_dir}@{revision}"),
        repo,
        lineage: release.lineage.clone(),
        release_id: release.release_id.clone(),
        sequence: release.sequence,
        revision,
        release_digest: release::sha256_hex(&document),
        uploaded_objects: uploaded,
        reused_objects: reused,
        published_unix: crate::installer::unix_now(),
        server_release: release.server_release.clone(),
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

    // Cloned out before the loop consumes them: `remote_files` borrows the
    // manifest, and the loop below needs owned paths.
    let wanted: Vec<ConfigFile> = remote_files(&manifest).cloned().collect();
    log::debug!(
        "download_config_files: {} file(s) listed in the manifest ({} config, {} across {} custom data pack(s))",
        wanted.len(),
        manifest.config_files.len(),
        wanted.len() - manifest.config_files.len(),
        manifest.custom_datapacks.len()
    );

    let mut config_files = Vec::with_capacity(wanted.len());
    let mut total_downloaded_bytes = 0usize;
    for config_file in wanted {
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
                    "Failed to download {} (status {})",
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

    log::debug!(
        "download_config_files: {} files downloaded",
        config_files.len()
    );
    Ok(config_files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_file_an_update_ships_is_validated_and_downloaded() {
        // Custom data pack files are attacker-controlled relative paths exactly
        // like config files, and are fetched the same way. Validating only
        // `config_files` would have let a traversing data pack path through the
        // one check standing between a downloaded manifest and disk.
        let manifest = Manifest {
            update_type: Some("full".to_string()),
            mods: Vec::new(),
            resourcepacks: Vec::new(),
            shaderpacks: Vec::new(),
            datapacks: Vec::new(),
            custom_datapacks: vec![crate::composables::manifest::CustomDatapack {
                name: "vanillatweaks".to_string(),
                archived: false,
                files: vec![ConfigFile {
                    filename: "pack.mcmeta".to_string(),
                    relative_path: "datapacks/vanillatweaks/pack.mcmeta".to_string(),
                }],
            }],
            config_files: vec![ConfigFile {
                filename: "server.properties".to_string(),
                relative_path: "config/server.properties".to_string(),
            }],
        };

        let paths: Vec<&str> = remote_files(&manifest)
            .map(|file| file.relative_path.as_str())
            .collect();

        assert_eq!(
            paths,
            vec![
                "config/server.properties",
                "datapacks/vanillatweaks/pack.mcmeta"
            ]
        );
        assert!(validate_remote_config_manifest(&manifest).is_ok());
    }

    #[test]
    fn a_traversing_custom_data_pack_path_is_refused() {
        let manifest = Manifest {
            update_type: Some("full".to_string()),
            mods: Vec::new(),
            resourcepacks: Vec::new(),
            shaderpacks: Vec::new(),
            datapacks: Vec::new(),
            custom_datapacks: vec![crate::composables::manifest::CustomDatapack {
                name: "evil".to_string(),
                archived: true,
                files: vec![ConfigFile {
                    filename: "evil.zip".to_string(),
                    relative_path: "../../../evil.zip".to_string(),
                }],
            }],
            config_files: Vec::new(),
        };

        assert!(validate_remote_config_manifest(&manifest).is_err());
    }

    #[test]
    fn only_a_rate_limit_is_waited_on_and_for_as_long_as_github_says() {
        use reqwest::header::{HeaderMap, HeaderValue};
        use reqwest::StatusCode;
        let headers = |pairs: &[(&'static str, &'static str)]| {
            let mut map = HeaderMap::new();
            for (name, value) in pairs {
                map.insert(*name, HeaderValue::from_static(value));
            }
            map
        };
        let none = HeaderMap::new();

        // A secondary limit, with and without a stated wait.
        assert_eq!(
            rate_limit_wait(
                StatusCode::TOO_MANY_REQUESTS,
                &headers(&[("retry-after", "42")]),
                "",
                0,
                0
            ),
            Some(42)
        );
        assert_eq!(
            rate_limit_wait(StatusCode::TOO_MANY_REQUESTS, &none, "", 0, 0),
            Some(10)
        );
        assert_eq!(
            rate_limit_wait(StatusCode::TOO_MANY_REQUESTS, &none, "", 2, 0),
            Some(40)
        );
        assert_eq!(
            rate_limit_wait(
                StatusCode::FORBIDDEN,
                &none,
                r#"{"message":"You have exceeded a secondary rate limit."}"#,
                1,
                0
            ),
            Some(20)
        );
        // The primary limit says when it resets.
        assert_eq!(
            rate_limit_wait(
                StatusCode::FORBIDDEN,
                &headers(&[
                    ("x-ratelimit-remaining", "0"),
                    ("x-ratelimit-reset", "1060")
                ]),
                "",
                0,
                1000
            ),
            Some(61)
        );
        // A long wait is taken in bounded steps, so progress keeps reporting.
        assert_eq!(
            rate_limit_wait(
                StatusCode::TOO_MANY_REQUESTS,
                &headers(&[("retry-after", "3600")]),
                "",
                0,
                0
            ),
            Some(MAX_RATE_LIMIT_WAIT_SECS)
        );

        // Real failures are not waited on: a bad token, a missing repository,
        // a plain permission error.
        assert_eq!(
            rate_limit_wait(
                StatusCode::UNAUTHORIZED,
                &none,
                r#"{"message":"Bad credentials"}"#,
                0,
                0
            ),
            None
        );
        assert_eq!(
            rate_limit_wait(StatusCode::NOT_FOUND, &none, "", 0, 0),
            None
        );
        assert_eq!(
            rate_limit_wait(
                StatusCode::FORBIDDEN,
                &none,
                r#"{"message":"Resource not accessible by personal access token"}"#,
                0,
                0
            ),
            None
        );
    }

    /// Read-only questions put to the real GitHub, about a long-lived public
    /// repository that is nobody's in particular. Ignored by default: CI should
    /// not depend on the network, and unauthenticated calls are rationed.
    ///
    /// Everything the release transport reads rests on four behaviours that
    /// the stand-in storage in the roundtrip test merely imitates. This asks
    /// the real service whether it behaves that way.
    #[tokio::test]
    #[ignore = "talks to api.github.com and raw.githubusercontent.com"]
    async fn the_real_github_answers_the_way_the_release_transport_assumes() {
        let endpoints = GithubEndpoints {
            api: "https://api.github.com".to_string(),
            raw: "https://raw.githubusercontent.com".to_string(),
        };
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("client");
        let (owner, repo) = ("github", "gitignore");
        let get = |url: String| {
            let client = client.clone();
            async move {
                client
                    .get(&url)
                    .header("User-Agent", "cemm-app-tauri")
                    .send()
                    .await
                    .unwrap_or_else(|error| panic!("GET {url}: {error}"))
            }
        };

        // 1. A branch name resolves to one full commit id, as plain text.
        let head = resolve_branch_head(&client, &endpoints, owner, repo)
            .await
            .expect("the branch head should resolve without signing in");
        assert!(is_commit_sha(&head), "got: {head}");

        // 2. A file is readable at that commit, and is the same file the
        //    branch serves: a commit is a fixed view of the tree.
        let at_commit = get(endpoints
            .raw_file_url(owner, repo, &head, "", "README.md")
            .unwrap())
        .await;
        assert!(at_commit.status().is_success(), "{}", at_commit.status());
        let pinned = at_commit.bytes().await.unwrap();
        assert!(!pinned.is_empty());

        // 3. A path that is not there is a 404, at a commit as on a branch.
        //    That is how "no release at this code" is told apart from an error.
        let missing = get(endpoints
            .raw_file_url(
                owner,
                repo,
                &head,
                "no-such-pack/no-such-release",
                RELEASE_FILE_NAME,
            )
            .unwrap())
        .await;
        assert_eq!(missing.status(), reqwest::StatusCode::NOT_FOUND);

        // 4. The whole tree of a commit can be listed in one call, with paths
        //    and a flag saying whether the listing is complete.
        let commit: serde_json::Value = get(format!(
            "{}/repos/{owner}/{repo}/git/commits/{head}",
            endpoints.api
        ))
        .await
        .json()
        .await
        .expect("commit json");
        let tree_id = commit["tree"]["sha"].as_str().expect("tree id");
        let tree: serde_json::Value = get(format!(
            "{}/repos/{owner}/{repo}/git/trees/{tree_id}?recursive=1",
            endpoints.api
        ))
        .await
        .json()
        .await
        .expect("tree json");
        assert!(tree["truncated"].is_boolean(), "got: {}", tree["truncated"]);
        let paths: Vec<&str> = tree["tree"]
            .as_array()
            .expect("tree entries")
            .iter()
            .filter_map(|entry| entry["path"].as_str())
            .collect();
        assert!(paths.contains(&"README.md"));
        assert!(
            paths.iter().any(|path| path.contains('/')),
            "a recursive listing names files below the top level"
        );

        // 5. The contents API answers 200 for a folder that exists and 404 for
        //    one that does not: the check that a release id is still free.
        let existing = get(format!(
            "{}/repos/{owner}/{repo}/contents/Global?ref={PUBLISH_BRANCH}",
            endpoints.api
        ))
        .await;
        assert!(existing.status().is_success(), "{}", existing.status());
        let absent = get(format!(
            "{}/repos/{owner}/{repo}/contents/no-such-pack/no-such-release?ref={PUBLISH_BRANCH}",
            endpoints.api
        ))
        .await;
        assert_eq!(absent.status(), reqwest::StatusCode::NOT_FOUND);

        println!(
            "head {head}; {} bytes at that commit; {} tree entries, truncated={}",
            pinned.len(),
            paths.len(),
            tree["truncated"]
        );
    }

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
            custom_datapacks: Vec::new(),
            config_files: vec![config_file; MAX_REMOTE_CONFIG_FILES + 1],
        };

        assert!(validate_remote_config_manifest(&manifest).is_err());
    }
}
