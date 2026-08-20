use serde_json::json;
use tauri::State;

use crate::composables::github::ConfigFileWithContent;
use crate::composables::manifest::{Manifest, UpdateInfo};
use crate::service::ServiceClient;

#[tauri::command]
pub async fn read_file(service: State<'_, ServiceClient>, path: String) -> Result<String, String> {
    service
        .call_typed("file.read", json!({ "path": path }))
        .await
}

#[tauri::command]
pub async fn write_file(
    service: State<'_, ServiceClient>,
    path: Option<String>,
    content: Option<String>,
    dir: Option<String>,
    files: Option<Vec<(String, String)>>,
) -> Result<(), String> {
    service
        .call_typed(
            "file.write",
            json!({
                "path": path,
                "content": content,
                "dir": dir,
                "files": files
            }),
        )
        .await
}

#[tauri::command]
pub async fn read_directory_recursive(
    service: State<'_, ServiceClient>,
    dir_path: String,
    base_path: String,
) -> Result<Vec<ConfigFileWithContent>, String> {
    service
        .call_typed(
            "config.read_directory",
            json!({ "dirPath": dir_path, "basePath": base_path }),
        )
        .await
}

#[tauri::command]
pub async fn is_binary_file(
    service: State<'_, ServiceClient>,
    path: String,
) -> Result<bool, String> {
    service
        .call_typed("path.is_binary", json!({ "path": path }))
        .await
}

#[tauri::command]
pub async fn validate_path(
    service: State<'_, ServiceClient>,
    path: String,
) -> Result<serde_json::Value, String> {
    service
        .call_typed("path.validate", json!({ "path": path }))
        .await
}

#[tauri::command]
pub async fn parse_minecraft_instance(
    service: State<'_, ServiceClient>,
    path: String,
) -> Result<Manifest, String> {
    service
        .call_typed("manifest.parse_instance", json!({ "path": path }))
        .await
}

#[tauri::command]
pub async fn compare_manifests(
    service: State<'_, ServiceClient>,
    old: Manifest,
    new: Manifest,
) -> Result<UpdateInfo, String> {
    service
        .call_typed("manifest.compare", json!({ "old": old, "new": new }))
        .await
}

#[tauri::command]
pub async fn upload_update(
    service: State<'_, ServiceClient>,
    repo: String,
    token: String,
    uuid: String,
    modpack_key: Option<String>,
    manifest: Manifest,
    config_files: Vec<ConfigFileWithContent>,
) -> Result<(), String> {
    service
        .call_typed(
            "github.upload_update",
            json!({
                "repo": repo,
                "token": token,
                "uuid": uuid,
                "modpackKey": modpack_key,
                "manifest": manifest,
                "configFiles": config_files
            }),
        )
        .await
}

#[tauri::command]
pub async fn download_manifest(
    service: State<'_, ServiceClient>,
    repo: String,
    uuid: String,
    modpack_key: Option<String>,
) -> Result<Manifest, String> {
    service
        .call_typed(
            "github.download_manifest",
            json!({ "repo": repo, "uuid": uuid, "modpackKey": modpack_key }),
        )
        .await
}

#[tauri::command]
pub async fn download_config_files(
    service: State<'_, ServiceClient>,
    repo: String,
    uuid: String,
    modpack_key: Option<String>,
    manifest: Manifest,
) -> Result<Vec<ConfigFileWithContent>, String> {
    service
        .call_typed(
            "github.download_config_files",
            json!({
                "repo": repo,
                "uuid": uuid,
                "modpackKey": modpack_key,
                "manifest": manifest
            }),
        )
        .await
}
