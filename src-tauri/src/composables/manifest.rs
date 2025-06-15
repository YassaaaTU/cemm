use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigFile {
    pub filename: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
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
        let addon_struct = Addon {
            addon_file_id,
            addon_name: addon_name.clone(),
            addon_project_id,
            cdn_download_url: cdn_download_url.clone(),
            mod_folder_path: mod_folder_path.clone(),
            version: version.clone(),
            web_site_url: addon.web_site_url.clone(),
        };
        let cat = category_name.to_lowercase();
        let folder = mod_folder_path.to_lowercase();
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
        mods,
        resourcepacks,
        shaderpacks,
        datapacks,
        config_files: Vec::new(), // Empty for MinecraftInstance conversion
    })
}

#[command]
pub fn compare_manifests(old: Manifest, new: Manifest) -> Result<UpdateInfo, String> {
    log::info!("compare_manifests: comparing manifests");
    let old_ids: std::collections::HashSet<_> = old.mods.iter().map(|a| &a.addon_name).collect();
    let new_ids: std::collections::HashSet<_> = new.mods.iter().map(|a| &a.addon_name).collect();
    let added: Vec<Addon> = new
        .mods
        .iter()
        .filter(|a| !old_ids.contains(&a.addon_name))
        .cloned()
        .collect();
    let removed: Vec<String> = old
        .mods
        .iter()
        .filter(|a| !new_ids.contains(&a.addon_name))
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
