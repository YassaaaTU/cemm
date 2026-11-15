//! Turning an admin's profile into a release, and refusing to when it is not one.
//!
//! A legacy publish ships whatever CurseForge's record lists plus whichever
//! config files the admin remembered to tick. Both halves fail quietly: a jar
//! with no CurseForge record is simply absent, and a script nobody selected
//! is simply not sent.
//!
//! Capture works the other way round. The admin states once, in
//! `cemm-pack.json`, what the pack consists of — its identity, its content
//! roots, its pins, the settings it requires — and capture reads the profile
//! against that statement. Anything on disk the statement does not account
//! for is an error, not an omission. The result is a complete `Release` and a
//! report of everything that stands between it and publication.

use crate::composables::release::{
    curseforge_fingerprint, digest_file, has_conflict_markers, protected_reason, sha256_hex,
    three_way, validate_release, validate_release_path, AddonCategory, AdoptionRule, FilePolicy,
    FileRole, FileSource, Inventory, InventoryAddon, OfficialBase, Release, ReleaseAddon,
    ReleaseFile, RequiredSetting, RetiredItem, Role, ServerRelease, ThreeWayReport, CAP_ADDONS,
    CAP_FILES, CAP_SETTINGS, MAX_MANAGED_FILE_BYTES, MAX_RELEASE_ADDON_BYTES, RELEASE_MIN_CEMM,
    RELEASE_SCHEMA, SPEC_FILE_NAME,
};
use crate::release_install::{apply_json_setting, SettingOutcome};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use ts_rs::TS;

const MAX_CAPTURED_FILES: usize = 50_000;
/// How many earlier hashes of one path a release remembers.
const MAX_PREVIOUS_HASHES: usize = 24;
const CONFLICT_SCAN_LIMIT: u64 = 4 * 1024 * 1024;

/// Never content, wherever a root happens to contain them.
const ALWAYS_EXCLUDED: [&str; 9] = [
    "*.snbt_merged",
    "*.bak",
    "*.tmp",
    "*.log",
    "*.log.gz",
    "*.pyc",
    "Thumbs.db",
    ".DS_Store",
    "desktop.ini",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export, export_to = "generated/")]
pub struct PinSpec {
    #[ts(type = "number")]
    pub project_id: u64,
    /// The exact file the pin holds. Optional, but stating it turns "whatever
    /// is installed" into a checked decision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub file_id: Option<u64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
}

fn default_true() -> bool {
    true
}

/// A file or folder of the profile that is part of the pack.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export, export_to = "generated/")]
pub struct ContentRoot {
    pub path: String,
    #[serde(default)]
    pub role: FileRole,
    #[serde(default)]
    pub policy: FilePolicy,
    /// Whether the bytes are uploaded. Switched off, the files are still
    /// inventoried, verified and owned — they are just not hosted, which is
    /// the honest setting for content the admin may not redistribute.
    #[serde(default = "default_true")]
    pub distribute: bool,
    /// Glob patterns, relative to the profile, left out of this root.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
}

/// The admin's standing description of a pack lineage: `cemm-pack.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(deny_unknown_fields)]
#[ts(export, export_to = "generated/")]
pub struct ReleaseSpec {
    pub lineage: String,
    pub name: String,
    #[serde(default = "default_role")]
    pub role: Role,
    pub base: OfficialBase,
    /// Stated to be checked; read from the profile when left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub minecraft: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub loader: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub server_release: Option<ServerRelease>,
    #[serde(default)]
    pub adoption: AdoptionRule,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_from: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pins: Vec<PinSpec>,
    /// CurseForge projects in the profile that are not part of the pack.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(type = "number[]")]
    pub exclude_addons: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<ContentRoot>,
    /// Files in the addon folders that are deliberately not part of the pack.
    /// Naming them is what distinguishes a decision from an oversight.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub local_only: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settings: Vec<RequiredSetting>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manual_steps: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known_issues: Vec<String>,
}

fn default_role() -> Role {
    Role::Client
}

/// Reads `cemm-pack.json` from a profile. `None` means the pack has no release
/// line, and publishing it stays the legacy flow it always was.
pub fn read_release_spec(modpack_path: String) -> Result<Option<ReleaseSpec>, String> {
    let path = Path::new(&modpack_path).join(SPEC_FILE_NAME);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Failed to read {}: {error}", path.display())),
    };
    serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map(Some)
        .map_err(|error| format!("{SPEC_FILE_NAME} is not valid: {error}"))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRequest {
    pub modpack_path: String,
    pub release_id: String,
    #[serde(default)]
    pub notes: String,
    /// The release this one follows. Its history is what lets a player who
    /// skipped it, or never had CEMM track it, still be brought up to date.
    #[serde(default)]
    pub previous: Option<Release>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct CaptureSummary {
    pub addons: u32,
    pub pinned: u32,
    pub files: u32,
    pub hosted_files: u32,
    pub settings: u32,
    pub retired: u32,
    /// Files whose bytes this release introduces.
    pub new_payloads: u32,
    #[ts(type = "number")]
    pub new_payload_bytes: u64,
    pub addons_added: Vec<String>,
    pub addons_updated: Vec<String>,
    pub addons_removed: Vec<String>,
    pub files_added: u32,
    pub files_changed: u32,
    pub files_removed: u32,
    /// Paths inside the content roots that were left out on purpose.
    pub skipped_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "generated/")]
pub struct ReleaseDraft {
    pub release: Release,
    /// Anything here means the draft must not be published.
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub summary: CaptureSummary,
}

// ---------------------------------------------------------------------------
// Globs
// ---------------------------------------------------------------------------

fn glob_segment(pattern: &[u8], text: &[u8]) -> bool {
    match (pattern.first(), text.first()) {
        (None, None) => true,
        (Some(b'*'), _) => {
            glob_segment(&pattern[1..], text)
                || (!text.is_empty() && glob_segment(pattern, &text[1..]))
        }
        (Some(b'?'), Some(_)) => glob_segment(&pattern[1..], &text[1..]),
        (Some(expected), Some(actual)) if expected.eq_ignore_ascii_case(actual) => {
            glob_segment(&pattern[1..], &text[1..])
        }
        _ => false,
    }
}

fn glob_path(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => {
            glob_path(rest, path) || (!path.is_empty() && glob_path(pattern, &path[1..]))
        }
        Some((segment, rest)) => path.split_first().is_some_and(|(first, remaining)| {
            glob_segment(segment.as_bytes(), first.as_bytes()) && glob_path(rest, remaining)
        }),
    }
}

/// `*` within a segment, `**` across segments, `?` for one character. A
/// pattern with no slash matches a file name at any depth; a pattern naming a
/// folder also matches everything inside it.
pub(crate) fn glob_matches(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim().trim_matches('/');
    if pattern.is_empty() {
        return false;
    }
    let path_segments: Vec<&str> = path.split('/').collect();
    if !pattern.contains('/') {
        return path_segments
            .last()
            .is_some_and(|name| glob_segment(pattern.as_bytes(), name.as_bytes()));
    }
    let pattern_segments: Vec<&str> = pattern.split('/').collect();
    if glob_path(&pattern_segments, &path_segments) {
        return true;
    }
    let mut inside = pattern_segments;
    inside.push("**");
    glob_path(&inside, &path_segments)
}

fn excluded_by(patterns: &[String], path: &str) -> bool {
    ALWAYS_EXCLUDED
        .iter()
        .any(|pattern| glob_matches(pattern, path))
        || patterns.iter().any(|pattern| glob_matches(pattern, path))
}

// ---------------------------------------------------------------------------
// Reading a profile
// ---------------------------------------------------------------------------

struct Walked {
    files: Vec<String>,
    skipped: u32,
}

/// Every regular file under `root`, as profile-relative forward-slash paths.
/// Symlinks are not followed: a link inside a content root must not be able to
/// pull files from outside the profile into a release.
fn walk_root(
    profile: &Path,
    root: &str,
    exclude: &[String],
    warnings: &mut Vec<String>,
) -> Result<Walked, String> {
    let start = profile.join(root);
    let metadata = std::fs::symlink_metadata(&start)
        .map_err(|error| format!("Content path '{root}' cannot be read: {error}"))?;
    let mut walked = Walked {
        files: Vec::new(),
        skipped: 0,
    };
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Content path '{root}' is a link; CEMM only publishes real files"
        ));
    }
    if metadata.is_file() {
        walked.files.push(root.to_string());
        return Ok(walked);
    }

    let mut pending: Vec<(PathBuf, String)> = vec![(start, root.trim_end_matches('/').to_string())];
    while let Some((directory, relative)) = pending.pop() {
        let entries = std::fs::read_dir(&directory)
            .map_err(|error| format!("Failed to read {}: {error}", directory.display()))?;
        for entry in entries {
            let entry = entry
                .map_err(|error| format!("Failed to read {}: {error}", directory.display()))?;
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                warnings.push(format!(
                    "A file under '{relative}' has a name that is not valid Unicode and was left out."
                ));
                continue;
            };
            let child = format!("{relative}/{name}");
            let kind = entry
                .file_type()
                .map_err(|error| format!("Failed to inspect {child}: {error}"))?;
            if kind.is_symlink() {
                warnings.push(format!("'{child}' is a link and was left out."));
                continue;
            }
            if protected_reason(&child).is_some() || excluded_by(exclude, &child) {
                walked.skipped += 1;
                continue;
            }
            if kind.is_dir() {
                pending.push((entry.path(), child));
            } else if kind.is_file() {
                if walked.files.len() >= MAX_CAPTURED_FILES {
                    return Err(format!(
                        "More than {MAX_CAPTURED_FILES} files under the content roots; narrow them or add excludes"
                    ));
                }
                walked.files.push(child);
            }
        }
    }
    walked.files.sort();
    Ok(walked)
}

struct InstalledRecord {
    project_id: u64,
    name: String,
    category: AddonCategory,
    file_id: u64,
    file_name: String,
    enabled: bool,
    locked: bool,
    download_url: Option<String>,
    website_url: Option<String>,
    file_length: Option<u64>,
    fingerprint: Option<u32>,
    sha1: Option<String>,
    requires: Vec<u64>,
}

struct InstanceFacts {
    minecraft: String,
    loader: String,
    records: Vec<InstalledRecord>,
}

fn read_instance_facts(profile: &Path) -> Result<InstanceFacts, String> {
    let path = profile.join("minecraftinstance.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("{} is not valid JSON: {error}", path.display()))?;

    let mut records = Vec::new();
    for addon in value
        .get("installedAddons")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(category) = addon
            .get("modFolderPath")
            .and_then(Value::as_str)
            .and_then(|folder| folder.rsplit(['/', '\\']).next())
            .and_then(AddonCategory::from_directory)
        else {
            // The modpack archive itself, recorded in `downloads`, and anything
            // else that is not addon content.
            continue;
        };
        let (Some(project_id), Some(installed)) = (
            addon.get("addonID").and_then(Value::as_u64),
            addon.get("installedFile"),
        ) else {
            continue;
        };
        let (Some(file_id), Some(file_name)) = (
            installed.get("id").and_then(Value::as_u64),
            installed.get("fileName").and_then(Value::as_str),
        ) else {
            continue;
        };
        let on_disk = addon
            .get("fileNameOnDisk")
            .and_then(Value::as_str)
            .unwrap_or(file_name);
        records.push(InstalledRecord {
            project_id,
            name: addon
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(file_name)
                .to_string(),
            category,
            file_id,
            file_name: file_name.to_string(),
            enabled: addon.get("isEnabled").and_then(Value::as_bool) != Some(false)
                && !on_disk.ends_with(".disabled"),
            locked: addon.get("isLocked").and_then(Value::as_bool) == Some(true),
            download_url: installed
                .get("downloadUrl")
                .and_then(Value::as_str)
                .filter(|url| !url.trim().is_empty())
                .map(str::to_string),
            website_url: addon
                .get("webSiteURL")
                .and_then(Value::as_str)
                .filter(|url| !url.is_empty())
                .map(str::to_string),
            file_length: installed.get("fileLength").and_then(Value::as_u64),
            fingerprint: installed
                .get("packageFingerprint")
                .and_then(Value::as_u64)
                .and_then(|fingerprint| u32::try_from(fingerprint).ok()),
            sha1: installed
                .get("hashes")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .find(|hash| hash.get("type").and_then(Value::as_u64) == Some(1))
                .and_then(|hash| hash.get("value").and_then(Value::as_str))
                .map(str::to_lowercase),
            requires: installed
                .get("dependencies")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                // CurseForge's relation type 3 is "required dependency".
                .filter(|dependency| dependency.get("type").and_then(Value::as_u64) == Some(3))
                .filter_map(|dependency| dependency.get("addonId").and_then(Value::as_u64))
                .collect(),
        });
    }

    Ok(InstanceFacts {
        minecraft: value
            .get("gameVersion")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        loader: value
            .pointer("/baseModLoader/name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        records,
    })
}

fn looks_like_text(path: &str) -> bool {
    let lower = path.to_lowercase();
    [
        ".js",
        ".ts",
        ".json",
        ".json5",
        ".snbt",
        ".toml",
        ".txt",
        ".cfg",
        ".mcmeta",
        ".lang",
        ".properties",
        ".md",
        ".yaml",
        ".yml",
        ".ini",
        ".conf",
        ".xml",
        ".zs",
        ".mcfunction",
    ]
    .iter()
    .any(|suffix| lower.ends_with(suffix))
}

fn is_under(root: &str, path: &str) -> bool {
    let root = root.trim_end_matches('/').to_lowercase();
    let path = path.to_lowercase();
    path == root || path.starts_with(&format!("{root}/"))
}

// ---------------------------------------------------------------------------
// Capture
// ---------------------------------------------------------------------------

/// Reads a profile into a complete release and reports what stands in the way
/// of publishing it. Changes nothing, on disk or in the repository.
pub fn capture_release(request: CaptureRequest) -> Result<ReleaseDraft, String> {
    let profile = PathBuf::from(&request.modpack_path);
    let spec = read_release_spec(request.modpack_path.clone())?.ok_or_else(|| {
        format!(
            "This profile has no {SPEC_FILE_NAME}, so it is not set up to publish releases. Add one describing the pack, or publish a legacy update instead."
        )
    })?;
    let facts = read_instance_facts(&profile)?;
    let previous = request.previous.as_ref();

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut summary = CaptureSummary::default();

    // ---- Identity -----------------------------------------------------------------
    let pick =
        |stated: &Option<String>, actual: &str, what: &str, errors: &mut Vec<String>| match stated
            .as_deref()
        {
            Some(stated) if stated != actual => {
                errors.push(format!(
                    "{SPEC_FILE_NAME} says {what} is {stated}, but this profile is {actual}."
                ));
                stated.to_string()
            }
            _ => actual.to_string(),
        };
    let minecraft = pick(&spec.minecraft, &facts.minecraft, "Minecraft", &mut errors);
    let loader = pick(&spec.loader, &facts.loader, "the mod loader", &mut errors);

    let mut sequence = 1;
    if let Some(previous) = previous {
        if previous.lineage != spec.lineage {
            errors.push(format!(
                "The previous release belongs to '{}', not '{}'.",
                previous.lineage, spec.lineage
            ));
        }
        if previous.release_id == request.release_id {
            errors.push(format!(
                "Release id '{}' is the previous release's. Every release needs its own.",
                request.release_id
            ));
        }
        sequence = previous.sequence + 1;
        if previous.base != spec.base {
            warnings.push(format!(
                "The official base changes from file {} to file {}. Make sure the three-way review of that upgrade is finished.",
                previous.base.file_id, spec.base.file_id
            ));
        }
    }

    // ---- Addons ---------------------------------------------------------------------
    let excluded_projects: HashSet<u64> = spec.exclude_addons.iter().copied().collect();
    let mut addons: Vec<ReleaseAddon> = Vec::new();
    let mut recorded_files: HashSet<String> = HashSet::new();
    let mut seen_projects: HashSet<u64> = HashSet::new();

    for record in &facts.records {
        let relative = format!("{}/{}", record.category.directory(), record.file_name);
        recorded_files.insert(relative.to_lowercase());
        recorded_files.insert(format!("{relative}.disabled").to_lowercase());
        if excluded_projects.contains(&record.project_id) {
            continue;
        }
        if !seen_projects.insert(record.project_id) {
            errors.push(format!(
                "CurseForge lists project {} ('{}') more than once in this profile.",
                record.project_id, record.name
            ));
            continue;
        }

        let on_disk = if record.enabled {
            profile.join(&relative)
        } else {
            profile.join(format!("{relative}.disabled"))
        };
        let bytes = match std::fs::metadata(&on_disk) {
            Ok(metadata) if metadata.len() > MAX_RELEASE_ADDON_BYTES => {
                errors.push(format!("'{}' is larger than CEMM installs.", record.name));
                continue;
            }
            Ok(_) => match std::fs::read(&on_disk) {
                Ok(bytes) => bytes,
                Err(error) => {
                    errors.push(format!("Failed to read {relative}: {error}"));
                    continue;
                }
            },
            Err(_) => {
                errors.push(format!(
                    "CurseForge lists '{}' ({}), but the file is not in the profile. Repair or remove it in CurseForge first.",
                    record.name, relative
                ));
                continue;
            }
        };

        // The launcher's own evidence that the jar on disk is the file its
        // record names. A mismatch is exactly the drift that makes records
        // untrustworthy, and it must not be published under that file id.
        let size = bytes.len() as u64;
        let fingerprint = curseforge_fingerprint(&bytes);
        if record.file_length.is_some_and(|length| length != size)
            || record
                .fingerprint
                .is_some_and(|expected| expected != fingerprint)
        {
            errors.push(format!(
                "{relative} is not the file CurseForge's record describes (file id {}). The jar was replaced by hand or the record is stale; let CurseForge rescan the profile, then check again.",
                record.file_id
            ));
        }

        let download_url = record
            .download_url
            .clone()
            .filter(|url| crate::installer::validate_addon_download_url(&record.name, url).is_ok());
        if download_url.is_none() {
            warnings.push(format!(
                "'{}' has no CurseForge download link. Players who do not already have {} must download it by hand before the release will apply.",
                record.name, record.file_name
            ));
        }

        let pin = spec
            .pins
            .iter()
            .find(|pin| pin.project_id == record.project_id);
        if let Some(pin) = pin {
            if pin.file_id.is_some_and(|file_id| file_id != record.file_id) {
                errors.push(format!(
                    "'{}' is pinned to file {} in {SPEC_FILE_NAME}, but this profile has file {}.",
                    record.name,
                    pin.file_id.unwrap_or_default(),
                    record.file_id
                ));
            }
            if !record.enabled {
                errors.push(format!("'{}' is pinned but switched off.", record.name));
            }
            if !record.locked {
                warnings.push(format!(
                    "'{}' is pinned in {SPEC_FILE_NAME} but not locked in your own CurseForge profile.",
                    record.name
                ));
            }
        }

        addons.push(ReleaseAddon {
            project_id: record.project_id,
            file_id: record.file_id,
            name: record.name.clone(),
            category: record.category,
            file_name: record.file_name.clone(),
            enabled: record.enabled,
            size,
            sha256: sha256_hex(&bytes),
            sha1: record.sha1.clone(),
            fingerprint: Some(fingerprint),
            download_url,
            website_url: record.website_url.clone(),
            pinned: pin.is_some(),
            requires: record.requires.clone(),
        });
    }
    for pin in &spec.pins {
        if !addons
            .iter()
            .any(|addon| addon.project_id == pin.project_id)
        {
            errors.push(format!(
                "{SPEC_FILE_NAME} pins CurseForge project {}, which is not in this profile.",
                pin.project_id
            ));
        }
    }
    addons.sort_by(|a, b| {
        (a.category, a.file_name.to_lowercase()).cmp(&(b.category, b.file_name.to_lowercase()))
    });

    // ---- Managed files ----------------------------------------------------------------
    let mut files: BTreeMap<String, ReleaseFile> = BTreeMap::new();
    for root in &spec.content {
        let root_path = root.path.trim().trim_matches('/').to_string();
        if let Err(error) = validate_release_path(&root_path) {
            errors.push(error);
            continue;
        }
        if let Some(reason) = protected_reason(&root_path) {
            errors.push(format!(
                "Content path '{root_path}' cannot be published: {reason}."
            ));
            continue;
        }
        let walked = match walk_root(&profile, &root_path, &root.exclude, &mut warnings) {
            Ok(walked) => walked,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        summary.skipped_files += walked.skipped;

        for path in walked.files {
            if let Err(error) = validate_release_path(&path) {
                errors.push(error);
                continue;
            }
            if recorded_files.contains(&path.to_lowercase()) {
                errors.push(format!(
                    "{path} is a CurseForge addon and is already part of the release as one. Remove it from the content paths."
                ));
                continue;
            }
            let absolute = profile.join(&path);
            let digest = match digest_file(&absolute) {
                Ok(digest) => digest,
                Err(error) => {
                    errors.push(format!("Failed to read {path}: {error}"));
                    continue;
                }
            };
            if digest.size > MAX_MANAGED_FILE_BYTES {
                errors.push(format!("{path} is larger than CEMM distributes."));
                continue;
            }
            if looks_like_text(&path) && digest.size <= CONFLICT_SCAN_LIMIT {
                if let Ok(text) = std::fs::read_to_string(&absolute) {
                    if has_conflict_markers(&text) {
                        errors.push(format!(
                            "{path} still contains merge-conflict markers. Finish resolving it before publishing."
                        ));
                    }
                }
            }

            let before = previous.and_then(|previous| {
                previous
                    .files
                    .iter()
                    .find(|file| file.path.eq_ignore_ascii_case(&path))
            });
            let unchanged = before.is_some_and(|before| before.sha256 == digest.sha256);
            let hosted =
                root.distribute || (unchanged && before.is_some_and(|before| before.hosted));
            if previous.is_some() && !unchanged && !hosted {
                errors.push(format!(
                    "{path} is new or changed in this release, but its content root is not distributed, so no player could receive it. Distribute that root, or revert the change."
                ));
            }
            let mut earlier: Vec<String> = Vec::new();
            if let Some(before) = before {
                if before.sha256 != digest.sha256 {
                    earlier.push(before.sha256.clone());
                }
                earlier.extend(
                    before
                        .previous
                        .iter()
                        .filter(|hash| **hash != digest.sha256)
                        .cloned(),
                );
                earlier.dedup();
                earlier.truncate(MAX_PREVIOUS_HASHES);
            }

            let lower = path.to_lowercase();
            let entry = ReleaseFile {
                path: path.clone(),
                sha256: digest.sha256,
                size: digest.size,
                source: if lower.ends_with(".jar")
                    || lower.ends_with(".dll")
                    || lower.ends_with(".exe")
                {
                    FileSource::LocalBinary
                } else {
                    FileSource::Authored
                },
                policy: root.policy,
                role: root.role,
                hosted,
                previous: earlier,
            };
            if files.insert(path.clone(), entry).is_some() {
                errors.push(format!(
                    "{path} is covered by more than one content path in {SPEC_FILE_NAME}."
                ));
            }
        }
    }

    // ---- Nothing in the addon folders may be unaccounted for -----------------------------
    let mut unaccounted_errors: Vec<String> = Vec::new();
    let mut personal_packs: Vec<String> = Vec::new();
    for category in AddonCategory::ALL {
        let directory = profile.join(category.directory());
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();
        names.sort();
        for name in names {
            let relative = format!("{}/{name}", category.directory());
            let lower = relative.to_lowercase();
            let accounted = recorded_files.contains(&lower)
                || files.keys().any(|path| is_under(&relative, path))
                || spec
                    .content
                    .iter()
                    .any(|root| is_under(&root.path, &relative))
                || spec
                    .local_only
                    .iter()
                    .any(|pattern| glob_matches(pattern, &relative))
                || excluded_by(&[], &relative);
            if accounted {
                continue;
            }
            let is_pack = lower.ends_with(".jar")
                || lower.ends_with(".zip")
                || directory.join(&name).is_dir();
            match category {
                // Mods are code and data packs are gameplay: neither may go
                // missing from a release because nobody listed it.
                AddonCategory::Mods => unaccounted_errors.push(relative),
                AddonCategory::Datapacks if is_pack => unaccounted_errors.push(relative),
                // Shaders and resource packs are a player's own choice far
                // more often than they are pack content. Worth a mention,
                // not worth stopping for.
                AddonCategory::Shaderpacks | AddonCategory::Resourcepacks if is_pack => {
                    personal_packs.push(relative)
                }
                _ => {}
            }
        }
    }

    for relative in unaccounted_errors {
        errors.push(format!(
            "{relative} is in the profile but CurseForge has no record of it and {SPEC_FILE_NAME} does not mention it. List it under \"content\" to ship it, or under \"local_only\" to leave it out on purpose."
        ));
    }
    if !personal_packs.is_empty() {
        warnings.push(format!(
            "{} shader or resource pack(s) here are not from CurseForge and not listed in {SPEC_FILE_NAME}, so they are treated as your own and left out ({}{}). List one under \"content\" if the pack needs it.",
            personal_packs.len(),
            personal_packs.iter().take(3).cloned().collect::<Vec<_>>().join(", "),
            if personal_packs.len() > 3 { ", …" } else { "" }
        ));
    }

    // ---- Required settings -----------------------------------------------------------------
    for setting in &spec.settings {
        let shipped = files
            .values()
            .find(|file| file.path.eq_ignore_ascii_case(&setting.path));
        match std::fs::read_to_string(profile.join(&setting.path)) {
            Ok(text) => match apply_json_setting(&text, &setting.pointer, &setting.value) {
                Ok(SettingOutcome::AlreadySet) => {}
                Ok(SettingOutcome::Changed { .. }) => errors.push(format!(
                    "{} requires {} to be {}, but your own profile has it set differently. Set it here first, so the release describes a profile that exists.",
                    setting.path,
                    setting.pointer,
                    serde_json::to_string(&setting.value).unwrap_or_default()
                )),
                Err(problem) => errors.push(format!(
                    "The required setting {} in {} cannot be applied: {problem}.",
                    setting.pointer, setting.path
                )),
            },
            Err(error) => errors.push(format!(
                "The required setting {} is in {}, which cannot be read here: {error}.",
                setting.pointer, setting.path
            )),
        }
        match shipped {
            Some(file) if file.policy == FilePolicy::Managed => errors.push(format!(
                "{} is published as a managed file and also carries a required setting. Publish it with \"policy\": \"default\" so players keep their own settings in it.",
                setting.path
            )),
            Some(_) => {}
            None => warnings.push(format!(
                "{} is not published as a default. A player whose game has not created that file yet cannot apply the release until it has.",
                setting.path
            )),
        }
    }

    // ---- What earlier releases held that this one does not ------------------------------------
    let mut retired: BTreeSet<RetiredItem> = BTreeSet::new();
    if let Some(previous) = previous {
        retired.extend(previous.retired.iter().cloned());
        for before in &previous.addons {
            let path = before.relative_path();
            let kept = addons.iter().any(|addon| {
                addon.relative_path().eq_ignore_ascii_case(&path) && addon.sha256 == before.sha256
            });
            if !kept {
                retired.insert(RetiredItem {
                    path,
                    sha256: before.sha256.clone(),
                    project_id: Some(before.project_id),
                });
            }
        }
        for before in &previous.files {
            if files
                .keys()
                .any(|path| path.eq_ignore_ascii_case(&before.path))
            {
                continue;
            }
            for sha256 in std::iter::once(&before.sha256).chain(before.previous.iter()) {
                retired.insert(RetiredItem {
                    path: before.path.clone(),
                    sha256: sha256.clone(),
                    project_id: None,
                });
            }
        }
    }

    // ---- Dependencies -----------------------------------------------------------------------------
    let shipped_projects: HashSet<u64> = addons
        .iter()
        .filter(|addon| addon.enabled)
        .map(|addon| addon.project_id)
        .collect();
    let previously_shipped: HashSet<u64> = previous
        .map(|previous| {
            previous
                .addons
                .iter()
                .map(|addon| addon.project_id)
                .collect()
        })
        .unwrap_or_default();
    let mut unmet: Vec<String> = Vec::new();
    for addon in addons.iter().filter(|addon| addon.enabled) {
        for required in &addon.requires {
            if shipped_projects.contains(required) {
                continue;
            }
            if previously_shipped.contains(required) {
                errors.push(format!(
                    "'{}' requires CurseForge project {required}, which the previous release had and this one drops. Keep it, replace it, or remove '{}'.",
                    addon.name, addon.name
                ));
            } else {
                unmet.push(addon.name.clone());
            }
        }
    }
    if !unmet.is_empty() {
        unmet.sort();
        unmet.dedup();
        warnings.push(format!(
            "{} addon(s) list a required dependency that is not in the pack ({}{}). That is normal when the dependency is bundled or optional in practice; check any you did not expect.",
            unmet.len(),
            unmet.iter().take(4).cloned().collect::<Vec<_>>().join(", "),
            if unmet.len() > 4 { ", …" } else { "" }
        ));
    }

    // ---- Assemble -----------------------------------------------------------------------------------
    let files: Vec<ReleaseFile> = files.into_values().collect();
    let mut requires = Vec::new();
    if !addons.is_empty() {
        requires.push(CAP_ADDONS.to_string());
    }
    if !files.is_empty() || !retired.is_empty() {
        requires.push(CAP_FILES.to_string());
    }
    if !spec.settings.is_empty() {
        requires.push(CAP_SETTINGS.to_string());
    }

    let release = Release {
        schema: RELEASE_SCHEMA,
        min_cemm: RELEASE_MIN_CEMM.to_string(),
        requires,
        lineage: spec.lineage.clone(),
        release_id: request.release_id.trim().to_string(),
        sequence,
        name: spec.name.clone(),
        role: spec.role,
        base: spec.base.clone(),
        minecraft,
        loader,
        supported_from: spec.supported_from.clone(),
        adoption: spec.adoption.clone(),
        server_release: spec.server_release.clone(),
        notes: request.notes.trim().to_string(),
        manual_steps: spec.manual_steps.clone(),
        known_issues: spec.known_issues.clone(),
        addons,
        files,
        settings: spec.settings.clone(),
        retired: retired.into_iter().collect(),
    };
    if let Err(error) = validate_release(&release) {
        errors.push(error);
    }
    if release.role == Role::Client && spec.server_release.is_none() {
        warnings.push(
            "No matching server release is named in cemm-pack.json. Players will not be told which server build this goes with.".to_string(),
        );
    }

    // ---- Summary ---------------------------------------------------------------------------------------
    summary.addons = release.addons.len() as u32;
    summary.pinned = release.addons.iter().filter(|addon| addon.pinned).count() as u32;
    summary.files = release.files.len() as u32;
    summary.hosted_files = release.files.iter().filter(|file| file.hosted).count() as u32;
    summary.settings = release.settings.len() as u32;
    summary.retired = release.retired.len() as u32;

    let hosted_before: HashSet<&str> = previous
        .map(|previous| {
            previous
                .files
                .iter()
                .filter(|file| file.hosted)
                .map(|file| file.sha256.as_str())
                .collect()
        })
        .unwrap_or_default();
    let mut counted: HashSet<&str> = HashSet::new();
    for file in release.files.iter().filter(|file| file.hosted) {
        if !hosted_before.contains(file.sha256.as_str()) && counted.insert(file.sha256.as_str()) {
            summary.new_payloads += 1;
            summary.new_payload_bytes += file.size;
        }
    }
    if let Some(previous) = previous {
        let before: HashMap<u64, &ReleaseAddon> = previous
            .addons
            .iter()
            .map(|addon| (addon.project_id, addon))
            .collect();
        for addon in &release.addons {
            match before.get(&addon.project_id) {
                None => summary.addons_added.push(addon.name.clone()),
                Some(old) if old.file_id != addon.file_id || old.sha256 != addon.sha256 => {
                    summary.addons_updated.push(format!(
                        "{} ({} → {})",
                        addon.name, old.file_name, addon.file_name
                    ));
                }
                Some(_) => {}
            }
        }
        for addon in &previous.addons {
            if !release
                .addons
                .iter()
                .any(|now| now.project_id == addon.project_id)
            {
                summary.addons_removed.push(addon.name.clone());
            }
        }
        let before_files: HashMap<String, &ReleaseFile> = previous
            .files
            .iter()
            .map(|file| (file.path.to_lowercase(), file))
            .collect();
        for file in &release.files {
            match before_files.get(&file.path.to_lowercase()) {
                None => summary.files_added += 1,
                Some(old) if old.sha256 != file.sha256 => summary.files_changed += 1,
                Some(_) => {}
            }
        }
        summary.files_removed = previous
            .files
            .iter()
            .filter(|old| {
                !release
                    .files
                    .iter()
                    .any(|file| file.path.eq_ignore_ascii_case(&old.path))
            })
            .count() as u32;

        // A pin is a standing decision; moving it should be seen, not slipped in.
        for addon in release.addons.iter().filter(|addon| addon.pinned) {
            if let Some(old) = before.get(&addon.project_id) {
                if old.pinned && old.file_id != addon.file_id {
                    warnings.push(format!(
                        "The pinned addon '{}' moves from {} to {}. Confirm that is intended.",
                        addon.name, old.file_name, addon.file_name
                    ));
                }
            }
        }
    }

    errors.sort();
    errors.dedup();
    warnings.sort();
    warnings.dedup();
    Ok(ReleaseDraft {
        release,
        errors,
        warnings,
        summary,
    })
}

// ---------------------------------------------------------------------------
// Reviewing an official base upgrade
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRequest {
    /// The previous official release, as installed or extracted.
    pub base_path: String,
    /// The custom pack as last released. Its `cemm-pack.json` names the
    /// content roots and pins all three sides are compared on.
    pub custom_path: String,
    /// The new official release.
    pub upstream_path: String,
}

/// Reads one side of the comparison.
///
/// Accepts an installed CurseForge profile (`minecraftinstance.json`) or an
/// extracted CurseForge pack archive (`manifest.json` plus `overrides/`), which
/// is the form an untouched official download comes in.
fn read_inventory(directory: &Path, spec: &ReleaseSpec) -> Result<Inventory, String> {
    let mut inventory = Inventory::default();
    let mut warnings = Vec::new();
    let excluded: HashSet<u64> = spec.exclude_addons.iter().copied().collect();

    let content_root = if directory.join("minecraftinstance.json").is_file() {
        for record in read_instance_facts(directory)?.records {
            if excluded.contains(&record.project_id) {
                continue;
            }
            inventory.addons.insert(
                record.project_id,
                InventoryAddon {
                    file_id: record.file_id,
                    name: record.name,
                    file_name: record.file_name,
                    requires: record.requires,
                },
            );
        }
        directory.to_path_buf()
    } else if directory.join("manifest.json").is_file() {
        let text = std::fs::read_to_string(directory.join("manifest.json"))
            .map_err(|error| format!("Failed to read {}: {error}", directory.display()))?;
        let manifest: Value =
            serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|error| {
                format!(
                    "manifest.json in {} is not valid: {error}",
                    directory.display()
                )
            })?;
        for file in manifest
            .get("files")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let (Some(project_id), Some(file_id)) = (
                file.get("projectID").and_then(Value::as_u64),
                file.get("fileID").and_then(Value::as_u64),
            ) else {
                continue;
            };
            if excluded.contains(&project_id) {
                continue;
            }
            inventory.addons.insert(
                project_id,
                InventoryAddon {
                    file_id,
                    name: format!("project {project_id}"),
                    file_name: format!("file {file_id}"),
                    requires: Vec::new(),
                },
            );
        }
        let overrides = manifest
            .get("overrides")
            .and_then(Value::as_str)
            .unwrap_or("overrides");
        directory.join(overrides)
    } else {
        return Err(format!(
            "{} is neither a CurseForge profile (minecraftinstance.json) nor an extracted pack archive (manifest.json).",
            directory.display()
        ));
    };

    for root in &spec.content {
        let root_path = root.path.trim().trim_matches('/');
        if !content_root.join(root_path).exists() {
            continue;
        }
        for path in walk_root(&content_root, root_path, &root.exclude, &mut warnings)?.files {
            let digest = digest_file(&content_root.join(&path))
                .map_err(|error| format!("Failed to read {path}: {error}"))?;
            inventory.files.insert(path, digest.sha256);
        }
    }
    Ok(inventory)
}

/// Compares the previous official release, the custom pack and the new
/// official release, and says for every path and addon which side to take —
/// or that a person has to decide. Reads three folders; writes nothing.
pub fn review_upgrade(request: ReviewRequest) -> Result<ThreeWayReport, String> {
    let spec = read_release_spec(request.custom_path.clone())?.ok_or_else(|| {
        format!("The custom pack has no {SPEC_FILE_NAME} to say what it consists of.")
    })?;
    let base = read_inventory(Path::new(&request.base_path), &spec)?;
    let custom = read_inventory(Path::new(&request.custom_path), &spec)?;
    let upstream = read_inventory(Path::new(&request.upstream_path), &spec)?;
    let pinned: Vec<u64> = spec.pins.iter().map(|pin| pin.project_id).collect();

    let mut report = three_way(&base, &custom, &upstream, &pinned);
    // Local binaries are workarounds by nature. Whether the new official
    // release makes one redundant is a question about behaviour, not bytes.
    for path in custom
        .files
        .keys()
        .filter(|path| path.to_lowercase().ends_with(".jar"))
    {
        report.review.push(format!(
            "{path} is a local fix. Check whether the new official release already includes what it does before carrying it forward."
        ));
    }
    report.review.sort();
    report.review.dedup();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs_match_names_anywhere_and_paths_from_the_root() {
        assert!(glob_matches(
            "*.snbt_merged",
            "config/ftbquests/quests/chapters/a.snbt_merged"
        ));
        assert!(glob_matches("*.SNBT_MERGED", "a.snbt_merged"));
        assert!(!glob_matches("*.snbt", "a.snbt_merged"));

        assert!(glob_matches(
            "kubejs/exported/**",
            "kubejs/exported/tags/item.json"
        ));
        // Naming a folder excludes what is inside it.
        assert!(glob_matches(
            "kubejs/exported",
            "kubejs/exported/tags/item.json"
        ));
        assert!(glob_matches(
            "config/jei/*.bak.*",
            "config/jei/ingredients.bak.1"
        ));
        assert!(!glob_matches(
            "config/jei/*.bak.*",
            "config/jei/sub/ingredients.bak.1"
        ));
        assert!(glob_matches(
            "config/**/cache.json",
            "config/a/b/cache.json"
        ));
        assert!(glob_matches("config/**/cache.json", "config/cache.json"));
        assert!(glob_matches("mods/local-?.jar", "mods/local-a.jar"));
        assert!(!glob_matches("mods/local-?.jar", "mods/local-ab.jar"));
        assert!(!glob_matches("", "anything"));
    }

    #[test]
    fn the_documented_example_is_a_valid_spec() {
        let spec: ReleaseSpec =
            serde_json::from_str(include_str!("../../../docs/examples/cemm-pack.fusion.json"))
                .expect("docs/examples/cemm-pack.fusion.json must parse as a ReleaseSpec");

        assert_eq!(spec.lineage, "atmons-fusion");
        assert!(crate::composables::release::is_valid_lineage(&spec.lineage));
        assert_eq!(
            (spec.base.project_id, spec.base.file_id),
            (1356598, 9571494)
        );
        assert_eq!(spec.pins[0].project_id, 1530014);
        assert_eq!(spec.pins[0].file_id, Some(8420412));
        assert_eq!(
            spec.settings[0].pointer,
            "/general/compatibility/sableCompat"
        );
        assert_eq!(spec.settings[0].value, serde_json::json!(false));
        // The file a required setting lives in is shared, never replaced whole.
        let tweaks = spec
            .content
            .iter()
            .find(|root| root.path == spec.settings[0].path)
            .expect("the settings file is published");
        assert_eq!(tweaks.policy, FilePolicy::Default);
        for root in &spec.content {
            assert!(validate_release_path(&root.path).is_ok());
            assert!(protected_reason(&root.path).is_none());
        }
    }

    #[test]
    fn shaders_and_resource_packs_of_ones_own_do_not_block_a_release() {
        let pack = admin_pack();
        pack.write("shaderpacks/MyFavouriteShader.zip", "personal");
        pack.write("shaderpacks/MyFavouriteShader.zip.txt", "personal settings");
        pack.write("resourcepacks/my-textures.zip", "personal");
        // A data pack is gameplay, and still has to be accounted for.
        pack.write("datapacks/forgotten.zip", "gameplay");

        let draft = pack.capture("r1", None);

        assert_eq!(draft.errors.len(), 1, "{:#?}", draft.errors);
        assert!(draft.errors[0].contains("datapacks/forgotten.zip"));
        assert!(
            draft
                .warnings
                .iter()
                .any(|warning| warning.contains("2 shader or resource pack(s)")),
            "{:#?}",
            draft.warnings
        );
        assert!(!draft
            .release
            .files
            .iter()
            .any(|file| file.path.starts_with("shaderpacks")
                || file.path.starts_with("resourcepacks")));
    }

    #[test]
    fn a_spec_with_a_misspelt_key_is_refused_rather_than_half_read() {
        // `contents` for `content` would otherwise publish a pack with no
        // managed files and no complaint.
        let error = serde_json::from_str::<ReleaseSpec>(
            r#"{"lineage":"x","name":"X","base":{"project_id":1,"file_id":2},"contents":[]}"#,
        )
        .expect_err("unknown keys must be refused")
        .to_string();
        assert!(error.contains("contents"), "got: {error}");
    }

    const BRIDGE: u64 = 1530014;

    struct Pack {
        _temp: tempfile::TempDir,
        root: PathBuf,
    }

    impl Pack {
        fn write(&self, relative: &str, content: &str) {
            let path = self.root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
            std::fs::write(path, content).expect("write");
        }

        fn remove(&self, relative: &str) {
            std::fs::remove_file(self.root.join(relative)).expect("remove");
        }

        fn path(&self) -> String {
            self.root.to_string_lossy().into_owned()
        }

        /// A record as CurseForge would have written it for a file holding
        /// exactly `content`: length and fingerprint agree with the bytes.
        fn record(&self, project_id: u64, file_id: u64, file_name: &str, content: &str) -> Value {
            serde_json::json!({
                "addonID": project_id,
                "name": format!("Addon {project_id}"),
                "modFolderPath": format!("{}\\mods", self.root.display()),
                "fileNameOnDisk": file_name,
                "webSiteURL": format!("https://www.curseforge.com/minecraft/mc-mods/addon-{project_id}"),
                "isEnabled": true,
                "isLocked": project_id == BRIDGE,
                "installedFile": {
                    "id": file_id,
                    "fileName": file_name,
                    "fileLength": content.len(),
                    "packageFingerprint": curseforge_fingerprint(content.as_bytes()),
                    "downloadUrl": format!("https://edge.forgecdn.net/files/{file_name}"),
                    "hashes": [{ "type": 1, "value": "0F287C29B59A08C5BCB43139B23DBD9087CE2F21" }, { "type": 2, "value": "md5" }],
                    "dependencies": [{ "addonId": 2, "type": 3 }, { "addonId": 999, "type": 2 }],
                },
            })
        }

        fn set_instance(&self, addons: Vec<Value>) {
            self.write(
                "minecraftinstance.json",
                &serde_json::json!({
                    "baseModLoader": { "name": "neoforge-21.1.265" },
                    "gameVersion": "1.21.1",
                    "name": "Whatever The Admin Called It",
                    "installedAddons": addons,
                })
                .to_string(),
            );
        }

        fn set_spec(&self, edit: impl FnOnce(&mut Value)) {
            let mut spec = serde_json::json!({
                "lineage": "test-fusion",
                "name": "Test Fusion",
                "base": { "project_id": 1356598, "file_id": 9571494, "name": "All the Mons 2.0.1" },
                "minecraft": "1.21.1",
                "loader": "neoforge-21.1.265",
                "server_release": { "name": "TestFusion-Server-1" },
                "pins": [{ "project_id": BRIDGE, "file_id": 8420412, "reason": "1.2.0 breaks claims" }],
                "content": [
                    { "path": "kubejs" },
                    { "path": "config/ftbquests/quests" },
                    { "path": "mods/atmons-ore-hammer-fix-1.0.0.jar" },
                    { "path": "config/tacztweaks.json", "policy": "default" }
                ],
                "settings": [{
                    "path": "config/tacztweaks.json",
                    "pointer": "/general/compatibility/sableCompat",
                    "value": false,
                    "reason": "TaCZ handles Sable natively"
                }],
                "known_issues": ["Scorched Guns per-round reload exploit is not fixed."]
            });
            edit(&mut spec);
            self.write(
                SPEC_FILE_NAME,
                &serde_json::to_string_pretty(&spec).unwrap(),
            );
        }

        fn capture(&self, release_id: &str, previous: Option<&Release>) -> ReleaseDraft {
            capture_release(CaptureRequest {
                modpack_path: self.path(),
                release_id: release_id.to_string(),
                notes: "notes".to_string(),
                previous: previous.cloned(),
            })
            .expect("capture should produce a draft")
        }
    }

    /// The admin's own profile: three CurseForge mods, one local jar with no
    /// record, authored scripts and quests, generated junk, and personal data.
    fn admin_pack() -> Pack {
        let temp = tempfile::tempdir().expect("temp dir");
        let pack = Pack {
            root: temp.path().join("All the Mons Fusion"),
            _temp: temp,
        };
        pack.write("mods/alpha-1.jar", "alpha one");
        pack.write("mods/beta-1.jar", "beta one");
        pack.write("mods/bridge-1.1.1.jar", "bridge 1.1.1");
        pack.write("mods/atmons-ore-hammer-fix-1.0.0.jar", "hammer fix");
        pack.write("kubejs/server_scripts/recipes.js", "recipes v1");
        pack.write("kubejs/server_scripts/old.js", "old v1");
        pack.write(
            "config/ftbquests/quests/chapters/guns.snbt",
            "{ id: \"1A\" }",
        );
        pack.write(
            "config/ftbquests/quests/chapters/guns.snbt_merged",
            "generated",
        );
        pack.write("kubejs/exported/cache.log", "noise");
        pack.write(
            "config/tacztweaks.json",
            "{\n  \"general\": { \"compatibility\": { \"sableCompat\": false } },\n  \"keyActions\": { \"reload\": \"r\" }\n}\n",
        );
        pack.write("options.txt", "fov:90");
        pack.write("saves/World/level.dat", "world");
        pack.set_instance(vec![
            pack.record(1, 10, "alpha-1.jar", "alpha one"),
            pack.record(2, 20, "beta-1.jar", "beta one"),
            pack.record(BRIDGE, 8420412, "bridge-1.1.1.jar", "bridge 1.1.1"),
        ]);
        pack.set_spec(|_| {});
        pack
    }

    #[test]
    fn capture_describes_the_whole_pack_including_what_curseforge_does_not_know() {
        let pack = admin_pack();
        let draft = pack.capture("r1", None);

        assert!(draft.errors.is_empty(), "{:#?}", draft.errors);
        let release = &draft.release;
        // Identity comes from the spec, never from the profile's name or folder.
        assert_eq!(release.lineage, "test-fusion");
        assert_eq!(release.code_path(), "test-fusion/r1");
        assert_eq!(release.sequence, 1);
        assert_eq!(release.base.file_id, 9571494);
        assert_eq!(release.minecraft, "1.21.1");
        assert_eq!(release.loader, "neoforge-21.1.265");
        assert_eq!(
            release.server_release.as_ref().unwrap().name,
            "TestFusion-Server-1"
        );
        assert_eq!(release.requires, vec![CAP_ADDONS, CAP_FILES, CAP_SETTINGS]);

        // Exact CurseForge identity plus integrity, per addon.
        assert_eq!(release.addons.len(), 3);
        let alpha = release
            .addons
            .iter()
            .find(|addon| addon.project_id == 1)
            .unwrap();
        assert_eq!(alpha.file_id, 10);
        assert_eq!(alpha.sha256, sha256_hex(b"alpha one"));
        assert_eq!(alpha.size, 9);
        assert_eq!(
            alpha.sha1.as_deref(),
            Some("0f287c29b59a08c5bcb43139b23dbd9087ce2f21")
        );
        assert_eq!(
            alpha.fingerprint,
            Some(curseforge_fingerprint(b"alpha one"))
        );
        assert_eq!(alpha.requires, vec![2], "required dependencies only");
        let bridge = release
            .addons
            .iter()
            .find(|addon| addon.project_id == BRIDGE)
            .unwrap();
        assert!(bridge.pinned);
        assert!(!alpha.pinned);

        // The local jar is in the inventory as what it is.
        let paths: Vec<&str> = release
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        assert_eq!(
            paths,
            vec![
                "config/ftbquests/quests/chapters/guns.snbt",
                "config/tacztweaks.json",
                "kubejs/server_scripts/old.js",
                "kubejs/server_scripts/recipes.js",
                "mods/atmons-ore-hammer-fix-1.0.0.jar",
            ],
            "generated and excluded files must not be captured"
        );
        let hammer = release.files.last().unwrap();
        assert_eq!(hammer.source, FileSource::LocalBinary);
        assert_eq!(hammer.sha256, sha256_hex(b"hammer fix"));
        assert!(hammer.hosted);
        let tweaks = &release.files[1];
        assert_eq!(tweaks.policy, FilePolicy::Default);
        assert_eq!(draft.summary.skipped_files, 2);
        assert_eq!(draft.summary.new_payloads, 5);
        assert_eq!(release.known_issues.len(), 1);
    }

    #[test]
    fn the_admins_own_profile_is_already_exactly_its_release() {
        // Capture and plan have to agree on what a profile contains, or a
        // release would describe something its own author does not have.
        let pack = admin_pack();
        let draft = pack.capture("r1", None);
        assert!(draft.errors.is_empty(), "{:#?}", draft.errors);

        let plan = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(crate::release_install::plan_release(
                pack.path(),
                draft.release.clone(),
                "digest".to_string(),
            ))
            .expect("plan");

        assert!(plan.blockers.is_empty(), "{:#?}", plan.blockers);
        assert!(plan.conflicts.is_empty());
        assert_eq!(plan.change_count, 0, "{plan:#?}");
        assert!(plan.extras.is_empty());
    }

    #[test]
    fn a_file_nobody_accounted_for_stops_the_release_instead_of_vanishing_from_it() {
        let pack = admin_pack();
        pack.set_spec(|spec| {
            spec["content"].as_array_mut().unwrap().remove(2);
        });

        let draft = pack.capture("r1", None);
        let error = draft
            .errors
            .iter()
            .find(|error| error.contains("atmons-ore-hammer-fix-1.0.0.jar"))
            .expect("the unrecorded jar must be reported");
        assert!(error.contains("local_only"), "got: {error}");

        // Saying so on purpose is the other way to settle it.
        pack.set_spec(|spec| {
            spec["content"].as_array_mut().unwrap().remove(2);
            spec["local_only"] = serde_json::json!(["mods/atmons-ore-hammer-fix-*.jar"]);
        });
        let draft = pack.capture("r1", None);
        assert!(draft.errors.is_empty(), "{:#?}", draft.errors);
        assert!(!draft
            .release
            .files
            .iter()
            .any(|file| file.path.ends_with("hammer-fix-1.0.0.jar")));
    }

    #[test]
    fn a_record_that_no_longer_describes_its_file_is_drift_and_blocks() {
        let pack = admin_pack();
        // The jar was swapped by hand; CurseForge's record still names file 10.
        pack.write("mods/alpha-1.jar", "alpha, hand-replaced");

        let draft = pack.capture("r1", None);
        assert!(
            draft
                .errors
                .iter()
                .any(|error| error.contains("mods/alpha-1.jar")
                    && error.contains("not the file CurseForge's record describes")),
            "{:#?}",
            draft.errors
        );

        // A record with no file at all is the same class of problem.
        pack.remove("mods/alpha-1.jar");
        let draft = pack.capture("r1", None);
        assert!(draft
            .errors
            .iter()
            .any(|error| error.contains("is not in the profile")));
    }

    #[test]
    fn a_pin_that_does_not_match_the_profile_blocks() {
        let pack = admin_pack();
        pack.set_spec(|spec| spec["pins"][0]["file_id"] = serde_json::json!(9999999));
        let draft = pack.capture("r1", None);
        assert!(draft
            .errors
            .iter()
            .any(|error| error.contains("pinned to file 9999999")));

        pack.set_spec(|spec| spec["pins"][0]["project_id"] = serde_json::json!(424242));
        let draft = pack.capture("r1", None);
        assert!(draft
            .errors
            .iter()
            .any(|error| error.contains("pins CurseForge project 424242")));
    }

    #[test]
    fn the_next_release_remembers_what_earlier_ones_held() {
        let pack = admin_pack();
        let first = pack.capture("r1", None).release;

        // alpha updated, beta dropped, one script edited, one deleted.
        pack.remove("mods/alpha-1.jar");
        pack.remove("mods/beta-1.jar");
        pack.write("mods/alpha-2.jar", "alpha two");
        pack.write("kubejs/server_scripts/recipes.js", "recipes v2");
        pack.remove("kubejs/server_scripts/old.js");
        pack.set_instance(vec![
            {
                let mut alpha = pack.record(1, 11, "alpha-2.jar", "alpha two");
                alpha["installedFile"]["dependencies"] = serde_json::json!([]);
                alpha
            },
            {
                let mut bridge = pack.record(BRIDGE, 8420412, "bridge-1.1.1.jar", "bridge 1.1.1");
                bridge["installedFile"]["dependencies"] = serde_json::json!([]);
                bridge
            },
        ]);

        let draft = pack.capture("r2", Some(&first));
        assert!(draft.errors.is_empty(), "{:#?}", draft.errors);
        let second = draft.release;
        assert_eq!(second.sequence, 2);

        let retired: HashSet<(&str, Option<u64>)> = second
            .retired
            .iter()
            .map(|item| (item.path.as_str(), item.project_id))
            .collect();
        assert!(retired.contains(&("mods/alpha-1.jar", Some(1))));
        assert!(retired.contains(&("mods/beta-1.jar", Some(2))));
        assert!(retired.contains(&("kubejs/server_scripts/old.js", None)));
        let recipes = second
            .files
            .iter()
            .find(|file| file.path == "kubejs/server_scripts/recipes.js")
            .unwrap();
        assert_eq!(recipes.previous, vec![sha256_hex(b"recipes v1")]);

        assert_eq!(
            draft.summary.addons_updated,
            vec!["Addon 1 (alpha-1.jar → alpha-2.jar)"]
        );
        assert_eq!(draft.summary.addons_removed, vec!["Addon 2"]);
        assert_eq!(
            (draft.summary.files_changed, draft.summary.files_removed),
            (1, 1)
        );
        // Only the bytes this release introduces are uploaded.
        assert_eq!(draft.summary.new_payloads, 1);

        // A file edited in r2 and dropped in r3 is retirable in both forms, so
        // a player still on r1 is cleaned up too.
        pack.remove("kubejs/server_scripts/recipes.js");
        let third = pack.capture("r3", Some(&second)).release;
        let recipe_hashes: HashSet<&str> = third
            .retired
            .iter()
            .filter(|item| item.path == "kubejs/server_scripts/recipes.js")
            .map(|item| item.sha256.as_str())
            .collect();
        assert_eq!(recipe_hashes.len(), 2);
        // And what r2 retired is carried forward.
        assert!(third
            .retired
            .iter()
            .any(|item| item.path == "mods/beta-1.jar"));
        assert_eq!(third.sequence, 3);
    }

    #[test]
    fn dropping_a_dependency_another_addon_still_needs_blocks() {
        let pack = admin_pack();
        let first = pack.capture("r1", None).release;
        // beta (project 2) is what alpha requires.
        pack.remove("mods/beta-1.jar");
        pack.set_instance(vec![
            pack.record(1, 10, "alpha-1.jar", "alpha one"),
            pack.record(BRIDGE, 8420412, "bridge-1.1.1.jar", "bridge 1.1.1"),
        ]);

        let draft = pack.capture("r2", Some(&first));
        assert!(
            draft
                .errors
                .iter()
                .any(|error| error.contains("requires CurseForge project 2")),
            "{:#?}",
            draft.errors
        );
    }

    #[test]
    fn content_that_is_inventoried_but_not_hosted_cannot_silently_change() {
        let pack = admin_pack();
        pack.set_spec(|spec| spec["content"][0]["distribute"] = serde_json::json!(false));

        // A first release may leave it unhosted: players have it from the export.
        let draft = pack.capture("r1", None);
        assert!(draft.errors.is_empty(), "{:#?}", draft.errors);
        let recipes = draft
            .release
            .files
            .iter()
            .find(|file| file.path == "kubejs/server_scripts/recipes.js")
            .unwrap();
        assert!(!recipes.hosted);
        let first = draft.release;

        // Changing it afterwards would publish a target nobody can reach.
        pack.write("kubejs/server_scripts/recipes.js", "recipes v2");
        let draft = pack.capture("r2", Some(&first));
        assert!(
            draft
                .errors
                .iter()
                .any(|error| error.contains("recipes.js") && error.contains("not distributed")),
            "{:#?}",
            draft.errors
        );
    }

    #[test]
    fn unfinished_merges_and_unmet_settings_block_publication() {
        let pack = admin_pack();
        pack.write(
            "kubejs/server_scripts/recipes.js",
            "a\n<<<<<<< fusion\nours\n=======\ntheirs\n>>>>>>> official\n",
        );
        pack.write(
            "config/tacztweaks.json",
            "{ \"general\": { \"compatibility\": { \"sableCompat\": true } } }",
        );

        let draft = pack.capture("r1", None);
        assert!(draft
            .errors
            .iter()
            .any(|error| error.contains("merge-conflict markers")));
        assert!(draft
            .errors
            .iter()
            .any(|error| error.contains("your own profile has it set differently")));
    }

    #[test]
    fn personal_and_local_data_cannot_be_named_as_content() {
        for root in [
            "saves",
            "options.txt",
            "minecraftinstance.json",
            "logs",
            "../outside",
        ] {
            let pack = admin_pack();
            pack.set_spec(|spec| {
                spec["content"]
                    .as_array_mut()
                    .unwrap()
                    .push(serde_json::json!({ "path": root }));
            });
            let draft = pack.capture("r1", None);
            assert!(
                !draft.errors.is_empty(),
                "'{root}' must be refused as content"
            );
            assert!(!draft
                .release
                .files
                .iter()
                .any(|file| file.path.starts_with("saves") || file.path == "options.txt"));
        }
    }

    #[test]
    fn a_profile_without_a_spec_is_not_a_release_pack() {
        let pack = admin_pack();
        pack.remove(SPEC_FILE_NAME);
        assert_eq!(read_release_spec(pack.path()).unwrap(), None);
        assert!(capture_release(CaptureRequest {
            modpack_path: pack.path(),
            release_id: "r1".to_string(),
            notes: String::new(),
            previous: None,
        })
        .expect_err("no spec, no release")
        .contains(SPEC_FILE_NAME));
    }

    /// An official download as it comes: a pack archive, extracted.
    fn official_archive(name: &str, files: &[(&str, &str)], addons: &[(u64, u64)]) -> Pack {
        let temp = tempfile::tempdir().expect("temp dir");
        let pack = Pack {
            root: temp.path().join(name),
            _temp: temp,
        };
        pack.write(
            "manifest.json",
            &serde_json::json!({
                "overrides": "overrides",
                "files": addons
                    .iter()
                    .map(|(project, file)| serde_json::json!({ "projectID": project, "fileID": file, "required": true }))
                    .collect::<Vec<_>>(),
            })
            .to_string(),
        );
        for (path, content) in files {
            pack.write(&format!("overrides/{path}"), content);
        }
        pack
    }

    #[test]
    fn an_official_upgrade_is_reviewed_three_ways_from_real_folders() {
        // B: the official release Fusion was built on.
        let base = official_archive(
            "official-2.0.1",
            &[
                ("kubejs/server_scripts/recipes.js", "official recipes"),
                ("kubejs/server_scripts/old.js", "old v1"),
                ("kubejs/server_scripts/upstream-only.js", "u1"),
                (
                    "config/ftbquests/quests/chapters/guns.snbt",
                    "{ id: \"1A\" }",
                ),
            ],
            &[(1, 10), (2, 20), (BRIDGE, 8420412)],
        );
        // F: Fusion, which rewrote recipes and kept the rest.
        let fusion = admin_pack();
        fusion.write("kubejs/server_scripts/upstream-only.js", "u1");
        // N: a synthetic next official release. It is not a real ATMons
        // release; it exists to put every row of the matrix in one place.
        let upstream = official_archive(
            "official-next",
            &[
                // changed on both sides, differently
                (
                    "kubejs/server_scripts/recipes.js",
                    "official recipes, rewritten",
                ),
                ("kubejs/server_scripts/old.js", "old v1"),
                // changed upstream only
                ("kubejs/server_scripts/upstream-only.js", "u2"),
                // added upstream
                ("kubejs/server_scripts/new-official.js", "new"),
                (
                    "config/ftbquests/quests/chapters/guns.snbt",
                    "{ id: \"1A\" }",
                ),
            ],
            // alpha updated upstream, beta dropped upstream, the pinned bridge moved.
            &[(1, 11), (BRIDGE, 9000000)],
        );

        let report = review_upgrade(ReviewRequest {
            base_path: base.path(),
            custom_path: fusion.path(),
            upstream_path: upstream.path(),
        })
        .expect("review");

        let decision = |key: &str| {
            report
                .items
                .iter()
                .find(|item| item.key == key)
                .unwrap_or_else(|| panic!("no decision for {key}: {:#?}", report.items))
                .decision
        };
        use crate::composables::release::ThreeWayDecision::*;
        assert_eq!(decision("kubejs/server_scripts/recipes.js"), Conflict);
        assert_eq!(
            decision("kubejs/server_scripts/upstream-only.js"),
            TakeUpstream
        );
        assert_eq!(
            decision("kubejs/server_scripts/new-official.js"),
            TakeUpstream
        );
        // Fusion's own additions survive the upgrade untouched.
        assert_eq!(decision("mods/atmons-ore-hammer-fix-1.0.0.jar"), KeepCustom);
        assert_eq!(decision("config/tacztweaks.json"), KeepCustom);
        assert_eq!(decision("project 1"), TakeUpstream);
        assert_eq!(decision("project 2"), TakeUpstream);
        // The pin is never moved by an upgrade on its own.
        assert_eq!(decision(&format!("project {BRIDGE}")), Conflict);
        assert_eq!(report.conflicts, 2);
        // alpha requires beta, which upstream removed.
        assert!(
            report
                .review
                .iter()
                .any(|note| note.contains("requires CurseForge project 2")),
            "{:#?}",
            report.review
        );
        assert!(report
            .review
            .iter()
            .any(|note| note.contains("atmons-ore-hammer-fix-1.0.0.jar is a local fix")));
        // Unchanged paths are counted, not listed.
        assert!(report
            .items
            .iter()
            .all(|item| item.key != "kubejs/server_scripts/old.js"));
        assert!(report.unchanged >= 2);
    }

    /// Read-only check against a real CurseForge profile named by
    /// `CEMM_REFERENCE_PROFILE`. Ignored by default: CI has no such profile.
    #[test]
    #[ignore = "needs CEMM_REFERENCE_PROFILE pointing at a real CurseForge profile"]
    fn the_fingerprint_agrees_with_every_record_of_a_real_profile() {
        let profile = PathBuf::from(
            std::env::var("CEMM_REFERENCE_PROFILE").expect("CEMM_REFERENCE_PROFILE must be set"),
        );
        let facts = read_instance_facts(&profile).expect("instance should parse");
        let (mut checked, mut wrong) = (0, Vec::new());
        for record in facts.records.iter().filter(|record| record.enabled) {
            let path = profile
                .join(record.category.directory())
                .join(&record.file_name);
            let (Some(expected), Ok(bytes)) = (record.fingerprint, std::fs::read(&path)) else {
                continue;
            };
            checked += 1;
            if curseforge_fingerprint(&bytes) != expected
                || record.file_length != Some(bytes.len() as u64)
            {
                wrong.push(record.file_name.clone());
            }
        }
        println!(
            "checked {checked} records, {} disagree: {wrong:?}",
            wrong.len()
        );
        assert!(checked > 0, "no record could be checked");
        assert!(
            wrong.is_empty(),
            "fingerprint or length mismatch: {wrong:?}"
        );
    }
}
