use anyhow::Result;
use serde::{Deserialize, Serialize};
use tauri::command;

use crate::composables::manifest::Manifest;

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

#[command]
pub async fn upload_update(
    repo: String,
    token: String,
    uuid: String,
    manifest: Manifest,
    config_files: Vec<ConfigFileWithContent>,
) -> Result<(), String> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use reqwest::Client;
    use serde_json::json;

    // Parse repo as "owner/repo"
    let mut parts = repo.splitn(2, '/');
    let owner = parts.next().ok_or("Invalid repo format")?;
    let repo_name = parts.next().ok_or("Invalid repo format")?;
    let client = Client::new();
    let user_agent = "cemm-app-tauri";

    // Step 1: Get the current commit SHA of main branch
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

    // Create blobs for config files
    let mut config_blob_shas = Vec::new();
    for file in &config_files {
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

    // Step 4: Create a new tree with all files
    // Note: This will automatically overwrite any existing files at the same paths
    // because Git tree creation replaces the entire directory structure
    let mut tree_items = vec![
        json!({
            "path": format!("{}/manifest.json", uuid),
            "mode": "100644",
            "type": "blob",
            "sha": manifest_blob_sha
        })
    ];

    // Add config files to tree (will overwrite existing config files if same UUID)
    for (i, file) in config_files.iter().enumerate() {
        tree_items.push(json!({
            "path": format!("{}/{}", uuid, file.relative_path),
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

    Ok(())
}

#[command]
pub async fn download_manifest(
    repo: String,
    uuid: String,
) -> Result<Manifest, String> {
    use reqwest::Client;
    use serde_json::Value;
    
    // Debug logging
    eprintln!("download_manifest called with repo: '{}', uuid: '{}'", repo, uuid);
    
    let mut parts = repo.splitn(2, '/');
    let owner = parts.next().ok_or("Invalid repo format")?;
    let repo_name = parts.next().ok_or("Invalid repo format")?;
    let api_base = format!("https://api.github.com/repos/{owner}/{repo_name}/contents/{uuid}");
    let client = Client::new();
    let user_agent = "cemm-app-tauri";

    eprintln!("API URL: {}", api_base);

    // List files in the uuid folder first
    let list_url = &api_base;
    let list_res = client
        .get(list_url)
        .header("User-Agent", user_agent)
        .send()
        .await
        .map_err(|e| {
            eprintln!("Request error: {}", e);
            e.to_string()
        })?;
    
    eprintln!("GitHub API response status: {}", list_res.status());
    
    let status = list_res.status();
    if !status.is_success() {
        let error_text = list_res.text().await.unwrap_or_default();
        eprintln!("GitHub API error response: {}", error_text);
        return Err(format!(
            "Failed to list update files (status {}): {}",
            status,
            error_text
        ));
    }
    let files: Vec<Value> = list_res.json().await.map_err(|e| {
        eprintln!("JSON parsing error: {}", e);
        e.to_string()
    })?;
    
    eprintln!("Found {} files in directory", files.len());
    for file in &files {
        if let Some(name) = file["name"].as_str() {
            eprintln!("File: {}", name);
        }
    }

    // Find manifest.json
    let manifest_file = files
        .iter()
        .find(|f| f["name"] == "manifest.json")
        .ok_or_else(|| {
            eprintln!("manifest.json not found in directory listing");
            "manifest.json not found".to_string()
        })?;
    let manifest_url = manifest_file["download_url"]
        .as_str()
        .ok_or_else(|| {
            eprintln!("No download_url found for manifest.json");
            "No download_url for manifest.json".to_string()
        })?;
    
    eprintln!("Downloading manifest from: {}", manifest_url);
    
    // Download manifest.json directly
    let manifest_res = client
        .get(manifest_url)
        .header("User-Agent", user_agent)
        .send()
        .await
        .map_err(|e| {
            eprintln!("Manifest download error: {}", e);
            e.to_string()
        })?;
    
    let manifest_json = manifest_res.text().await.map_err(|e| {
        eprintln!("Failed to read manifest response text: {}", e);
        e.to_string()
    })?;
    
    eprintln!("Downloaded manifest JSON (first 200 chars): {}", 
              &manifest_json.chars().take(200).collect::<String>());
    
    let manifest: Manifest = serde_json::from_str(&manifest_json).map_err(|e| {
        eprintln!("Failed to parse manifest JSON: {}", e);
        e.to_string()
    })?;
    
    eprintln!("Successfully parsed manifest");
    Ok(manifest)
}

#[command]
pub async fn download_config_files(
    repo: String,
    uuid: String,
    manifest: Manifest,
) -> Result<Vec<ConfigFileWithContent>, String> {
    use reqwest::Client;
    
    let mut parts = repo.splitn(2, '/');
    let owner = parts.next().ok_or("Invalid repo format")?;
    let repo_name = parts.next().ok_or("Invalid repo format")?;
    let client = Client::new();
    let user_agent = "cemm-app-tauri";

    eprintln!("Downloading {} config files from manifest", manifest.config_files.len());

    // Download config files based on manifest list
    let mut config_files = Vec::new();
    for config_file in manifest.config_files {
        let file_url = format!("https://api.github.com/repos/{owner}/{repo_name}/contents/{uuid}/{}", config_file.relative_path);
        eprintln!("Downloading config file from: {}", file_url);
        
        let file_res = client
            .get(&file_url)
            .header("User-Agent", user_agent)
            .send()
            .await
            .map_err(|e| e.to_string())?;
            
        if !file_res.status().is_success() {
            return Err(format!(
                "Failed to download config file {}: {}",
                config_file.relative_path,
                file_res.text().await.unwrap_or_default()
            ));
        }
        
        let file_data: serde_json::Value = file_res.json().await.map_err(|e| e.to_string())?;
        let download_url = file_data["download_url"]
            .as_str()
            .ok_or("No download_url for config file")?;
        
        // Download the actual file content
        let content_res = client
            .get(download_url)
            .header("User-Agent", user_agent)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let content = content_res.text().await.map_err(|e| e.to_string())?;
        
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
) -> Result<DownloadResult, String> {
    let manifest = download_manifest(repo.clone(), uuid.clone()).await?;
    let config_files = download_config_files(repo, uuid, manifest.clone()).await?;
    
    Ok(DownloadResult {
        manifest,
        config_files,
    })
}
