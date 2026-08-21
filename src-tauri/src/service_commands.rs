use serde_json::json;
use tauri::State;

use crate::composables::github::ConfigFileWithContent;
use crate::composables::instances::{CachedIcon, PackLibrary};
use crate::composables::manifest::{Manifest, UpdateInfo};
use crate::installer::{ConfigFile as InstallerConfigFile, InstallOptions, UpdateDiff};
use crate::service::protocol::Method;
use crate::service::ServiceClient;

#[tauri::command]
pub async fn read_file(service: State<'_, ServiceClient>, path: String) -> Result<String, String> {
    service
        .call_typed(Method::FileRead, json!({ "path": path }))
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
            Method::FileWrite,
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
            Method::ConfigReadDirectory,
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
        .call_typed(Method::PathIsBinary, json!({ "path": path }))
        .await
}

#[tauri::command]
pub async fn validate_path(
    service: State<'_, ServiceClient>,
    path: String,
) -> Result<serde_json::Value, String> {
    service
        .call_typed(Method::PathValidate, json!({ "path": path }))
        .await
}

#[tauri::command]
pub async fn parse_minecraft_instance(
    service: State<'_, ServiceClient>,
    path: String,
) -> Result<Manifest, String> {
    service
        .call_typed(Method::ManifestParseInstance, json!({ "path": path }))
        .await
}

/// The diff behind the update preview. Same function the installer uses to
/// decide what to delete, so the dialog cannot understate the change.
#[tauri::command]
pub async fn get_update_diff(
    service: State<'_, ServiceClient>,
    old: Option<Manifest>,
    new: Manifest,
) -> Result<UpdateDiff, String> {
    service
        .call_typed(Method::ManifestDiff, json!({ "old": old, "new": new }))
        .await
}

#[tauri::command]
pub async fn compare_manifests(
    service: State<'_, ServiceClient>,
    old: Manifest,
    new: Manifest,
) -> Result<UpdateInfo, String> {
    service
        .call_typed(Method::ManifestCompare, json!({ "old": old, "new": new }))
        .await
}

#[tauri::command]
// Tauri maps these named arguments directly from the established frontend
// payload. Wrapping them would be a breaking IPC contract change for no domain
// benefit; this function remains a zero-logic compatibility adapter.
#[allow(clippy::too_many_arguments)]
pub async fn upload_update(
    service: State<'_, ServiceClient>,
    operation_id: String,
    repo: String,
    token: String,
    uuid: String,
    modpack_key: Option<String>,
    manifest: Manifest,
    config_files: Vec<ConfigFileWithContent>,
) -> Result<(), String> {
    service
        .call_typed(
            Method::GithubUploadUpdate,
            json!({
                "operationId": operation_id,
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
            Method::GithubDownloadManifest,
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
            Method::GithubDownloadConfigFiles,
            json!({
                "repo": repo,
                "uuid": uuid,
                "modpackKey": modpack_key,
                "manifest": manifest
            }),
        )
        .await
}

#[tauri::command]
pub async fn scan_pack_library(
    service: State<'_, ServiceClient>,
    instances_dir: Option<String>,
) -> Result<PackLibrary, String> {
    service
        .call_typed(
            Method::LibraryScan,
            json!({ "instancesDir": instances_dir }),
        )
        .await
}

#[tauri::command]
pub async fn cache_pack_icons(
    service: State<'_, ServiceClient>,
    urls: Vec<String>,
) -> Result<Vec<CachedIcon>, String> {
    service
        .call_typed(Method::LibraryCacheIcons, json!({ "urls": urls }))
        .await
}

#[tauri::command]
pub async fn install_update(
    service: State<'_, ServiceClient>,
    operation_id: String,
    modpack_path: String,
    manifest: Manifest,
    config_files: Vec<InstallerConfigFile>,
    options: Option<InstallOptions>,
) -> Result<(), String> {
    service
        .call_typed(
            Method::InstallApplyUpdate,
            json!({
                "operationId": operation_id,
                "modpackPath": modpack_path,
                "manifest": manifest,
                "configFiles": config_files,
                "options": options
            }),
        )
        .await
}
