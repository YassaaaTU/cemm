//! Applying a release to a profile.
//!
//! Two operations, and the second is built on the first. `plan_release` reads
//! the profile — CEMM's own record, CurseForge's record and the files actually
//! there — and says what reaching the release would take: what is already
//! right, what is fetched, what is retired, what the player has to decide, and
//! what stops the update outright. It writes nothing. `apply_release` plans
//! again from fresh reads, refuses to continue unless that plan is the one the
//! player reviewed, stages every byte, and hands the result to the same
//! journaled transaction legacy installs use.
//!
//! The record this leaves behind, `.cemm/state.json`, is the one authoritative
//! statement of what CEMM applied to a profile and therefore of what CEMM may
//! later take away. Ownership is claimed only for content proven to be the
//! release's: a file is owned once its hash matched, never because it sat in a
//! folder the release also uses.

use crate::composables::github::{self, GithubEndpoints};
use crate::composables::manifest::{Addon, Manifest};
use crate::composables::release::{
    digest_if_present, sha256_hex, AddonCategory, FileDigest, FilePolicy, FileSource, OfficialBase,
    Release, ReleaseAddon, RequiredSetting, Role, ServerRelease, STATE_DIR,
};
use crate::installer::{
    self, unix_now, Expected, InstallProgress, InstallProgressCallback, ProfileLock,
    TransactionOptions, INSTALLED_MANIFEST_FILE, INSTALL_TRANSACTION_DIR,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use ts_rs::TS;

pub const STATE_SCHEMA: u32 = 1;
const STATE_FILE_NAME: &str = "state.json";
const INSTANCE_FILE: &str = "minecraftinstance.json";

pub(crate) fn state_relative_path() -> String {
    format!("{STATE_DIR}/{STATE_FILE_NAME}")
}

// ---------------------------------------------------------------------------
// Applied state
// ---------------------------------------------------------------------------

/// A file CEMM put in place, or found already exactly as a release describes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct OwnedItem {
    /// The path the file actually occupies, including a `.disabled` suffix.
    pub path: String,
    /// The content CEMM vouches for. A file that no longer hashes to this was
    /// changed by someone else and is no longer CEMM's to delete unasked.
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub project_id: Option<u64>,
}

/// A pack default, and whether the player has made the file their own.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct SuppliedDefault {
    pub path: String,
    /// What the pack last offered for this path.
    pub supplied_sha256: String,
    /// What was actually on disk when the release was applied. Differs from
    /// `supplied_sha256` exactly when the player's own version was kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub local_sha256: Option<String>,
    pub kept_local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "generated/")]
pub struct AppliedSetting {
    pub path: String,
    pub pointer: String,
    #[ts(type = "unknown")]
    pub value: Value,
}

/// A legacy config-only code applied on top of a release. Recorded rather than
/// folded in, because such a code replaces whole files with no hashes to say
/// what it wrote: the profile is the release plus this, and the next plan will
/// show any managed file it overwrote as changed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct LegacyOverlay {
    #[ts(type = "number")]
    pub applied_unix: u64,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "generated/")]
pub struct AppliedState {
    pub schema: u32,
    pub lineage: String,
    pub release_id: String,
    #[ts(type = "number")]
    pub sequence: u64,
    pub release_name: String,
    /// SHA-256 of the release document that was applied. A later fetch of the
    /// same id with a different digest is a reused code.
    pub release_digest: String,
    /// The repository revision the release and its payloads were read at.
    pub revision: String,
    pub base: OfficialBase,
    pub minecraft: String,
    pub loader: String,
    #[ts(type = "number")]
    pub applied_unix: u64,
    pub cemm_version: String,
    pub owned: Vec<OwnedItem>,
    #[serde(default)]
    pub defaults: Vec<SuppliedDefault>,
    #[serde(default)]
    pub settings: Vec<AppliedSetting>,
    #[serde(default)]
    pub legacy_overlays: Vec<LegacyOverlay>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub server_release: Option<ServerRelease>,
}

/// Reads `.cemm/state.json`. Absent is a profile CEMM has not adopted; present
/// but unreadable is an error, because treating it as absent would re-adopt the
/// profile and forget everything CEMM is entitled to remove.
pub(crate) fn read_applied_state(profile: &Path) -> Result<Option<AppliedState>, String> {
    let path = profile.join(STATE_DIR).join(STATE_FILE_NAME);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Failed to read {}: {error}", path.display())),
    };
    let state: AppliedState = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "{} is not readable ({error}). CEMM will not guess what it installed here; restore the file from a backup or ask for help before updating.",
            path.display()
        )
    })?;
    if state.schema != STATE_SCHEMA {
        return Err(format!(
            "{} was written by a newer CEMM (state schema {}). Update CEMM before changing this profile.",
            path.display(),
            state.schema
        ));
    }
    Ok(Some(state))
}

/// What a profile currently follows, for a screen that wants to say so.
pub fn profile_release_status(modpack_path: String) -> Result<Option<AppliedState>, String> {
    read_applied_state(Path::new(&modpack_path))
}

// ---------------------------------------------------------------------------
// The plan
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "generated/")]
pub enum PlanMode {
    /// CEMM has no record of this profile and is taking it into the lineage.
    Adopt,
    Upgrade,
    /// The same release again: verify, and repair whatever drifted.
    Reapply,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct PlanIssue {
    /// Stable machine name: `lineage`, `loader`, `in-use`, `manual-download`…
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "generated/")]
pub enum ConflictKind {
    /// A managed file the player changed; the release needs its own content.
    ModifiedManagedFile,
    /// Owned content the release retires, changed since CEMM put it there.
    ModifiedRetiredFile,
    /// A pack default changed in the release and by the player.
    DefaultChangedBothSides,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct PlanConflict {
    /// The path, which is also what an approval names.
    pub id: String,
    pub kind: ConflictKind,
    /// Whether the update cannot complete until this is approved. An optional
    /// conflict left unapproved keeps the player's file and records that.
    pub required: bool,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "generated/")]
pub enum AddonOp {
    /// Already exactly the release's file.
    Verified,
    Install,
    Replace,
    /// Present but switched off; switched back on.
    Enable,
    /// Active, and the release has it switched off.
    Disable,
    Retire,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct AddonAction {
    #[ts(type = "number")]
    pub project_id: u64,
    pub name: String,
    pub category: AddonCategory,
    pub file_name: String,
    pub op: AddonOp,
    /// The file being replaced or retired, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub from_file: Option<String>,
    pub pinned: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "generated/")]
pub enum FileOp {
    Verified,
    Add,
    /// Earlier pack content, replaced with the release's.
    Replace,
    /// Changed locally; replaced only once the conflict is approved.
    Repair,
    Retire,
    /// A default the player changed, left as they have it.
    KeepLocal,
    /// Server-only content, not installed into a client profile.
    SkipRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct FileAction {
    pub path: String,
    pub op: FileOp,
    pub policy: FilePolicy,
    pub source: FileSource,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "generated/")]
pub enum SettingOp {
    AlreadySet,
    Apply,
    /// The file is being written by this update and already carries the value.
    WithFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct SettingAction {
    pub path: String,
    pub pointer: String,
    pub op: SettingOp,
    /// The value now, as JSON text; `null` when the key is absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub current: Option<String>,
    pub required: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "generated/")]
pub enum PinState {
    AlreadyPinned,
    /// CurseForge has a record of the addon; CEMM locks it.
    WillPin,
    /// CurseForge has no record to lock. The player pins it by hand once the
    /// launcher has picked the file up.
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct PinAction {
    #[ts(type = "number")]
    pub project_id: u64,
    pub name: String,
    pub file_name: String,
    pub state: PinState,
}

/// What CEMM will change in CurseForge's own record of the profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct LauncherPlan {
    pub records_updated: u32,
    pub records_removed: u32,
    /// Addons CEMM installs that CurseForge has no record of yet. The launcher
    /// recognises files by fingerprint on its next scan; CEMM does not invent
    /// a record for it.
    pub records_missing: u32,
    pub pins: Vec<PinAction>,
}

/// Something in the profile that is neither the release's nor CEMM's.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct ExtraItem {
    pub path: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct InstallPlan {
    pub lineage: String,
    pub release_id: String,
    pub release_name: String,
    pub mode: PlanMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub from_release: Option<String>,
    /// Reasons the update cannot be applied at all. Nothing else in the plan
    /// can be acted on while this is non-empty.
    pub blockers: Vec<PlanIssue>,
    pub conflicts: Vec<PlanConflict>,
    pub addons: Vec<AddonAction>,
    pub files: Vec<FileAction>,
    pub settings: Vec<SettingAction>,
    pub launcher: LauncherPlan,
    pub extras: Vec<ExtraItem>,
    /// Things worth saying that do not stop anything.
    pub notes: Vec<String>,
    pub download_count: u32,
    #[ts(type = "number")]
    pub download_bytes: u64,
    /// How many files this update writes or removes. Zero means the profile
    /// already is the release.
    pub change_count: u32,
    /// Digest of everything above. Apply recomputes the plan from fresh reads
    /// and proceeds only if it gets this value again.
    pub fingerprint: String,
    /// An earlier update on this profile was cut off partway, and the profile
    /// was put back to how it was before that update so this plan could be
    /// made against something real. Not part of the fingerprint: it describes
    /// how the plan was reached, not what it will do.
    #[serde(default)]
    pub recovered_interrupted: bool,
}

/// What an install did, for the screen that reports it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct ApplyReport {
    pub lineage: String,
    pub release_id: String,
    pub release_name: String,
    pub mode: PlanMode,
    pub change_count: u32,
    pub downloaded: u32,
    pub retired: u32,
    /// Where the replaced files were kept, relative to the profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub backup_dir: Option<String>,
    pub kept_local: Vec<String>,
    pub launcher: LauncherPlan,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub server_release: Option<ServerRelease>,
    pub manual_steps: Vec<String>,
    pub known_issues: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// CurseForge's record of the profile
// ---------------------------------------------------------------------------

struct InstanceView {
    text: String,
    value: Value,
    digest: String,
}

fn read_instance(profile: &Path) -> Result<Option<InstanceView>, String> {
    let path = profile.join(INSTANCE_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Failed to read {}: {error}", path.display())),
    };
    let value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("{} is not valid JSON: {error}", path.display()))?;
    let digest = sha256_hex(text.as_bytes());
    Ok(Some(InstanceView {
        text,
        value,
        digest,
    }))
}

#[derive(Debug, Clone)]
struct InstanceRecord {
    project_id: u64,
    name: String,
    category: Option<AddonCategory>,
    /// As CurseForge records it, `.disabled` suffix included.
    file_name_on_disk: String,
    locked: bool,
}

impl InstanceRecord {
    /// The file's name without the switched-off suffix.
    fn canonical_file_name(&self) -> &str {
        self.file_name_on_disk
            .strip_suffix(".disabled")
            .unwrap_or(&self.file_name_on_disk)
    }
}

fn folder_category(mod_folder_path: &str) -> Option<AddonCategory> {
    let leaf = mod_folder_path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(mod_folder_path);
    AddonCategory::from_directory(leaf)
}

fn instance_records(value: &Value) -> Vec<InstanceRecord> {
    let Some(addons) = value.get("installedAddons").and_then(Value::as_array) else {
        return Vec::new();
    };
    addons
        .iter()
        .filter_map(|addon| {
            let project_id = addon.get("addonID")?.as_u64()?;
            let installed = addon.get("installedFile");
            let file_name_on_disk = addon
                .get("fileNameOnDisk")
                .and_then(Value::as_str)
                .or_else(|| installed?.get("fileName")?.as_str())?
                .to_string();
            Some(InstanceRecord {
                project_id,
                name: addon
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                category: addon
                    .get("modFolderPath")
                    .and_then(Value::as_str)
                    .and_then(folder_category),
                file_name_on_disk,
                locked: addon.get("isLocked").and_then(Value::as_bool) == Some(true),
            })
        })
        .collect()
}

/// One narrow edit to an `installedAddons` entry.
#[derive(Debug, Clone)]
enum RecordChange {
    /// The project now has this file, in this enabled state.
    Set {
        addon: ReleaseAddon,
    },
    Remove {
        project_id: u64,
    },
    Pin {
        project_id: u64,
    },
}

/// Applies record changes to CurseForge's instance document and returns how
/// many entries were updated and removed.
///
/// Deliberately small. It touches the fields that say which file a project has
/// and whether it is enabled or locked, on entries that already exist, and
/// drops entries for addons the release retired — a record with no file makes
/// the launcher offer to "repair" a mod the pack removed. Everything else in
/// the document, known or not, is carried through untouched: this is the
/// player's own record of their own profile, never the admin's.
fn patch_instance(value: &mut Value, changes: &[RecordChange]) -> (u32, u32) {
    let Some(addons) = value
        .get_mut("installedAddons")
        .and_then(Value::as_array_mut)
    else {
        return (0, 0);
    };
    let mut updated = HashSet::new();
    let mut removed = 0u32;

    for change in changes {
        match change {
            RecordChange::Remove { project_id } => {
                let before = addons.len();
                addons.retain(|addon| {
                    addon.get("addonID").and_then(Value::as_u64) != Some(*project_id)
                });
                removed += (before - addons.len()) as u32;
            }
            RecordChange::Pin { project_id } => {
                for addon in addons.iter_mut().filter(|addon| {
                    addon.get("addonID").and_then(Value::as_u64) == Some(*project_id)
                }) {
                    if addon.get("isLocked").and_then(Value::as_bool) != Some(true) {
                        addon["isLocked"] = Value::Bool(true);
                        updated.insert(*project_id);
                    }
                }
            }
            RecordChange::Set { addon: target } => {
                for addon in addons.iter_mut().filter(|addon| {
                    addon.get("addonID").and_then(Value::as_u64) == Some(target.project_id)
                }) {
                    let on_disk = if target.enabled {
                        target.file_name.clone()
                    } else {
                        format!("{}.disabled", target.file_name)
                    };
                    let folder = addon
                        .get("modFolderPath")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    addon["fileNameOnDisk"] = Value::String(on_disk.clone());
                    addon["isEnabled"] = Value::Bool(target.enabled);
                    if let (Some(folder), true) = (folder, addon.get("filePaths").is_some()) {
                        let separator = if folder.contains('\\') { '\\' } else { '/' };
                        addon["filePaths"] = serde_json::json!([format!(
                            "{}{separator}{on_disk}",
                            folder.trim_end_matches(['/', '\\'])
                        )]);
                    }
                    if let Some(installed) = addon
                        .get_mut("installedFile")
                        .and_then(Value::as_object_mut)
                    {
                        installed.insert("id".to_string(), target.file_id.into());
                        installed.insert("fileName".to_string(), target.file_name.clone().into());
                        installed.insert("fileLength".to_string(), target.size.into());
                        if installed.contains_key("fileNameOnDisk") {
                            installed.insert(
                                "fileNameOnDisk".to_string(),
                                target.file_name.clone().into(),
                            );
                        }
                        if let Some(url) = target.download_url.as_deref() {
                            installed.insert("downloadUrl".to_string(), url.into());
                        }
                        if let Some(fingerprint) = target.fingerprint {
                            installed.insert("packageFingerprint".to_string(), fingerprint.into());
                        }
                        // The old file's hashes describe the old file.
                        installed.insert(
                            "hashes".to_string(),
                            match target.sha1.as_deref() {
                                Some(sha1) => serde_json::json!([{ "type": 1, "value": sha1 }]),
                                None => serde_json::json!([]),
                            },
                        );
                    }
                    updated.insert(target.project_id);
                }
            }
        }
    }

    (updated.len() as u32, removed)
}

/// Serialises the patched document the way the original was written, so the
/// launcher's file changes by the edits made and nothing else.
fn render_like(original: &str, value: &Value) -> Result<String, String> {
    render_json_like(original, value)
}

// ---------------------------------------------------------------------------
// Required settings inside a file the player owns
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SettingOutcome {
    AlreadySet,
    Changed {
        text: String,
        previous: Option<Value>,
    },
}

fn pointer_tokens(pointer: &str) -> Vec<String> {
    pointer[1..]
        .split('/')
        .map(|token| token.replace("~1", "/").replace("~0", "~"))
        .collect()
}

/// The whitespace a pretty-printed document indents with, or `None` when it is
/// written on one line.
fn detect_indent(text: &str) -> Option<String> {
    for line in text.lines().skip(1) {
        let indent: String = line
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        if !indent.is_empty() {
            return Some(indent);
        }
        if !line.trim().is_empty() {
            // A first nested line with no indent: treat as two spaces.
            return Some("  ".to_string());
        }
    }
    None
}

fn render_json_like(original: &str, value: &Value) -> Result<String, String> {
    let has_bom = original.starts_with('\u{feff}');
    let body = original.trim_start_matches('\u{feff}');
    let mut text = match detect_indent(body) {
        Some(indent) => {
            let mut buffer = Vec::new();
            let formatter = serde_json::ser::PrettyFormatter::with_indent(indent.as_bytes());
            let mut serializer = serde_json::Serializer::with_formatter(&mut buffer, formatter);
            value
                .serialize(&mut serializer)
                .map_err(|error| format!("Failed to encode JSON: {error}"))?;
            String::from_utf8(buffer).map_err(|error| format!("Failed to encode JSON: {error}"))?
        }
        None => serde_json::to_string(value)
            .map_err(|error| format!("Failed to encode JSON: {error}"))?,
    };
    if body.contains("\r\n") {
        text = text.replace('\n', "\r\n");
    }
    if body.ends_with('\n') {
        text.push_str(if body.ends_with("\r\n") { "\r\n" } else { "\n" });
    }
    if has_bom {
        text.insert(0, '\u{feff}');
    }
    Ok(text)
}

/// Sets one key in a JSON document and leaves the rest as it found it.
///
/// This is the whole of CEMM's mixed-file support, and deliberately so: one
/// format, one operation. The file must be plain JSON, and the key's parent
/// must already exist as an object — CEMM changes a value the mod already has
/// a place for, it does not design the mod's config for it. Anything else is
/// an error naming exactly what to do by hand, because a guessed edit to a
/// file full of someone's keybinds is worse than no edit.
pub(crate) fn apply_json_setting(
    original: &str,
    pointer: &str,
    required: &Value,
) -> Result<SettingOutcome, String> {
    let mut document: Value = serde_json::from_str(original.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("it is not plain JSON ({error})"))?;

    let tokens = pointer_tokens(pointer);
    let (last, parents) = tokens.split_last().ok_or("the pointer names no key")?;

    let mut current = &mut document;
    let mut walked = String::new();
    for token in parents {
        walked.push('/');
        walked.push_str(token);
        current = match current {
            Value::Object(map) => map
                .get_mut(token)
                .ok_or_else(|| format!("it has no '{walked}' section"))?,
            Value::Array(items) => token
                .parse::<usize>()
                .ok()
                .and_then(|index| items.get_mut(index))
                .ok_or_else(|| format!("it has no '{walked}' entry"))?,
            _ => return Err(format!("'{walked}' is not a section")),
        };
    }

    let Value::Object(parent) = current else {
        return Err(format!(
            "'{}' is not a section that can hold '{last}'",
            if walked.is_empty() { "/" } else { &walked }
        ));
    };

    let previous = parent.get(last).cloned();
    if previous.as_ref() == Some(required) {
        return Ok(SettingOutcome::AlreadySet);
    }
    parent.insert(last.clone(), required.clone());

    Ok(SettingOutcome::Changed {
        text: render_json_like(original, &document)?,
        previous,
    })
}

// ---------------------------------------------------------------------------
// Planning
// ---------------------------------------------------------------------------

/// Where a staged payload comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PayloadOrigin {
    /// A CurseForge CDN URL.
    Addon { url: String, name: String },
    /// A content-addressed object in the release repository.
    Object,
}

#[derive(Debug, Clone)]
struct Download {
    origin: PayloadOrigin,
    relative_path: String,
    sha256: String,
    size: u64,
    /// Performed only when this conflict id is approved.
    conflict: Option<String>,
}

#[derive(Debug, Clone)]
struct Removal {
    relative_path: String,
    /// The content seen at plan time; rechecked before anything moves.
    sha256: String,
    conflict: Option<String>,
}

/// A file switched on or off: staged under its new name, removed under its old.
#[derive(Debug, Clone)]
struct Rename {
    from: String,
    to: String,
    sha256: String,
}

#[derive(Debug, Clone)]
struct SettingWrite {
    relative_path: String,
    pointer: String,
    value: Value,
}

#[derive(Debug, Clone)]
struct DefaultDecision {
    path: String,
    supplied_sha256: String,
    /// What is on disk if the player's file is kept.
    local_sha256: Option<String>,
    /// The conflict whose approval switches this to the pack's default.
    conflict: Option<String>,
    kept_local: bool,
}

/// Everything apply needs that the reviewable plan does not show.
#[derive(Debug, Default)]
struct Staging {
    downloads: Vec<Download>,
    removals: Vec<Removal>,
    renames: Vec<Rename>,
    setting_writes: Vec<SettingWrite>,
    /// What each path a download lands on currently holds.
    replaced: HashMap<String, Expected>,
    record_changes: Vec<RecordChange>,
    owned: Vec<OwnedItem>,
    defaults: Vec<DefaultDecision>,
}

struct Planned {
    plan: InstallPlan,
    staging: Staging,
    instance_digest: Option<String>,
    previous_overlays: Vec<LegacyOverlay>,
}

struct Planner<'a> {
    profile: &'a Path,
    state: Option<&'a AppliedState>,
    digests: HashMap<String, Option<FileDigest>>,
    blockers: Vec<PlanIssue>,
    conflicts: Vec<PlanConflict>,
    notes: Vec<String>,
    staging: Staging,
}

impl<'a> Planner<'a> {
    fn block(&mut self, code: &str, message: String) {
        self.blockers.push(PlanIssue {
            code: code.to_string(),
            message,
        });
    }

    /// The digest of a profile-relative path, hashed at most once per plan.
    ///
    /// Remembered by the path exactly as spelt. Folding case here would be
    /// right on Windows and wrong everywhere two spellings are two files.
    fn digest(&mut self, relative_path: &str) -> Result<Option<FileDigest>, String> {
        let key = relative_path.to_string();
        if let Some(known) = self.digests.get(&key) {
            return Ok(known.clone());
        }
        let absolute = installer::validate_path_within_base(self.profile, relative_path)?;
        let digest = digest_if_present(&absolute)?;
        self.digests.insert(key, digest.clone());
        Ok(digest)
    }

    fn owned_hash(&self, relative_path: &str) -> Option<&str> {
        self.state?
            .owned
            .iter()
            .find(|item| item.path.eq_ignore_ascii_case(relative_path))
            .map(|item| item.sha256.as_str())
    }
}

/// Whether two profile-relative paths name one file on this disk.
fn is_same_file(profile: &Path, first: &str, second: &str) -> bool {
    match (
        std::fs::canonicalize(profile.join(first)),
        std::fs::canonicalize(profile.join(second)),
    ) {
        (Ok(first), Ok(second)) => first == second,
        _ => false,
    }
}

fn json_text(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn plan_fingerprint(plan: &InstallPlan) -> String {
    let mut unsigned = plan.clone();
    unsigned.fingerprint = String::new();
    unsigned.recovered_interrupted = false;
    sha256_hex(&serde_json::to_vec(&unsigned).unwrap_or_default())
}

/// Reads a profile and works out what applying `release` would do to it.
fn build_plan(profile: &Path, release: &Release, release_digest: &str) -> Result<Planned, String> {
    if !profile.is_dir() {
        return Err(format!(
            "The modpack folder does not exist: {}",
            profile.display()
        ));
    }

    let state = read_applied_state(profile)?;
    let instance = read_instance(profile)?;
    let mut planner = Planner {
        profile,
        state: state.as_ref(),
        digests: HashMap::new(),
        blockers: Vec::new(),
        conflicts: Vec::new(),
        notes: Vec::new(),
        staging: Staging::default(),
    };

    // ---- Is this release for this profile at all? ----------------------------
    if release.role != Role::Client {
        planner.block(
            "role",
            format!(
                "'{}' is a server release. CEMM installs client releases into CurseForge profiles; the server is deployed with the admin's own tooling.",
                release.name
            ),
        );
    }

    let mut mode = PlanMode::Adopt;
    let mut from_release = None;
    if let Some(state) = state.as_ref() {
        from_release = Some(state.release_id.clone());
        if state.lineage != release.lineage {
            planner.block(
                "lineage",
                format!(
                    "This profile follows the pack '{}' (release {}). This code is for '{}', a different pack. Choose the right profile, or the right code.",
                    state.lineage, state.release_id, release.lineage
                ),
            );
        } else if state.release_id == release.release_id {
            if state.release_digest != release_digest {
                planner.block(
                    "release-reused",
                    format!(
                        "Release '{}' was already applied here, but this code now leads to different content. A release never changes once published, so CEMM will not apply this. Ask your admin for a new code.",
                        release.release_id
                    ),
                );
            }
            mode = PlanMode::Reapply;
        } else if release.sequence < state.sequence {
            planner.block(
                "downgrade",
                format!(
                    "This profile is on release '{}', which is newer than '{}'. CEMM does not move a profile backwards: a world saved on the newer release is not safe to open on an older one. To go back, restore the profile backup and the matching world backup together.",
                    state.release_id, release.release_id
                ),
            );
        } else if release.sequence == state.sequence {
            planner.block(
                "sequence",
                format!(
                    "Releases '{}' and '{}' claim the same position in this pack's history. Ask your admin which one is current.",
                    state.release_id, release.release_id
                ),
            );
        } else {
            mode = PlanMode::Upgrade;
            if !release.supported_from.is_empty()
                && !release.supported_from.contains(&state.release_id)
            {
                planner.block(
                    "unsupported-start",
                    format!(
                        "Release '{}' can only be applied on top of: {}. This profile is on '{}'. Apply one of those first.",
                        release.release_id,
                        release.supported_from.join(", "),
                        state.release_id
                    ),
                );
            }
            if state.base != release.base {
                planner.notes.push(format!(
                    "This release moves the pack to a new official base: {} (file {}).",
                    if release.base.name.is_empty() {
                        "the official pack"
                    } else {
                        &release.base.name
                    },
                    release.base.file_id
                ));
            }
        }
    }

    let records = match instance.as_ref() {
        Some(instance) => {
            let value = &instance.value;
            let game_version = value
                .get("gameVersion")
                .and_then(Value::as_str)
                .unwrap_or("");
            if game_version != release.minecraft {
                planner.block(
                    "minecraft",
                    format!(
                        "This release is for Minecraft {}; this profile is Minecraft {}. CEMM cannot change a profile's Minecraft version. Import the pack's export for {} as a new profile and apply the code there.",
                        release.minecraft,
                        if game_version.is_empty() { "of an unknown version" } else { game_version },
                        release.minecraft
                    ),
                );
            }
            let loader = value
                .pointer("/baseModLoader/name")
                .and_then(Value::as_str)
                .unwrap_or("");
            if game_version == release.minecraft && loader != release.loader {
                planner.block(
                    "loader",
                    format!(
                        "This release needs the mod loader {}; this profile has {}. CEMM does not install loaders. In CurseForge open this profile's options, set the modloader version to {}, let it finish, then apply the code again.",
                        release.loader,
                        if loader.is_empty() { "none recorded" } else { loader },
                        release.loader
                    ),
                );
            }

            let linked_project = value
                .pointer("/installedModpack/addonID")
                .and_then(Value::as_u64)
                .filter(|id| *id != 0)
                .or_else(|| {
                    value
                        .get("projectID")
                        .and_then(Value::as_u64)
                        .filter(|id| *id != 0)
                });
            if state.is_none() {
                let official = linked_project == Some(release.base.project_id);
                if official && release.adoption.allow_official_profile {
                    planner.notes.push(format!(
                        "CurseForge still treats this profile as the official pack. Do not let CurseForge update, repair or change the version of it: that would overwrite '{}'.",
                        release.name
                    ));
                } else if let Some(project) = linked_project {
                    let message = if official {
                        format!(
                            "This profile is the official pack itself, which CurseForge keeps updated on its own and would overwrite '{}' the next time it does. Import the '{}' export as its own profile and apply the code there.",
                            release.name, release.name
                        )
                    } else {
                        format!(
                            "This profile is a different CurseForge modpack (project {project}), not '{}'. Choose the '{}' profile.",
                            release.name, release.name
                        )
                    };
                    planner.block("foreign-profile", message);
                }
            }
            instance_records(value)
        }
        None => {
            planner.block(
                "no-instance",
                format!(
                    "{} has no minecraftinstance.json, so it is not a CurseForge profile. A release needs one to check the Minecraft version and mod loader against.",
                    profile.display()
                ),
            );
            Vec::new()
        }
    };

    let record_for = |project_id: u64| {
        records
            .iter()
            .find(|record| record.project_id == project_id)
    };

    // ---- Addons ---------------------------------------------------------------
    let target_projects: HashSet<u64> = release
        .addons
        .iter()
        .map(|addon| addon.project_id)
        .collect();
    let mut target_paths = release.target_paths_lowercase();
    for addon in &release.addons {
        target_paths.insert(format!("{}.disabled", addon.relative_path()).to_lowercase());
    }
    // The same paths as the release spells them, to tell a path from its alias.
    let exact_targets: HashMap<String, String> = release
        .addons
        .iter()
        .flat_map(|addon| {
            let path = addon.relative_path();
            [format!("{path}.disabled"), path]
        })
        .chain(release.files.iter().map(|file| file.path.clone()))
        .map(|path| (path.to_lowercase(), path))
        .collect();

    let mut addon_actions = Vec::new();
    let mut launcher = LauncherPlan::default();
    let mut handled_paths: HashSet<String> = HashSet::new();
    let mut present_projects = 0usize;
    let mut enabled_projects = 0usize;

    for addon in &release.addons {
        let active = addon.relative_path();
        let disabled = format!("{active}.disabled");
        handled_paths.insert(active.to_lowercase());
        handled_paths.insert(disabled.to_lowercase());

        // Other files known to be this project's: an older version CurseForge
        // or CEMM recorded. They are what "one active version per project"
        // means in practice.
        let mut predecessors: Vec<String> = Vec::new();
        let mut consider = |planner: &mut Planner, name: &str| -> Result<(), String> {
            if name.eq_ignore_ascii_case(&addon.file_name)
                || !installer::is_safe_addon_file_name(name)
            {
                return Ok(());
            }
            for candidate in [
                format!("{}/{name}", addon.category.directory()),
                format!("{}/{name}.disabled", addon.category.directory()),
            ] {
                if planner.digest(&candidate)?.is_some()
                    && !predecessors
                        .iter()
                        .any(|known| known.eq_ignore_ascii_case(&candidate))
                {
                    predecessors.push(candidate);
                }
            }
            Ok(())
        };
        let record = record_for(addon.project_id);
        if let Some(record) =
            record.filter(|record| record.category.is_none_or(|c| c == addon.category))
        {
            let name = record.canonical_file_name().to_string();
            consider(&mut planner, &name)?;
        }
        let owned_names: Vec<String> = state
            .iter()
            .flat_map(|state| state.owned.iter())
            .filter(|item| item.project_id == Some(addon.project_id))
            .filter_map(|item| item.path.rsplit('/').next())
            .map(|name| name.strip_suffix(".disabled").unwrap_or(name).to_string())
            .collect();
        for name in owned_names {
            consider(&mut planner, &name)?;
        }
        for predecessor in &predecessors {
            handled_paths.insert(predecessor.to_lowercase());
        }

        let active_digest = planner.digest(&active)?;
        let disabled_digest = planner.digest(&disabled)?;
        let exact = |digest: &Option<FileDigest>| {
            digest
                .as_ref()
                .is_some_and(|digest| digest.sha256 == addon.sha256)
        };
        let project_present =
            active_digest.is_some() || disabled_digest.is_some() || !predecessors.is_empty();

        let op;
        let mut note = String::new();
        let mut from_file = predecessors
            .first()
            .and_then(|path| path.rsplit('/').next())
            .map(str::to_string);

        if addon.enabled {
            enabled_projects += 1;
            if project_present {
                present_projects += 1;
            }

            if exact(&active_digest) {
                if predecessors.is_empty() {
                    op = AddonOp::Verified;
                } else {
                    op = AddonOp::Replace;
                    note = "The new file is already here; the old version is removed.".to_string();
                }
            } else if active_digest.is_none() && exact(&disabled_digest) {
                op = AddonOp::Enable;
                note = "Switched off in this profile; the release needs it on.".to_string();
                planner.staging.renames.push(Rename {
                    from: disabled.clone(),
                    to: active.clone(),
                    sha256: addon.sha256.clone(),
                });
                planner
                    .staging
                    .replaced
                    .insert(active.clone(), Expected::Absent);
            } else {
                let Some(url) = addon.download_url.clone() else {
                    planner.block(
                        "manual-download",
                        format!(
                            "'{}' cannot be downloaded for you: its author does not allow third-party downloads. Download '{}' from {} and put it in the '{}' folder of this profile, then apply the code again.",
                            addon.name,
                            addon.file_name,
                            addon
                                .website_url
                                .as_deref()
                                .map(|site| format!("{site}/files/{}", addon.file_id))
                                .unwrap_or_else(|| format!("CurseForge (project {}, file {})", addon.project_id, addon.file_id)),
                            addon.category.directory()
                        ),
                    );
                    addon_actions.push(AddonAction {
                        project_id: addon.project_id,
                        name: addon.name.clone(),
                        category: addon.category,
                        file_name: addon.file_name.clone(),
                        op: AddonOp::Install,
                        from_file,
                        pinned: addon.pinned,
                        note: "Needs a manual download.".to_string(),
                    });
                    continue;
                };
                op = if project_present {
                    AddonOp::Replace
                } else {
                    AddonOp::Install
                };
                if active_digest.is_some() {
                    note = "The file here does not match the release and is replaced.".to_string();
                    from_file.get_or_insert_with(|| addon.file_name.clone());
                }
                planner.staging.downloads.push(Download {
                    origin: PayloadOrigin::Addon {
                        url,
                        name: addon.name.clone(),
                    },
                    relative_path: active.clone(),
                    sha256: addon.sha256.clone(),
                    size: addon.size,
                    conflict: None,
                });
                planner.staging.replaced.insert(
                    active.clone(),
                    match active_digest.as_ref() {
                        Some(digest) => Expected::Sha256(digest.sha256.clone()),
                        None => Expected::Absent,
                    },
                );
                // A switched-off copy of the same file would be a second
                // version of the project lying beside the active one.
                if let Some(digest) = disabled_digest.as_ref() {
                    planner.staging.removals.push(Removal {
                        relative_path: disabled.clone(),
                        sha256: digest.sha256.clone(),
                        conflict: None,
                    });
                }
            }

            for predecessor in &predecessors {
                if let Some(digest) = planner.digest(predecessor)? {
                    planner.staging.removals.push(Removal {
                        relative_path: predecessor.clone(),
                        sha256: digest.sha256,
                        conflict: None,
                    });
                }
            }
            planner.staging.owned.push(OwnedItem {
                path: active.clone(),
                sha256: addon.sha256.clone(),
                project_id: Some(addon.project_id),
            });
        } else {
            // Switched off in the release: nothing of the project may be active.
            let mut active_files: Vec<(String, FileDigest)> = Vec::new();
            if let Some(digest) = active_digest.clone() {
                active_files.push((active.clone(), digest));
            }
            for predecessor in predecessors
                .iter()
                .filter(|path| !path.ends_with(".disabled"))
            {
                if let Some(digest) = planner.digest(predecessor)? {
                    active_files.push((predecessor.clone(), digest));
                }
            }
            if active_files.is_empty() {
                op = AddonOp::Verified;
                note = "Switched off in this release.".to_string();
                if let Some(digest) = disabled_digest.as_ref() {
                    planner.staging.owned.push(OwnedItem {
                        path: disabled.clone(),
                        sha256: digest.sha256.clone(),
                        project_id: Some(addon.project_id),
                    });
                }
            } else {
                op = AddonOp::Disable;
                note = "The release switches this addon off.".to_string();
                for (path, digest) in active_files {
                    let target = format!("{path}.disabled");
                    let occupied = match planner.digest(&target)? {
                        Some(existing) => Expected::Sha256(existing.sha256),
                        None => Expected::Absent,
                    };
                    planner.staging.replaced.insert(target.clone(), occupied);
                    planner.staging.owned.push(OwnedItem {
                        path: target.clone(),
                        sha256: digest.sha256.clone(),
                        project_id: Some(addon.project_id),
                    });
                    planner.staging.renames.push(Rename {
                        from: path,
                        to: target,
                        sha256: digest.sha256,
                    });
                }
            }
        }

        // The launcher's record follows whenever the file it names changes.
        let record_matches = record.is_some_and(|record| {
            let expected = if addon.enabled {
                addon.file_name.clone()
            } else {
                format!("{}.disabled", addon.file_name)
            };
            record.file_name_on_disk == expected
        });
        match record {
            Some(_) if !record_matches || op != AddonOp::Verified => {
                planner.staging.record_changes.push(RecordChange::Set {
                    addon: addon.clone(),
                });
                launcher.records_updated += 1;
            }
            Some(_) => {}
            None if op != AddonOp::Verified => launcher.records_missing += 1,
            None => {}
        }
        if addon.pinned {
            let pin_state = match record {
                Some(record) if record.locked => PinState::AlreadyPinned,
                Some(_) => {
                    planner.staging.record_changes.push(RecordChange::Pin {
                        project_id: addon.project_id,
                    });
                    PinState::WillPin
                }
                None => PinState::Manual,
            };
            launcher.pins.push(PinAction {
                project_id: addon.project_id,
                name: addon.name.clone(),
                file_name: addon.file_name.clone(),
                state: pin_state,
            });
        }

        addon_actions.push(AddonAction {
            project_id: addon.project_id,
            name: addon.name.clone(),
            category: addon.category,
            file_name: addon.file_name.clone(),
            op,
            from_file: if op == AddonOp::Verified {
                None
            } else {
                from_file
            },
            pinned: addon.pinned,
            note,
        });
    }

    // A profile CEMM has never seen must look like this pack before it is
    // taken into the lineage; sharing a Minecraft version is not resemblance.
    if state.is_none() && enabled_projects > 0 {
        let percent = present_projects * 100 / enabled_projects;
        if percent < usize::from(release.adoption.min_project_overlap_percent) {
            planner.block(
                "not-this-pack",
                format!(
                    "Only {present_projects} of this release's {enabled_projects} addons are in this profile, so it does not look like '{}'. Import the pack's export as a profile first, then apply the code to that profile.",
                    release.name
                ),
            );
        }
    }

    // ---- Managed files ----------------------------------------------------------
    let mut file_actions = Vec::new();
    let mut written_files: HashSet<String> = HashSet::new();
    let previous_default = |path: &str| {
        state
            .iter()
            .flat_map(|state| state.defaults.iter())
            .find(|default| default.path.eq_ignore_ascii_case(path))
            .map(|default| default.supplied_sha256.clone())
    };

    for file in &release.files {
        handled_paths.insert(file.path.to_lowercase());
        if !file.role.applies_to(Role::Client) {
            file_actions.push(FileAction {
                path: file.path.clone(),
                op: FileOp::SkipRole,
                policy: file.policy,
                source: file.source,
                note: "Server-only content.".to_string(),
            });
            continue;
        }

        let local = planner.digest(&file.path)?;
        let local_hash = local.as_ref().map(|digest| digest.sha256.clone());
        let mut op = FileOp::Verified;
        let mut note = String::new();
        let mut conflict: Option<String> = None;
        let mut fetch = false;

        match file.policy {
            FilePolicy::Managed => {
                match local_hash.as_deref() {
                    Some(hash) if hash == file.sha256 => {}
                    None => {
                        op = FileOp::Add;
                        fetch = true;
                    }
                    Some(hash) => {
                        let known = planner.owned_hash(&file.path) == Some(hash)
                            || file.previous.iter().any(|previous| previous == hash);
                        fetch = true;
                        if known {
                            op = FileOp::Replace;
                        } else {
                            op = FileOp::Repair;
                            note = "Changed in this profile.".to_string();
                            conflict = Some(file.path.clone());
                            planner.conflicts.push(PlanConflict {
                                id: file.path.clone(),
                                kind: ConflictKind::ModifiedManagedFile,
                                required: true,
                                message: format!(
                                    "{} was changed in this profile and is not any version this pack has shipped. The release needs its own version; yours is kept in the backup.",
                                    file.path
                                ),
                            });
                        }
                    }
                }
                planner.staging.owned.push(OwnedItem {
                    path: file.path.clone(),
                    sha256: file.sha256.clone(),
                    project_id: None,
                });
            }
            FilePolicy::Default => {
                let supplied_before = previous_default(&file.path);
                let mut kept_local = false;
                match local_hash.as_deref() {
                    Some(hash) if hash == file.sha256 => {}
                    None => {
                        op = FileOp::Add;
                        fetch = true;
                    }
                    Some(hash) => {
                        let untouched = supplied_before.as_deref() == Some(hash)
                            || file.previous.iter().any(|previous| previous == hash);
                        let pack_changed = supplied_before
                            .as_deref()
                            .is_some_and(|before| before != file.sha256);
                        // A file the release sets individual keys in is shared:
                        // the pack owns those keys, the player owns the rest.
                        // Its whole-file default only ever fills an absence.
                        // Offering it as a replacement would put "take the
                        // pack's version" next to a file full of someone's
                        // keybinds, to deliver one value the settings pass
                        // delivers anyway.
                        let mixed = release
                            .settings
                            .iter()
                            .any(|setting| setting.path.eq_ignore_ascii_case(&file.path));
                        if untouched {
                            op = FileOp::Replace;
                            note = "Unchanged pack default, moved to the new one.".to_string();
                            fetch = true;
                        } else if mixed {
                            op = FileOp::KeepLocal;
                            kept_local = true;
                            note = "Your own settings are kept; only the values this pack requires are set.".to_string();
                        } else if pack_changed {
                            op = FileOp::KeepLocal;
                            kept_local = true;
                            note = "You changed this, and so did the pack. Yours is kept unless you choose the pack's.".to_string();
                            conflict = Some(file.path.clone());
                            fetch = true;
                            planner.conflicts.push(PlanConflict {
                                id: file.path.clone(),
                                kind: ConflictKind::DefaultChangedBothSides,
                                required: false,
                                message: format!(
                                    "{} is a personal setting you changed, and this release also changes the pack's default for it. Yours is kept unless you approve taking the pack's version.",
                                    file.path
                                ),
                            });
                        } else {
                            op = FileOp::KeepLocal;
                            kept_local = true;
                            note = "Your own version is kept.".to_string();
                        }
                    }
                }
                planner.staging.defaults.push(DefaultDecision {
                    path: file.path.clone(),
                    supplied_sha256: file.sha256.clone(),
                    local_sha256: local_hash.clone(),
                    conflict: conflict.clone(),
                    kept_local,
                });
            }
        }

        if fetch {
            if !file.hosted {
                match (file.policy, op) {
                    // A default nobody can deliver is not worth stopping for.
                    (FilePolicy::Default, _) => {
                        planner.notes.push(format!(
                            "{} is a pack default that is not hosted, so it stays as it is here.",
                            file.path
                        ));
                        planner
                            .conflicts
                            .retain(|existing| existing.id != file.path);
                        if let Some(decision) = planner.staging.defaults.last_mut() {
                            decision.conflict = None;
                            decision.kept_local = local_hash.is_some();
                        }
                        op = if local_hash.is_some() {
                            FileOp::KeepLocal
                        } else {
                            FileOp::Verified
                        };
                        note = "Not hosted with this release.".to_string();
                    }
                    (FilePolicy::Managed, _) => {
                        planner.block(
                            "not-hosted",
                            format!(
                                "{} is {} here, and this release does not host a copy CEMM could install. Restore it from the pack's export, or ask your admin to publish it with the release.",
                                file.path,
                                if local_hash.is_some() { "different" } else { "missing" }
                            ),
                        );
                    }
                }
            } else {
                planner.staging.downloads.push(Download {
                    origin: PayloadOrigin::Object,
                    relative_path: file.path.clone(),
                    sha256: file.sha256.clone(),
                    size: file.size,
                    conflict: conflict.clone(),
                });
                planner.staging.replaced.insert(
                    file.path.clone(),
                    match local_hash.as_ref() {
                        Some(hash) => Expected::Sha256(hash.clone()),
                        None => Expected::Absent,
                    },
                );
                // Only unconditional writes count as "the file is being
                // replaced" for the settings pass below.
                if conflict.is_none() {
                    written_files.insert(file.path.to_lowercase());
                }
            }
        }

        file_actions.push(FileAction {
            path: file.path.clone(),
            op,
            policy: file.policy,
            source: file.source,
            note,
        });
    }

    // ---- Retirements --------------------------------------------------------------
    let mut retirements: BTreeMap<String, (String, Option<u64>)> = BTreeMap::new();
    let mut retired_projects: HashSet<u64> = HashSet::new();

    if let Some(state) = state.as_ref() {
        for item in &state.owned {
            let key = item.path.to_lowercase();
            // An owned file whose path differs from a target path only by
            // letter case. On a case-insensitive disk that *is* the target,
            // under its old spelling. On a case-sensitive one it is a second
            // file the game would load beside the new one, and it has to go.
            let stale_spelling = exact_targets.get(&key).is_some_and(|target| {
                target != &item.path && !is_same_file(profile, &item.path, target)
            });
            if !stale_spelling && (target_paths.contains(&key) || handled_paths.contains(&key)) {
                continue;
            }
            let Some(local) = planner.digest(&item.path)? else {
                continue;
            };
            handled_paths.insert(key.clone());
            let display_name = item
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&item.path)
                .to_string();
            let project = item
                .project_id
                .filter(|project| !target_projects.contains(project));
            if local.sha256 == item.sha256 {
                retirements.insert(item.path.clone(), (local.sha256, project));
            } else {
                planner.conflicts.push(PlanConflict {
                    id: item.path.clone(),
                    kind: ConflictKind::ModifiedRetiredFile,
                    required: true,
                    message: format!(
                        "{} is no longer part of this pack, but it was changed in this profile since CEMM installed it. Removing it needs your approval; it is kept in the backup.",
                        item.path
                    ),
                });
                planner.staging.removals.push(Removal {
                    relative_path: item.path.clone(),
                    sha256: local.sha256,
                    conflict: Some(item.path.clone()),
                });
                if let Some(project) = project {
                    retired_projects.insert(project);
                }
                push_retirement_action(
                    &mut addon_actions,
                    &mut file_actions,
                    &item.path,
                    &display_name,
                    item.project_id,
                    &records,
                    true,
                );
            }
        }
    }
    // Former pack content this profile was never told it owned — the state a
    // freshly imported export is in. Removed only on an exact hash match.
    for retired in &release.retired {
        let mut candidates = vec![retired.path.clone()];
        if retired.project_id.is_some() {
            candidates.push(format!("{}.disabled", retired.path));
        }
        for candidate in candidates {
            let key = candidate.to_lowercase();
            if target_paths.contains(&key) || handled_paths.contains(&key) {
                continue;
            }
            let Some(local) = planner.digest(&candidate)? else {
                continue;
            };
            if local.sha256 != retired.sha256 {
                continue;
            }
            handled_paths.insert(key);
            let project = retired
                .project_id
                .filter(|project| !target_projects.contains(project));
            retirements.insert(candidate, (local.sha256, project));
        }
    }
    for (path, (sha256, project)) in &retirements {
        let display_name = path.rsplit('/').next().unwrap_or(path).to_string();
        planner.staging.removals.push(Removal {
            relative_path: path.clone(),
            sha256: sha256.clone(),
            conflict: None,
        });
        if let Some(project) = project {
            retired_projects.insert(*project);
        }
        push_retirement_action(
            &mut addon_actions,
            &mut file_actions,
            path,
            &display_name,
            *project,
            &records,
            false,
        );
    }
    for project in &retired_projects {
        if record_for(*project).is_some() {
            planner.staging.record_changes.push(RecordChange::Remove {
                project_id: *project,
            });
            launcher.records_removed += 1;
        }
    }

    // ---- Required settings ----------------------------------------------------------
    let mut setting_actions = Vec::new();
    for setting in &release.settings {
        let required = json_text(&setting.value);
        let shipped_now = written_files.contains(&setting.path.to_lowercase());
        if shipped_now {
            // Checked for real against the staged bytes during apply.
            planner.staging.setting_writes.push(SettingWrite {
                relative_path: setting.path.clone(),
                pointer: setting.pointer.clone(),
                value: setting.value.clone(),
            });
            setting_actions.push(SettingAction {
                path: setting.path.clone(),
                pointer: setting.pointer.clone(),
                op: SettingOp::WithFile,
                current: None,
                required,
                reason: setting.reason.clone(),
            });
            continue;
        }

        let absolute = installer::validate_path_within_base(profile, &setting.path)?;
        let text = match std::fs::read_to_string(&absolute) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                planner.block(
                    "setting-missing",
                    format!(
                        "This release needs {} set to {} in {}, but that file does not exist here yet. Start the game once so the mod creates it, close the game, then apply the code again.",
                        setting.pointer, required, setting.path
                    ),
                );
                continue;
            }
            Err(error) => {
                planner.block(
                    "setting-unreadable",
                    format!("{} could not be read ({error}).", setting.path),
                );
                continue;
            }
        };
        match apply_json_setting(&text, &setting.pointer, &setting.value) {
            Ok(SettingOutcome::AlreadySet) => setting_actions.push(SettingAction {
                path: setting.path.clone(),
                pointer: setting.pointer.clone(),
                op: SettingOp::AlreadySet,
                current: Some(required.clone()),
                required,
                reason: setting.reason.clone(),
            }),
            Ok(SettingOutcome::Changed { previous, .. }) => {
                let digest = planner.digest(&setting.path)?;
                if let Some(digest) = digest {
                    planner
                        .staging
                        .replaced
                        .entry(setting.path.clone())
                        .or_insert(Expected::Sha256(digest.sha256));
                }
                planner.staging.setting_writes.push(SettingWrite {
                    relative_path: setting.path.clone(),
                    pointer: setting.pointer.clone(),
                    value: setting.value.clone(),
                });
                setting_actions.push(SettingAction {
                    path: setting.path.clone(),
                    pointer: setting.pointer.clone(),
                    op: SettingOp::Apply,
                    current: previous.as_ref().map(json_text),
                    required,
                    reason: setting.reason.clone(),
                });
            }
            Err(problem) => planner.block(
                "setting-unsupported",
                format!(
                    "This release needs {} set to {} in {}, but CEMM cannot change that file safely: {problem}. Set that one value by hand and apply the code again; your other settings in the file are not touched either way.",
                    setting.pointer, required, setting.path
                ),
            ),
        }
    }

    // ---- What is here that nobody claims ------------------------------------------------
    let mut extras = Vec::new();
    for category in AddonCategory::ALL {
        let directory = profile.join(category.directory());
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .filter(|entry| entry.path().is_file())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();
        names.sort();
        for name in names {
            let relative = format!("{}/{name}", category.directory());
            if handled_paths.contains(&relative.to_lowercase()) {
                continue;
            }
            // Only jars and packs are worth naming; a stray readme is not.
            let canonical = name
                .strip_suffix(".disabled")
                .unwrap_or(&name)
                .to_lowercase();
            if !(canonical.ends_with(".jar") || canonical.ends_with(".zip")) {
                continue;
            }
            let label = records
                .iter()
                .find(|record| record.file_name_on_disk.eq_ignore_ascii_case(&name))
                .map(|record| record.name.clone())
                .filter(|label| !label.is_empty())
                .unwrap_or_else(|| name.clone());
            extras.push(ExtraItem {
                path: relative,
                label,
            });
        }
    }

    // ---- Is the game running? -----------------------------------------------------------
    if installer::is_open_elsewhere(&profile.join("logs").join("latest.log")) {
        planner.block(
            "in-use",
            "Minecraft is running in this profile. Close the game completely, then review the update again.".to_string(),
        );
    } else {
        let touched: Vec<String> = planner
            .staging
            .removals
            .iter()
            .map(|removal| removal.relative_path.clone())
            .chain(
                planner
                    .staging
                    .renames
                    .iter()
                    .map(|rename| rename.from.clone()),
            )
            .chain(planner.staging.replaced.keys().cloned())
            .collect();
        if let Some(busy) = touched
            .iter()
            .find(|relative| installer::is_open_elsewhere(&profile.join(relative)))
        {
            planner.block(
                "in-use",
                format!(
                    "{busy} is open in another program, most likely Minecraft. Close the game completely, then review the update again."
                ),
            );
        }
    }

    // ---- Totals ---------------------------------------------------------------------------
    planner.conflicts.sort_by(|a, b| a.id.cmp(&b.id));
    let unconditional = |conflict: &Option<String>| conflict.is_none();
    let download_count = planner
        .staging
        .downloads
        .iter()
        .filter(|download| unconditional(&download.conflict))
        .count() as u32;
    let download_bytes = planner
        .staging
        .downloads
        .iter()
        .filter(|download| unconditional(&download.conflict))
        .map(|download| download.size)
        .sum();
    let change_count = download_count
        + planner
            .staging
            .removals
            .iter()
            .filter(|removal| unconditional(&removal.conflict))
            .count() as u32
        + planner.staging.renames.len() as u32
        + setting_actions
            .iter()
            .filter(|action| action.op == SettingOp::Apply)
            .count() as u32;

    launcher.pins.sort_by_key(|pin| pin.project_id);
    let mut plan = InstallPlan {
        lineage: release.lineage.clone(),
        release_id: release.release_id.clone(),
        release_name: release.name.clone(),
        mode,
        from_release,
        blockers: planner.blockers,
        conflicts: planner.conflicts,
        addons: addon_actions,
        files: file_actions,
        settings: setting_actions,
        launcher,
        extras,
        notes: planner.notes,
        download_count,
        download_bytes,
        change_count,
        fingerprint: String::new(),
        recovered_interrupted: false,
    };
    plan.fingerprint = plan_fingerprint(&plan);

    Ok(Planned {
        plan,
        staging: planner.staging,
        instance_digest: instance.map(|instance| instance.digest),
        previous_overlays: state.map(|state| state.legacy_overlays).unwrap_or_default(),
    })
}

fn push_retirement_action(
    addon_actions: &mut Vec<AddonAction>,
    file_actions: &mut Vec<FileAction>,
    path: &str,
    display_name: &str,
    project: Option<u64>,
    records: &[InstanceRecord],
    conflicted: bool,
) {
    let category = path
        .split('/')
        .next()
        .and_then(AddonCategory::from_directory);
    let note = if conflicted {
        "Changed in this profile; removed only with your approval.".to_string()
    } else {
        String::new()
    };
    match (project, category) {
        (Some(project_id), Some(category)) => addon_actions.push(AddonAction {
            project_id,
            name: records
                .iter()
                .find(|record| record.project_id == project_id)
                .map(|record| record.name.clone())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| display_name.to_string()),
            category,
            file_name: display_name.to_string(),
            op: AddonOp::Retire,
            from_file: Some(display_name.to_string()),
            pinned: false,
            note,
        }),
        _ => file_actions.push(FileAction {
            path: path.to_string(),
            op: FileOp::Retire,
            policy: FilePolicy::Managed,
            source: if path.to_lowercase().ends_with(".jar") {
                FileSource::LocalBinary
            } else {
                FileSource::Authored
            },
            note,
        }),
    }
}

/// The plan for applying `release` to the profile at `modpack_path`.
///
/// Read-only, with one exception: if an earlier update was cut off partway,
/// its rollback is finished first. A plan made against a half-installed
/// profile would describe a starting point that stops existing the moment
/// apply recovers it, and the player would be approving something that can
/// never happen.
pub async fn plan_release(
    modpack_path: String,
    release: Release,
    release_digest: String,
) -> Result<InstallPlan, String> {
    crate::composables::release::validate_release(&release)?;
    let profile = Path::new(&modpack_path);
    let recovered = if installer::has_interrupted_install(profile) {
        let _lock = ProfileLock::acquire(profile, "recovering an interrupted update")?;
        installer::recover_interrupted_install(profile).await?;
        true
    } else {
        false
    };
    let mut plan = build_plan(profile, &release, &release_digest)?.plan;
    plan.recovered_interrupted = recovered;
    Ok(plan)
}

// ---------------------------------------------------------------------------
// Applying
// ---------------------------------------------------------------------------

/// Where payload bytes are read from.
pub(crate) enum PayloadSource {
    Remote {
        repo: String,
        revision: String,
    },
    /// Test storage: objects named by hash, addon files by name.
    #[cfg(test)]
    Local {
        objects: PathBuf,
        addons: PathBuf,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRequest {
    pub modpack_path: String,
    pub repo: String,
    pub release: Release,
    pub release_digest: String,
    pub revision: String,
    /// The fingerprint of the plan the player reviewed.
    pub plan_fingerprint: String,
    /// Conflict ids the player approved.
    #[serde(default)]
    pub approved: Vec<String>,
}

fn legacy_projection(release: &Release) -> Manifest {
    let of = |category: AddonCategory| -> Vec<Addon> {
        release
            .addons
            .iter()
            .filter(|addon| addon.category == category)
            .map(|addon| Addon {
                addon_file_id: addon.file_id,
                addon_name: addon.name.clone(),
                addon_project_id: addon.project_id,
                cdn_download_url: addon.download_url.clone().unwrap_or_default(),
                mod_folder_path: category.directory().to_string(),
                version: addon.file_name.clone(),
                web_site_url: addon.website_url.clone(),
                disabled: if addon.enabled { None } else { Some(true) },
                file_name_on_disk: addon.file_name.clone(),
            })
            .collect()
    };
    Manifest {
        update_type: None,
        mods: of(AddonCategory::Mods),
        resourcepacks: of(AddonCategory::Resourcepacks),
        shaderpacks: of(AddonCategory::Shaderpacks),
        datapacks: of(AddonCategory::Datapacks),
        custom_datapacks: Vec::new(),
        config_files: Vec::new(),
    }
}

async fn write_staged(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }
    tokio::fs::write(path, bytes)
        .await
        .map_err(|error| format!("Failed to stage {}: {error}", path.display()))
}

/// Applies a release through the shared install transaction.
pub async fn apply_release(
    request: ApplyRequest,
    progress: InstallProgressCallback,
) -> Result<ApplyReport, String> {
    let source = PayloadSource::Remote {
        repo: request.repo.clone(),
        revision: request.revision.clone(),
    };
    apply_release_from(request, source, progress).await
}

pub(crate) async fn apply_release_from(
    request: ApplyRequest,
    source: PayloadSource,
    progress: InstallProgressCallback,
) -> Result<ApplyReport, String> {
    let ApplyRequest {
        modpack_path,
        repo: _,
        release,
        release_digest,
        revision,
        plan_fingerprint: reviewed_fingerprint,
        approved,
    } = request;
    crate::composables::release::validate_release(&release)?;
    let profile = PathBuf::from(&modpack_path);
    let report_progress = |done: usize, total: usize, message: &str| {
        progress(InstallProgress {
            progress: if total == 0 {
                100.0
            } else {
                (done.min(total) as f64) / (total as f64) * 100.0
            },
            message: message.to_string(),
        });
    };

    let _lock = ProfileLock::acquire(
        &profile,
        &format!("applying release {}", release.release_id),
    )?;
    installer::recover_interrupted_install(&profile).await?;

    report_progress(0, 1, "Checking the profile…");
    let Planned {
        plan,
        staging,
        instance_digest,
        previous_overlays,
    } = build_plan(&profile, &release, &release_digest)?;

    if let Some(blocker) = plan.blockers.first() {
        return Err(blocker.message.clone());
    }
    if plan.fingerprint != reviewed_fingerprint {
        return Err(
            "This profile changed after you reviewed the update, so what you approved is no longer what would happen. Review the update again. Nothing was changed."
                .to_string(),
        );
    }
    let approved: HashSet<&str> = approved.iter().map(String::as_str).collect();
    let unresolved: Vec<&str> = plan
        .conflicts
        .iter()
        .filter(|conflict| conflict.required && !approved.contains(conflict.id.as_str()))
        .map(|conflict| conflict.id.as_str())
        .collect();
    if !unresolved.is_empty() {
        return Err(format!(
            "This update cannot finish until you decide what happens to: {}. Nothing was changed.",
            unresolved.join(", ")
        ));
    }
    let active =
        |conflict: &Option<String>| conflict.as_deref().is_none_or(|id| approved.contains(id));

    let transaction_dir = profile.join(INSTALL_TRANSACTION_DIR);
    let staging_dir = transaction_dir.join("staging");
    let _ = tokio::fs::remove_dir_all(&transaction_dir).await;

    let downloads: Vec<&Download> = staging
        .downloads
        .iter()
        .filter(|download| active(&download.conflict))
        .collect();
    let total_steps = downloads.len() + 2;
    let mut staged_moves: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut staged_by_path: HashMap<String, PathBuf> = HashMap::new();
    let mut cleanup: Vec<PathBuf> = Vec::new();
    let mut preconditions: Vec<(PathBuf, Expected)> = Vec::new();

    // What this install actually changes, counted as it is staged. The plan's
    // own figure leaves out everything that hangs on a decision, and an
    // approved decision is exactly the change whose "before" most needs keeping.
    let mut settings_changed = 0u32;
    let active_removals = staging
        .removals
        .iter()
        .filter(|removal| active(&removal.conflict))
        .count() as u32;

    let stage_result: Result<(), String> = async {
        let fetcher = Fetcher::new(&source, &release.lineage)?;

        // ---- Payloads: fetched, then proven, before anything else is planned on them.
        for (index, download) in downloads.iter().enumerate() {
            let final_path = installer::validate_path_within_base(&profile, &download.relative_path)?;
            let staged = staging_dir.join("payload").join(index.to_string());
            let label = match &download.origin {
                PayloadOrigin::Addon { name, .. } => name.clone(),
                PayloadOrigin::Object => download.relative_path.clone(),
            };
            fetcher.fetch(download, &staged).await.map_err(|error| {
                format!("{label} could not be fetched as published ({error}). Nothing was changed.")
            })?;
            let digest = crate::composables::release::digest_file(&staged)
                .map_err(|error| format!("Failed to check {label}: {error}"))?;
            if digest.sha256 != download.sha256 || digest.size != download.size {
                return Err(format!(
                    "{label} did not arrive as published (expected {} bytes with hash {}…, got {} bytes with hash {}…). Nothing was changed. Try again; if it keeps happening, tell your admin.",
                    download.size,
                    &download.sha256[..12],
                    digest.size,
                    &digest.sha256[..12]
                ));
            }
            staged_by_path.insert(download.relative_path.to_lowercase(), staged.clone());
            staged_moves.push((staged, final_path));
            report_progress(index + 1, total_steps, &format!("Fetched {label}"));
        }

        // ---- Files switched on or off: a verified copy under the new name.
        for (index, rename) in staging.renames.iter().enumerate() {
            let from = installer::validate_path_within_base(&profile, &rename.from)?;
            let to = installer::validate_path_within_base(&profile, &rename.to)?;
            let staged = staging_dir.join("renamed").join(index.to_string());
            if let Some(parent) = staged.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
            }
            tokio::fs::copy(&from, &staged)
                .await
                .map_err(|error| format!("Failed to copy {}: {error}", from.display()))?;
            preconditions.push((from.clone(), Expected::Sha256(rename.sha256.clone())));
            cleanup.push(from);
            staged_moves.push((staged, to));
        }

        // ---- Required settings, against the bytes that will actually be there.
        let mut by_file: BTreeMap<String, Vec<&SettingWrite>> = BTreeMap::new();
        for write in &staging.setting_writes {
            by_file.entry(write.relative_path.clone()).or_default().push(write);
        }
        for (index, (relative_path, writes)) in by_file.into_iter().enumerate() {
            let final_path = installer::validate_path_within_base(&profile, &relative_path)?;
            let staged_source = staged_by_path.get(&relative_path.to_lowercase()).cloned();
            let source_path = staged_source.clone().unwrap_or_else(|| final_path.clone());
            let mut text = tokio::fs::read_to_string(&source_path)
                .await
                .map_err(|error| format!("Failed to read {relative_path}: {error}"))?;
            let mut changed = false;
            for write in writes {
                match apply_json_setting(&text, &write.pointer, &write.value).map_err(|problem| {
                    format!(
                        "{relative_path} cannot take the required setting {}: {problem}. Nothing was changed.",
                        write.pointer
                    )
                })? {
                    SettingOutcome::AlreadySet => {}
                    SettingOutcome::Changed { text: next, .. } => {
                        text = next;
                        changed = true;
                    }
                }
            }
            if !changed {
                continue;
            }
            settings_changed += 1;
            match staged_source {
                // The file is arriving in this update: correct the staged copy.
                Some(staged) => write_staged(&staged, text.as_bytes()).await?,
                None => {
                    let staged = staging_dir.join("settings").join(index.to_string());
                    write_staged(&staged, text.as_bytes()).await?;
                    staged_moves.push((staged, final_path));
                }
            }
        }

        for removal in staging.removals.iter().filter(|removal| active(&removal.conflict)) {
            let path = installer::validate_path_within_base(&profile, &removal.relative_path)?;
            preconditions.push((path.clone(), Expected::Sha256(removal.sha256.clone())));
            cleanup.push(path);
        }
        for (relative_path, expected) in &staging.replaced {
            let path = installer::validate_path_within_base(&profile, relative_path)?;
            preconditions.push((path, expected.clone()));
        }

        // ---- CurseForge's record, patched from a read taken now.
        if !staging.record_changes.is_empty() {
            #[cfg(test)]
            interference::run(&profile);
            let fresh = read_instance(&profile)?.ok_or(
                "minecraftinstance.json disappeared while the update was being prepared. Nothing was changed.",
            )?;
            if Some(&fresh.digest) != instance_digest.as_ref() {
                // CurseForge wrote in the meantime. Fine if it did not touch
                // anything this update decided on, which replanning settles.
                let again = build_plan(&profile, &release, &release_digest)?;
                if again.plan.fingerprint != plan.fingerprint {
                    return Err("CurseForge changed this profile while the update was being prepared. Nothing was changed. Let CurseForge finish, then review the update again.".to_string());
                }
            }
            let mut value = fresh.value.clone();
            patch_instance(&mut value, &staging.record_changes);
            let rendered = render_like(&fresh.text, &value)?;
            let staged = staging_dir.join(INSTANCE_FILE);
            write_staged(&staged, rendered.as_bytes()).await?;
            let live = profile.join(INSTANCE_FILE);
            preconditions.push((live.clone(), Expected::Sha256(fresh.digest.clone())));
            staged_moves.push((staged, live));
        }

        // ---- CEMM's own record, and the snapshot legacy tooling still reads.
        let mut owned = staging.owned.clone();
        let mut defaults = Vec::new();
        for decision in &staging.defaults {
            let take_pack = decision
                .conflict
                .as_deref()
                .is_some_and(|id| approved.contains(id));
            let kept_local = decision.kept_local && !take_pack;
            defaults.push(SuppliedDefault {
                path: decision.path.clone(),
                supplied_sha256: decision.supplied_sha256.clone(),
                local_sha256: if kept_local {
                    decision.local_sha256.clone()
                } else {
                    None
                },
                kept_local,
            });
        }
        owned.sort_by(|a, b| a.path.cmp(&b.path));
        owned.dedup_by(|a, b| a.path.eq_ignore_ascii_case(&b.path));
        let state = AppliedState {
            schema: STATE_SCHEMA,
            lineage: release.lineage.clone(),
            release_id: release.release_id.clone(),
            sequence: release.sequence,
            release_name: release.name.clone(),
            release_digest: release_digest.clone(),
            revision: revision.clone(),
            base: release.base.clone(),
            minecraft: release.minecraft.clone(),
            loader: release.loader.clone(),
            applied_unix: unix_now(),
            cemm_version: env!("CARGO_PKG_VERSION").to_string(),
            owned,
            defaults,
            settings: release
                .settings
                .iter()
                .map(|setting: &RequiredSetting| AppliedSetting {
                    path: setting.path.clone(),
                    pointer: setting.pointer.clone(),
                    value: setting.value.clone(),
                })
                .collect(),
            legacy_overlays: if downloads.is_empty()
                && active_removals == 0
                && staging.renames.is_empty()
                && settings_changed == 0
            {
                previous_overlays.clone()
            } else {
                // Whatever a legacy code overwrote has just been reconciled.
                Vec::new()
            },
            server_release: release.server_release.clone(),
        };
        let staged_state = staging_dir.join("state.json");
        write_staged(
            &staged_state,
            &serde_json::to_vec_pretty(&state)
                .map_err(|error| format!("Failed to encode the applied state: {error}"))?,
        )
        .await?;
        staged_moves.push((staged_state, profile.join(STATE_DIR).join(STATE_FILE_NAME)));

        let staged_manifest = staging_dir.join(INSTALLED_MANIFEST_FILE);
        write_staged(
            &staged_manifest,
            &serde_json::to_vec_pretty(&legacy_projection(&release))
                .map_err(|error| format!("Failed to encode the installed manifest: {error}"))?,
        )
        .await?;
        staged_moves.push((staged_manifest, profile.join(INSTALLED_MANIFEST_FILE)));
        Ok(())
    }
    .await;

    if let Err(error) = stage_result {
        // Nothing live has been touched; only the staging area exists.
        let _ = tokio::fs::remove_dir_all(&transaction_dir).await;
        return Err(error);
    }

    report_progress(total_steps - 1, total_steps, "Finalizing installed files…");
    let content_changes =
        downloads.len() as u32 + active_removals + staging.renames.len() as u32 + settings_changed;
    // CurseForge's own record counts too: it is the player's file, and an
    // edit to it, however small, should be something they can take back.
    let retain_backup = (content_changes > 0 || !staging.record_changes.is_empty()).then(|| {
        format!(
            "{STATE_DIR}/backups/{:012}-{}",
            unix_now(),
            release.release_id
        )
    });
    let outcome = installer::finalize_install_transaction_with(
        &profile,
        &transaction_dir,
        cleanup,
        staged_moves,
        TransactionOptions {
            preconditions,
            retain_backup,
        },
        |_, _| Ok(()),
    )
    .await?;

    // ---- Say what is true now, from the disk. ---------------------------------------
    let mut warnings = Vec::new();
    if let Some(error) = outcome.backup_error {
        warnings.push(format!(
            "The update was installed, but the replaced files could not be kept as a backup: {error}"
        ));
    }
    for download in &downloads {
        let path = profile.join(&download.relative_path);
        if std::fs::metadata(&path).map(|metadata| metadata.len()).ok() != Some(download.size) {
            return Err(format!(
                "{} is not in place after the update. The profile is in an unexpected state; apply the code again to repair it.",
                download.relative_path
            ));
        }
    }
    for setting in &release.settings {
        let path = profile.join(&setting.path);
        let holds = std::fs::read_to_string(&path).ok().is_some_and(|text| {
            matches!(
                apply_json_setting(&text, &setting.pointer, &setting.value),
                Ok(SettingOutcome::AlreadySet)
            )
        });
        if !holds {
            return Err(format!(
                "{} does not hold the required setting {} after the update. Apply the code again to repair it.",
                setting.path, setting.pointer
            ));
        }
    }
    if plan.launcher.records_missing > 0 {
        warnings.push(format!(
            "{} newly installed addon(s) are not in CurseForge's list yet. CurseForge adds files it recognises the next time it scans this profile.",
            plan.launcher.records_missing
        ));
    }
    let manual_pins: Vec<&PinAction> = plan
        .launcher
        .pins
        .iter()
        .filter(|pin| pin.state == PinState::Manual)
        .collect();
    for pin in manual_pins {
        warnings.push(format!(
            "'{}' must stay on {}. CurseForge has no record of it to lock, so once it appears in the profile's mod list, lock it there by hand.",
            pin.name, pin.file_name
        ));
    }

    let kept_local = plan
        .files
        .iter()
        .filter(|file| file.op == FileOp::KeepLocal && !approved.contains(file.path.as_str()))
        .map(|file| file.path.clone())
        .collect();
    let retired = staging
        .removals
        .iter()
        .filter(|removal| active(&removal.conflict))
        .count() as u32;

    report_progress(total_steps, total_steps, "Installation complete!");
    Ok(ApplyReport {
        lineage: release.lineage.clone(),
        release_id: release.release_id.clone(),
        release_name: release.name.clone(),
        mode: plan.mode,
        change_count: content_changes,
        downloaded: downloads.len() as u32,
        retired,
        backup_dir: outcome.backup_dir.and_then(|path| {
            path.strip_prefix(&profile)
                .ok()
                .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        }),
        kept_local,
        launcher: plan.launcher,
        server_release: release.server_release.clone(),
        manual_steps: release.manual_steps.clone(),
        known_issues: release.known_issues.clone(),
        warnings,
    })
}

/// Lets a test play CurseForge writing to the profile at the one moment it
/// matters: after the plan was made and before CEMM reads the launcher's
/// record to patch it. Nothing in a shipped build can reach this.
#[cfg(test)]
pub(crate) mod interference {
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    type Hook = Box<dyn FnOnce() + Send>;
    static BEFORE_RECORD_PATCH: Mutex<Vec<(PathBuf, Hook)>> = Mutex::new(Vec::new());

    pub(crate) fn before_record_patch(profile: &Path, hook: impl FnOnce() + Send + 'static) {
        BEFORE_RECORD_PATCH
            .lock()
            .unwrap()
            .push((profile.to_path_buf(), Box::new(hook)));
    }

    pub(crate) fn run(profile: &Path) {
        let due: Vec<Hook> = {
            let mut hooks = BEFORE_RECORD_PATCH.lock().unwrap();
            let (due, rest): (Vec<_>, Vec<_>) =
                hooks.drain(..).partition(|(path, _)| path == profile);
            *hooks = rest;
            due.into_iter().map(|(_, hook)| hook).collect()
        };
        for hook in due {
            hook();
        }
    }
}

/// Fetches payloads from wherever this install reads them.
struct Fetcher<'a> {
    source: &'a PayloadSource,
    lineage: &'a str,
    addon_client: reqwest::Client,
    object_client: reqwest::Client,
    endpoints: GithubEndpoints,
}

impl<'a> Fetcher<'a> {
    fn new(source: &'a PayloadSource, lineage: &'a str) -> Result<Self, String> {
        // The same rules a legacy install downloads under: CurseForge's CDN
        // only, including across redirects.
        let addon_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if installer::is_trusted_addon_url(attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }))
            .build()
            .map_err(|error| format!("Failed to build HTTP client: {error}"))?;
        let object_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|error| format!("Failed to build HTTP client: {error}"))?;
        Ok(Self {
            source,
            lineage,
            addon_client,
            object_client,
            endpoints: GithubEndpoints::resolve(),
        })
    }

    async fn fetch(&self, download: &Download, staged: &Path) -> Result<(), String> {
        match (self.source, &download.origin) {
            (PayloadSource::Remote { .. }, PayloadOrigin::Addon { url, name }) => {
                installer::validate_addon_download_url(name, url)?;
                installer::download_and_save(
                    &self.addon_client,
                    url,
                    staged,
                    // One byte of slack would hide a truncated-then-padded
                    // file; the hash check that follows is the real gate.
                    download.size,
                )
                .await
            }
            (PayloadSource::Remote { repo, revision }, PayloadOrigin::Object) => {
                github::download_object(
                    &self.object_client,
                    &self.endpoints,
                    repo,
                    revision,
                    self.lineage,
                    &download.sha256,
                    download.size,
                    staged,
                )
                .await
            }
            #[cfg(test)]
            (PayloadSource::Local { objects, addons }, origin) => {
                let from = match origin {
                    PayloadOrigin::Object => objects.join(&download.sha256),
                    PayloadOrigin::Addon { url, .. } => {
                        addons.join(url.rsplit('/').next().unwrap_or_default())
                    }
                };
                if let Some(parent) = staged.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                std::fs::copy(&from, staged)
                    .map(drop)
                    .map_err(|error| format!("Failed to download {}: {error}", from.display()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composables::release::tests::test_release;
    use crate::composables::release::{FileRole, RetiredItem, SettingFormat};
    use std::sync::Arc;

    const BRIDGE: u64 = 1530014;

    /// A throwaway profile, plus the two places payloads are fetched from.
    struct Fixture {
        _temp: tempfile::TempDir,
        profile: PathBuf,
        objects: PathBuf,
        addons: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().expect("temp dir");
            let fixture = Self {
                profile: temp.path().join("Fusion Profile"),
                objects: temp.path().join("objects"),
                addons: temp.path().join("cdn"),
                _temp: temp,
            };
            for directory in [&fixture.profile, &fixture.objects, &fixture.addons] {
                std::fs::create_dir_all(directory).expect("create dir");
            }
            std::fs::create_dir_all(fixture.profile.join("mods")).expect("mods");
            fixture.set_instance(Vec::new());
            fixture
        }

        fn write(&self, relative: &str, content: &str) {
            let path = self.profile.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
            std::fs::write(path, content).expect("write");
        }

        fn read(&self, relative: &str) -> String {
            std::fs::read_to_string(self.profile.join(relative))
                .unwrap_or_else(|error| panic!("{relative} should be readable: {error}"))
        }

        fn exists(&self, relative: &str) -> bool {
            self.profile.join(relative).exists()
        }

        fn set_instance(&self, addons: Vec<Value>) {
            self.set_instance_with(addons, serde_json::json!({}));
        }

        fn set_instance_with(&self, addons: Vec<Value>, extra: Value) {
            let mut instance = serde_json::json!({
                "baseModLoader": { "name": "neoforge-21.1.265", "forgeVersion": "21.1.265" },
                "gameVersion": "1.21.1",
                "name": "Fusion Profile",
                "allocatedMemory": 10240,
                "somethingOnlyCurseForgeKnows": { "keep": [1, 2, 3] },
                "installedAddons": addons,
            });
            for (key, value) in extra.as_object().expect("object") {
                instance[key] = value.clone();
            }
            std::fs::write(
                self.profile.join(INSTANCE_FILE),
                serde_json::to_string_pretty(&instance).expect("serialize"),
            )
            .expect("write instance");
        }

        /// A CurseForge record the way the launcher writes one, with fields
        /// CEMM has no business touching.
        fn record(&self, project_id: u64, file_id: u64, file_name: &str) -> Value {
            let folder = format!("{}\\mods", self.profile.display());
            serde_json::json!({
                "addonID": project_id,
                "name": format!("Addon {project_id}"),
                "modFolderPath": folder,
                "fileNameOnDisk": file_name,
                "filePaths": [format!("{folder}\\{file_name}")],
                "isEnabled": true,
                "isLocked": false,
                "dateInstalled": "2026-11-13T19:54:07.934414Z",
                "authors": [{ "name": "someone" }],
                "installedFile": {
                    "id": file_id,
                    "fileName": file_name,
                    "fileNameOnDisk": file_name,
                    "fileLength": 1,
                    "packageFingerprint": 1,
                    "downloadUrl": format!("https://edge.forgecdn.net/files/{file_name}"),
                    "hashes": [{ "type": 1, "value": "old" }],
                    "modules": [{ "name": "META-INF" }],
                },
            })
        }

        fn instance(&self) -> Value {
            serde_json::from_str(&self.read(INSTANCE_FILE)).expect("instance json")
        }

        fn instance_record(&self, project_id: u64) -> Option<Value> {
            self.instance()["installedAddons"]
                .as_array()
                .expect("addons")
                .iter()
                .find(|addon| addon["addonID"] == project_id)
                .cloned()
        }

        /// Makes a release's payloads fetchable.
        fn host(&self, release: &Release, contents: &[(&str, &str)]) {
            for (name, content) in contents {
                let hash = sha256_hex(content.as_bytes());
                std::fs::write(self.objects.join(&hash), content).expect("host object");
                std::fs::write(self.addons.join(name), content).expect("host addon");
            }
            let _ = release;
        }

        /// The plan as computed, with no side effects at all.
        fn plan(&self, release: &Release) -> InstallPlan {
            build_plan(&self.profile, release, &digest_of(release))
                .expect("planning should succeed")
                .plan
        }

        /// The plan as a player gets it, through the public entry point.
        async fn review(&self, release: &Release) -> InstallPlan {
            plan_release(
                self.profile.to_string_lossy().into_owned(),
                release.clone(),
                digest_of(release),
            )
            .await
            .expect("planning should succeed")
        }

        async fn apply_reviewed(
            &self,
            release: &Release,
            fingerprint: String,
            approved: &[&str],
        ) -> Result<ApplyReport, String> {
            apply_release_from(
                ApplyRequest {
                    modpack_path: self.profile.to_string_lossy().into_owned(),
                    repo: "owner/repo".to_string(),
                    release: release.clone(),
                    release_digest: digest_of(release),
                    revision: "0123456789abcdef0123456789abcdef01234567".to_string(),
                    plan_fingerprint: fingerprint,
                    approved: approved.iter().map(|id| id.to_string()).collect(),
                },
                PayloadSource::Local {
                    objects: self.objects.clone(),
                    addons: self.addons.clone(),
                },
                Arc::new(|_| {}),
            )
            .await
        }

        async fn apply(&self, release: &Release, approved: &[&str]) -> Result<ApplyReport, String> {
            let fingerprint = self.review(release).await.fingerprint;
            self.apply_reviewed(release, fingerprint, approved).await
        }

        /// Every file in the profile by content, for "nothing changed" checks.
        fn snapshot(&self) -> BTreeMap<String, String> {
            let mut files = BTreeMap::new();
            let mut pending = vec![self.profile.clone()];
            while let Some(directory) = pending.pop() {
                for entry in std::fs::read_dir(&directory).expect("read dir").flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        pending.push(path);
                    } else {
                        let relative = path
                            .strip_prefix(&self.profile)
                            .expect("inside profile")
                            .to_string_lossy()
                            .replace('\\', "/");
                        files.insert(relative, sha256_hex(&std::fs::read(&path).expect("read")));
                    }
                }
            }
            files
        }

        fn state(&self) -> AppliedState {
            read_applied_state(&self.profile)
                .expect("state readable")
                .expect("state present")
        }

        fn backups(&self) -> Vec<PathBuf> {
            let mut backups: Vec<PathBuf> = std::fs::read_dir(self.profile.join(".cemm/backups"))
                .map(|entries| entries.flatten().map(|entry| entry.path()).collect())
                .unwrap_or_default();
            backups.sort();
            backups
        }
    }

    fn digest_of(release: &Release) -> String {
        sha256_hex(&serde_json::to_vec(release).expect("serialize"))
    }

    fn addon(project_id: u64, file_id: u64, file_name: &str, content: &str) -> ReleaseAddon {
        ReleaseAddon {
            project_id,
            file_id,
            name: format!("Addon {project_id}"),
            category: AddonCategory::Mods,
            file_name: file_name.to_string(),
            enabled: true,
            size: content.len() as u64,
            sha256: sha256_hex(content.as_bytes()),
            sha1: Some("da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string()),
            fingerprint: Some(4242),
            download_url: Some(format!("https://edge.forgecdn.net/files/{file_name}")),
            website_url: Some(format!(
                "https://www.curseforge.com/minecraft/mc-mods/addon-{project_id}"
            )),
            pinned: false,
            requires: Vec::new(),
        }
    }

    fn managed(path: &str, content: &str) -> crate::composables::release::ReleaseFile {
        crate::composables::release::ReleaseFile {
            path: path.to_string(),
            sha256: sha256_hex(content.as_bytes()),
            size: content.len() as u64,
            source: if path.ends_with(".jar") {
                FileSource::LocalBinary
            } else {
                FileSource::Authored
            },
            policy: FilePolicy::Managed,
            role: FileRole::Both,
            hosted: true,
            previous: Vec::new(),
        }
    }

    fn default_file(path: &str, content: &str) -> crate::composables::release::ReleaseFile {
        let mut file = managed(path, content);
        file.policy = FilePolicy::Default;
        file
    }

    fn sable_setting() -> RequiredSetting {
        RequiredSetting {
            path: "config/tacztweaks.json".to_string(),
            format: SettingFormat::Json,
            pointer: "/general/compatibility/sableCompat".to_string(),
            value: serde_json::json!(false),
            reason: "TaCZ handles Sable natively now".to_string(),
        }
    }

    const TWEAKS_PLAYER: &str = "{\n  \"version\": 3,\n  \"general\": {\n    \"compatibility\": {\n      \"sableCompat\": true,\n      \"other\": 1.0\n    }\n  },\n  \"keyActions\": {\n    \"reload\": \"key.keyboard.g\",\n    \"melee\": \"key.mouse.4\"\n  }\n}\n";

    /// The first release of the test pack, and a profile freshly imported from
    /// its export: every file is there, and CEMM has never seen it.
    fn imported() -> (Fixture, Release) {
        let fixture = Fixture::new();
        let mut release = test_release("r1", 1);
        release.addons = vec![
            addon(1, 10, "alpha-1.jar", "alpha one"),
            addon(2, 20, "beta-1.jar", "beta one"),
            addon(BRIDGE, 8420412, "bridge-1.1.1.jar", "bridge 1.1.1"),
        ];
        release.addons[2].pinned = true;
        release.files = vec![
            managed("mods/atmons-ore-hammer-fix-1.0.0.jar", "hammer fix"),
            managed("kubejs/server_scripts/recipes.js", "recipes v1"),
            managed("kubejs/server_scripts/old.js", "old v1"),
            default_file("config/tacztweaks.json", TWEAKS_PLAYER),
        ];

        fixture.write("mods/alpha-1.jar", "alpha one");
        fixture.write("mods/beta-1.jar", "beta one");
        fixture.write("mods/bridge-1.1.1.jar", "bridge 1.1.1");
        fixture.write("mods/atmons-ore-hammer-fix-1.0.0.jar", "hammer fix");
        fixture.write("mods/players-own-minimap.jar", "the player added this");
        fixture.write("kubejs/server_scripts/recipes.js", "recipes v1");
        fixture.write("kubejs/server_scripts/old.js", "old v1");
        fixture.write("config/tacztweaks.json", TWEAKS_PLAYER);
        fixture.write("options.txt", "fov:90\nkey_key.jump:key.keyboard.space\n");
        fixture.write("saves/World/level.dat", "world data");
        fixture.write("local/ftbquests/progress.snbt", "claimed rewards");
        fixture.set_instance(vec![
            fixture.record(1, 10, "alpha-1.jar"),
            fixture.record(2, 20, "beta-1.jar"),
            fixture.record(BRIDGE, 8420412, "bridge-1.1.1.jar"),
            fixture.record(777, 7, "players-own-minimap.jar"),
        ]);
        (fixture, release)
    }

    /// The release after `imported`'s: alpha updated, beta dropped, a script
    /// changed and another removed, and the compatibility flag required.
    fn second(first: &Release) -> Release {
        let mut release = test_release("r2", 2);
        release.addons = vec![
            addon(1, 11, "alpha-2.jar", "alpha two"),
            first.addons[2].clone(),
        ];
        release.files = vec![
            first.files[0].clone(),
            {
                let mut recipes = managed("kubejs/server_scripts/recipes.js", "recipes v2");
                recipes.previous = vec![sha256_hex(b"recipes v1")];
                recipes
            },
            first.files[3].clone(),
        ];
        release.settings = vec![sable_setting()];
        release.retired = vec![
            RetiredItem {
                path: "mods/alpha-1.jar".to_string(),
                sha256: sha256_hex(b"alpha one"),
                project_id: Some(1),
            },
            RetiredItem {
                path: "mods/beta-1.jar".to_string(),
                sha256: sha256_hex(b"beta one"),
                project_id: Some(2),
            },
            RetiredItem {
                path: "kubejs/server_scripts/old.js".to_string(),
                sha256: sha256_hex(b"old v1"),
                project_id: None,
            },
        ];
        release
    }

    fn host_second(fixture: &Fixture, release: &Release) {
        fixture.host(
            release,
            &[("alpha-2.jar", "alpha two"), ("recipes", "recipes v2")],
        );
    }

    #[tokio::test]
    async fn adopting_an_imported_profile_verifies_and_owns_without_changing_it() {
        let (fixture, release) = imported();
        let before = fixture.snapshot();
        let before_instance = fixture.read(INSTANCE_FILE);

        let plan = fixture.plan(&release);
        assert_eq!(plan.mode, PlanMode::Adopt);
        assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
        assert!(plan.conflicts.is_empty());
        assert_eq!(plan.change_count, 0);
        assert_eq!(plan.download_count, 0, "an import already has every file");
        // The player's own mod is reported, and stays the player's.
        assert_eq!(plan.extras.len(), 1);
        assert_eq!(plan.extras[0].path, "mods/players-own-minimap.jar");

        let report = fixture.apply(&release, &[]).await.expect("adoption");
        assert_eq!(report.change_count, 0);
        // No pack file was replaced. The one thing rewritten is CurseForge's
        // record, to lock the pin, and the version before that is kept.
        let backup = fixture
            .profile
            .join(report.backup_dir.expect("the record's previous version"));
        let kept: Vec<String> = serde_json::from_str::<Value>(
            &std::fs::read_to_string(backup.join("index.json")).unwrap(),
        )
        .unwrap()["replaced"]
            .as_array()
            .unwrap()
            .iter()
            .map(|path| path.as_str().unwrap().to_string())
            .collect();
        assert_eq!(kept, vec![INSTANCE_FILE]);
        assert_eq!(
            std::fs::read_to_string(backup.join("files").join(INSTANCE_FILE)).unwrap(),
            before_instance
        );

        let after = fixture.snapshot();
        for (path, hash) in &before {
            if path == INSTANCE_FILE {
                continue; // the bridge pin is asserted separately below
            }
            assert_eq!(after.get(path), Some(hash), "{path} must be untouched");
        }

        let state = fixture.state();
        assert_eq!(state.lineage, "test-fusion");
        assert_eq!(state.release_id, "r1");
        let owned: HashSet<&str> = state.owned.iter().map(|item| item.path.as_str()).collect();
        // The local jar with no CurseForge record is owned on its hash.
        assert!(owned.contains("mods/atmons-ore-hammer-fix-1.0.0.jar"));
        assert!(owned.contains("mods/alpha-1.jar"));
        assert!(owned.contains("kubejs/server_scripts/old.js"));
        // An extra is never owned, and a default is the player's file.
        assert!(!owned.contains("mods/players-own-minimap.jar"));
        assert!(!owned.contains("config/tacztweaks.json"));
        assert_eq!(state.defaults.len(), 1);
        assert!(!state.defaults[0].kept_local);

        // The pin is made effective in the launcher's own record.
        assert_eq!(fixture.instance_record(BRIDGE).unwrap()["isLocked"], true);
        assert_eq!(fixture.instance_record(1).unwrap()["isLocked"], false);
    }

    #[tokio::test]
    async fn an_update_replaces_retires_and_patches_in_one_transaction() {
        let (fixture, first) = imported();
        fixture.apply(&first, &[]).await.expect("adoption");
        let release = second(&first);
        host_second(&fixture, &release);
        let personal = [
            "options.txt",
            "saves/World/level.dat",
            "local/ftbquests/progress.snbt",
            "mods/players-own-minimap.jar",
        ]
        .map(|path| (path, fixture.read(path)));

        let plan = fixture.plan(&release);
        assert_eq!(plan.mode, PlanMode::Upgrade);
        assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
        assert!(plan.conflicts.is_empty(), "{:?}", plan.conflicts);
        let op_of = |project: u64| {
            plan.addons
                .iter()
                .find(|action| action.project_id == project)
                .map(|action| action.op)
        };
        assert_eq!(op_of(1), Some(AddonOp::Replace));
        assert_eq!(op_of(2), Some(AddonOp::Retire));
        assert_eq!(op_of(BRIDGE), Some(AddonOp::Verified));
        assert_eq!(plan.settings[0].op, SettingOp::Apply);
        assert_eq!(plan.settings[0].current.as_deref(), Some("true"));

        let report = fixture.apply(&release, &[]).await.expect("update");

        // One active version per project, the right bytes, nothing left behind.
        assert_eq!(fixture.read("mods/alpha-2.jar"), "alpha two");
        assert!(!fixture.exists("mods/alpha-1.jar"));
        assert!(!fixture.exists("mods/beta-1.jar"), "dropped by the release");
        assert_eq!(fixture.read("mods/bridge-1.1.1.jar"), "bridge 1.1.1");
        assert_eq!(
            fixture.read("kubejs/server_scripts/recipes.js"),
            "recipes v2"
        );
        assert!(!fixture.exists("kubejs/server_scripts/old.js"));
        assert_eq!(
            fixture.read("mods/atmons-ore-hammer-fix-1.0.0.jar"),
            "hammer fix"
        );

        // The required flag is set; everything else in the file is the player's.
        let tweaks: Value = serde_json::from_str(&fixture.read("config/tacztweaks.json")).unwrap();
        let original: Value = serde_json::from_str(TWEAKS_PLAYER).unwrap();
        assert_eq!(tweaks["general"]["compatibility"]["sableCompat"], false);
        assert_eq!(tweaks["keyActions"], original["keyActions"]);
        assert_eq!(tweaks["general"]["compatibility"]["other"], 1.0);
        assert_eq!(tweaks["version"], 3);

        for (path, content) in personal {
            assert_eq!(fixture.read(path), content, "{path} must be untouched");
        }

        // CurseForge's record follows the files, and only where it has to.
        let alpha = fixture.instance_record(1).expect("alpha record");
        assert_eq!(alpha["installedFile"]["id"], 11);
        assert_eq!(alpha["installedFile"]["fileName"], "alpha-2.jar");
        assert_eq!(alpha["fileNameOnDisk"], "alpha-2.jar");
        assert_eq!(alpha["installedFile"]["packageFingerprint"], 4242);
        assert!(alpha["filePaths"][0]
            .as_str()
            .unwrap()
            .ends_with("\\mods\\alpha-2.jar"));
        assert_eq!(
            alpha["authors"][0]["name"], "someone",
            "unknown fields survive"
        );
        assert_eq!(alpha["installedFile"]["modules"][0]["name"], "META-INF");
        assert!(
            fixture.instance_record(2).is_none(),
            "a retired addon's record goes"
        );
        assert!(
            fixture.instance_record(777).is_some(),
            "the player's record stays"
        );
        let instance = fixture.instance();
        assert_eq!(instance["allocatedMemory"], 10240);
        assert_eq!(instance["somethingOnlyCurseForgeKnows"]["keep"][2], 3);
        assert_eq!(instance["name"], "Fusion Profile");
        assert_eq!(report.launcher.records_updated, 1);
        assert_eq!(report.launcher.records_removed, 1);

        // What was replaced is kept, laid out as the profile was.
        let backup = fixture
            .profile
            .join(report.backup_dir.expect("a backup was kept"));
        assert_eq!(
            std::fs::read_to_string(backup.join("files/mods/alpha-1.jar")).unwrap(),
            "alpha one"
        );
        assert_eq!(
            std::fs::read_to_string(backup.join("files/config/tacztweaks.json")).unwrap(),
            TWEAKS_PLAYER
        );
        let index: Value =
            serde_json::from_str(&std::fs::read_to_string(backup.join("index.json")).unwrap())
                .unwrap();
        assert!(index["created"]
            .as_array()
            .unwrap()
            .iter()
            .any(|path| path == "mods/alpha-2.jar"));

        assert_eq!(fixture.state().release_id, "r2");
        assert!(!fixture.exists(INSTALL_TRANSACTION_DIR));
        assert!(!fixture.exists(".cemm/lock"));
    }

    #[tokio::test]
    async fn applying_the_same_release_twice_changes_nothing_the_second_time() {
        let (fixture, first) = imported();
        fixture.apply(&first, &[]).await.expect("adoption");
        let release = second(&first);
        host_second(&fixture, &release);
        fixture.apply(&release, &[]).await.expect("update");
        let settled = fixture.snapshot();
        let backups = fixture.backups().len();

        let plan = fixture.plan(&release);
        assert_eq!(plan.mode, PlanMode::Reapply);
        assert_eq!(plan.change_count, 0, "{plan:#?}");
        assert_eq!(plan.download_count, 0);

        let report = fixture.apply(&release, &[]).await.expect("reapply");
        assert_eq!(report.change_count, 0);
        assert!(report.backup_dir.is_none());
        assert_eq!(fixture.backups().len(), backups, "no backup for a no-op");

        let mut after = fixture.snapshot();
        let mut before = settled;
        // The record's timestamp is the only thing a reapply rewrites.
        after.remove(".cemm/state.json");
        before.remove(".cemm/state.json");
        assert_eq!(after, before);
    }

    #[tokio::test]
    async fn skipping_a_release_arrives_at_the_same_profile_as_taking_each_one() {
        let third = |second: &Release| {
            let mut release = test_release("r3", 3);
            release.addons = vec![
                addon(1, 12, "alpha-3.jar", "alpha three"),
                second.addons[1].clone(),
            ];
            release.files = vec![second.files[0].clone(), second.files[2].clone()];
            release.settings = second.settings.clone();
            release.retired = second.retired.clone();
            release.retired.extend([
                RetiredItem {
                    path: "mods/alpha-2.jar".to_string(),
                    sha256: sha256_hex(b"alpha two"),
                    project_id: Some(1),
                },
                RetiredItem {
                    path: "kubejs/server_scripts/recipes.js".to_string(),
                    sha256: sha256_hex(b"recipes v2"),
                    project_id: None,
                },
                RetiredItem {
                    path: "kubejs/server_scripts/recipes.js".to_string(),
                    sha256: sha256_hex(b"recipes v1"),
                    project_id: None,
                },
            ]);
            release
        };

        // One player takes every release.
        let (stepwise, first) = imported();
        let second_release = second(&first);
        let third_release = third(&second_release);
        stepwise.apply(&first, &[]).await.expect("r1");
        host_second(&stepwise, &second_release);
        stepwise.apply(&second_release, &[]).await.expect("r2");
        stepwise.host(&third_release, &[("alpha-3.jar", "alpha three")]);
        stepwise.apply(&third_release, &[]).await.expect("r3");

        // Another was on r1 and goes straight to r3.
        let (skipping, _) = imported();
        skipping.apply(&first, &[]).await.expect("r1");
        skipping.host(&third_release, &[("alpha-3.jar", "alpha three")]);
        skipping
            .apply(&third_release, &[])
            .await
            .expect("r3 from r1");

        // A third imported the r1 export and never ran CEMM before r3: no
        // ownership record at all, only the release's retired hashes.
        let (unadopted, _) = imported();
        unadopted.host(&third_release, &[("alpha-3.jar", "alpha three")]);
        let plan = unadopted.plan(&third_release);
        assert_eq!(plan.mode, PlanMode::Adopt);
        unadopted
            .apply(&third_release, &[])
            .await
            .expect("r3 on a bare import");

        let content = |fixture: &Fixture| {
            let mut files = fixture.snapshot();
            files.retain(|path, _| !path.starts_with(".cemm/") && path != INSTANCE_FILE);
            files
        };
        assert_eq!(content(&stepwise), content(&skipping));
        assert_eq!(content(&stepwise), content(&unadopted));

        for fixture in [&stepwise, &skipping, &unadopted] {
            let jars: Vec<String> = std::fs::read_dir(fixture.profile.join("mods"))
                .unwrap()
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|name| name.starts_with("alpha"))
                .collect();
            assert_eq!(jars, vec!["alpha-3.jar"], "exactly one active alpha");
            assert!(!fixture.exists("kubejs/server_scripts/recipes.js"));
            assert!(!fixture.exists("kubejs/server_scripts/old.js"));
            assert!(fixture.exists("mods/players-own-minimap.jar"));
        }
    }

    #[tokio::test]
    async fn former_pack_content_is_retired_on_an_exact_hash_and_never_by_name() {
        let (fixture, first) = imported();
        // Never adopted, and the player has put their own file where the
        // release would otherwise find one of its old scripts.
        fixture.write("kubejs/server_scripts/old.js", "the player rewrote this");
        let release = second(&first);
        host_second(&fixture, &release);

        fixture.apply(&release, &[]).await.expect("update");

        assert_eq!(
            fixture.read("kubejs/server_scripts/old.js"),
            "the player rewrote this",
            "same name, different content: not the pack's to delete"
        );
        assert!(!fixture.exists("mods/beta-1.jar"), "exact match: retired");
    }

    #[tokio::test]
    async fn the_wrong_pack_role_or_loader_is_blocked_before_anything_is_touched() {
        struct Case {
            name: &'static str,
            code: &'static str,
            setup: fn(&Fixture, &mut Release),
        }
        let cases = [
            Case {
                name: "a server release",
                code: "role",
                setup: |_, release| release.role = Role::Server,
            },
            Case {
                name: "another Minecraft version",
                code: "minecraft",
                setup: |_, release| release.minecraft = "1.20.1".to_string(),
            },
            Case {
                name: "another loader",
                code: "loader",
                setup: |_, release| release.loader = "neoforge-21.1.300".to_string(),
            },
            Case {
                name: "the official pack's own profile",
                code: "foreign-profile",
                setup: |fixture, _| {
                    let addons = fixture.instance()["installedAddons"]
                        .as_array()
                        .unwrap()
                        .clone();
                    fixture.set_instance_with(
                        addons,
                        serde_json::json!({ "installedModpack": { "addonID": 1356598 } }),
                    );
                },
            },
            Case {
                name: "an unrelated profile on the same Minecraft version",
                code: "not-this-pack",
                setup: |_, release| {
                    release.addons = (100..120)
                        .map(|project| {
                            addon(project, project, &format!("other-{project}.jar"), "x")
                        })
                        .collect();
                },
            },
        ];

        for case in cases {
            let (fixture, mut release) = imported();
            (case.setup)(&fixture, &mut release);
            let before = fixture.snapshot();

            let plan = fixture.plan(&release);
            assert!(
                plan.blockers
                    .iter()
                    .any(|blocker| blocker.code == case.code),
                "{}: expected a '{}' blocker, got {:?}",
                case.name,
                case.code,
                plan.blockers
            );
            let error = fixture
                .apply(&release, &[])
                .await
                .expect_err("a blocked plan must not apply");
            assert!(!error.is_empty());
            assert_eq!(fixture.snapshot(), before, "{}: profile changed", case.name);
            assert!(
                !fixture.exists(".cemm"),
                "{}: left a record behind",
                case.name
            );
        }
    }

    #[tokio::test]
    async fn a_profile_linked_to_the_official_pack_is_adopted_only_when_the_admin_says_so() {
        let link = |fixture: &Fixture| {
            let addons = fixture.instance()["installedAddons"]
                .as_array()
                .unwrap()
                .clone();
            fixture.set_instance_with(
                addons,
                serde_json::json!({ "installedModpack": { "addonID": 1356598 } }),
            );
        };

        let (fixture, mut release) = imported();
        link(&fixture);
        assert_eq!(fixture.plan(&release).blockers[0].code, "foreign-profile");

        release.adoption.allow_official_profile = true;
        let plan = fixture.plan(&release);
        assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
        assert!(
            plan.notes
                .iter()
                .any(|note| note.contains("Do not let CurseForge update")),
            "the player must be told what the link means: {:?}",
            plan.notes
        );
        fixture.apply(&release, &[]).await.expect("adoption");

        // The switch is about the official pack only. Another modpack's
        // profile is still the wrong profile.
        let (other, mut release) = imported();
        release.adoption.allow_official_profile = true;
        let addons = other.instance()["installedAddons"]
            .as_array()
            .unwrap()
            .clone();
        other.set_instance_with(
            addons,
            serde_json::json!({ "installedModpack": { "addonID": 424242 } }),
        );
        assert_eq!(other.plan(&release).blockers[0].code, "foreign-profile");
    }

    #[tokio::test]
    async fn a_file_renamed_only_in_letter_case_ends_up_under_one_name() {
        let (fixture, first) = imported();
        fixture.apply(&first, &[]).await.expect("adoption");

        // The release respells a script's name and changes it.
        let mut release = test_release("r2", 2);
        release.addons = first.addons.clone();
        release.files = first.files.clone();
        release.files[1] = {
            let mut respelt = managed("kubejs/server_scripts/Recipes.js", "recipes v2");
            respelt.previous = vec![sha256_hex(b"recipes v1")];
            respelt
        };
        fixture.host(&release, &[("recipes", "recipes v2")]);

        fixture.apply(&release, &[]).await.expect("update");

        // Exactly one file answers to that name, whatever the disk thinks of
        // case: the release's spelling, with the release's content.
        let spellings: Vec<String> =
            std::fs::read_dir(fixture.profile.join("kubejs/server_scripts"))
                .unwrap()
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|name| name.eq_ignore_ascii_case("recipes.js"))
                .collect();
        assert_eq!(spellings, vec!["Recipes.js"]);
        assert_eq!(
            fixture.read("kubejs/server_scripts/Recipes.js"),
            "recipes v2"
        );
        assert_eq!(fixture.plan(&release).change_count, 0);
    }

    #[tokio::test]
    async fn a_launcher_edit_made_during_the_update_is_kept_alongside_cemms_own() {
        let (fixture, first) = imported();
        fixture.apply(&first, &[]).await.expect("adoption");
        let release = second(&first);
        host_second(&fixture, &release);
        let reviewed = fixture.review(&release).await.fingerprint;

        // CurseForge saves the profile while the downloads run: play time,
        // and a field CEMM has never heard of. Nothing the update decided on.
        let instance_file = fixture.profile.join(INSTANCE_FILE);
        interference::before_record_patch(&fixture.profile, move || {
            let mut instance: Value =
                serde_json::from_str(&std::fs::read_to_string(&instance_file).unwrap()).unwrap();
            instance["playedCount"] = serde_json::json!(13);
            instance["writtenByCurseForgeJustNow"] = serde_json::json!({ "keep": true });
            std::fs::write(
                &instance_file,
                serde_json::to_string_pretty(&instance).unwrap(),
            )
            .unwrap();
        });

        fixture
            .apply_reviewed(&release, reviewed, &[])
            .await
            .expect("an unrelated launcher edit does not stop the update");

        let instance = fixture.instance();
        // Both writers' changes are in the file: neither overwrote the other.
        assert_eq!(instance["playedCount"], 13);
        assert_eq!(instance["writtenByCurseForgeJustNow"]["keep"], true);
        assert_eq!(
            fixture.instance_record(1).unwrap()["installedFile"]["id"],
            11
        );
        assert!(fixture.instance_record(2).is_none());
        assert_eq!(fixture.read("mods/alpha-2.jar"), "alpha two");
    }

    #[tokio::test]
    async fn a_launcher_edit_that_changes_the_plan_stops_the_update_before_anything_moves() {
        let (fixture, first) = imported();
        fixture.apply(&first, &[]).await.expect("adoption");
        let release = second(&first);
        host_second(&fixture, &release);
        let reviewed = fixture.review(&release).await.fingerprint;
        let before = fixture.snapshot();

        // This time CurseForge unlocks the pinned bridge mid-update — something
        // the reviewed plan had counted on being locked.
        let instance_file = fixture.profile.join(INSTANCE_FILE);
        interference::before_record_patch(&fixture.profile, move || {
            let mut instance: Value =
                serde_json::from_str(&std::fs::read_to_string(&instance_file).unwrap()).unwrap();
            for addon in instance["installedAddons"].as_array_mut().unwrap() {
                if addon["addonID"] == BRIDGE {
                    addon["isLocked"] = Value::Bool(false);
                }
            }
            std::fs::write(
                &instance_file,
                serde_json::to_string_pretty(&instance).unwrap(),
            )
            .unwrap();
        });

        let error = fixture
            .apply_reviewed(&release, reviewed, &[])
            .await
            .expect_err("a launcher edit that changes the plan must stop the update");
        assert!(
            error.contains("CurseForge changed this profile while the update was being prepared"),
            "got: {error}"
        );

        // CurseForge's own edit stands; nothing of CEMM's was applied.
        let mut after = fixture.snapshot();
        let mut expected = before;
        after.remove(INSTANCE_FILE);
        expected.remove(INSTANCE_FILE);
        assert_eq!(after, expected);
        assert_eq!(fixture.instance_record(BRIDGE).unwrap()["isLocked"], false);
        assert_eq!(
            fixture.instance_record(1).unwrap()["installedFile"]["id"],
            10
        );
        assert_eq!(fixture.state().release_id, "r1");
        assert!(!fixture.exists(INSTALL_TRANSACTION_DIR));
    }

    #[tokio::test]
    async fn another_lineage_a_downgrade_and_a_reused_code_are_refused() {
        let (fixture, first) = imported();
        let release = second(&first);
        host_second(&fixture, &release);
        fixture.apply(&first, &[]).await.expect("adoption");
        fixture.apply(&release, &[]).await.expect("update");
        let before = fixture.snapshot();
        let code_of =
            |plan: &InstallPlan| plan.blockers.first().map(|blocker| blocker.code.clone());

        let mut other_pack = release.clone();
        other_pack.lineage = "some-other-pack".to_string();
        assert_eq!(
            code_of(&fixture.plan(&other_pack)).as_deref(),
            Some("lineage")
        );

        assert_eq!(code_of(&fixture.plan(&first)).as_deref(), Some("downgrade"));

        // Same id, different content: somebody republished under an old code.
        let mut tampered = release.clone();
        tampered.notes = "quietly different".to_string();
        assert_eq!(
            code_of(&fixture.plan(&tampered)).as_deref(),
            Some("release-reused")
        );

        let mut needs_r5 = test_release("r9", 9);
        needs_r5.addons = release.addons.clone();
        needs_r5.supported_from = vec!["r5".to_string()];
        assert_eq!(
            code_of(&fixture.plan(&needs_r5)).as_deref(),
            Some("unsupported-start")
        );

        for refused in [&other_pack, &first, &tampered, &needs_r5] {
            assert!(fixture.apply(refused, &[]).await.is_err());
        }
        assert_eq!(fixture.snapshot(), before);
    }

    #[tokio::test]
    async fn a_managed_file_the_player_changed_stops_the_update_until_it_is_approved() {
        let (fixture, first) = imported();
        fixture.apply(&first, &[]).await.expect("adoption");
        fixture.write(
            "kubejs/server_scripts/recipes.js",
            "the player's own recipes",
        );
        let release = second(&first);
        host_second(&fixture, &release);
        let before = fixture.snapshot();

        let plan = fixture.plan(&release);
        let conflict = plan
            .conflicts
            .iter()
            .find(|conflict| conflict.id == "kubejs/server_scripts/recipes.js")
            .expect("the edit is a conflict");
        assert_eq!(conflict.kind, ConflictKind::ModifiedManagedFile);
        assert!(conflict.required);

        let error = fixture
            .apply(&release, &[])
            .await
            .expect_err("an undecided required conflict stops the update");
        assert!(error.contains("recipes.js"), "got: {error}");
        assert_eq!(
            fixture.snapshot(),
            before,
            "a refused update changes nothing"
        );

        let report = fixture
            .apply(&release, &["kubejs/server_scripts/recipes.js"])
            .await
            .expect("approved repair");
        assert_eq!(
            fixture.read("kubejs/server_scripts/recipes.js"),
            "recipes v2"
        );
        let backup = fixture.profile.join(report.backup_dir.unwrap());
        assert_eq!(
            std::fs::read_to_string(backup.join("files/kubejs/server_scripts/recipes.js")).unwrap(),
            "the player's own recipes"
        );
    }

    #[tokio::test]
    async fn an_approved_replacement_keeps_the_players_version_even_when_it_is_the_only_change() {
        // The plan counts no unconditional change here: the one thing that
        // happens hangs on the player's approval. Their version must still be
        // kept, because the screen says it is.
        let (fixture, first) = imported();
        fixture.apply(&first, &[]).await.expect("adoption");
        fixture.write(
            "kubejs/server_scripts/recipes.js",
            "the player's own recipes",
        );
        fixture.host(&first, &[("recipes", "recipes v1")]);

        let plan = fixture.plan(&first);
        assert_eq!(plan.mode, PlanMode::Reapply);
        assert_eq!(plan.change_count, 0);
        assert_eq!(plan.conflicts.len(), 1);

        let report = fixture
            .apply(&first, &["kubejs/server_scripts/recipes.js"])
            .await
            .expect("approved repair");

        assert_eq!(report.change_count, 1);
        assert_eq!(
            fixture.read("kubejs/server_scripts/recipes.js"),
            "recipes v1"
        );
        let backup = fixture
            .profile
            .join(report.backup_dir.expect("the player's version is kept"));
        assert_eq!(
            std::fs::read_to_string(backup.join("files/kubejs/server_scripts/recipes.js")).unwrap(),
            "the player's own recipes"
        );
    }

    #[tokio::test]
    async fn an_owned_file_changed_before_its_retirement_needs_approval_too() {
        let (fixture, first) = imported();
        fixture.apply(&first, &[]).await.expect("adoption");
        fixture.write(
            "kubejs/server_scripts/old.js",
            "edited after CEMM adopted it",
        );
        let release = second(&first);
        host_second(&fixture, &release);

        let plan = fixture.plan(&release);
        let conflict = plan
            .conflicts
            .iter()
            .find(|conflict| conflict.id == "kubejs/server_scripts/old.js")
            .expect("a changed owned file is a conflict, not a deletion");
        assert_eq!(conflict.kind, ConflictKind::ModifiedRetiredFile);
        assert!(fixture.apply(&release, &[]).await.is_err());
        assert!(fixture.exists("kubejs/server_scripts/old.js"));

        fixture
            .apply(&release, &["kubejs/server_scripts/old.js"])
            .await
            .expect("approved retirement");
        assert!(!fixture.exists("kubejs/server_scripts/old.js"));
    }

    #[tokio::test]
    async fn a_personal_default_is_migrated_while_untouched_and_kept_once_changed() {
        let fixture = Fixture::new();
        let release_with = |id: &str, sequence: u64, content: &str| {
            let mut release = test_release(id, sequence);
            release.files = vec![default_file("config/ui.json", content)];
            release
        };
        let r1 = release_with("r1", 1, "{\"scale\":1}");
        let r2 = release_with("r2", 2, "{\"scale\":2}");
        let r3 = release_with("r3", 3, "{\"scale\":3}");
        fixture.host(
            &r1,
            &[
                ("d1", "{\"scale\":1}"),
                ("d2", "{\"scale\":2}"),
                ("d3", "{\"scale\":3}"),
            ],
        );

        // Absent: the pack's default is applied.
        fixture.apply(&r1, &[]).await.expect("r1");
        assert_eq!(fixture.read("config/ui.json"), "{\"scale\":1}");

        // Untouched since: it follows the pack to the new default.
        let plan = fixture.plan(&r2);
        assert!(plan.conflicts.is_empty());
        assert_eq!(plan.files[0].op, FileOp::Replace);
        fixture.apply(&r2, &[]).await.expect("r2");
        assert_eq!(fixture.read("config/ui.json"), "{\"scale\":2}");

        // Changed by the player, and by the pack: visible, and theirs by default.
        fixture.write("config/ui.json", "{\"scale\":9}");
        let plan = fixture.plan(&r3);
        assert_eq!(plan.conflicts.len(), 1);
        assert_eq!(
            plan.conflicts[0].kind,
            ConflictKind::DefaultChangedBothSides
        );
        assert!(!plan.conflicts[0].required);
        let report = fixture
            .apply(&r3, &[])
            .await
            .expect("r3 keeping the player's file");
        assert_eq!(fixture.read("config/ui.json"), "{\"scale\":9}");
        assert_eq!(report.kept_local, vec!["config/ui.json"]);
        // The record says what is really there, apart from what was offered.
        let default = &fixture.state().defaults[0];
        assert!(default.kept_local);
        assert_eq!(default.supplied_sha256, sha256_hex(b"{\"scale\":3}"));
        assert_eq!(
            default.local_sha256.as_deref(),
            Some(sha256_hex(b"{\"scale\":9}").as_str())
        );

        // Asking for the pack's version is honoured on a later pass.
        let r4 = release_with("r4", 4, "{\"scale\":4}");
        fixture.host(&r4, &[("d4", "{\"scale\":4}")]);
        fixture
            .apply(&r4, &["config/ui.json"])
            .await
            .expect("r4 taking the pack's");
        assert_eq!(fixture.read("config/ui.json"), "{\"scale\":4}");
        assert!(!fixture.state().defaults[0].kept_local);
    }

    #[tokio::test]
    async fn a_shared_settings_file_is_patched_and_never_offered_for_replacement() {
        // The TaCZ Tweaks case in full: the pack's default for the file changes
        // (the admin flipped the flag in their own copy), the player's file was
        // already different (their own keybinds), and one key is required.
        let fixture = Fixture::new();
        let admin_before = "{\n  \"general\": {\n    \"compatibility\": {\n      \"sableCompat\": true\n    }\n  },\n  \"keyActions\": {}\n}\n";
        let admin_after = admin_before.replace("true", "false");
        let mut r1 = test_release("r1", 1);
        r1.files = vec![default_file("config/tacztweaks.json", admin_before)];
        let mut r2 = test_release("r2", 2);
        r2.files = vec![default_file("config/tacztweaks.json", &admin_after)];
        r2.settings = vec![sable_setting()];
        fixture.host(&r2, &[("after", &admin_after)]);
        fixture.write("config/tacztweaks.json", TWEAKS_PLAYER);
        fixture.apply(&r1, &[]).await.expect("r1");

        let plan = fixture.plan(&r2);
        assert!(plan.conflicts.is_empty(), "{:?}", plan.conflicts);
        assert_eq!(plan.files[0].op, FileOp::KeepLocal);
        assert_eq!(plan.settings[0].op, SettingOp::Apply);
        assert_eq!(
            plan.download_count, 0,
            "the pack's copy is not even fetched"
        );
        fixture.apply(&r2, &[]).await.expect("r2");

        assert_eq!(
            fixture.read("config/tacztweaks.json"),
            TWEAKS_PLAYER.replace("\"sableCompat\": true", "\"sableCompat\": false"),
            "one value changed; every other byte is the player's"
        );
        // Recorded as what it is: the player's file, not the pack's default.
        let default = &fixture.state().defaults[0];
        assert!(default.kept_local);
        assert_eq!(default.supplied_sha256, sha256_hex(admin_after.as_bytes()));

        // A player who has no such file yet gets the pack's, flag included.
        let fresh = Fixture::new();
        fresh.host(&r2, &[("after", &admin_after)]);
        fresh
            .apply(&r2, &[])
            .await
            .expect("r2 on a profile without the file");
        assert_eq!(fresh.read("config/tacztweaks.json"), admin_after);
    }

    #[test]
    fn a_required_setting_changes_one_key_and_keeps_the_rest_as_written() {
        let outcome = apply_json_setting(
            TWEAKS_PLAYER,
            "/general/compatibility/sableCompat",
            &serde_json::json!(false),
        )
        .expect("should apply");
        let SettingOutcome::Changed { text, previous } = outcome else {
            panic!("the value differs, so the file must change");
        };
        assert_eq!(previous, Some(serde_json::json!(true)));
        // Same bytes apart from the one value: order, indentation, the float
        // written as a float, the trailing newline.
        assert_eq!(
            text,
            TWEAKS_PLAYER.replace("\"sableCompat\": true", "\"sableCompat\": false")
        );

        assert_eq!(
            apply_json_setting(
                &text,
                "/general/compatibility/sableCompat",
                &serde_json::json!(false)
            ),
            Ok(SettingOutcome::AlreadySet)
        );

        // Windows line endings and tab indentation are kept as found.
        let tabbed = "{\r\n\t\"a\": {\r\n\t\t\"b\": 1\r\n\t}\r\n}";
        let SettingOutcome::Changed { text, .. } =
            apply_json_setting(tabbed, "/a/b", &serde_json::json!(2)).unwrap()
        else {
            panic!("should change");
        };
        assert_eq!(text, "{\r\n\t\"a\": {\r\n\t\t\"b\": 2\r\n\t}\r\n}");
    }

    #[test]
    fn a_setting_cemm_cannot_place_safely_is_refused_with_the_reason() {
        let value = serde_json::json!(false);
        // No such section: CEMM does not invent the mod's config layout.
        let error = apply_json_setting(
            "{\"general\":{}}",
            "/general/compatibility/sableCompat",
            &value,
        )
        .expect_err("missing section");
        assert!(error.contains("/general/compatibility"), "got: {error}");
        // Not JSON at all (JSON5 comments, TOML, a truncated file).
        assert!(apply_json_setting("// comment\n{}", "/a", &value)
            .expect_err("not json")
            .contains("not plain JSON"));
        assert!(apply_json_setting("a = 1", "/a", &value).is_err());
        // The parent exists but is a value, not a section.
        assert!(apply_json_setting("{\"general\":5}", "/general/x", &value).is_err());
    }

    #[tokio::test]
    async fn an_unsupported_settings_file_blocks_with_a_way_forward() {
        let (fixture, first) = imported();
        fixture.apply(&first, &[]).await.expect("adoption");
        fixture.write(
            "config/tacztweaks.json",
            "// hand-edited\n{ sableCompat: true }",
        );
        let release = second(&first);
        host_second(&fixture, &release);
        let before = fixture.snapshot();

        let plan = fixture.plan(&release);
        let blocker = plan
            .blockers
            .iter()
            .find(|blocker| blocker.code == "setting-unsupported")
            .expect("an unreadable settings file blocks");
        assert!(
            blocker.message.contains("by hand"),
            "got: {}",
            blocker.message
        );
        assert!(fixture.apply(&release, &[]).await.is_err());
        assert_eq!(fixture.snapshot(), before);
    }

    #[tokio::test]
    async fn a_payload_that_does_not_match_its_hash_leaves_the_profile_as_it_was() {
        let (fixture, first) = imported();
        fixture.apply(&first, &[]).await.expect("adoption");
        let release = second(&first);
        // The CDN serves something else under alpha-2's name.
        fixture.host(&release, &[("recipes", "recipes v2")]);
        std::fs::write(fixture.addons.join("alpha-2.jar"), "not alpha two").unwrap();
        let before = fixture.snapshot();

        let error = fixture.apply(&release, &[]).await.expect_err("wrong hash");
        assert!(
            error.contains("did not arrive as published"),
            "got: {error}"
        );
        assert_eq!(fixture.snapshot(), before);
        assert!(!fixture.exists(INSTALL_TRANSACTION_DIR));
        assert_eq!(
            fixture.state().release_id,
            "r1",
            "success is never recorded early"
        );

        // A download that fails outright is no different.
        std::fs::remove_file(fixture.addons.join("alpha-2.jar")).unwrap();
        assert!(fixture.apply(&release, &[]).await.is_err());
        assert_eq!(fixture.snapshot(), before);
    }

    #[tokio::test]
    async fn an_addon_that_needs_a_manual_download_blocks_until_the_exact_file_is_there() {
        let (fixture, first) = imported();
        fixture.apply(&first, &[]).await.expect("adoption");
        let mut release = second(&first);
        release.addons[0].download_url = None;
        fixture.host(&release, &[("recipes", "recipes v2")]);

        let plan = fixture.plan(&release);
        let blocker = &plan.blockers[0];
        assert_eq!(blocker.code, "manual-download");
        assert!(blocker.message.contains("alpha-2.jar"));
        assert!(
            blocker.message.contains("/files/11"),
            "got: {}",
            blocker.message
        );
        assert!(fixture.apply(&release, &[]).await.is_err());

        // The wrong file under the right name is still not the release's.
        fixture.write("mods/alpha-2.jar", "some other build");
        assert_eq!(fixture.plan(&release).blockers[0].code, "manual-download");

        fixture.write("mods/alpha-2.jar", "alpha two");
        assert!(fixture.plan(&release).blockers.is_empty());
        fixture
            .apply(&release, &[])
            .await
            .expect("applies once the file is in place");
        assert!(!fixture.exists("mods/alpha-1.jar"));
    }

    #[tokio::test]
    async fn a_file_the_release_does_not_host_cannot_be_repaired_and_says_so() {
        let (fixture, mut first) = imported();
        first.files[1].hosted = false;
        fixture
            .apply(&first, &[])
            .await
            .expect("adoption needs no payloads");
        std::fs::remove_file(fixture.profile.join("kubejs/server_scripts/recipes.js")).unwrap();

        let plan = fixture.plan(&first);
        assert_eq!(plan.blockers[0].code, "not-hosted");
        assert!(plan.blockers[0].message.contains("pack's export"));
    }

    #[tokio::test]
    async fn a_profile_that_changes_after_review_is_not_updated_on_a_stale_approval() {
        let (fixture, first) = imported();
        fixture.apply(&first, &[]).await.expect("adoption");
        let release = second(&first);
        host_second(&fixture, &release);
        let reviewed = fixture.plan(&release).fingerprint;

        // CurseForge updates beta behind CEMM's back while the dialog is open.
        fixture.write("mods/beta-1.jar", "beta, replaced by the launcher");
        let before = fixture.snapshot();

        let error = fixture
            .apply_reviewed(&release, reviewed, &[])
            .await
            .expect_err("a stale review must not be applied");
        assert!(error.contains("changed after you reviewed"), "got: {error}");
        assert_eq!(fixture.snapshot(), before);
    }

    #[tokio::test]
    async fn a_second_update_on_the_same_profile_waits_for_the_first() {
        let (fixture, first) = imported();
        let fingerprint = fixture.plan(&first).fingerprint;
        let held = ProfileLock::acquire(&fixture.profile, "applying release r0").expect("lock");

        let error = fixture
            .apply_reviewed(&first, fingerprint.clone(), &[])
            .await
            .expect_err("the profile is locked");
        assert!(error.contains("already running"), "got: {error}");
        assert!(read_applied_state(&fixture.profile).unwrap().is_none());

        drop(held);
        fixture
            .apply_reviewed(&first, fingerprint, &[])
            .await
            .expect("free again once the first has finished");
    }

    #[tokio::test]
    async fn a_lock_left_by_a_crashed_process_does_not_strand_the_profile() {
        let (fixture, first) = imported();
        // What a killed CEMM leaves: the file, and nobody holding it.
        fixture.write(
            ".cemm/lock",
            &serde_json::json!({ "pid": std::process::id(), "started_unix": 1, "operation": "x" })
                .to_string(),
        );

        fixture
            .apply(&first, &[])
            .await
            .expect("a stale lock is cleared");
        assert!(!fixture.exists(".cemm/lock"));
    }

    #[tokio::test]
    async fn an_interrupted_install_is_rolled_back_before_the_next_one_plans() {
        let (fixture, first) = imported();
        fixture.apply(&first, &[]).await.expect("adoption");
        let release = second(&first);
        host_second(&fixture, &release);
        let settled = fixture.snapshot();
        let backups_before = fixture.backups().len();

        // The state a process killed mid-promotion leaves: alpha-1 moved to the
        // backup, alpha-2 already in place, and a journal saying so.
        let transaction = fixture.profile.join(INSTALL_TRANSACTION_DIR);
        std::fs::create_dir_all(transaction.join("backup")).unwrap();
        std::fs::rename(
            fixture.profile.join("mods/alpha-1.jar"),
            transaction.join("backup/0"),
        )
        .unwrap();
        fixture.write("mods/alpha-2.jar", "alpha two");
        std::fs::write(
            transaction.join("journal.json"),
            serde_json::json!({
                "entries": [
                    { "relative_path": "mods/alpha-1.jar", "had_original": true },
                    { "relative_path": "mods/alpha-2.jar", "had_original": false }
                ],
                "retain_backup": ".cemm/backups/000000000001-r2"
            })
            .to_string(),
        )
        .unwrap();

        // Recovery restores the profile to exactly r1 ...
        installer::recover_interrupted_install(&fixture.profile)
            .await
            .expect("recovery");
        assert_eq!(fixture.snapshot(), settled);
        assert_eq!(
            fixture.backups().len(),
            backups_before,
            "a rolled-back install keeps no backup of its own"
        );

        // ... and the same happens on its own at the start of the next apply.
        std::fs::create_dir_all(transaction.join("backup")).unwrap();
        std::fs::rename(
            fixture.profile.join("mods/alpha-1.jar"),
            transaction.join("backup/0"),
        )
        .unwrap();
        std::fs::write(
            transaction.join("journal.json"),
            serde_json::json!({
                "entries": [{ "relative_path": "mods/alpha-1.jar", "had_original": true }]
            })
            .to_string(),
        )
        .unwrap();
        // The review is made against the restored profile, and says it was.
        let plan = fixture.review(&release).await;
        assert!(plan.recovered_interrupted);
        assert!(
            fixture.exists("mods/alpha-1.jar"),
            "restored before planning"
        );
        assert!(!fixture.exists(INSTALL_TRANSACTION_DIR));
        assert_eq!(plan.fingerprint, fixture.plan(&release).fingerprint);
        fixture
            .apply_reviewed(&release, plan.fingerprint, &[])
            .await
            .expect("update after recovery");
        assert_eq!(fixture.read("mods/alpha-2.jar"), "alpha two");
        assert!(!fixture.exists("mods/alpha-1.jar"));
    }

    #[tokio::test]
    async fn switched_off_files_are_resolved_to_one_intended_version() {
        let (fixture, first) = imported();
        fixture.apply(&first, &[]).await.expect("adoption");
        // The player switched alpha off in CurseForge.
        std::fs::rename(
            fixture.profile.join("mods/alpha-1.jar"),
            fixture.profile.join("mods/alpha-1.jar.disabled"),
        )
        .unwrap();
        let mut record = fixture.record(1, 10, "alpha-1.jar.disabled");
        record["isEnabled"] = Value::Bool(false);
        record["installedFile"]["fileNameOnDisk"] = "alpha-1.jar".into();
        fixture.set_instance(vec![
            record,
            fixture.record(2, 20, "beta-1.jar"),
            fixture.record(BRIDGE, 8420412, "bridge-1.1.1.jar"),
        ]);

        // The release updates alpha and switches beta off.
        let mut release = second(&first);
        let mut beta = first.addons[1].clone();
        beta.enabled = false;
        release.addons.push(beta);
        release
            .retired
            .retain(|retired| retired.project_id != Some(2));
        host_second(&fixture, &release);

        let plan = fixture.plan(&release);
        let op_of = |project: u64| {
            plan.addons
                .iter()
                .find(|action| action.project_id == project)
                .map(|action| action.op)
        };
        assert_eq!(op_of(1), Some(AddonOp::Replace));
        assert_eq!(op_of(2), Some(AddonOp::Disable));
        fixture.apply(&release, &[]).await.expect("update");

        // The stale switched-off jar is gone, not left to be re-enabled later.
        assert!(!fixture.exists("mods/alpha-1.jar.disabled"));
        assert!(!fixture.exists("mods/alpha-1.jar"));
        assert_eq!(fixture.read("mods/alpha-2.jar"), "alpha two");
        // Beta is off, under the launcher's own convention, with its content.
        assert!(!fixture.exists("mods/beta-1.jar"));
        assert_eq!(fixture.read("mods/beta-1.jar.disabled"), "beta one");

        let alpha = fixture.instance_record(1).unwrap();
        assert_eq!(alpha["isEnabled"], true);
        assert_eq!(alpha["fileNameOnDisk"], "alpha-2.jar");
        let beta = fixture.instance_record(2).unwrap();
        assert_eq!(beta["isEnabled"], false);
        assert_eq!(beta["fileNameOnDisk"], "beta-1.jar.disabled");

        // And it stays that way: nothing to do on a second pass.
        assert_eq!(fixture.plan(&release).change_count, 0);
    }

    #[tokio::test]
    async fn a_pin_without_a_launcher_record_is_a_stated_manual_step() {
        let (fixture, mut first) = imported();
        // CurseForge has not picked the bridge up yet.
        fixture.set_instance(vec![
            fixture.record(1, 10, "alpha-1.jar"),
            fixture.record(2, 20, "beta-1.jar"),
        ]);
        first.adoption.min_project_overlap_percent = 50;

        let plan = fixture.plan(&first);
        assert_eq!(plan.launcher.pins[0].state, PinState::Manual);
        let report = fixture.apply(&first, &[]).await.expect("adoption");
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("lock it there by hand")),
            "got: {:?}",
            report.warnings
        );
    }

    #[tokio::test]
    async fn server_only_content_is_not_installed_into_a_client_profile() {
        let (fixture, mut first) = imported();
        let mut server_only = managed("defaultconfigs/server-only.toml", "server");
        server_only.role = FileRole::Server;
        first.files.push(server_only);

        let plan = fixture.plan(&first);
        assert!(plan
            .files
            .iter()
            .any(|file| file.path == "defaultconfigs/server-only.toml"
                && file.op == FileOp::SkipRole));
        fixture.apply(&first, &[]).await.expect("adoption");
        assert!(!fixture.exists("defaultconfigs/server-only.toml"));
    }

    #[tokio::test]
    async fn a_release_cannot_reach_worlds_options_or_launcher_records() {
        let (fixture, first) = imported();
        for path in [
            "saves/World/level.dat",
            "options.txt",
            "minecraftinstance.json",
            ".cemm/state.json",
        ] {
            let mut hostile = first.clone();
            hostile.files.push(managed(path, "overwritten"));
            let before = fixture.snapshot();

            assert!(
                plan_release(
                    fixture.profile.to_string_lossy().into_owned(),
                    hostile.clone(),
                    digest_of(&hostile)
                )
                .await
                .is_err(),
                "{path} must be refused at planning"
            );
            assert!(fixture
                .apply_reviewed(&hostile, String::new(), &[])
                .await
                .is_err());
            assert_eq!(fixture.snapshot(), before, "{path} must be untouched");
        }
    }

    #[tokio::test]
    async fn a_legacy_config_code_keeps_the_release_record_and_a_legacy_full_code_is_refused() {
        use crate::composables::manifest::{ConfigFile, ConfigFileWithContent};

        let (fixture, first) = imported();
        fixture.apply(&first, &[]).await.expect("adoption");
        let owned_before = fixture.state().owned;
        let legacy = |update_type: &str| Manifest {
            update_type: Some(update_type.to_string()),
            mods: Vec::new(),
            resourcepacks: Vec::new(),
            shaderpacks: Vec::new(),
            datapacks: Vec::new(),
            custom_datapacks: Vec::new(),
            config_files: vec![ConfigFile {
                filename: "extra.toml".to_string(),
                relative_path: "config/extra.toml".to_string(),
            }],
        };
        let payload = || {
            vec![ConfigFileWithContent {
                filename: "extra.toml".to_string(),
                relative_path: "config/extra.toml".to_string(),
                content: "legacy = true".to_string(),
                is_binary: None,
            }]
        };
        let baseline = crate::composables::manifest::resolve_install_baseline(
            fixture.profile.to_string_lossy().into_owned(),
        )
        .unwrap()
        .map(|baseline| baseline.manifest);

        // A legacy full update is an empty addon list here: applied, it would
        // delete every managed mod.
        let before = fixture.snapshot();
        let error = installer::install_update_with_progress(
            fixture.profile.to_string_lossy().into_owned(),
            legacy("full"),
            payload(),
            Some(crate::installer::InstallOptions {
                old_manifest: baseline.clone(),
                cleanup_old: true,
            }),
            Arc::new(|_| {}),
        )
        .await
        .expect_err("a legacy full update must be refused on a release profile");
        assert!(error.contains("release line"), "got: {error}");
        assert_eq!(fixture.snapshot(), before);

        // A config-only one still applies, removes no addon, and is recorded
        // on top of the release rather than in place of it.
        installer::install_update_with_progress(
            fixture.profile.to_string_lossy().into_owned(),
            legacy("config"),
            payload(),
            Some(crate::installer::InstallOptions {
                old_manifest: baseline,
                cleanup_old: true,
            }),
            Arc::new(|_| {}),
        )
        .await
        .expect("legacy config-only still works");

        assert_eq!(fixture.read("config/extra.toml"), "legacy = true");
        assert!(fixture.exists("mods/alpha-1.jar"));
        assert!(fixture.exists("mods/atmons-ore-hammer-fix-1.0.0.jar"));
        let state = fixture.state();
        assert_eq!(state.release_id, "r1");
        assert_eq!(state.schema, STATE_SCHEMA);
        assert_eq!(state.owned, owned_before, "ownership history is intact");
        assert_eq!(state.legacy_overlays.len(), 1);
        assert_eq!(state.legacy_overlays[0].paths, vec!["config/extra.toml"]);
        // The legacy snapshot still lists the release's addons as installed.
        let snapshot: Manifest =
            serde_json::from_str(&fixture.read(INSTALLED_MANIFEST_FILE)).unwrap();
        let listed: HashSet<u64> = snapshot
            .mods
            .iter()
            .map(|addon| addon.addon_project_id)
            .collect();
        for project in [1, 2, BRIDGE] {
            assert!(
                listed.contains(&project),
                "project {project} dropped from the snapshot"
            );
        }
    }

    #[test]
    fn an_unreadable_state_file_is_an_error_not_a_fresh_profile() {
        let (fixture, _) = imported();
        fixture.write(".cemm/state.json", "{ truncated");
        let error = read_applied_state(&fixture.profile).expect_err("must not be ignored");
        assert!(error.contains("will not guess"), "got: {error}");

        fixture.write(
            ".cemm/state.json",
            &serde_json::json!({ "schema": 99 }).to_string(),
        );
        assert!(read_applied_state(&fixture.profile).is_err());
    }

    #[test]
    fn patching_the_launcher_record_leaves_every_other_byte_alone() {
        let (fixture, first) = imported();
        let original = fixture.read(INSTANCE_FILE);
        let value: Value = serde_json::from_str(&original).unwrap();

        // No changes: the document comes back byte for byte.
        assert_eq!(render_like(&original, &value).unwrap(), original);

        // One pin: exactly one line differs.
        let mut patched = value.clone();
        let (updated, removed) =
            patch_instance(&mut patched, &[RecordChange::Pin { project_id: BRIDGE }]);
        assert_eq!((updated, removed), (1, 0));
        let rendered = render_like(&original, &patched).unwrap();
        let differing: Vec<(&str, &str)> = original
            .lines()
            .zip(rendered.lines())
            .filter(|(before, after)| before != after)
            .collect();
        assert_eq!(differing.len(), 1, "{differing:?}");
        assert!(differing[0].1.contains("\"isLocked\": true"));
        let _ = first;
    }

    /// Read-only check against a real CurseForge profile named by
    /// `CEMM_REFERENCE_PROFILE`. Ignored by default: CI has no such profile.
    #[test]
    #[ignore = "needs CEMM_REFERENCE_PROFILE pointing at a real CurseForge profile"]
    fn a_real_instance_document_survives_parse_and_render_byte_for_byte() {
        let profile = PathBuf::from(
            std::env::var("CEMM_REFERENCE_PROFILE").expect("CEMM_REFERENCE_PROFILE must be set"),
        );
        let original = std::fs::read_to_string(profile.join(INSTANCE_FILE)).expect("instance");
        let value: Value =
            serde_json::from_str(original.trim_start_matches('\u{feff}')).expect("valid JSON");
        let rendered = render_like(&original, &value).expect("render");

        if rendered != original {
            let at = original
                .bytes()
                .zip(rendered.bytes())
                .position(|(a, b)| a != b)
                .unwrap_or(original.len().min(rendered.len()));
            let window = |text: &str| {
                let start = at.saturating_sub(60);
                String::from_utf8_lossy(&text.as_bytes()[start..(at + 60).min(text.len())])
                    .into_owned()
            };
            panic!(
                "render differs at byte {at} ({} vs {} bytes)\n original: {:?}\n rendered: {:?}",
                original.len(),
                rendered.len(),
                window(&original),
                window(&rendered)
            );
        }
        println!(
            "{} bytes, {} records: identical",
            original.len(),
            instance_records(&value).len()
        );
    }
}
