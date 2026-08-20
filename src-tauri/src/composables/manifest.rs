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
    /// CurseForge's own enable/disable switch, and the authoritative signal for
    /// it: the app writes `false` here in the same pass that renames the file on
    /// disk to `*.disabled`.
    #[serde(rename = "isEnabled")]
    is_enabled: Option<bool>,
    /// The name the file actually carries on disk — `sodium.jar.disabled` for a
    /// switched-off addon. `installedFile.fileName` stays at the canonical
    /// `sodium.jar` either way, which is why that field cannot answer this.
    #[serde(rename = "fileNameOnDisk")]
    file_name_on_disk: Option<String>,
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

/// The four content folders CEMM distributes. Anything else CurseForge lists in
/// `installedAddons` — notably the modpack archive itself, which it records with
/// `modFolderPath` pointing at `downloads` — is not addon content and must never
/// reach a manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Mods,
    ResourcePacks,
    ShaderPacks,
    DataPacks,
}

fn folder_leaf(mod_folder_path: &str) -> &str {
    mod_folder_path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(mod_folder_path)
}

/// Where an addon belongs, decided from the folder it actually sits in and
/// falling back to CurseForge's category label.
///
/// Folder first, because it is ground truth — it is the directory the file was
/// written to. The label is only a fallback, and it has to be one: CurseForge
/// leaves `categorySection` null on a large share of entries (138 of 8382 across
/// the author's own 39 instances). Requiring it, as this function's predecessor
/// did, silently dropped every one of those addons from the manifest the admin
/// then published.
fn classify(category_name: Option<&str>, mod_folder_path: &str) -> Option<Category> {
    match folder_leaf(mod_folder_path).to_lowercase().as_str() {
        "mods" => return Some(Category::Mods),
        "resourcepacks" => return Some(Category::ResourcePacks),
        "shaderpacks" => return Some(Category::ShaderPacks),
        "datapacks" => return Some(Category::DataPacks),
        _ => {}
    }

    let category = category_name?.to_lowercase();
    if category.contains("shader") {
        Some(Category::ShaderPacks)
    } else if category.contains("resource") {
        Some(Category::ResourcePacks)
    } else if category.contains("datapack") || category.contains("data pack") {
        Some(Category::DataPacks)
    } else if category.contains("modpack") {
        // The pack itself, not something installed into it.
        None
    } else if category.contains("mod") {
        Some(Category::Mods)
    } else {
        None
    }
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
    // Fallback disabled detection, for an instance edited outside CurseForge: a
    // `*.disabled` file whose entry in the JSON still claims to be enabled.
    let base_dir = Path::new(&path).parent().unwrap_or_else(|| Path::new("."));
    let disabled_mods = find_disabled_files(base_dir.join("mods"));
    let disabled_resourcepacks = find_disabled_files(base_dir.join("resourcepacks"));
    let disabled_shaderpacks = find_disabled_files(base_dir.join("shaderpacks"));
    let disabled_datapacks = find_disabled_files(base_dir.join("datapacks"));
    let mut mods = Vec::new();
    let mut resourcepacks = Vec::new();
    let mut shaderpacks = Vec::new();
    let mut datapacks = Vec::new();
    let mut disabled_count = 0usize;
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
        let category_name = addon
            .category_section
            .as_ref()
            .and_then(|section| section.name.as_deref());
        let Some(category) = classify(category_name, mod_folder_path) else {
            log::info!(
                "parse_minecraft_instance: skipping non-addon entry '{addon_name}' in {mod_folder_path}"
            );
            continue;
        };

        // Three independent signals, any one of which means "switched off".
        // `isEnabled` is CurseForge's own flag and is what its UI writes when the
        // toggle is flipped; the other two catch an instance edited by hand.
        let disabled_on_disk = match category {
            Category::Mods => &disabled_mods,
            Category::ResourcePacks => &disabled_resourcepacks,
            Category::ShaderPacks => &disabled_shaderpacks,
            Category::DataPacks => &disabled_datapacks,
        };
        let is_disabled = addon.is_enabled == Some(false)
            || addon
                .file_name_on_disk
                .as_deref()
                .is_some_and(|name| name.ends_with(".disabled"))
            || disabled_on_disk.contains(version);
        if is_disabled {
            disabled_count += 1;
        }

        let addon_struct = Addon {
            addon_file_id,
            addon_name: addon_name.clone(),
            addon_project_id,
            cdn_download_url: cdn_download_url.clone(),
            mod_folder_path: mod_folder_path.clone(),
            version: version.clone(),
            web_site_url: addon.web_site_url.clone(),
            // Left as None rather than Some(false) when enabled: the field is
            // `skip_serializing_if = "Option::is_none"`, so this keeps published
            // manifests byte-identical to what earlier CEMM versions wrote.
            disabled: if is_disabled { Some(true) } else { None },
            // Deliberately the canonical name, not `addon.fileNameOnDisk`, which
            // carries the `.disabled` suffix. The installer resolves both forms
            // (see installer.rs), and a player receiving this manifest needs the
            // name the file will have once it is installed for them.
            file_name_on_disk: version.clone(),
        };
        match category {
            Category::Mods => mods.push(addon_struct),
            Category::ResourcePacks => resourcepacks.push(addon_struct),
            Category::ShaderPacks => shaderpacks.push(addon_struct),
            Category::DataPacks => datapacks.push(addon_struct),
        }
    }
    log::info!(
        "parse_minecraft_instance: {} mods, {} resourcepacks, {} shaderpacks, {} datapacks ({disabled_count} disabled)",
        mods.len(),
        resourcepacks.len(),
        shaderpacks.len(),
        datapacks.len()
    );
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

    #[test]
    fn classify_reads_the_folder_before_the_category_label() {
        // The folder is where the file actually is; the label is metadata that
        // may be stale, empty or absent.
        assert_eq!(
            classify(None, r"D:\Games\Instances\ATM10\mods"),
            Some(Category::Mods)
        );
        assert_eq!(
            classify(None, "/home/p/Instances/ATM10/shaderpacks"),
            Some(Category::ShaderPacks)
        );
        assert_eq!(
            classify(Some(""), r"D:\Games\Instances\ATM10\resourcepacks"),
            Some(Category::ResourcePacks)
        );
        assert_eq!(
            classify(Some("Mods"), r"D:\Games\Instances\ATM10\datapacks"),
            Some(Category::DataPacks)
        );
    }

    #[test]
    fn classify_falls_back_to_the_category_label_for_an_unknown_folder() {
        assert_eq!(
            classify(Some("Shaders"), r"D:\Games\Instances\ATM10\somewhere"),
            Some(Category::ShaderPacks)
        );
        assert_eq!(
            classify(Some("Resource Packs"), r"D:\somewhere"),
            Some(Category::ResourcePacks)
        );
        assert_eq!(
            classify(Some("Data Packs"), r"D:\somewhere"),
            Some(Category::DataPacks)
        );
        assert_eq!(
            classify(Some("Mods"), r"D:\somewhere"),
            Some(Category::Mods)
        );
        assert_eq!(classify(None, r"D:\somewhere"), None);
    }

    #[test]
    fn classify_rejects_the_modpack_archive_itself() {
        // CurseForge lists the pack it installed among `installedAddons`, in the
        // `downloads` folder under a "Modpacks" category. Publishing that entry
        // would tell every player to download a whole modpack zip into `mods`.
        assert_eq!(
            classify(Some("Modpacks"), r"D:\Games\Instances\Nightfall\downloads"),
            None
        );
    }

    /// Writes a throwaway `minecraftinstance.json` and returns its path plus the
    /// directory holding it, so the caller can also plant files on disk.
    fn write_instance(addons: serde_json::Value) -> (tempfile::TempDir, String) {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let path = temp.path().join("minecraftinstance.json");
        let body = serde_json::json!({ "installedAddons": addons });
        fs::write(&path, serde_json::to_string(&body).expect("serialize"))
            .expect("failed to write instance");
        let path = path.to_string_lossy().into_owned();
        (temp, path)
    }

    fn installed_addon(
        name: &str,
        file_name: &str,
        folder: &str,
        extra: serde_json::Value,
    ) -> serde_json::Value {
        let mut entry = serde_json::json!({
            "addonID": 1234,
            "name": name,
            "modFolderPath": folder,
            "installedFile": {
                "id": 5678,
                "fileName": file_name,
                "downloadUrl": format!("https://edge.forgecdn.net/{file_name}"),
            },
        });
        let object = entry.as_object_mut().expect("object");
        for (key, value) in extra.as_object().expect("extra must be an object") {
            object.insert(key.clone(), value.clone());
        }
        entry
    }

    #[test]
    fn parse_keeps_addons_curseforge_left_uncategorised() {
        // CurseForge writes `categorySection: null` on a large share of entries.
        // Requiring it dropped those addons from the manifest entirely — 138 of
        // 8382 across the author's own instances — so an admin published an
        // update that quietly told players to delete perfectly good mods.
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let mods_dir = temp_dir.path().join("mods");
        fs::create_dir_all(&mods_dir).expect("mods dir");
        let path = temp_dir.path().join("minecraftinstance.json");
        let body = serde_json::json!({
            "installedAddons": [installed_addon(
                "Reputation",
                "Reputation-1.18-0.9.9.jar",
                &mods_dir.to_string_lossy(),
                serde_json::json!({ "categorySection": serde_json::Value::Null }),
            )]
        });
        fs::write(&path, serde_json::to_string(&body).expect("serialize")).expect("write");

        let manifest = parse_minecraft_instance(path.to_string_lossy().into_owned())
            .expect("parse should succeed");

        assert_eq!(manifest.mods.len(), 1);
        assert_eq!(manifest.mods[0].addon_name, "Reputation");
        assert_eq!(manifest.mods[0].disabled, None);
    }

    #[test]
    fn parse_marks_addons_curseforge_has_switched_off() {
        let (_temp, path) = write_instance(serde_json::json!([
            installed_addon(
                "Sophisticated Backpacks",
                "sophisticatedbackpacks-3.25.90.jar",
                r"D:\Games\Instances\ATM10\mods",
                serde_json::json!({
                    "categorySection": { "name": "Mods" },
                    "isEnabled": false,
                    "fileNameOnDisk": "sophisticatedbackpacks-3.25.90.jar.disabled",
                }),
            ),
            installed_addon(
                "Sodium",
                "sodium-0.5.jar",
                r"D:\Games\Instances\ATM10\mods",
                serde_json::json!({
                    "categorySection": { "name": "Mods" },
                    "isEnabled": true,
                    "fileNameOnDisk": "sodium-0.5.jar",
                }),
            ),
        ]));

        let manifest = parse_minecraft_instance(path).expect("parse should succeed");

        assert_eq!(manifest.mods.len(), 2);
        let off = &manifest.mods[0];
        assert_eq!(off.disabled, Some(true));
        // The manifest carries the name the file will have once installed for a
        // player, never the local `.disabled` form.
        assert_eq!(off.file_name_on_disk, "sophisticatedbackpacks-3.25.90.jar");
        assert_eq!(manifest.mods[1].disabled, None);
    }

    #[test]
    fn parse_marks_files_renamed_to_disabled_by_hand() {
        // No `isEnabled` and no `.disabled` in the JSON — only the file on disk
        // says so, which is what editing an instance outside CurseForge leaves.
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let mods_dir = temp_dir.path().join("mods");
        fs::create_dir_all(&mods_dir).expect("mods dir");
        fs::write(mods_dir.join("wthit-forge-4.13.6.jar.disabled"), b"").expect("plant file");
        let path = temp_dir.path().join("minecraftinstance.json");
        let body = serde_json::json!({
            "installedAddons": [installed_addon(
                "WTHIT",
                "wthit-forge-4.13.6.jar",
                &mods_dir.to_string_lossy(),
                serde_json::json!({ "categorySection": { "name": "Mods" } }),
            )]
        });
        fs::write(&path, serde_json::to_string(&body).expect("serialize")).expect("write");

        let manifest = parse_minecraft_instance(path.to_string_lossy().into_owned())
            .expect("parse should succeed");

        assert_eq!(manifest.mods.len(), 1);
        assert_eq!(manifest.mods[0].disabled, Some(true));
    }

    #[test]
    fn parse_leaves_the_modpack_archive_out_of_the_manifest() {
        let (_temp, path) = write_instance(serde_json::json!([installed_addon(
            "NightfallCraft - The Casket of Reveries",
            "The Casket of Reveries -2.2.8.6.zip",
            r"D:\Games\Instances\Nightfall\downloads",
            serde_json::json!({ "categorySection": { "name": "Modpacks" } }),
        )]));

        let manifest = parse_minecraft_instance(path).expect("parse should succeed");

        assert!(manifest.mods.is_empty());
        assert!(manifest.resourcepacks.is_empty());
        assert!(manifest.shaderpacks.is_empty());
        assert!(manifest.datapacks.is_empty());
    }
}
