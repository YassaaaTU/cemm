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

// Not persisted or published — computed fresh per compare_manifests call and
// consumed only by the admin preview, so renaming its serde output is safe.
// (Unlike `Manifest`/`Addon`, which are published and must never gain rename_all.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub uuid: String,
    pub timestamp: String,
    pub added_addons: Vec<Addon>,
    pub removed_addons: Vec<String>,
    /// Project IDs of addons present in both manifests with a changed version.
    /// Matches `installer::UpdateDiff::updated_addon_ids`, which is what the
    /// installer actually acts on — keeping this the same shape means the
    /// admin preview and the installer agree on what "updated" means.
    pub updated_addon_ids: Vec<u64>,
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

    // Helper function to process a single addon category. Identity is matched on
    // addon_project_id, the same key installer::calculate_update_diff uses — a
    // renamed addon (same project, different addon_name) is therefore reported as
    // "updated" here too, rather than as "removed + added" (F-P2-12). Matching by
    // name previously meant this preview and the installer's own diff could
    // disagree on the same manifest pair.
    fn process_category(
        old_addons: &[Addon],
        new_addons: &[Addon],
        added: &mut Vec<Addon>,
        removed: &mut Vec<String>,
        updated: &mut Vec<u64>,
    ) {
        // Find added addons (no matching project ID in old, and not disabled)
        for new_addon in new_addons {
            let exists_in_old = old_addons
                .iter()
                .any(|a| a.addon_project_id == new_addon.addon_project_id);
            if !exists_in_old && new_addon.disabled != Some(true) {
                added.push(new_addon.clone());
            }
        }

        // Find removed addons (no matching project ID in new, or disabled in new).
        // Skip if old addon was already disabled - can't "remove" something that wasn't active.
        for old_addon in old_addons {
            if old_addon.disabled.unwrap_or(false) {
                continue;
            }
            match new_addons
                .iter()
                .find(|a| a.addon_project_id == old_addon.addon_project_id)
            {
                None => removed.push(old_addon.addon_name.clone()),
                Some(new_addon) if new_addon.disabled == Some(true) => {
                    removed.push(old_addon.addon_name.clone())
                }
                Some(_) => {}
            }
        }

        // Find updated addons (same project ID present in both, version changed)
        for old_addon in old_addons {
            if old_addon.disabled.unwrap_or(false) {
                continue;
            }
            if let Some(new_addon) = new_addons
                .iter()
                .find(|a| a.addon_project_id == old_addon.addon_project_id)
            {
                if new_addon.disabled != Some(true) && old_addon.version != new_addon.version {
                    updated.push(old_addon.addon_project_id);
                }
            }
        }
    }

    let mut added: Vec<Addon> = Vec::new();
    let mut removed: Vec<String> = Vec::new();
    let mut updated: Vec<u64> = Vec::new();

    // Process all addon categories
    process_category(&old.mods, &new.mods, &mut added, &mut removed, &mut updated);
    process_category(
        &old.resourcepacks,
        &new.resourcepacks,
        &mut added,
        &mut removed,
        &mut updated,
    );
    process_category(
        &old.shaderpacks,
        &new.shaderpacks,
        &mut added,
        &mut removed,
        &mut updated,
    );
    process_category(
        &old.datapacks,
        &new.datapacks,
        &mut added,
        &mut removed,
        &mut updated,
    );

    log::info!(
        "compare_manifests: {} added, {} removed, {} updated",
        added.len(),
        removed.len(),
        updated.len()
    );

    let update_info = UpdateInfo {
        uuid: Uuid::new_v4().to_string(),
        timestamp: Utc::now().to_rfc3339(),
        added_addons: added,
        removed_addons: removed,
        updated_addon_ids: updated,
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

/// Hosts a manifest-supplied `webSiteURL` is legitimately allowed to point at.
/// CurseForge only ever issues `https://www.curseforge.com/...` (and its bare
/// `curseforge.com` form) for this field — anything else is either a mistake or
/// an attacker-controlled manifest attempting to launch an arbitrary target via
/// `opener::open` (F-P1-3), which on Windows resolves through ShellExecute and
/// will happily open a UNC path or local file if not stopped here.
const ALLOWED_OPEN_URL_HOSTS: &[&str] = &["www.curseforge.com", "curseforge.com"];

fn validate_open_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|_| "Invalid or unsupported URL".to_string())?;

    if parsed.scheme() != "https" {
        return Err(format!(
            "Refusing to open non-https URL scheme: {}",
            parsed.scheme()
        ));
    }

    match parsed.host_str() {
        Some(host) if ALLOWED_OPEN_URL_HOSTS.contains(&host) => Ok(()),
        Some(host) => Err(format!("Refusing to open disallowed host: {}", host)),
        None => Err("Refusing to open a URL with no host".to_string()),
    }
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    validate_open_url(&url)?;
    opener::open(url).map_err(|e| format!("Failed to open browser: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_url_rejects_non_https_and_disallowed_hosts() {
        for bad in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "http://www.curseforge.com/minecraft/mc-mods/sodium", // http, not https
            "https://evil.example.com/",
            "https://curseforge.com.evil.example.com/", // host-confusable, not a real subdomain
            "\\\\attacker.example\\share\\setup.exe",
            "not a url",
            "",
        ] {
            assert!(
                validate_open_url(bad).is_err(),
                "expected '{bad}' to be rejected"
            );
        }
    }

    #[test]
    fn open_url_accepts_curseforge_hosts() {
        for good in [
            "https://www.curseforge.com/minecraft/mc-mods/sodium",
            "https://curseforge.com/minecraft/mc-mods/sodium",
        ] {
            assert!(
                validate_open_url(good).is_ok(),
                "expected '{good}' to be accepted"
            );
        }
    }

    fn make_addon(project_id: u64, name: &str, version: &str) -> Addon {
        Addon {
            addon_file_id: project_id,
            addon_name: name.to_string(),
            addon_project_id: project_id,
            cdn_download_url: format!("https://edge.forgecdn.net/{name}"),
            mod_folder_path: "mods".to_string(),
            version: version.to_string(),
            web_site_url: None,
            disabled: None,
            file_name_on_disk: format!("{name}.jar"),
        }
    }

    fn make_manifest(mods: Vec<Addon>) -> Manifest {
        Manifest {
            update_type: Some("full".to_string()),
            mods,
            resourcepacks: Vec::new(),
            shaderpacks: Vec::new(),
            datapacks: Vec::new(),
            config_files: Vec::new(),
        }
    }

    #[test]
    fn renamed_addon_with_version_bump_is_updated_not_removed_and_added() {
        // Same project_id, different addon_name AND version — the exact shape
        // that used to read as "removed + added" under name-based matching
        // (F-P2-12), while installer::calculate_update_diff already treated it
        // as "updated". This asserts compare_manifests now agrees.
        let old = make_manifest(vec![make_addon(1, "Sodium", "sodium-mc1.20-0.5.jar")]);
        let new = make_manifest(vec![make_addon(
            1,
            "Sodium Renamed",
            "sodium-mc1.20-0.6.jar",
        )]);

        let info = compare_manifests(old, new).expect("compare_manifests should succeed");

        assert!(
            info.added_addons.is_empty(),
            "renamed addon must not be reported as added"
        );
        assert!(
            info.removed_addons.is_empty(),
            "renamed addon must not be reported as removed"
        );
        assert_eq!(info.updated_addon_ids, vec![1]);
    }

    #[test]
    fn renamed_addon_with_same_version_is_neither_added_removed_nor_updated() {
        // A pure rename with no version change is invisible to both this
        // function and the installer's diff — consistent, if quiet.
        let old = make_manifest(vec![make_addon(1, "Sodium", "sodium-mc1.20-0.5.jar")]);
        let new = make_manifest(vec![make_addon(
            1,
            "Sodium Renamed",
            "sodium-mc1.20-0.5.jar",
        )]);

        let info = compare_manifests(old, new).expect("compare_manifests should succeed");

        assert!(info.added_addons.is_empty());
        assert!(info.removed_addons.is_empty());
        assert!(info.updated_addon_ids.is_empty());
    }

    #[test]
    fn genuinely_new_addon_is_added() {
        let old = make_manifest(vec![make_addon(1, "Sodium", "1.0.jar")]);
        let new = make_manifest(vec![
            make_addon(1, "Sodium", "1.0.jar"),
            make_addon(2, "Lithium", "1.0.jar"),
        ]);

        let info = compare_manifests(old, new).expect("compare_manifests should succeed");

        assert_eq!(info.added_addons.len(), 1);
        assert_eq!(info.added_addons[0].addon_project_id, 2);
        assert!(info.removed_addons.is_empty());
        assert!(info.updated_addon_ids.is_empty());
    }

    #[test]
    fn genuinely_removed_addon_is_removed() {
        let old = make_manifest(vec![
            make_addon(1, "Sodium", "1.0.jar"),
            make_addon(2, "Lithium", "1.0.jar"),
        ]);
        let new = make_manifest(vec![make_addon(1, "Sodium", "1.0.jar")]);

        let info = compare_manifests(old, new).expect("compare_manifests should succeed");

        assert!(info.added_addons.is_empty());
        assert_eq!(info.removed_addons, vec!["Lithium".to_string()]);
        assert!(info.updated_addon_ids.is_empty());
    }

    #[test]
    fn update_info_serializes_with_camel_case_keys() {
        let info = UpdateInfo {
            uuid: "test-uuid".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            added_addons: Vec::new(),
            removed_addons: Vec::new(),
            updated_addon_ids: vec![42],
        };

        let json = serde_json::to_value(&info).expect("UpdateInfo should serialize");
        let obj = json
            .as_object()
            .expect("UpdateInfo should serialize to an object");

        for key in [
            "uuid",
            "timestamp",
            "addedAddons",
            "removedAddons",
            "updatedAddonIds",
        ] {
            assert!(
                obj.contains_key(key),
                "expected camelCase key '{key}' in serialized UpdateInfo, got: {obj:?}"
            );
        }
        // snake_case must not leak through now that rename_all is applied.
        assert!(!obj.contains_key("added_addons"));
        assert!(!obj.contains_key("updated_addon_ids"));
    }
}
