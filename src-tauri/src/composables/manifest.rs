use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::command;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Addon {
    pub addon_file_id: u64,
    pub addon_name: String,
    pub addon_project_id: u64,
    pub cdn_download_url: String,
    pub mod_folder_path: String,
    pub version: String,
    #[serde(rename = "webSiteURL")]
    pub web_site_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(rename = "fileNameOnDisk")]
    pub file_name_on_disk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigFile {
    pub filename: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    #[serde(rename = "updateType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_type: Option<String>, // "full" or "config"
    pub mods: Vec<Addon>,
    pub resourcepacks: Vec<Addon>,
    pub shaderpacks: Vec<Addon>,
    pub datapacks: Vec<Addon>,
    pub config_files: Vec<ConfigFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateInfo {
    pub uuid: String,
    pub timestamp: String,
    pub added_addons: Vec<Addon>,
    pub removed_addons: Vec<String>,
    pub config_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MinecraftInstance {
    #[serde(rename = "installedAddons")]
    installed_addons: Vec<InstalledAddon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstalledAddon {
    #[serde(rename = "addonID")]
    addon_id: Option<u64>,
    #[serde(rename = "name")]
    name: Option<String>,
    #[serde(rename = "modFolderPath")]
    mod_folder_path: Option<String>,
    #[serde(rename = "installedFile")]
    installed_file: Option<InstalledFile>,
    #[serde(rename = "categorySection")]
    category_section: Option<CategorySection>,
    #[serde(rename = "webSiteURL")]
    web_site_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstalledFile {
    id: Option<u64>,
    #[serde(rename = "fileName")]
    file_name: Option<String>,
    #[serde(rename = "downloadUrl")]
    download_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CategorySection {
    name: Option<String>,
}

#[command]
pub fn parse_minecraft_instance(path: String) -> Result<Manifest, String> {
    log::info!("parse_minecraft_instance: reading {path}");
    let content = fs::read_to_string(&path).map_err(|e| {
        log::error!("parse_minecraft_instance: failed to read {path}: {e}");
        e.to_string()
    })?;
    let instance: MinecraftInstance = serde_json::from_str(&content).map_err(|e| {
        log::error!("parse_minecraft_instance: failed to parse JSON: {e}");
        e.to_string()
    })?;
    // Scan for .disabled files in relevant folders
    let base_dir = Path::new(&path).parent().unwrap_or_else(|| Path::new("."));
    let disabled_mods = find_disabled_files(base_dir.join("mods"));
    let disabled_resourcepacks = find_disabled_files(base_dir.join("resourcepacks"));
    let disabled_shaderpacks = find_disabled_files(base_dir.join("shaderpacks"));
    let disabled_datapacks = find_disabled_files(base_dir.join("datapacks"));
    let mut mods = Vec::new();
    let mut resourcepacks = Vec::new();
    let mut shaderpacks = Vec::new();
    let mut datapacks = Vec::new();
    for addon in instance.installed_addons {
        let Some(installed_file) = &addon.installed_file else {
            continue;
        };
        let Some(addon_name) = &addon.name else {
            continue;
        };
        let Some(mod_folder_path) = &addon.mod_folder_path else {
            continue;
        };
        let Some(category_section) = &addon.category_section else {
            continue;
        };
        let Some(category_name) = &category_section.name else {
            continue;
        };
        let Some(addon_file_id) = installed_file.id else {
            continue;
        };
        let Some(addon_project_id) = addon.addon_id else {
            continue;
        };
        let Some(version) = &installed_file.file_name else {
            continue;
        };
        let Some(cdn_download_url) = &installed_file.download_url else {
            continue;
        };
        // Determine if this addon is disabled by checking for .disabled file
        let mut disabled = None;
        let cat = category_name.to_lowercase();
        let folder = mod_folder_path.to_lowercase();
        let file_name = version;
        if cat.contains("shader") || folder.ends_with("shaderpacks") {
            if disabled_shaderpacks.contains(file_name) {
                disabled = Some(true);
            }
        } else if cat.contains("resource") || folder.ends_with("resourcepacks") {
            if disabled_resourcepacks.contains(file_name) {
                disabled = Some(true);
            }
        } else if cat.contains("datapack") || folder.ends_with("datapacks") {
            if disabled_datapacks.contains(file_name) {
                disabled = Some(true);
            }
        } else {
            if disabled_mods.contains(file_name) {
                disabled = Some(true);
            }
        }
        let addon_struct = Addon {
            addon_file_id,
            addon_name: addon_name.clone(),
            addon_project_id,
            cdn_download_url: cdn_download_url.clone(),
            mod_folder_path: mod_folder_path.clone(),
            version: version.clone(),
            web_site_url: addon.web_site_url.clone(),
            disabled,
            file_name_on_disk: version.clone(), // Use the version field which contains the filename
        };
        if cat.contains("shader") || folder.ends_with("shaderpacks") {
            shaderpacks.push(addon_struct);
        } else if cat.contains("resource") || folder.ends_with("resourcepacks") {
            resourcepacks.push(addon_struct);
        } else if cat.contains("datapack") || folder.ends_with("datapacks") {
            datapacks.push(addon_struct);
        } else {
            mods.push(addon_struct);
        }
    }
    Ok(Manifest {
        update_type: None, // Default to None for MinecraftInstance conversion
        mods,
        resourcepacks,
        shaderpacks,
        datapacks,
        config_files: Vec::new(), // Empty for MinecraftInstance conversion
    })
}

fn find_disabled_files(dir: PathBuf) -> Vec<String> {
    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "disabled" {
                    // Get the file stem (e.g., modname.jar from modname.jar.disabled)
                    if let Some(file_stem) = path.file_stem() {
                        // file_stem is OsStr, convert to &str
                        if let Some(stem_str) = file_stem.to_str() {
                            result.push(stem_str.to_string());
                        }
                    }
                }
            }
        }
    }
    result
}

#[command]
pub fn compare_manifests(old: Manifest, new: Manifest) -> Result<UpdateInfo, String> {
    log::info!("compare_manifests: comparing manifests");
    let old_ids: std::collections::HashSet<_> = old.mods.iter().map(|a| &a.addon_name).collect();
    let new_ids: std::collections::HashSet<_> = new.mods.iter().map(|a| &a.addon_name).collect();
    let added: Vec<Addon> = new
        .mods
        .iter()
        .filter(|a| !old_ids.contains(&a.addon_name) && a.disabled != Some(true))
        .cloned()
        .collect();
    // Build a set of disabled addon names in the new manifest
    let disabled_in_new: std::collections::HashSet<_> = new
        .mods
        .iter()
        .filter(|a| a.disabled == Some(true))
        .map(|a| &a.addon_name)
        .collect();
    let removed: Vec<String> = old
        .mods
        .iter()
        .filter(|a| {
            !new_ids.contains(&a.addon_name) || disabled_in_new.contains(&a.addon_name)
        })
        .map(|a| a.addon_name.clone())
        .collect();
    let update_info = UpdateInfo {
        uuid: Uuid::new_v4().to_string(),
        timestamp: Utc::now().to_rfc3339(),
        added_addons: added,
        removed_addons: removed,
        config_files: vec![], // Placeholder, fill as needed
    };
    log::info!("compare_manifests: update info generated");
    Ok(update_info)
}

fn slugify_curseforge_name(name: &str) -> String {
    // Lowercase, replace spaces/underscores with dashes, preserve brackets, remove other non-url-safe chars
    let mut slug = name.to_lowercase();
    // Replace underscores and whitespace with dash
    slug = slug.replace([' ', '_'], "-");
    // Remove all characters except alphanumeric, dash, and brackets
    slug = slug
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '[' || *c == ']')
        .collect();
    // Remove multiple dashes
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    // Remove leading/trailing dashes
    slug.trim_matches('-').to_string()
}

#[tauri::command]
pub fn open_curseforge_url(addon_name: String) -> Result<(), String> {
    let slug = slugify_curseforge_name(&addon_name);
    let url = format!("https://www.curseforge.com/minecraft/mc-mods/{}", slug);
    opener::open(url).map_err(|e| format!("Failed to open browser: {e}"))
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    opener::open(url).map_err(|e| format!("Failed to open browser: {e}"))
}
