//! Reading the local CurseForge library.
//!
//! CEMM's whole job starts with "which modpack", and until now that meant a
//! native folder dialog every single time — for a maintainer with 39 instances,
//! several of whose folder names disagree with the pack inside them. This module
//! reads what CurseForge already knows so the app can offer the list instead.
//!
//! Everything here is read-only. Nothing in this file writes to an instance.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::command;

/// A CurseForge instance group, by id, as `groups.json` records them.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackGroup {
    pub id: String,
    pub name: String,
}

/// One modpack, at the level of detail a library card needs — deliberately not
/// the manifest. Loading an instance for real still goes through
/// `parse_minecraft_instance`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackSummary {
    /// The folder holding `minecraftinstance.json`. This is the identity key
    /// everywhere else in the app — `appStore.modpackPath` and CEMM's own
    /// per-pack history are both keyed on it.
    pub instance_path: String,
    pub instance_file: String,
    /// The directory's own name, which is NOT always the pack's name: in the
    /// author's library the folder `All the Mods 10 - ATM10 (2)` holds a pack
    /// called `Aeronautics`. Both are surfaced so a card can never be ambiguous.
    pub folder_name: String,
    pub name: String,
    pub game_version: Option<String>,
    /// Loader family only — `NeoForge`, not `neoforge-21.1.228`. The build
    /// number does not help anyone tell two packs apart, and the Minecraft
    /// version is already its own field.
    pub loader: Option<String>,
    pub group_id: Option<String>,
    pub addon_count: usize,
    /// RFC 3339. CurseForge writes year 0001 for "never played"; that is passed
    /// through as-is and read as "never" on the other side.
    pub last_played: Option<String>,
    pub played_count: u64,
    /// A `data:` URI, or absent. See `read_icon` for why it is inlined.
    pub icon: Option<String>,
    pub project_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackLibrary {
    /// Where the scan actually looked, if anywhere.
    pub instances_dir: Option<String>,
    /// `curseforge` when discovered from CurseForge's own settings, `manual`
    /// when the caller supplied the folder, `none` when there was nothing to
    /// scan. The UI needs to tell "you have no packs" apart from "I could not
    /// find CurseForge", because only one of those is the user's problem.
    pub source: String,
    pub packs: Vec<PackSummary>,
    pub groups: Vec<PackGroup>,
    /// Folders that looked like instances but could not be read. Reported rather
    /// than failing the whole scan: one corrupt instance must not cost the user
    /// the other thirty-eight.
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct BaseModLoader {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstanceHeader {
    name: Option<String>,
    game_version: Option<String>,
    group_id: Option<String>,
    project_id: Option<u64>,
    played_count: Option<u64>,
    last_played: Option<String>,
    profile_image_path: Option<String>,
    base_mod_loader: Option<BaseModLoader>,
    /// Counted, never materialised. Building 8382 addon structs to render 36
    /// cards is work no card uses; skipping them takes the whole 58 MB scan of
    /// the author's library to ~19 ms of JSON.
    installed_addons: Option<Vec<serde::de::IgnoredAny>>,
}

#[derive(Debug, Deserialize)]
struct RawGroup {
    id: Option<String>,
    name: Option<String>,
}

/// Where CurseForge keeps `storage.json` and its agent data.
fn curseforge_config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|appdata| PathBuf::from(appdata).join("CurseForge"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        // CurseForge ships no official Linux build. The community wrappers keep
        // this layout under the XDG config dir; where they do not, discovery
        // simply fails and the user points CEMM at the folder themselves, which
        // is a supported path rather than an error.
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .map(|base| base.join("CurseForge"))
    }
}

/// CurseForge's library root, from its own settings.
///
/// `storage.json` is a flat map whose values are themselves JSON *documents
/// encoded as strings* — `minecraft-settings` has to be parsed a second time to
/// reach `minecraftRoot`.
fn discover_instances_dir() -> Option<PathBuf> {
    let config = curseforge_config_dir()?;
    let storage = fs::read_to_string(config.join("storage.json")).ok()?;
    let root: serde_json::Value = serde_json::from_str(&storage).ok()?;
    let settings_raw = root.get("minecraft-settings")?.as_str()?;
    let settings: serde_json::Value = serde_json::from_str(settings_raw).ok()?;
    let minecraft_root = settings.get("minecraftRoot")?.as_str()?;
    if minecraft_root.trim().is_empty() {
        return None;
    }
    let dir = Path::new(minecraft_root).join("Instances");
    dir.is_dir().then_some(dir)
}

fn read_groups() -> Vec<PackGroup> {
    let Some(config) = curseforge_config_dir() else {
        return Vec::new();
    };
    let path = config.join("agent").join("GameInstances").join("groups.json");
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(raw) = serde_json::from_str::<Vec<RawGroup>>(&text) else {
        log::warn!("read_groups: {} did not parse as a group list", path.display());
        return Vec::new();
    };
    raw.into_iter()
        .filter_map(|group| {
            Some(PackGroup {
                id: group.id?,
                name: group.name?,
            })
        })
        .filter(|group| !group.id.is_empty() && !group.name.is_empty())
        .collect()
}

/// `neoforge-21.1.228` -> `NeoForge`. Unrecognised families keep whatever
/// CurseForge wrote, capitalised, rather than being dropped.
fn pretty_loader(raw: &str) -> Option<String> {
    let family = raw.split('-').next()?.trim();
    if family.is_empty() {
        return None;
    }
    Some(match family.to_lowercase().as_str() {
        "neoforge" => "NeoForge".to_string(),
        "forge" => "Forge".to_string(),
        "fabric" => "Fabric".to_string(),
        "quilt" => "Quilt".to_string(),
        _ => {
            let mut chars = family.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => return None,
            }
        }
    })
}

const MAX_ICON_BYTES: u64 = 512 * 1024;

/// The pack's icon, inlined as a `data:` URI.
///
/// Inlined rather than served over the asset protocol because the alternative is
/// widening the webview's filesystem scope for three 200 KB PNGs. Two rules
/// bound it: the value comes from a JSON file CEMM did not write, so it may only
/// resolve inside the instance's own folder; and CurseForge sometimes stores a
/// CDN URL here, which a screen that must render on a cold offline launch cannot
/// fetch. Either way the card falls back to its coloured initial.
fn read_icon(instance_dir: &Path, raw: Option<&str>) -> Option<String> {
    let raw = raw?.trim();
    if raw.is_empty() || raw.starts_with("http://") || raw.starts_with("https://") {
        return None;
    }

    let candidate = fs::canonicalize(raw).ok()?;
    let root = fs::canonicalize(instance_dir).ok()?;
    if !candidate.starts_with(&root) {
        log::warn!("read_icon: refusing icon outside its instance: {raw}");
        return None;
    }

    let meta = fs::metadata(&candidate).ok()?;
    if !meta.is_file() || meta.len() > MAX_ICON_BYTES {
        return None;
    }

    let mime = match candidate.extension()?.to_str()?.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => return None,
    };

    let bytes = fs::read(&candidate).ok()?;
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn summarise(instance_dir: &Path) -> Result<PackSummary, String> {
    let instance_file = instance_dir.join("minecraftinstance.json");
    let text = fs::read_to_string(&instance_file).map_err(|e| e.to_string())?;
    let header: InstanceHeader = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let folder_name = instance_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();

    let name = header
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(&folder_name)
        .to_string();

    Ok(PackSummary {
        instance_path: instance_dir.to_string_lossy().into_owned(),
        instance_file: instance_file.to_string_lossy().into_owned(),
        folder_name,
        name,
        game_version: header
            .game_version
            .filter(|version| !version.trim().is_empty()),
        loader: header
            .base_mod_loader
            .and_then(|loader| loader.name)
            .as_deref()
            .and_then(pretty_loader),
        // CurseForge writes JSON null for "no group"; an empty string would mean
        // the same thing and must not become a group nothing matches.
        group_id: header.group_id.filter(|id| !id.trim().is_empty()),
        addon_count: header
            .installed_addons
            .map(|addons| addons.len())
            .unwrap_or(0),
        last_played: header.last_played,
        played_count: header.played_count.unwrap_or(0),
        icon: read_icon(instance_dir, header.profile_image_path.as_deref()),
        project_id: header.project_id.filter(|id| *id > 0),
    })
}

#[command]
pub fn scan_pack_library(instances_dir: Option<String>) -> Result<PackLibrary, String> {
    let requested = instances_dir
        .as_deref()
        .map(str::trim)
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from);

    let (dir, source) = match requested {
        Some(dir) => (Some(dir), "manual"),
        None => match discover_instances_dir() {
            Some(dir) => (Some(dir), "curseforge"),
            None => (None, "none"),
        },
    };

    let Some(dir) = dir else {
        log::info!("scan_pack_library: no instances directory found");
        return Ok(PackLibrary {
            instances_dir: None,
            source: source.to_string(),
            packs: Vec::new(),
            groups: Vec::new(),
            warnings: Vec::new(),
        });
    };

    let entries = fs::read_dir(&dir).map_err(|e| {
        log::error!("scan_pack_library: cannot read {}: {e}", dir.display());
        format!("Could not read {}: {e}", dir.display())
    })?;

    let mut packs = Vec::new();
    let mut warnings = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || !path.join("minecraftinstance.json").is_file() {
            // Not an instance. CEMM's own `.cemm_backups` folder and the stray
            // directories left behind by renames both land here, and neither is
            // worth telling the user about.
            continue;
        }
        match summarise(&path) {
            Ok(pack) => packs.push(pack),
            Err(error) => {
                log::warn!("scan_pack_library: skipping {}: {error}", path.display());
                warnings.push(format!(
                    "{}: {error}",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("instance")
                ));
            }
        }
    }

    // A stable default order. The UI re-sorts by CEMM's own history, which this
    // side knows nothing about.
    packs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    log::info!(
        "scan_pack_library: {} packs in {} ({source})",
        packs.len(),
        dir.display()
    );

    Ok(PackLibrary {
        instances_dir: Some(dir.to_string_lossy().into_owned()),
        source: source.to_string(),
        packs,
        groups: read_groups(),
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_loader_names_the_family_and_drops_the_build() {
        assert_eq!(pretty_loader("neoforge-21.1.228").as_deref(), Some("NeoForge"));
        assert_eq!(pretty_loader("forge-47.4.13").as_deref(), Some("Forge"));
        assert_eq!(
            pretty_loader("fabric-0.18.2-1.21.10").as_deref(),
            Some("Fabric")
        );
        assert_eq!(pretty_loader("quilt-0.1.0").as_deref(), Some("Quilt"));
        // Unknown families are kept rather than dropped — a loader CEMM has not
        // heard of is still information the card should carry.
        assert_eq!(pretty_loader("cleanroom-1.0").as_deref(), Some("Cleanroom"));
        assert_eq!(pretty_loader(""), None);
    }

    fn write_instance(dir: &Path, body: serde_json::Value) {
        fs::create_dir_all(dir).expect("instance dir");
        fs::write(
            dir.join("minecraftinstance.json"),
            serde_json::to_string(&body).expect("serialize"),
        )
        .expect("write instance");
    }

    #[test]
    fn scan_reads_names_versions_loaders_and_addon_counts() {
        let temp = tempfile::tempdir().expect("temp dir");
        write_instance(
            &temp.path().join("All the Mods 10 - ATM10 (2)"),
            serde_json::json!({
                "name": "Aeronautics",
                "gameVersion": "1.21.1",
                "baseModLoader": { "name": "neoforge-21.1.228" },
                "groupId": "882ce074",
                "playedCount": 127,
                "lastPlayed": "2026-08-16T17:10:41Z",
                "installedAddons": [{}, {}, {}]
            }),
        );

        let library = scan_pack_library(Some(temp.path().to_string_lossy().into_owned()))
            .expect("scan should succeed");

        assert_eq!(library.source, "manual");
        assert_eq!(library.packs.len(), 1);
        let pack = &library.packs[0];
        // The pack's own name, not the folder's — and the folder kept alongside
        // it, because these disagree in the real library.
        assert_eq!(pack.name, "Aeronautics");
        assert_eq!(pack.folder_name, "All the Mods 10 - ATM10 (2)");
        assert_eq!(pack.game_version.as_deref(), Some("1.21.1"));
        assert_eq!(pack.loader.as_deref(), Some("NeoForge"));
        assert_eq!(pack.group_id.as_deref(), Some("882ce074"));
        assert_eq!(pack.addon_count, 3);
        assert_eq!(pack.played_count, 127);
    }

    #[test]
    fn scan_falls_back_to_the_folder_name_and_treats_null_group_as_ungrouped() {
        let temp = tempfile::tempdir().expect("temp dir");
        write_instance(
            &temp.path().join("Unnamed Pack"),
            serde_json::json!({
                "name": "   ",
                "groupId": serde_json::Value::Null,
                "installedAddons": []
            }),
        );

        let library = scan_pack_library(Some(temp.path().to_string_lossy().into_owned()))
            .expect("scan should succeed");

        assert_eq!(library.packs.len(), 1);
        assert_eq!(library.packs[0].name, "Unnamed Pack");
        assert_eq!(library.packs[0].group_id, None);
        assert_eq!(library.packs[0].addon_count, 0);
    }

    #[test]
    fn scan_skips_folders_that_are_not_instances() {
        let temp = tempfile::tempdir().expect("temp dir");
        write_instance(
            &temp.path().join("Real Pack"),
            serde_json::json!({ "name": "Real Pack", "installedAddons": [] }),
        );
        // CEMM's own backup folder, and a leftover from a rename holding nothing
        // but a manifest. Both exist in the author's Instances directory.
        fs::create_dir_all(temp.path().join(".cemm_backups")).expect("backups dir");
        fs::write(temp.path().join(".cemm_backups/backup_registry.json"), "[]").expect("registry");
        fs::create_dir_all(temp.path().join("FTB Evolution (1)")).expect("stray dir");
        fs::write(temp.path().join("FTB Evolution (1)/cemm-manifest.json"), "{}").expect("stray");

        let library = scan_pack_library(Some(temp.path().to_string_lossy().into_owned()))
            .expect("scan should succeed");

        assert_eq!(library.packs.len(), 1);
        assert_eq!(library.packs[0].name, "Real Pack");
        assert!(library.warnings.is_empty());
    }

    #[test]
    fn one_unreadable_instance_does_not_fail_the_scan() {
        let temp = tempfile::tempdir().expect("temp dir");
        write_instance(
            &temp.path().join("Good"),
            serde_json::json!({ "name": "Good", "installedAddons": [] }),
        );
        let broken = temp.path().join("Broken");
        fs::create_dir_all(&broken).expect("broken dir");
        fs::write(broken.join("minecraftinstance.json"), "{ not json").expect("write broken");

        let library = scan_pack_library(Some(temp.path().to_string_lossy().into_owned()))
            .expect("scan should succeed despite one bad instance");

        assert_eq!(library.packs.len(), 1);
        assert_eq!(library.packs[0].name, "Good");
        assert_eq!(library.warnings.len(), 1);
        assert!(library.warnings[0].starts_with("Broken:"));
    }

    #[test]
    fn missing_directory_reports_rather_than_scanning() {
        let temp = tempfile::tempdir().expect("temp dir");
        let missing = temp.path().join("nope");
        let result = scan_pack_library(Some(missing.to_string_lossy().into_owned()));
        assert!(result.is_err(), "a missing folder is the caller's mistake");
    }

    #[test]
    fn icon_outside_the_instance_folder_is_refused() {
        let temp = tempfile::tempdir().expect("temp dir");
        let outside = temp.path().join("secret.png");
        fs::write(&outside, b"\x89PNG not really").expect("write outside file");

        let instance = temp.path().join("Pack");
        write_instance(
            &instance,
            serde_json::json!({
                "name": "Pack",
                "profileImagePath": outside.to_string_lossy(),
                "installedAddons": []
            }),
        );

        let library = scan_pack_library(Some(temp.path().to_string_lossy().into_owned()))
            .expect("scan should succeed");

        assert_eq!(library.packs.len(), 1);
        assert_eq!(
            library.packs[0].icon, None,
            "an icon path escaping its instance must not be read"
        );
    }

    #[test]
    fn icon_url_is_not_fetched() {
        let temp = tempfile::tempdir().expect("temp dir");
        write_instance(
            &temp.path().join("Pack"),
            serde_json::json!({
                "name": "Pack",
                "profileImagePath": "https://media.forgecdn.net/avatars/1182/438/638755918649288941.png",
                "installedAddons": []
            }),
        );

        let library = scan_pack_library(Some(temp.path().to_string_lossy().into_owned()))
            .expect("scan should succeed");

        assert_eq!(library.packs[0].icon, None);
    }

    #[test]
    fn icon_inside_the_instance_folder_is_inlined() {
        let temp = tempfile::tempdir().expect("temp dir");
        let instance = temp.path().join("Pack");
        fs::create_dir_all(instance.join("profileImage")).expect("image dir");
        let icon = instance.join("profileImage").join("icon.png");
        fs::write(&icon, b"pretend-png-bytes").expect("write icon");
        write_instance(
            &instance,
            serde_json::json!({
                "name": "Pack",
                "profileImagePath": icon.to_string_lossy(),
                "installedAddons": []
            }),
        );

        let library = scan_pack_library(Some(temp.path().to_string_lossy().into_owned()))
            .expect("scan should succeed");

        let icon = library.packs[0].icon.as_deref().expect("icon should inline");
        assert!(icon.starts_with("data:image/png;base64,"));
    }
}
