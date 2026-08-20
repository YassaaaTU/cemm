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

/// A CurseForge instance group, by id, as `groups.json` records them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackGroup {
    pub id: String,
    pub name: String,
}

/// One modpack, at the level of detail a library card needs — deliberately not
/// the manifest. Loading an instance for real still goes through
/// `parse_minecraft_instance`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    /// The pack's artwork on CurseForge's CDN, for a pack installed from there
    /// and not yet cached locally. The scan never fetches it — that would put a
    /// network round trip in front of a screen that must open offline — so the
    /// caller asks for these separately and the card fills in when they land.
    pub icon_url: Option<String>,
    pub project_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
struct InstalledModpack {
    thumbnail_url: Option<String>,
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
    /// The pack CurseForge installed, which is where its artwork lives. Only
    /// the thumbnail is read; the rest of the entry is the modpack archive and
    /// is deliberately kept out of the manifest (see `classify`).
    installed_modpack: Option<InstalledModpack>,
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
/// Pull the Minecraft root out of the text of CurseForge's `storage.json`.
///
/// Split out from the file reading because it is the most fragile guess in the
/// module and the only part of discovery that can be tested: CurseForge stores
/// the entire minecraft settings *document* as a JSON string under
/// `minecraft-settings`, so this is two parses deep and neither layer is a
/// format CEMM has any claim on. An update to CurseForge that reshapes either
/// one breaks discovery silently — which is the reason the manual folder picker
/// is a first-class path rather than an error handler.
fn minecraft_root_from_storage(storage: &str) -> Option<String> {
    let root: serde_json::Value = serde_json::from_str(storage).ok()?;
    let settings_raw = root.get("minecraft-settings")?.as_str()?;
    let settings: serde_json::Value = serde_json::from_str(settings_raw).ok()?;
    let minecraft_root = settings.get("minecraftRoot")?.as_str()?.trim();
    (!minecraft_root.is_empty()).then(|| minecraft_root.to_string())
}

fn discover_instances_dir() -> Option<PathBuf> {
    let config = curseforge_config_dir()?;
    let storage = fs::read_to_string(config.join("storage.json")).ok()?;
    let minecraft_root = minecraft_root_from_storage(&storage)?;
    let dir = Path::new(&minecraft_root).join("Instances");
    dir.is_dir().then_some(dir)
}

/// The groups CurseForge records, from the text of its `groups.json`.
///
/// A group with no id or no name cannot be a filter pill, so it is dropped
/// rather than shown as a blank one. A file that does not parse at all costs the
/// library its pills and nothing else.
fn parse_groups(text: &str) -> Vec<PackGroup> {
    let Ok(raw) = serde_json::from_str::<Vec<RawGroup>>(text) else {
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

fn read_groups() -> Vec<PackGroup> {
    let Some(config) = curseforge_config_dir() else {
        return Vec::new();
    };
    let path = config
        .join("agent")
        .join("GameInstances")
        .join("groups.json");
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let groups = parse_groups(&text);
    if groups.is_empty() {
        log::warn!("read_groups: {} yielded no usable groups", path.display());
    }
    groups
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
            let first = chars.next()?;
            first.to_uppercase().collect::<String>() + chars.as_str()
        }
    })
}

const MAX_ICON_BYTES: u64 = 512 * 1024;

/// The only host CurseForge serves pack artwork from. A `thumbnailUrl` comes out
/// of a JSON file CEMM did not write, so it is treated as untrusted input: an
/// allowlist here is what stops a doctored instance turning the library into a
/// request to an arbitrary server.
const ALLOWED_ICON_HOSTS: &[&str] = &["media.forgecdn.net"];

fn validate_icon_url(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    if parsed.scheme() != "https" {
        return None;
    }
    match parsed.host_str() {
        Some(host) if ALLOWED_ICON_HOSTS.contains(&host) => Some(parsed.to_string()),
        Some(host) => {
            log::warn!("validate_icon_url: refusing icon host {host}");
            None
        }
        None => None,
    }
}

/// A stable filename for a cached icon, derived from its URL path.
///
/// Deliberately not a hash: the URL path is already unique per asset, and a
/// readable name means the cache directory can be understood by looking at it.
/// Every character outside the safe set becomes `-`, so nothing here can escape
/// the cache directory.
fn cache_file_name(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let extension = Path::new(parsed.path())
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .filter(|value| matches!(value.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif"))?;

    let stem: String = parsed
        .path()
        .trim_matches('/')
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();

    // Bounded so a pathological URL cannot produce a filename the OS rejects.
    // The tail rather than the head, because what distinguishes two CurseForge
    // assets is the last segment of their path — truncating from the front is
    // what keeps them from colliding. Byte indexing is safe here: the map above
    // leaves nothing but ASCII alphanumerics and `-`.
    let stem = stem[stem.len().saturating_sub(96)..].to_string();

    Some(format!("{stem}.{extension}"))
}

fn mime_for(path: &Path) -> Option<&'static str> {
    Some(match path.extension()?.to_str()?.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => return None,
    })
}

/// Read an image off disk as a `data:` URI, or nothing.
///
/// The single place the size cap and the format table are applied, so a card
/// that declines to show a picture declines for the same reasons wherever the
/// picture came from. Every refusal returns `None` because none of them is
/// something the user has to act on — but the oversize one is logged, since it
/// is the only one where the file is right there and looks fine.
fn as_data_uri(path: &Path) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    if meta.len() > MAX_ICON_BYTES {
        log::debug!(
            "as_data_uri: {} is {} bytes, over the {MAX_ICON_BYTES} cap",
            path.display(),
            meta.len()
        );
        return None;
    }
    let mime = mime_for(path)?;
    let bytes = fs::read(path).ok()?;
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

/// A cached CurseForge icon, already on disk from a previous run.
fn cached_icon(cache_dir: Option<&Path>, url: &str) -> Option<String> {
    let path = cache_dir?.join(cache_file_name(url)?);
    as_data_uri(&path)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedIcon {
    pub url: String,
    /// A `data:` URI. Absent when the fetch failed, so the caller can stop
    /// asking for that one without treating it as an error worth reporting.
    pub icon: Option<String>,
}

/// Fetch pack artwork from CurseForge's CDN and keep it on disk.
///
/// This is the app's second network exception after GitHub, and it is bounded to
/// make that acceptable: an allowlisted host, https only, a size cap, a short
/// timeout, and a permanent on-disk cache so a given pack is fetched exactly
/// once ever. Every subsequent launch — including offline ones — is served from
/// that cache by `scan_pack_library`, which never touches the network itself.
///
/// Failures are reported per icon rather than failing the batch: an unreachable
/// CDN should cost a card its picture, nothing more.
pub async fn cache_pack_icons_in(
    cache_dir: PathBuf,
    urls: Vec<String>,
) -> Result<Vec<CachedIcon>, String> {
    fs::create_dir_all(&cache_dir)
        .map_err(|error| format!("Could not open the icon cache directory: {error}"))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        // The allowlist is the bounding control on this exception, and reqwest's
        // default policy follows up to ten redirects — so a 302 out of an
        // allowlisted host would have carried the request to one that is not,
        // with `validate_icon_url` having checked only the address it started
        // from. An artwork URL is not worth following anywhere.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("Could not create HTTP client: {e}"))?;

    let mut results = Vec::new();
    for url in urls {
        let Some(valid) = validate_icon_url(&url) else {
            results.push(CachedIcon { url, icon: None });
            continue;
        };
        let Some(file_name) = cache_file_name(&valid) else {
            results.push(CachedIcon { url, icon: None });
            continue;
        };
        let path = cache_dir.join(&file_name);

        // Someone else may have cached it since the scan read the directory.
        if let Some(icon) = as_data_uri(&path) {
            results.push(CachedIcon {
                url,
                icon: Some(icon),
            });
            continue;
        }

        let icon = match fetch_icon(&client, &valid, &path).await {
            Ok(icon) => Some(icon),
            Err(error) => {
                log::warn!("cache_pack_icons: {valid} failed: {error}");
                None
            }
        };
        results.push(CachedIcon { url, icon });
    }

    Ok(results)
}

async fn fetch_icon(client: &reqwest::Client, url: &str, path: &Path) -> Result<String, String> {
    let response = client
        .get(url)
        .header("User-Agent", "cemm-app-tauri")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    // Checked before reading as well as after: an honest Content-Length lets a
    // oversized image be refused without pulling it into memory first.
    if let Some(length) = response.content_length() {
        if length > MAX_ICON_BYTES {
            return Err(format!("icon is {length} bytes, over the cap"));
        }
    }

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_ICON_BYTES {
        return Err(format!("icon is {} bytes, over the cap", bytes.len()));
    }

    // The MIME comes from the URL's file extension, so a 200 carrying something
    // that is not an image would be written to the cache and inlined as a
    // well-formed `data:image/png` — a card showing a broken tile, permanently,
    // because the cache is never revisited. Refused here instead, which leaves
    // the coloured initial and a URL that can be asked for again later.
    if !looks_like_image(&bytes) {
        return Err("response body is not an image".to_string());
    }

    write_atomically(path, &bytes)?;
    as_data_uri(path).ok_or_else(|| "fetched icon was not a readable image".to_string())
}

/// Whether the bytes open with the signature of a format this cache serves.
///
/// Not a full decode — just enough that a captive-portal login page or an error
/// document cannot be filed away as a PNG.
fn looks_like_image(bytes: &[u8]) -> bool {
    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF];
    const GIF87: &[u8] = b"GIF87a";
    const GIF89: &[u8] = b"GIF89a";

    if bytes.starts_with(PNG) || bytes.starts_with(JPEG) {
        return true;
    }
    if bytes.starts_with(GIF87) || bytes.starts_with(GIF89) {
        return true;
    }
    // RIFF....WEBP — the four size bytes in between are not fixed.
    bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP"
}

/// Write via a temporary file in the same directory, then rename over the target.
///
/// `rename` is atomic on both NTFS and POSIX, so a reader never sees a partial
/// file. It matters here because two batches can be caching the same URL at
/// once, and a truncated read would be inlined as a broken image that survives
/// every later launch — the cache is written once and never checked again.
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    let parent = path
        .parent()
        .ok_or_else(|| "icon cache path has no directory".to_string())?;
    let temp = parent.join(format!(
        ".{}.{}.tmp",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));

    fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    if let Err(error) = fs::rename(&temp, path) {
        // Best effort: a leftover temp file is harmless, but it should not
        // outlive the failure that produced it.
        let _ = fs::remove_file(&temp);
        return Err(error.to_string());
    }
    Ok(())
}

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

    as_data_uri(&candidate)
}

fn summarise(instance_dir: &Path, cache_dir: Option<&Path>) -> Result<PackSummary, String> {
    let instance_file = instance_dir.join("minecraftinstance.json");
    let text = fs::read_to_string(&instance_file).map_err(|e| e.to_string())?;
    let header: InstanceHeader = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let thumbnail = header
        .installed_modpack
        .and_then(|modpack| modpack.thumbnail_url)
        .as_deref()
        .and_then(validate_icon_url);

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
        // A local image the user chose always wins: it is what CurseForge itself
        // shows, and it needs no network. The CDN thumbnail is the fallback, and
        // only ever as an already-cached file — the scan does not fetch.
        icon: read_icon(instance_dir, header.profile_image_path.as_deref())
            .or_else(|| cached_icon(cache_dir, thumbnail.as_deref()?)),
        icon_url: thumbnail,
        project_id: header.project_id.filter(|id| *id > 0),
    })
}

pub(crate) fn scan_library(
    instances_dir: Option<String>,
    cache_dir: Option<&Path>,
) -> Result<PackLibrary, String> {
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
        match summarise(&path, cache_dir) {
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
    packs.sort_by_key(|pack| pack.name.to_lowercase());

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
        assert_eq!(
            pretty_loader("neoforge-21.1.228").as_deref(),
            Some("NeoForge")
        );
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

        let library = scan_library(Some(temp.path().to_string_lossy().into_owned()), None)
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

        let library = scan_library(Some(temp.path().to_string_lossy().into_owned()), None)
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
        fs::write(
            temp.path().join("FTB Evolution (1)/cemm-manifest.json"),
            "{}",
        )
        .expect("stray");

        let library = scan_library(Some(temp.path().to_string_lossy().into_owned()), None)
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

        let library = scan_library(Some(temp.path().to_string_lossy().into_owned()), None)
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
        let result = scan_library(Some(missing.to_string_lossy().into_owned()), None);
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

        let library = scan_library(Some(temp.path().to_string_lossy().into_owned()), None)
            .expect("scan should succeed");

        assert_eq!(library.packs.len(), 1);
        assert_eq!(
            library.packs[0].icon, None,
            "an icon path escaping its instance must not be read"
        );
    }

    #[test]
    fn icon_url_allowlist_refuses_anything_but_curseforge_over_https() {
        for bad in [
            "http://media.forgecdn.net/avatars/1/2.png",
            "https://evil.example.com/avatars/1/2.png",
            "https://media.forgecdn.net.evil.example.com/1.png",
            "file:///etc/passwd",
            "not a url",
            "",
        ] {
            assert!(validate_icon_url(bad).is_none(), "expected '{bad}' refused");
        }
        assert!(validate_icon_url(
            "https://media.forgecdn.net/avatars/thumbnails/1182/438/256/256/1.png"
        )
        .is_some());
    }

    #[test]
    fn the_minecraft_root_is_read_from_a_json_document_stored_as_a_json_string() {
        let storage = serde_json::json!({
            "minecraft-settings": serde_json::to_string(&serde_json::json!({
                "minecraftRoot": "D:\\Games\\curseforge\\minecraft",
                "allocatedMemory": 8192
            }))
            .expect("inner document"),
            "something-else": { "unrelated": true }
        })
        .to_string();

        assert_eq!(
            minecraft_root_from_storage(&storage).as_deref(),
            Some("D:\\Games\\curseforge\\minecraft")
        );
    }

    #[test]
    fn a_reshaped_storage_file_reports_nothing_rather_than_guessing() {
        // Each of these is a shape a CurseForge update could plausibly move to,
        // and every one of them has to end at the manual folder picker rather
        // than at a wrong path or a panic.
        let settings = |body: serde_json::Value| {
            serde_json::json!({ "minecraft-settings": body.to_string() }).to_string()
        };

        for storage in [
            // Not JSON at all.
            "".to_string(),
            "not json".to_string(),
            // The key is gone.
            serde_json::json!({ "other": "{}" }).to_string(),
            // Nested as a real object instead of a string — the likeliest change.
            serde_json::json!({ "minecraft-settings": { "minecraftRoot": "D:\\x" } }).to_string(),
            // The inner string is not a JSON document.
            serde_json::json!({ "minecraft-settings": "still not json" }).to_string(),
            // The root is missing, the wrong type, empty, or only whitespace.
            settings(serde_json::json!({ "allocatedMemory": 8192 })),
            settings(serde_json::json!({ "minecraftRoot": 42 })),
            settings(serde_json::json!({ "minecraftRoot": "" })),
            settings(serde_json::json!({ "minecraftRoot": "   " })),
        ] {
            assert_eq!(
                minecraft_root_from_storage(&storage),
                None,
                "expected no root from {storage}"
            );
        }
    }

    #[test]
    fn groups_without_an_id_or_a_name_are_dropped_rather_than_shown_blank() {
        let groups = parse_groups(
            &serde_json::json!([
                { "id": "a1", "name": "Modded" },
                { "id": "b2", "name": null },
                { "name": "No id at all" },
                { "id": "", "name": "Empty id" },
                { "id": "c3", "name": "" },
                { "id": "d4", "name": "Servers" }
            ])
            .to_string(),
        );

        assert_eq!(
            groups,
            vec![
                PackGroup {
                    id: "a1".into(),
                    name: "Modded".into()
                },
                PackGroup {
                    id: "d4".into(),
                    name: "Servers".into()
                },
            ]
        );

        // A file of the wrong shape costs the library its pills and nothing more.
        assert!(parse_groups("{}").is_empty());
        assert!(parse_groups("not json").is_empty());
        assert!(parse_groups("[]").is_empty());
    }

    #[test]
    fn only_real_image_signatures_are_accepted_into_the_cache() {
        assert!(looks_like_image(&[
            0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00
        ]));
        assert!(looks_like_image(&[0xFF, 0xD8, 0xFF, 0xE0]));
        assert!(looks_like_image(b"GIF89a....."));
        assert!(looks_like_image(b"RIFF\x00\x00\x00\x00WEBPVP8 "));

        // The cases this exists for: the MIME is taken from the URL's extension,
        // so anything served under a .png that is not one would otherwise be
        // cached and inlined as a permanently broken tile.
        assert!(!looks_like_image(b"<!DOCTYPE html><html>login</html>"));
        assert!(!looks_like_image(b"{\"error\":\"not found\"}"));
        assert!(!looks_like_image(b"RIFF\x00\x00\x00\x00WAVEfmt "));
        assert!(!looks_like_image(b""));
        assert!(!looks_like_image(b"RIFF"));
    }

    #[test]
    fn an_interrupted_cache_write_leaves_no_partial_file_behind() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = dir.path().join("icon.png");

        write_atomically(&target, b"first").expect("should write");
        assert_eq!(fs::read(&target).expect("read"), b"first");

        // Overwriting is the concurrent case: a second batch caching the same URL
        // must not be observable as a truncated file.
        write_atomically(&target, b"second-and-longer").expect("should overwrite");
        assert_eq!(fs::read(&target).expect("read"), b"second-and-longer");

        // Nothing left in the cache directory but the icon itself.
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .expect("list")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name())
            .filter(|name| name != "icon.png")
            .collect();
        assert!(leftovers.is_empty(), "temp files remained: {leftovers:?}");
    }

    #[test]
    fn cache_file_name_is_stable_distinct_and_cannot_escape_the_cache_dir() {
        let a = cache_file_name(
            "https://media.forgecdn.net/avatars/thumbnails/1182/438/256/256/638755918649288941.png",
        )
        .expect("should name");
        let b = cache_file_name(
            "https://media.forgecdn.net/avatars/thumbnails/1182/438/256/256/638755918649288942.png",
        )
        .expect("should name");

        assert_ne!(a, b, "different assets must not share a cache entry");
        assert_eq!(
            a,
            cache_file_name(
                "https://media.forgecdn.net/avatars/thumbnails/1182/438/256/256/638755918649288941.png"
            )
            .expect("should name"),
            "the same asset must map to the same file every run"
        );
        assert!(a.ends_with(".png"));
        // Nothing that could climb out of the cache directory survives.
        assert!(!a.contains('/'));
        assert!(!a.contains('\\'));
        assert!(!a.contains(".."));

        // A URL that is not an image at all has no cache entry.
        assert_eq!(
            cache_file_name("https://media.forgecdn.net/files/1/2.zip"),
            None
        );
    }

    #[test]
    fn a_cached_curseforge_thumbnail_is_used_without_any_fetch() {
        let temp = tempfile::tempdir().expect("temp dir");
        let cache = temp.path().join("cache");
        fs::create_dir_all(&cache).expect("cache dir");
        let url =
            "https://media.forgecdn.net/avatars/thumbnails/1182/438/256/256/638755918649288941.png";
        fs::write(
            cache.join(cache_file_name(url).expect("name")),
            b"pretend-png-bytes",
        )
        .expect("plant cached icon");

        let instances = temp.path().join("instances");
        write_instance(
            &instances.join("All the Mods 10"),
            serde_json::json!({
                "name": "All the Mods 10",
                "installedModpack": { "thumbnailUrl": url },
                "installedAddons": []
            }),
        );

        let library = scan_library(
            Some(instances.to_string_lossy().into_owned()),
            Some(cache.as_path()),
        )
        .expect("scan should succeed");

        assert_eq!(library.packs.len(), 1);
        assert!(
            library.packs[0]
                .icon
                .as_deref()
                .expect("cached icon should be used")
                .starts_with("data:image/png;base64,"),
            "a previously cached thumbnail must render with no network at all"
        );
        assert_eq!(library.packs[0].icon_url.as_deref(), Some(url));
    }

    #[test]
    fn an_uncached_thumbnail_is_reported_but_never_fetched_by_the_scan() {
        let temp = tempfile::tempdir().expect("temp dir");
        let url =
            "https://media.forgecdn.net/avatars/thumbnails/1182/438/256/256/638755918649288941.png";
        write_instance(
            &temp.path().join("All the Mods 10"),
            serde_json::json!({
                "name": "All the Mods 10",
                "installedModpack": { "thumbnailUrl": url },
                "installedAddons": []
            }),
        );

        let library = scan_library(Some(temp.path().to_string_lossy().into_owned()), None)
            .expect("scan should succeed");

        // The scan opens a screen that must work offline, so it hands the URL
        // back rather than going and getting it.
        assert_eq!(library.packs[0].icon, None);
        assert_eq!(library.packs[0].icon_url.as_deref(), Some(url));
    }

    #[test]
    fn a_local_image_wins_over_the_curseforge_thumbnail() {
        let temp = tempfile::tempdir().expect("temp dir");
        let instance = temp.path().join("Pack");
        fs::create_dir_all(instance.join("profileImage")).expect("image dir");
        let icon = instance.join("profileImage").join("icon.png");
        fs::write(&icon, b"local-bytes").expect("write icon");
        write_instance(
            &instance,
            serde_json::json!({
                "name": "Pack",
                "profileImagePath": icon.to_string_lossy(),
                "installedModpack": {
                    "thumbnailUrl": "https://media.forgecdn.net/avatars/thumbnails/1/2/3/4/5.png"
                },
                "installedAddons": []
            }),
        );

        let library = scan_library(Some(temp.path().to_string_lossy().into_owned()), None)
            .expect("scan should succeed");

        assert!(library.packs[0].icon.is_some(), "the local image is used");
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

        let library = scan_library(Some(temp.path().to_string_lossy().into_owned()), None)
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

        let library = scan_library(Some(temp.path().to_string_lossy().into_owned()), None)
            .expect("scan should succeed");

        let icon = library.packs[0]
            .icon
            .as_deref()
            .expect("icon should inline");
        assert!(icon.starts_with("data:image/png;base64,"));
    }
}
