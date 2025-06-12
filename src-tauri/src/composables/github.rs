use anyhow::Result;
use serde::{Deserialize, Serialize};
use tauri::command;

use crate::composables::manifest::Manifest;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadResult {
    pub manifest: Manifest,
    pub config_files: Vec<ConfigFile>,
}

#[command]
pub async fn upload_update(
    repo: String,
    token: String,
    uuid: String,
    manifest: Manifest,
    config_files: Vec<ConfigFile>,
) -> Result<(), String> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use reqwest::Client;
    use serde_json::json;

    // Parse repo as "owner/repo"
    let mut parts = repo.splitn(2, '/');
    let owner = parts.next().ok_or("Invalid repo format")?;
    let repo_name = parts.next().ok_or("Invalid repo format")?;
    let api_base = format!("https://api.github.com/repos/{owner}/{repo_name}/contents/{uuid}");
    let client = Client::new();
    let user_agent = "cemm-app-tauri";

    // Upload manifest.json
    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    let manifest_url = format!("{}/manifest.json", api_base);
    let manifest_req = client
        .put(&manifest_url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", user_agent)
        .json(&json!({
            "message": format!("Upload manifest for update {}", uuid),
            "content": STANDARD.encode(manifest_json),
            "branch": "main"
        }));
    let manifest_res = manifest_req.send().await.map_err(|e| e.to_string())?;
    if !manifest_res.status().is_success() {
        return Err(format!(
            "Failed to upload manifest: {}",
            manifest_res.text().await.unwrap_or_default()
        ));
    }

    // Upload config files
    for file in config_files {
        let file_url = format!("{}/{}", api_base, file.path);
        let file_req = client
            .put(&file_url)
            .header("Authorization", format!("token {}", token))
            .header("User-Agent", user_agent)
            .json(&json!({
                "message": format!("Upload config file {} for update {}", file.path, uuid),
                "content": STANDARD.encode(file.content),
                "branch": "main"
            }));
        let file_res = file_req.send().await.map_err(|e| e.to_string())?;
        if !file_res.status().is_success() {
            return Err(format!(
                "Failed to upload config file {}: {}",
                file.path,
                file_res.text().await.unwrap_or_default()
            ));
        }
    }
    Ok(())
}

#[command]
pub async fn download_update(
    repo: String,
    uuid: String,
) -> Result<DownloadResult, String> {
    use reqwest::Client;
    use serde_json::Value;
    let mut parts = repo.splitn(2, '/');
    let owner = parts.next().ok_or("Invalid repo format")?;
    let repo_name = parts.next().ok_or("Invalid repo format")?;
    let api_base = format!("https://api.github.com/repos/{owner}/{repo_name}/contents/{uuid}");
    let client = Client::new();
    let user_agent = "cemm-app-tauri";

    // List files in the uuid folder
    let list_url = &api_base;
    let list_res = client
        .get(list_url)
        .header("User-Agent", user_agent)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !list_res.status().is_success() {
        return Err(format!(
            "Failed to list update files: {}",
            list_res.text().await.unwrap_or_default()
        ));
    }
    let files: Vec<Value> = list_res.json().await.map_err(|e| e.to_string())?;

    // Download manifest.json
    let manifest_file = files
        .iter()
        .find(|f| f["name"] == "manifest.json")
        .ok_or("manifest.json not found")?;
    let manifest_url = manifest_file["download_url"]
        .as_str()
        .ok_or("No download_url for manifest.json")?;
    let manifest_res = client
        .get(manifest_url)
        .header("User-Agent", user_agent)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let manifest_json = manifest_res.text().await.map_err(|e| e.to_string())?;
    let manifest: Manifest = serde_json::from_str(&manifest_json).map_err(|e| e.to_string())?;

    // Download config files
    let mut config_files = Vec::new();
    for file in files {
        let name = file["name"].as_str().unwrap_or("");
        if name == "manifest.json" {
            continue;
        }
        let download_url = file["download_url"]
            .as_str()
            .ok_or("No download_url for config file")?;
        let file_res = client
            .get(download_url)
            .header("User-Agent", user_agent)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let content = file_res.text().await.map_err(|e| e.to_string())?;
        config_files.push(ConfigFile {
            path: name.to_string(),
            content,
        });
    }
    Ok(DownloadResult {
        manifest,
        config_files: config_files,
    })
}
