//! The versioned release contract (schema 2).
//!
//! A legacy `Manifest` is the payload of one update: four addon arrays and a
//! list of config files, with nothing saying which pack it belongs to, which
//! client can read it, or what the files should hash to. That was enough while
//! an update meant "the admin's mods right now".
//!
//! A `Release` is a different statement. It describes the complete reviewed
//! target state of one pack lineage — every addon by exact CurseForge file,
//! every managed file by content hash, the settings the pack requires, and the
//! former contents it is entitled to retire — so a client can reach it from any
//! earlier release of the same lineage, or from a freshly imported profile,
//! and can prove afterwards that it did.
//!
//! This module holds the contract itself and everything that is true of it
//! regardless of who is asking: types, the compatibility gate, path and
//! protected-data rules, hashing, and the three-way comparison an admin uses
//! when the official base moves. Capturing one from a profile lives in
//! `release_capture`, applying one in `release_install`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::Read;
use std::path::Path;
use ts_rs::TS;

/// The schema this build writes and reads. A legacy manifest is, in effect,
/// schema 1; it never carried the number.
pub const RELEASE_SCHEMA: u32 = 2;

/// The release document's name inside its folder. Deliberately not
/// `cemm-manifest.json`: that name is what every released client fetches and
/// parses as a legacy manifest, and it is occupied by the tombstone below.
pub const RELEASE_FILE_NAME: &str = "cemm-release.json";

/// The first CEMM version that reads schema 2. Written into every release for
/// the benefit of people; the machine gate is `schema` plus `requires`.
pub const RELEASE_MIN_CEMM: &str = "2.2.0";

pub const CAP_ADDONS: &str = "addons-v1";
pub const CAP_FILES: &str = "managed-files-v1";
pub const CAP_SETTINGS: &str = "json-settings-v1";

/// Everything this build knows how to apply. A release naming anything else is
/// refused before a single file is planned, which is what lets the format grow
/// without a newer document ever being half-understood.
pub const SUPPORTED_CAPABILITIES: [&str; 3] = [CAP_ADDONS, CAP_FILES, CAP_SETTINGS];

/// The admin-authored description of a pack lineage, kept at the instance root.
pub const SPEC_FILE_NAME: &str = "cemm-pack.json";

/// Where CEMM keeps its own record inside a profile.
pub const STATE_DIR: &str = ".cemm";

const MAX_RELEASE_ADDONS: usize = 5_000;
const MAX_RELEASE_FILES: usize = 50_000;
const MAX_RELEASE_SETTINGS: usize = 200;
const MAX_RELEASE_RETIRED: usize = 200_000;
const MAX_RELEASE_PATH_BYTES: usize = 240;
pub const MAX_MANAGED_FILE_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_RELEASE_ADDON_BYTES: u64 = 500 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "generated/")]
pub enum AddonCategory {
    Mods,
    Resourcepacks,
    Shaderpacks,
    Datapacks,
}

impl AddonCategory {
    pub const ALL: [AddonCategory; 4] = [
        Self::Mods,
        Self::Resourcepacks,
        Self::Shaderpacks,
        Self::Datapacks,
    ];

    /// The directory under the profile root this category installs into.
    pub const fn directory(self) -> &'static str {
        match self {
            Self::Mods => "mods",
            Self::Resourcepacks => "resourcepacks",
            Self::Shaderpacks => "shaderpacks",
            Self::Datapacks => "datapacks",
        }
    }

    pub fn from_directory(directory: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|category| category.directory().eq_ignore_ascii_case(directory))
    }
}

/// Which kind of installation a release describes. A client release is applied
/// by CEMM into a CurseForge profile; a server release is recorded for pairing
/// and deployed by the admin's own tooling.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "generated/")]
pub enum Role {
    Client,
    Server,
}

/// Where a managed file belongs. A client install skips `Server` entries rather
/// than copying server-only content into a player's profile.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "generated/")]
pub enum FileRole {
    Client,
    Server,
    #[default]
    Both,
}

impl FileRole {
    pub fn applies_to(self, role: Role) -> bool {
        matches!(
            (self, role),
            (Self::Both, _) | (Self::Client, Role::Client) | (Self::Server, Role::Server)
        )
    }
}

/// Who a file belongs to once it is on a player's disk.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "generated/")]
pub enum FilePolicy {
    /// Release content. It must match before an install can call itself
    /// complete, and it is retired when a later release drops it.
    #[default]
    Managed,
    /// A pack-supplied default for something a player is free to change.
    /// Applied when absent, migrated while untouched, and left alone — with the
    /// difference recorded — once the player has made it their own.
    Default,
}

/// What kind of thing a managed file is, for the preview to say so plainly.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "generated/")]
pub enum FileSource {
    /// Scripts, configs, quests, data and assets written for the pack.
    #[default]
    Authored,
    /// A binary supplied by the admin with no CurseForge project behind it —
    /// code the game will load, vouched for by nobody but the release.
    LocalBinary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct OfficialBase {
    #[ts(type = "number")]
    pub project_id: u64,
    #[ts(type = "number")]
    pub file_id: u64,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct ServerRelease {
    /// What the admin calls the matching server build.
    pub name: String,
    /// Hash of the server archive, when there is one to quote.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

/// How much of a release has to be recognisably present before a profile CEMM
/// has no record of may be adopted into the lineage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct AdoptionRule {
    /// Share of the release's enabled addon projects that must already be in
    /// the profile, in percent. A matching Minecraft version is not evidence
    /// that a profile is this pack; nine tenths of its mods are.
    pub min_project_overlap_percent: u8,
    /// Whether a profile CurseForge still links to the official pack may be
    /// adopted. Off by default: CurseForge updates and repairs such a profile
    /// on its own and would overwrite the custom pack when it does. An admin
    /// whose players' imports do carry that link switches this on, and the
    /// player is then told not to let CurseForge update the profile.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub allow_official_profile: bool,
}

impl Default for AdoptionRule {
    fn default() -> Self {
        Self {
            min_project_overlap_percent: 90,
            allow_official_profile: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct ReleaseAddon {
    #[ts(type = "number")]
    pub project_id: u64,
    #[ts(type = "number")]
    pub file_id: u64,
    pub name: String,
    pub category: AddonCategory,
    /// The canonical filename, never the `.disabled` form.
    pub file_name: String,
    pub enabled: bool,
    #[ts(type = "number")]
    pub size: u64,
    pub sha256: String,
    /// CurseForge's own SHA-1 for the file, recorded as published.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub sha1: Option<String>,
    /// CurseForge's package fingerprint, which its launcher matches files on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fingerprint: Option<u32>,
    /// `None` when CurseForge does not offer the file for third-party
    /// download. Such an addon is never reported installed unless the exact
    /// file is already on disk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub download_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub website_url: Option<String>,
    /// An intentional pin: this exact file, and the launcher told not to move it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pinned: bool,
    /// Project ids CurseForge lists as required dependencies of this file.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(type = "number[]")]
    pub requires: Vec<u64>,
}

impl ReleaseAddon {
    /// `mods/sodium.jar` — the path the enabled file occupies.
    pub fn relative_path(&self) -> String {
        format!("{}/{}", self.category.directory(), self.file_name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct ReleaseFile {
    /// Relative to the profile root, forward slashes, already normalised.
    pub path: String,
    pub sha256: String,
    #[ts(type = "number")]
    pub size: u64,
    #[serde(default)]
    pub source: FileSource,
    #[serde(default)]
    pub policy: FilePolicy,
    #[serde(default)]
    pub role: FileRole,
    /// Whether the bytes are in the repository's object store. An inventory
    /// entry that is not hosted is still verified and owned; it simply cannot
    /// be delivered or repaired by CEMM, and the plan says so up front.
    pub hosted: bool,
    /// Hashes this path held in earlier releases of the lineage. Content
    /// matching one is provably the pack's own and may be replaced without
    /// asking; anything else was changed by the player.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub previous: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "generated/")]
pub enum SettingFormat {
    #[default]
    Json,
}

/// One value the pack requires inside a file the player otherwise owns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "generated/")]
pub struct RequiredSetting {
    pub path: String,
    #[serde(default)]
    pub format: SettingFormat,
    /// RFC 6901 JSON pointer to the key, e.g. `/general/compatibility/sableCompat`.
    pub pointer: String,
    #[ts(type = "unknown")]
    pub value: serde_json::Value,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
}

/// Former lineage content a client may remove when it finds exactly this.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, TS)]
#[ts(export, export_to = "generated/")]
pub struct RetiredItem {
    pub path: String,
    pub sha256: String,
    /// Set for addon files, so the launcher's record can be retired with them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub project_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "generated/")]
pub struct Release {
    pub schema: u32,
    pub min_cemm: String,
    pub requires: Vec<String>,
    /// The pack's stable identity. Survives renames and official upgrades.
    pub lineage: String,
    /// Unique and immutable within the lineage.
    pub release_id: String,
    /// Strictly increasing within the lineage; what "newer" means.
    #[ts(type = "number")]
    pub sequence: u64,
    pub name: String,
    pub role: Role,
    /// The exact official pack file this release was built from.
    pub base: OfficialBase,
    pub minecraft: String,
    pub loader: String,
    /// Release ids an upgrade is supported from. Empty means any earlier
    /// release of this lineage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_from: Vec<String>,
    #[serde(default)]
    pub adoption: AdoptionRule,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub server_release: Option<ServerRelease>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manual_steps: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known_issues: Vec<String>,
    pub addons: Vec<ReleaseAddon>,
    pub files: Vec<ReleaseFile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settings: Vec<RequiredSetting>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retired: Vec<RetiredItem>,
}

impl Release {
    /// `lineage/release_id` — the update code without its revision, and the
    /// folder the release lives in.
    pub fn code_path(&self) -> String {
        format!("{}/{}", self.lineage, self.release_id)
    }

    /// Every path the target state puts an active file at, lower-cased.
    pub fn target_paths_lowercase(&self) -> HashSet<String> {
        self.addons
            .iter()
            .filter(|addon| addon.enabled)
            .map(|addon| addon.relative_path().to_lowercase())
            .chain(self.files.iter().map(|file| file.path.to_lowercase()))
            .collect()
    }
}

/// The repository path of a hosted payload. Content-addressed, so a path can
/// never come to mean different bytes and identical files are stored once.
pub fn object_path(lineage: &str, sha256: &str) -> String {
    format!("{lineage}/objects/{}/{sha256}", &sha256[..2])
}

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

pub fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDigest {
    pub sha256: String,
    pub size: u64,
}

/// SHA-256 and length of a file, read in one streaming pass.
pub fn digest_file(path: &Path) -> std::io::Result<FileDigest> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 256 * 1024];
    let mut size = 0u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        size += read as u64;
    }
    Ok(FileDigest {
        sha256: hex(&hasher.finalize()),
        size,
    })
}

/// The digest of a file that may not be there. Absent is an answer, not an error.
pub fn digest_if_present(path: &Path) -> Result<Option<FileDigest>, String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => digest_file(path)
            .map(Some)
            .map_err(|error| format!("Failed to hash {}: {error}", path.display())),
        Ok(_) => Err(format!(
            "{} is not a regular file, so CEMM will not treat it as pack content",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Failed to inspect {}: {error}", path.display())),
    }
}

/// CurseForge's package fingerprint: MurmurHash2 (seed 1) over the file with
/// tab, line feed, carriage return and space removed.
///
/// The launcher identifies files by this number, which is why CEMM both checks
/// it at capture — it is the upstream evidence that a jar on the admin's disk
/// is the file its record claims — and writes it back when it updates a record.
pub fn curseforge_fingerprint(bytes: &[u8]) -> u32 {
    const M: u32 = 0x5bd1_e995;
    const R: u32 = 24;
    let is_whitespace = |byte: u8| matches!(byte, 9 | 10 | 13 | 32);
    let length = bytes.iter().filter(|byte| !is_whitespace(**byte)).count() as u32;

    let mut hash: u32 = 1 ^ length;
    let mut pending: u32 = 0;
    let mut pending_bits: u32 = 0;
    for &byte in bytes {
        if is_whitespace(byte) {
            continue;
        }
        pending |= u32::from(byte) << pending_bits;
        pending_bits += 8;
        if pending_bits == 32 {
            let mut k = pending.wrapping_mul(M);
            k ^= k >> R;
            k = k.wrapping_mul(M);
            hash = hash.wrapping_mul(M);
            hash ^= k;
            pending = 0;
            pending_bits = 0;
        }
    }
    if pending_bits > 0 {
        hash ^= pending;
        hash = hash.wrapping_mul(M);
    }
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(M);
    hash ^= hash >> 15;
    hash
}

// ---------------------------------------------------------------------------
// Paths and the protected-data policy
// ---------------------------------------------------------------------------

const WINDOWS_RESERVED_NAMES: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Whether a relative path is one CEMM is willing to write, delete or publish.
///
/// Stricter than `validate_path_within_base`, which only has to keep a path
/// inside the profile. A release path is also stored in a repository, compared
/// across operating systems and matched against earlier releases, so it has to
/// mean the same file everywhere: forward slashes, no dot segments, nothing
/// Windows would silently rewrite (trailing dots and spaces, device names) or
/// reinterpret (a colon is a drive letter or an NTFS alternate data stream).
pub fn validate_release_path(path: &str) -> Result<(), String> {
    let refuse = |why: &str| Err(format!("Refusing path '{path}': {why}"));

    if path.is_empty() {
        return refuse("it is empty");
    }
    if path.len() > MAX_RELEASE_PATH_BYTES {
        return refuse("it is too long");
    }
    if path.starts_with('/') || path.starts_with('~') {
        return refuse("it is not relative to the profile");
    }
    if path.contains('\\') {
        return refuse("it uses backslashes; release paths use forward slashes");
    }
    if path.contains(':') {
        return refuse("a colon would name a drive or an alternate data stream");
    }
    if path
        .chars()
        .any(|c| c.is_control() || matches!(c, '<' | '>' | '"' | '|' | '?' | '*'))
    {
        return refuse("it contains a character that is not valid in a filename");
    }

    for segment in path.split('/') {
        if segment.is_empty() {
            return refuse("it has an empty segment");
        }
        if segment == "." || segment == ".." {
            return refuse("it has a dot segment");
        }
        if segment.ends_with('.') || segment.ends_with(' ') || segment.starts_with(' ') {
            return refuse("a segment starts or ends with a space or dot, which Windows rewrites");
        }
        let stem = segment.split('.').next().unwrap_or(segment);
        if WINDOWS_RESERVED_NAMES
            .iter()
            .any(|reserved| stem.eq_ignore_ascii_case(reserved))
        {
            return refuse("it uses a reserved device name");
        }
    }

    Ok(())
}

/// Why a path is off limits to releases, or `None` when it is not.
///
/// Enforced on both sides. At capture it keeps an admin from publishing their
/// own saves, options, launcher identity or credentials because a folder
/// happened to be selected. At install it is the reason a downloaded release
/// cannot reach those files in a player's profile no matter what it lists.
pub fn protected_reason(path: &str) -> Option<&'static str> {
    let lower = path.to_lowercase();
    let segments: Vec<&str> = lower.split('/').collect();
    let first = segments.first().copied().unwrap_or_default();
    let last = segments.last().copied().unwrap_or_default();

    match first {
        ".cemm" | ".cemm-transaction" => return Some("it is CEMM's own record"),
        ".git" => return Some("it is version-control data"),
        "saves" | "world" | "backups" | "simplebackups" => {
            return Some("worlds and their backups are never release content")
        }
        "logs" | "crash-reports" | "debug" | "downloads" => {
            return Some("logs, crash reports and downloads are runtime output")
        }
        "screenshots" | "journeymap" | "local" | "schematics-local" => {
            return Some("it is a player's own data")
        }
        _ => {}
    }
    if first.starts_with("xaero") {
        return Some("it is a player's own map data");
    }

    if segments.len() == 1 {
        match first {
            "minecraftinstance.json" | "launcher_profiles.json" | "launcher_accounts.json" => {
                return Some("launcher records are local to each machine")
            }
            "options.txt"
            | "optionsof.txt"
            | "optionsshaders.txt"
            | "servers.dat"
            | "servers.dat_old"
            | "user-prefs.json"
            | "command_history.txt" => return Some("it holds a player's personal settings"),
            "usercache.json" | "usernamecache.json" => {
                return Some("it is a per-machine player cache")
            }
            "cemm-manifest.json" | "cemm-pack.json" => return Some("it is CEMM's own record"),
            "server.properties"
            | "eula.txt"
            | "ops.json"
            | "whitelist.json"
            | "banned-players.json"
            | "banned-ips.json"
            | "user_jvm_args.txt" => return Some("it is host-local server configuration"),
            _ => {}
        }
    }

    if last == ".env"
        || last.starts_with(".env.")
        || last == "private.key"
        || last == "id_rsa"
        || last == "id_ed25519"
        || [".key", ".pem", ".pfx", ".p12", ".jks", ".keystore"]
            .iter()
            .any(|suffix| last.ends_with(suffix))
    {
        return Some("it looks like a credential or key");
    }
    if last.ends_with(".snbt_merged") || last == "session.lock" {
        return Some("it is generated by the game, not authored");
    }
    if segments
        .iter()
        .any(|segment| *segment == ".git" || *segment == "__pycache__")
    {
        return Some("it is tooling data, not pack content");
    }

    None
}

/// A path that is both well-formed and allowed to be release content.
pub fn validate_content_path(path: &str) -> Result<(), String> {
    validate_release_path(path)?;
    if let Some(reason) = protected_reason(path) {
        return Err(format!("Refusing path '{path}': {reason}"));
    }
    Ok(())
}

/// `[a-z0-9]` runs joined by single dashes. A lineage is used verbatim as a
/// repository folder, so it must already be what a sanitiser would produce.
pub fn is_valid_lineage(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

pub fn is_valid_release_id(value: &str) -> bool {
    let mut bytes = value.bytes();
    let starts_well = bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
    starts_well
        && value.len() <= 64
        && value != "objects"
        && !value.contains("..")
        && !value.ends_with('.')
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn is_plain_file_name(name: &str) -> bool {
    !name.contains('/') && validate_release_path(name).is_ok()
}

/// Whether a JSON pointer is one CEMM will follow: non-empty, absolute, and
/// with no empty reference token.
pub fn is_valid_setting_pointer(pointer: &str) -> bool {
    pointer.starts_with('/') && pointer.len() > 1 && !pointer[1..].split('/').any(str::is_empty)
}

// ---------------------------------------------------------------------------
// Compatibility gate and validation
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ReleaseHeader {
    schema: Option<u64>,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    min_cemm: Option<String>,
}

fn upgrade_message(min_cemm: Option<&str>) -> String {
    let needed = min_cemm
        .filter(|version| !version.trim().is_empty())
        .map(|version| format!("CEMM {version} or newer"))
        .unwrap_or_else(|| "a newer CEMM".to_string());
    format!(
        "This update was published for {needed}; this is CEMM {}. Update CEMM, then paste the code again. Nothing was changed.",
        env!("CARGO_PKG_VERSION")
    )
}

/// Reads a release document, refusing anything this build cannot fully apply.
///
/// The header is read on its own first, and leniently, so the answer to "is
/// this for me" never depends on being able to parse the rest: a document from
/// a future schema is reported as needing a newer CEMM, not as malformed.
pub fn parse_release(bytes: &[u8]) -> Result<Release, String> {
    let header: ReleaseHeader = serde_json::from_slice(bytes)
        .map_err(|error| format!("The release document is not readable: {error}"))?;

    let schema = header
        .schema
        .ok_or("The release document does not state its schema")?;
    if schema != u64::from(RELEASE_SCHEMA) {
        return Err(upgrade_message(header.min_cemm.as_deref()));
    }
    if header
        .requires
        .iter()
        .any(|capability| !SUPPORTED_CAPABILITIES.contains(&capability.as_str()))
    {
        return Err(upgrade_message(header.min_cemm.as_deref()));
    }

    let release: Release = serde_json::from_slice(bytes)
        .map_err(|error| format!("The release document is not valid: {error}"))?;
    validate_release(&release)?;
    Ok(release)
}

/// Checks everything about a release that does not depend on a profile.
///
/// Run on a downloaded document before anything is planned, and on a captured
/// one before anything is published, so the two sides agree on what a
/// well-formed release is.
pub fn validate_release(release: &Release) -> Result<(), String> {
    if release.schema != RELEASE_SCHEMA {
        return Err(format!(
            "Unsupported release schema {} (this build writes {RELEASE_SCHEMA})",
            release.schema
        ));
    }
    for capability in &release.requires {
        if !SUPPORTED_CAPABILITIES.contains(&capability.as_str()) {
            return Err(format!(
                "Release requires an unknown capability: {capability}"
            ));
        }
    }
    let needs = |capability: &str, used: bool, what: &str| {
        if used && !release.requires.iter().any(|listed| listed == capability) {
            Err(format!(
                "Release carries {what} but does not require '{capability}'"
            ))
        } else {
            Ok(())
        }
    };
    needs(CAP_ADDONS, !release.addons.is_empty(), "addons")?;
    needs(
        CAP_FILES,
        !release.files.is_empty() || !release.retired.is_empty(),
        "managed files",
    )?;
    needs(CAP_SETTINGS, !release.settings.is_empty(), "settings")?;

    if !is_valid_lineage(&release.lineage) {
        return Err(format!(
            "Invalid pack lineage '{}': use lowercase letters, digits and single dashes",
            release.lineage
        ));
    }
    if !is_valid_release_id(&release.release_id) {
        return Err(format!(
            "Invalid release id '{}': use lowercase letters, digits, dots, dashes and underscores",
            release.release_id
        ));
    }
    if release.sequence == 0 {
        return Err("A release sequence starts at 1".to_string());
    }
    if release.base.project_id == 0 || release.base.file_id == 0 {
        return Err(
            "A release must name the exact official pack file it was built from".to_string(),
        );
    }
    let plain = |value: &str| {
        !value.trim().is_empty()
            && value.len() <= 64
            && value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
    };
    if !plain(&release.minecraft) {
        return Err(format!("Invalid Minecraft version '{}'", release.minecraft));
    }
    if !plain(&release.loader) {
        return Err(format!("Invalid mod loader '{}'", release.loader));
    }
    for supported in &release.supported_from {
        if !is_valid_release_id(supported) {
            return Err(format!("Invalid supported_from release id '{supported}'"));
        }
    }
    if release.adoption.min_project_overlap_percent > 100 {
        return Err("Adoption overlap is a percentage".to_string());
    }
    if let Some(sha256) = release
        .server_release
        .as_ref()
        .and_then(|server| server.sha256.as_deref())
    {
        if !is_sha256_hex(sha256) {
            return Err("The server release hash is not a SHA-256".to_string());
        }
    }

    if release.addons.len() > MAX_RELEASE_ADDONS
        || release.files.len() > MAX_RELEASE_FILES
        || release.settings.len() > MAX_RELEASE_SETTINGS
        || release.retired.len() > MAX_RELEASE_RETIRED
    {
        return Err("The release lists more entries than CEMM accepts".to_string());
    }

    // One namespace for every path the target puts a file at, compared without
    // case: `Mods/x.jar` and `mods/X.jar` are one file on Windows and macOS.
    let mut occupied: HashMap<String, String> = HashMap::new();
    let mut claim = |path: String| -> Result<(), String> {
        match occupied.insert(path.to_lowercase(), path.clone()) {
            Some(existing) if existing == path => Err(format!(
                "The release lists '{path}' more than once"
            )),
            Some(existing) => Err(format!(
                "The release lists '{existing}' and '{path}', which are the same file on a case-insensitive disk"
            )),
            None => Ok(()),
        }
    };

    let mut projects = HashSet::new();
    for addon in &release.addons {
        if addon.project_id == 0 || addon.file_id == 0 {
            return Err(format!(
                "Addon '{}' is missing its CurseForge project or file id",
                addon.name
            ));
        }
        if !projects.insert(addon.project_id) {
            return Err(format!(
                "CurseForge project {} is listed twice; a release selects one file per project",
                addon.project_id
            ));
        }
        if !is_plain_file_name(&addon.file_name)
            || addon.file_name.to_lowercase().ends_with(".disabled")
        {
            return Err(format!(
                "Addon '{}' has an unusable file name '{}'",
                addon.name, addon.file_name
            ));
        }
        if !is_sha256_hex(&addon.sha256) {
            return Err(format!("Addon '{}' has no valid SHA-256", addon.name));
        }
        if addon.size > MAX_RELEASE_ADDON_BYTES {
            return Err(format!(
                "Addon '{}' is larger than CEMM installs",
                addon.name
            ));
        }
        if let Some(url) = addon.download_url.as_deref() {
            crate::installer::validate_addon_download_url(&addon.name, url)?;
        }
        let path = addon.relative_path();
        validate_content_path(&path)?;
        claim(path.clone())?;
        // The switched-off spelling belongs to the same addon.
        claim(format!("{path}.disabled"))?;
    }
    for addon in release.addons.iter().filter(|addon| addon.pinned) {
        if !addon.enabled {
            return Err(format!(
                "Addon '{}' is pinned but disabled; a pin holds an active file",
                addon.name
            ));
        }
    }

    for file in &release.files {
        validate_content_path(&file.path)?;
        if crate::installer::targets_reserved_install_path(&file.path) {
            return Err(format!(
                "Refusing path '{}': it is reserved for CEMM",
                file.path
            ));
        }
        if !is_sha256_hex(&file.sha256) || file.previous.iter().any(|hash| !is_sha256_hex(hash)) {
            return Err(format!("File '{}' has an invalid SHA-256", file.path));
        }
        if file.size > MAX_MANAGED_FILE_BYTES {
            return Err(format!(
                "File '{}' is larger than CEMM distributes",
                file.path
            ));
        }
        claim(file.path.clone())?;
    }

    let mut settings = HashSet::new();
    for setting in &release.settings {
        validate_content_path(&setting.path)?;
        if !is_valid_setting_pointer(&setting.pointer) {
            return Err(format!(
                "Setting pointer '{}' in '{}' is not a JSON pointer to a key",
                setting.pointer, setting.path
            ));
        }
        if !settings.insert((setting.path.to_lowercase(), setting.pointer.clone())) {
            return Err(format!(
                "Setting '{}' in '{}' is listed twice",
                setting.pointer, setting.path
            ));
        }
        // A required key inside a file the release also replaces wholesale
        // would be two instructions about the same bytes.
        if release.files.iter().any(|file| {
            file.path.eq_ignore_ascii_case(&setting.path) && file.policy == FilePolicy::Managed
        }) {
            return Err(format!(
                "'{}' is both a managed file and the target of a required setting; ship it as a default instead",
                setting.path
            ));
        }
    }

    for retired in &release.retired {
        validate_content_path(&retired.path)?;
        if !is_sha256_hex(&retired.sha256) {
            return Err(format!(
                "Retired entry '{}' has an invalid SHA-256",
                retired.path
            ));
        }
    }

    Ok(())
}

/// The document published under the legacy manifest's name beside a release.
///
/// Every released client — 1.x through 2.1 — fetches `cemm-manifest.json` and
/// deserialises it into a struct whose `mods` is a required array. Those
/// parsers ignore unknown fields, so a `minVersion` key would be read past and
/// a release would install as whatever its legacy-shaped fields happened to
/// say. Leaving the file out is no better: the client reports a 404 as a wrong
/// repository.
///
/// So `mods` is deliberately a string. Serde rejects it — before the client has
/// a manifest to preview, let alone install — and quotes the string in the
/// error, which is how a client that has never heard of schema 2 ends up
/// telling its user to update. The wording avoids every substring the old
/// frontend's error handler rewrites into a generic message.
pub fn legacy_tombstone(release: &Release) -> serde_json::Value {
    let message = format!(
        "This update needs CEMM {RELEASE_MIN_CEMM} or newer. Update CEMM, then paste the code again."
    );
    serde_json::json!({
        "updateType": "full",
        "mods": message,
        "cemm_release": {
            "schema": release.schema,
            "min_cemm": release.min_cemm,
            "file": RELEASE_FILE_NAME,
            "lineage": release.lineage,
            "release_id": release.release_id,
        },
    })
}

// ---------------------------------------------------------------------------
// Update codes
// ---------------------------------------------------------------------------

/// An update code split into where it points and which revision it is pinned to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCode {
    /// `modpack-key/update-id`, or a bare legacy id.
    pub path: String,
    /// The 40-character commit the code was published at, when it carries one.
    pub revision: Option<String>,
}

pub fn is_commit_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Splits `pack/release@commit` into its two halves.
///
/// The revision is optional because legacy codes never had one and because a
/// friend may be read a short code over voice chat; a code without it is
/// pinned to the branch head at the moment it is fetched instead.
pub fn parse_update_code(input: &str) -> Result<UpdateCode, String> {
    let normalized = input.trim().replace('\\', "/");
    let (path, revision) = match normalized.rsplit_once('@') {
        Some((path, revision)) => (path.trim().to_string(), Some(revision.trim().to_string())),
        None => (normalized, None),
    };

    if path.is_empty() {
        return Err("Update code is empty".to_string());
    }
    if path.contains("..") {
        return Err("Invalid update code: path traversal is not allowed".to_string());
    }
    if let Some(revision) = revision.as_deref() {
        if !is_commit_sha(revision) {
            return Err(
                "Invalid update code: the part after '@' must be the full 40-character revision"
                    .to_string(),
            );
        }
    }

    Ok(UpdateCode {
        path,
        revision: revision.map(|revision| revision.to_lowercase()),
    })
}

// ---------------------------------------------------------------------------
// Three-way comparison for an official base upgrade
// ---------------------------------------------------------------------------

/// A pack state reduced to what a comparison needs: content hashes by path, and
/// the selected CurseForge file per project.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inventory {
    /// path -> sha256
    pub files: BTreeMap<String, String>,
    /// project id -> selected file
    pub addons: BTreeMap<u64, InventoryAddon>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryAddon {
    pub file_id: u64,
    pub name: String,
    pub file_name: String,
    pub requires: Vec<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "generated/")]
pub enum ThreeWayDecision {
    /// Neither side moved.
    Unchanged,
    /// Only the official pack changed it — including by deleting it.
    TakeUpstream,
    /// Only the custom pack changed it — including by deleting it.
    KeepCustom,
    /// Both arrived at the same result.
    SameResult,
    /// Both changed it, differently. An admin decides; nothing is guessed.
    Conflict,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "generated/")]
pub enum ThreeWayKind {
    File,
    Addon,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct ThreeWayItem {
    pub kind: ThreeWayKind,
    /// A path, or `project <id>` for an addon.
    pub key: String,
    pub label: String,
    pub decision: ThreeWayDecision,
    /// What each side holds: a short hash or file name, or `null` when absent.
    pub base: Option<String>,
    pub custom: Option<String>,
    pub upstream: Option<String>,
    /// For a conflict, what kind; for anything else, empty.
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct ThreeWayReport {
    pub unchanged: u32,
    pub take_upstream: u32,
    pub keep_custom: u32,
    pub same_result: u32,
    /// Every decision that is not `Unchanged`, conflicts first.
    pub items: Vec<ThreeWayItem>,
    /// Things no hash comparison can settle and a person has to look at.
    pub review: Vec<String>,
    pub conflicts: u32,
}

fn decide<T: PartialEq>(
    base: Option<&T>,
    custom: Option<&T>,
    upstream: Option<&T>,
) -> ThreeWayDecision {
    match (custom == base, upstream == base) {
        (true, true) => ThreeWayDecision::Unchanged,
        (true, false) => ThreeWayDecision::TakeUpstream,
        (false, true) => ThreeWayDecision::KeepCustom,
        (false, false) if custom == upstream => ThreeWayDecision::SameResult,
        (false, false) => ThreeWayDecision::Conflict,
    }
}

fn conflict_reason(base: bool, custom: bool, upstream: bool, binary: bool) -> &'static str {
    match (base, custom, upstream) {
        (true, false, true) => "deleted in the custom pack, changed upstream",
        (true, true, false) => "changed in the custom pack, deleted upstream",
        (false, true, true) => "added on both sides with different content",
        _ if binary => "binary file changed on both sides",
        _ => "changed on both sides",
    }
}

fn looks_binary(path: &str) -> bool {
    let lower = path.to_lowercase();
    [
        ".jar",
        ".zip",
        ".png",
        ".jpg",
        ".jpeg",
        ".ogg",
        ".nbt",
        ".dat",
        ".bin",
        ".gz",
        ".schem",
        ".litematic",
        ".ttf",
        ".otf",
    ]
    .iter()
    .any(|suffix| lower.ends_with(suffix))
}

/// Classifies every path and addon across the previous official release
/// (`base`), the last custom release (`custom`) and the new official release
/// (`upstream`).
///
/// It proposes; it does not merge. The four clean outcomes are mechanical —
/// one side changed, or both agree — and everything else is returned as a
/// conflict with both sides named. `pinned` projects are a conflict whenever
/// upstream moves them, even though only one side "changed": the pin is the
/// custom pack's standing decision about that file.
pub fn three_way(
    base: &Inventory,
    custom: &Inventory,
    upstream: &Inventory,
    pinned: &[u64],
) -> ThreeWayReport {
    let mut report = ThreeWayReport::default();
    let short = |hash: &String| hash.chars().take(12).collect::<String>();

    let paths: BTreeSet<&String> = base
        .files
        .keys()
        .chain(custom.files.keys())
        .chain(upstream.files.keys())
        .collect();

    // Two spellings of one path cannot both be kept on Windows. Whichever side
    // introduced the second one, the candidate needs a single name chosen.
    let mut spellings: HashMap<String, BTreeSet<&String>> = HashMap::new();
    for path in &paths {
        spellings
            .entry(path.to_lowercase())
            .or_default()
            .insert(path);
    }
    let aliased: HashSet<&String> = spellings
        .values()
        .filter(|group| group.len() > 1)
        .flat_map(|group| group.iter().copied())
        .collect();

    for path in paths {
        let (b, c, u) = (
            base.files.get(path),
            custom.files.get(path),
            upstream.files.get(path),
        );
        let mut decision = decide(b, c, u);
        let mut reason = String::new();
        if aliased.contains(path) {
            decision = ThreeWayDecision::Conflict;
            reason = "another path differs from this one only by letter case".to_string();
        } else if decision == ThreeWayDecision::Conflict {
            reason = conflict_reason(b.is_some(), c.is_some(), u.is_some(), looks_binary(path))
                .to_string();
        }
        tally(&mut report, decision);
        if decision != ThreeWayDecision::Unchanged {
            report.items.push(ThreeWayItem {
                kind: ThreeWayKind::File,
                key: path.clone(),
                label: path.clone(),
                decision,
                base: b.map(short),
                custom: c.map(short),
                upstream: u.map(short),
                reason,
            });
        }
    }

    let projects: BTreeSet<u64> = base
        .addons
        .keys()
        .chain(custom.addons.keys())
        .chain(upstream.addons.keys())
        .copied()
        .collect();
    let mut resolved_projects: HashSet<u64> = HashSet::new();
    for project in &projects {
        let (b, c, u) = (
            base.addons.get(project),
            custom.addons.get(project),
            upstream.addons.get(project),
        );
        let file_id = |addon: Option<&InventoryAddon>| addon.map(|addon| addon.file_id);
        let mut decision = decide(
            file_id(b).as_ref(),
            file_id(c).as_ref(),
            file_id(u).as_ref(),
        );
        let mut reason = String::new();
        if pinned.contains(project) && file_id(u) != file_id(b) && file_id(u) != file_id(c) {
            decision = ThreeWayDecision::Conflict;
            reason = "pinned in the custom pack, and upstream selects a different file".to_string();
        } else if decision == ThreeWayDecision::Conflict {
            reason = conflict_reason(b.is_some(), c.is_some(), u.is_some(), true).to_string();
        }
        tally(&mut report, decision);

        let present_in_candidate = match decision {
            ThreeWayDecision::TakeUpstream => u.is_some(),
            ThreeWayDecision::Conflict => true,
            _ => c.is_some(),
        };
        if present_in_candidate {
            resolved_projects.insert(*project);
        }

        if decision != ThreeWayDecision::Unchanged {
            let named = b.or(c).or(u).map(|addon| addon.name.clone());
            let describe =
                |addon: Option<&InventoryAddon>| addon.map(|addon| addon.file_name.clone());
            report.items.push(ThreeWayItem {
                kind: ThreeWayKind::Addon,
                key: format!("project {project}"),
                label: named.unwrap_or_else(|| format!("project {project}")),
                decision,
                base: describe(b),
                custom: describe(c),
                upstream: describe(u),
                reason,
            });
        }
    }

    // A library upstream dropped may still be something a custom addon needs.
    // Hashes cannot see that; the dependency lists can.
    for (project, addon) in &custom.addons {
        if !resolved_projects.contains(project) {
            continue;
        }
        for required in &addon.requires {
            let was_there =
                base.addons.contains_key(required) || custom.addons.contains_key(required);
            if was_there && !resolved_projects.contains(required) {
                report.review.push(format!(
                    "'{}' requires CurseForge project {required}, which the new official release removes. Keep it, replace it, or drop '{}'.",
                    addon.name, addon.name
                ));
            }
        }
    }
    // A custom addon the official pack now ships itself is one copy, but the
    // reason it was added — and any workaround built around it — needs a look.
    for (project, addon) in &custom.addons {
        if !base.addons.contains_key(project) && upstream.addons.contains_key(project) {
            report.review.push(format!(
                "The new official release now includes '{}' itself. Keep one copy and check whether the custom overrides written for it are still needed.",
                addon.name
            ));
        }
    }

    report.review.sort();
    report.review.dedup();
    report.items.sort_by_key(|item| {
        (
            item.decision != ThreeWayDecision::Conflict,
            item.kind == ThreeWayKind::File,
            item.key.clone(),
        )
    });
    report
}

fn tally(report: &mut ThreeWayReport, decision: ThreeWayDecision) {
    match decision {
        ThreeWayDecision::Unchanged => report.unchanged += 1,
        ThreeWayDecision::TakeUpstream => report.take_upstream += 1,
        ThreeWayDecision::KeepCustom => report.keep_custom += 1,
        ThreeWayDecision::SameResult => report.same_result += 1,
        ThreeWayDecision::Conflict => report.conflicts += 1,
    }
}

/// Whether text carries merge-conflict markers. A candidate release that does
/// is a merge somebody did not finish.
pub fn has_conflict_markers(text: &str) -> bool {
    let mut opened = false;
    let mut separated = false;
    for line in text.lines() {
        if line.starts_with("<<<<<<< ") {
            opened = true;
        } else if opened && line.trim_end() == "=======" {
            separated = true;
        } else if opened && separated && line.starts_with(">>>>>>> ") {
            return true;
        }
    }
    false
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn hash_of(text: &str) -> String {
        sha256_hex(text.as_bytes())
    }

    pub(crate) fn test_addon(project_id: u64, file_id: u64, file_name: &str) -> ReleaseAddon {
        ReleaseAddon {
            project_id,
            file_id,
            name: format!("Addon {project_id}"),
            category: AddonCategory::Mods,
            file_name: file_name.to_string(),
            enabled: true,
            size: file_name.len() as u64,
            sha256: hash_of(file_name),
            sha1: None,
            fingerprint: None,
            download_url: Some(format!("https://edge.forgecdn.net/files/{file_name}")),
            website_url: None,
            pinned: false,
            requires: Vec::new(),
        }
    }

    pub(crate) fn test_file(path: &str, content: &str) -> ReleaseFile {
        ReleaseFile {
            path: path.to_string(),
            sha256: hash_of(content),
            size: content.len() as u64,
            source: FileSource::Authored,
            policy: FilePolicy::Managed,
            role: FileRole::Both,
            hosted: true,
            previous: Vec::new(),
        }
    }

    pub(crate) fn test_release(release_id: &str, sequence: u64) -> Release {
        Release {
            schema: RELEASE_SCHEMA,
            min_cemm: RELEASE_MIN_CEMM.to_string(),
            requires: SUPPORTED_CAPABILITIES
                .iter()
                .map(|capability| capability.to_string())
                .collect(),
            lineage: "test-fusion".to_string(),
            release_id: release_id.to_string(),
            sequence,
            name: "Test Fusion".to_string(),
            role: Role::Client,
            base: OfficialBase {
                project_id: 1356598,
                file_id: 9571494,
                name: "All the Mons 2.0.1".to_string(),
            },
            minecraft: "1.21.1".to_string(),
            loader: "neoforge-21.1.265".to_string(),
            supported_from: Vec::new(),
            adoption: AdoptionRule::default(),
            server_release: None,
            notes: String::new(),
            manual_steps: Vec::new(),
            known_issues: Vec::new(),
            addons: Vec::new(),
            files: Vec::new(),
            settings: Vec::new(),
            retired: Vec::new(),
        }
    }

    #[test]
    fn the_app_declares_one_version_and_can_apply_the_releases_it_publishes() {
        let version = env!("CARGO_PKG_VERSION");
        let package: serde_json::Value =
            serde_json::from_str(include_str!("../../../package.json")).expect("package.json");
        let tauri: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json")).expect("tauri.conf.json");
        assert_eq!(
            package["version"], version,
            "package.json disagrees with Cargo.toml"
        );
        assert_eq!(
            tauri["version"], version,
            "tauri.conf.json disagrees with Cargo.toml"
        );

        // Every release says which CEMM it needs. A build older than that
        // number would publish releases its own players are told to update for.
        let parts = |text: &str| -> Vec<u64> {
            text.split('.')
                .map(|part| part.parse().expect("a numeric version part"))
                .collect()
        };
        assert!(
            parts(version) >= parts(RELEASE_MIN_CEMM),
            "this build is {version} but tells players releases need {RELEASE_MIN_CEMM}"
        );
    }

    #[test]
    fn sha256_matches_the_published_test_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(is_sha256_hex(&sha256_hex(b"")));
        assert!(!is_sha256_hex("BA7816BF"));
    }

    #[test]
    fn digest_file_agrees_with_the_in_memory_hash() {
        let temp = tempfile::tempdir().expect("temp dir");
        let path = temp.path().join("payload.bin");
        let bytes: Vec<u8> = (0..700_000u32).map(|n| (n % 251) as u8).collect();
        std::fs::write(&path, &bytes).expect("write");

        let digest = digest_file(&path).expect("digest");

        assert_eq!(digest.size, bytes.len() as u64);
        assert_eq!(digest.sha256, sha256_hex(&bytes));
        assert_eq!(
            digest_if_present(&temp.path().join("absent")).expect("absent is fine"),
            None
        );
        assert!(
            digest_if_present(temp.path()).is_err(),
            "a directory is not pack content"
        );
    }

    #[test]
    fn the_fingerprint_ignores_whitespace_and_handles_every_tail_length() {
        // Whitespace is removed before hashing, so these are one input.
        assert_eq!(
            curseforge_fingerprint(b"ab cd\r\n\tef"),
            curseforge_fingerprint(b"abcdef")
        );
        // Lengths 0..=8 exercise every tail size on both sides of a block.
        let values: Vec<u32> = (0..=8)
            .map(|length| curseforge_fingerprint(&b"abcdefgh"[..length]))
            .collect();
        let distinct: HashSet<&u32> = values.iter().collect();
        assert_eq!(distinct.len(), values.len(), "got: {values:?}");
    }

    #[test]
    fn release_paths_must_mean_one_file_on_every_system() {
        for good in [
            "mods/sodium.jar",
            "kubejs/server_scripts/fusion/recipes.js",
            "config/Some Mod/settings.json",
            "config/ftbquests/quests/chapters/a.snbt",
        ] {
            assert!(validate_release_path(good).is_ok(), "{good} should pass");
        }

        for bad in [
            "",
            "/etc/passwd",
            "~/evil",
            "../outside",
            "config/../../outside",
            "config/./x.json",
            "config//x.json",
            r"config\x.json",
            "C:/Windows/x",
            "config/file.txt:stream",
            "config/aux.json",
            "config/NUL",
            "config/COM1.txt",
            "config/trailing./x",
            "config/trailing /x",
            "config/x.json ",
            "config/what?.json",
            "config/star*.json",
            "config/bell\u{7}.json",
        ] {
            assert!(
                validate_release_path(bad).is_err(),
                "'{bad}' should be refused"
            );
        }
    }

    #[test]
    fn personal_local_and_secret_data_is_protected() {
        for protected in [
            "saves/World/level.dat",
            "options.txt",
            "optionsof.txt",
            "servers.dat",
            "minecraftinstance.json",
            "MinecraftInstance.JSON",
            "logs/latest.log",
            "crash-reports/crash.txt",
            "screenshots/a.png",
            "journeymap/data/x",
            "xaero/minimap/x",
            "local/ftbchunks/data.snbt",
            ".cemm/state.json",
            ".cemm-transaction/journal.json",
            ".git/config",
            "cemm-manifest.json",
            "cemm-pack.json",
            "usercache.json",
            "server.properties",
            "eula.txt",
            "config/.env",
            "config/private.key",
            "config/signing.pem",
            "config/ftbquests/quests/chapters/a.snbt_merged",
            "tools/__pycache__/x.pyc",
        ] {
            assert!(
                protected_reason(protected).is_some(),
                "{protected} should be protected"
            );
            assert!(validate_content_path(protected).is_err());
        }

        for allowed in [
            "config/tacztweaks.json",
            "kubejs/startup_scripts/a.js",
            "mods/atmons-ore-hammer-fix-1.0.0.jar",
            "defaultconfigs/x-server.toml",
            "config/ftbquests/quests/chapters/a.snbt",
            // A config that merely mentions options is not the options file.
            "config/options.txt",
        ] {
            assert!(
                protected_reason(allowed).is_none(),
                "{allowed} should be allowed"
            );
        }
    }

    #[test]
    fn lineage_and_release_ids_are_already_repository_safe() {
        assert!(is_valid_lineage("atmons-fusion"));
        assert!(is_valid_lineage("pack2"));
        for bad in [
            "",
            "ATMons",
            "atmons fusion",
            "-x",
            "x-",
            "a--b",
            "a/b",
            "a.b",
        ] {
            assert!(!is_valid_lineage(bad), "'{bad}' should be refused");
        }

        assert!(is_valid_release_id("2026.11.15-1"));
        assert!(is_valid_release_id("r3_hotfix"));
        for bad in ["", "objects", "..", "a..b", "-a", "A1", "a/b", "a@b", "a."] {
            assert!(!is_valid_release_id(bad), "'{bad}' should be refused");
        }
    }

    #[test]
    fn a_well_formed_release_validates_and_round_trips() {
        let mut release = test_release("r1", 1);
        release.addons.push(test_addon(1, 10, "sodium-1.jar"));
        release.files.push(test_file("kubejs/a.js", "a"));
        release.settings.push(RequiredSetting {
            path: "config/tacztweaks.json".to_string(),
            format: SettingFormat::Json,
            pointer: "/general/compatibility/sableCompat".to_string(),
            value: serde_json::json!(false),
            reason: "native TaCZ support replaces it".to_string(),
        });

        validate_release(&release).expect("should validate");

        let bytes = serde_json::to_vec_pretty(&release).expect("serialize");
        assert_eq!(parse_release(&bytes).expect("parse"), release);
    }

    #[test]
    fn a_release_cannot_use_a_section_without_requiring_its_capability() {
        let mut release = test_release("r1", 1);
        release.files.push(test_file("kubejs/a.js", "a"));
        release.requires = vec![CAP_ADDONS.to_string()];

        let error = validate_release(&release).expect_err("must be refused");
        assert!(error.contains(CAP_FILES), "got: {error}");
    }

    #[test]
    fn paths_that_alias_on_a_case_insensitive_disk_are_refused() {
        let mut release = test_release("r1", 1);
        release.files.push(test_file("kubejs/Recipes.js", "a"));
        release.files.push(test_file("kubejs/recipes.js", "b"));

        let error = validate_release(&release).expect_err("must be refused");
        assert!(error.contains("case-insensitive"), "got: {error}");

        // An addon and a managed file cannot claim one jar either, in either
        // spelling: the disabled form belongs to the addon.
        let mut release = test_release("r1", 1);
        release.addons.push(test_addon(1, 10, "sodium-1.jar"));
        release
            .files
            .push(test_file("mods/Sodium-1.jar.disabled", "x"));
        assert!(validate_release(&release).is_err());
    }

    #[test]
    fn one_project_selects_exactly_one_file() {
        let mut release = test_release("r1", 1);
        release.addons.push(test_addon(1, 10, "sodium-1.jar"));
        release.addons.push(test_addon(1, 11, "sodium-2.jar"));

        let error = validate_release(&release).expect_err("must be refused");
        assert!(error.contains("one file per project"), "got: {error}");
    }

    #[test]
    fn a_release_cannot_reach_protected_or_reserved_paths() {
        for path in [
            "saves/World/level.dat",
            "options.txt",
            "minecraftinstance.json",
            ".cemm/state.json",
            "cemm-manifest.json",
            "../escape.txt",
            "config/x.json:ads",
        ] {
            let mut release = test_release("r1", 1);
            release.files.push(test_file(path, "x"));
            assert!(
                validate_release(&release).is_err(),
                "{path} must be refused"
            );

            let mut release = test_release("r1", 1);
            release.retired.push(RetiredItem {
                path: path.to_string(),
                sha256: hash_of("x"),
                project_id: None,
            });
            assert!(
                validate_release(&release).is_err(),
                "{path} must not be retirable"
            );
        }

        let mut release = test_release("r1", 1);
        release.settings.push(RequiredSetting {
            path: "options.txt".to_string(),
            format: SettingFormat::Json,
            pointer: "/fov".to_string(),
            value: serde_json::json!(90),
            reason: String::new(),
        });
        assert!(validate_release(&release).is_err());
    }

    #[test]
    fn addon_downloads_must_still_come_from_the_curseforge_cdn() {
        let mut release = test_release("r1", 1);
        let mut addon = test_addon(1, 10, "sodium-1.jar");
        addon.download_url = Some("https://evil.example.com/sodium-1.jar".to_string());
        release.addons.push(addon);

        let error = validate_release(&release).expect_err("must be refused");
        assert!(error.contains("forgecdn.net"), "got: {error}");
    }

    #[test]
    fn a_document_from_a_newer_schema_asks_for_an_update_instead_of_failing_to_parse() {
        // Nothing here but the header is something this build understands.
        let future = serde_json::json!({
            "schema": 3,
            "min_cemm": "3.0.0",
            "requires": ["addons-v2"],
            "payload": { "shape": "unknown to this build" }
        });
        let error = parse_release(&serde_json::to_vec(&future).unwrap())
            .expect_err("a newer schema must be refused");
        assert!(error.contains("CEMM 3.0.0 or newer"), "got: {error}");
        assert!(error.contains("Nothing was changed"), "got: {error}");

        // Same schema, but it needs something this build cannot do.
        let mut release = test_release("r1", 1);
        release.requires.push("toml-settings-v1".to_string());
        release.min_cemm = "2.5.0".to_string();
        let error = parse_release(&serde_json::to_vec(&release).unwrap())
            .expect_err("an unknown capability must be refused");
        assert!(error.contains("CEMM 2.5.0 or newer"), "got: {error}");
    }

    /// The parser every released client ships, frozen here field for field
    /// from the `cemm-v2.1.0` tag (and unchanged in shape since 1.6.0, which
    /// only lacks `custom_datapacks`). The live `Manifest` is free to evolve;
    /// this copy is what is actually installed on players' machines.
    mod legacy_v2_1_0 {
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        pub struct Addon {
            pub addon_file_id: u64,
            pub addon_name: String,
            pub addon_project_id: u64,
            pub cdn_download_url: String,
            pub mod_folder_path: String,
            pub version: String,
            #[serde(rename = "webSiteURL")]
            pub web_site_url: Option<String>,
            pub disabled: Option<bool>,
            #[serde(rename = "fileNameOnDisk")]
            pub file_name_on_disk: String,
        }

        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        pub struct ConfigFile {
            pub filename: String,
            pub relative_path: String,
        }

        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        pub struct CustomDatapack {
            pub name: String,
            pub archived: bool,
            pub files: Vec<ConfigFile>,
        }

        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        pub struct Manifest {
            #[serde(rename = "updateType")]
            pub update_type: Option<String>,
            pub mods: Vec<Addon>,
            pub resourcepacks: Vec<Addon>,
            pub shaderpacks: Vec<Addon>,
            pub datapacks: Vec<Addon>,
            #[serde(default)]
            pub custom_datapacks: Vec<CustomDatapack>,
            pub config_files: Vec<ConfigFile>,
        }
    }

    #[test]
    fn every_released_parser_rejects_a_release_folder_before_it_has_a_manifest() {
        let mut release = test_release("r1", 1);
        release.addons.push(test_addon(1, 10, "sodium-1.jar"));

        // What an old client downloads: the file published under the legacy name.
        let tombstone = serde_json::to_vec_pretty(&legacy_tombstone(&release)).unwrap();
        let error = serde_json::from_slice::<legacy_v2_1_0::Manifest>(&tombstone)
            .expect_err("the legacy parser must not produce a manifest")
            .to_string();

        // Serde quotes the offending string, which is the upgrade instruction.
        assert!(
            error.contains("This update needs CEMM 2.2.0 or newer"),
            "got: {error}"
        );
        // None of the substrings the 2.1.0 frontend rewrites into a generic
        // message may appear, or the instruction never reaches the user.
        let lowered = error.to_lowercase();
        for rewritten in [
            "network",
            "fetch",
            "connection",
            "timeout",
            "file not found",
            "no such file",
            "permission",
            "access denied",
            "json",
            "parse",
            "github",
            "repository",
            "auth",
            "token",
        ] {
            assert!(
                !lowered.contains(rewritten),
                "'{rewritten}' would trigger the old error rewrite: {error}"
            );
        }

        // And the release document itself is no better a manifest: had a client
        // fetched it by mistake, it must not read as an empty full update.
        let document = serde_json::to_vec(&release).unwrap();
        assert!(serde_json::from_slice::<legacy_v2_1_0::Manifest>(&document).is_err());
    }

    #[test]
    fn the_current_legacy_parser_rejects_the_tombstone_too() {
        let release = test_release("r1", 1);
        let tombstone = serde_json::to_vec(&legacy_tombstone(&release)).unwrap();
        assert!(
            serde_json::from_slice::<crate::composables::manifest::Manifest>(&tombstone).is_err()
        );
    }

    #[test]
    fn update_codes_split_into_a_path_and_an_optional_revision() {
        let revision = "0123456789abcdef0123456789abcdef01234567";

        assert_eq!(
            parse_update_code(&format!("  atmons-fusion/r2@{revision} ")).unwrap(),
            UpdateCode {
                path: "atmons-fusion/r2".to_string(),
                revision: Some(revision.to_string()),
            }
        );
        // Legacy codes are untouched: a bare id, or key/id.
        assert_eq!(
            parse_update_code("1723456789012").unwrap(),
            UpdateCode {
                path: "1723456789012".to_string(),
                revision: None
            }
        );
        assert_eq!(
            parse_update_code(r"my-pack\1723456789012").unwrap().path,
            "my-pack/1723456789012"
        );

        for bad in [
            "",
            "   ",
            "pack/r1@main",
            "pack/r1@abc123",
            "../x",
            "pack/../x@",
        ] {
            assert!(parse_update_code(bad).is_err(), "'{bad}' should be refused");
        }
    }

    fn inventory(files: &[(&str, &str)], addons: &[(u64, u64)]) -> Inventory {
        Inventory {
            files: files
                .iter()
                .map(|(path, content)| (path.to_string(), hash_of(content)))
                .collect(),
            addons: addons
                .iter()
                .map(|(project, file)| {
                    (
                        *project,
                        InventoryAddon {
                            file_id: *file,
                            name: format!("Addon {project}"),
                            file_name: format!("addon-{project}-{file}.jar"),
                            requires: Vec::new(),
                        },
                    )
                })
                .collect(),
        }
    }

    fn decision_for(report: &ThreeWayReport, key: &str) -> ThreeWayDecision {
        report
            .items
            .iter()
            .find(|item| item.key == key)
            .unwrap_or_else(|| panic!("no decision recorded for {key}"))
            .decision
    }

    #[test]
    fn the_three_way_matrix_takes_single_sided_changes_including_deletions() {
        let base = inventory(
            &[
                ("same.js", "v1"),
                ("upstream-edit.js", "v1"),
                ("upstream-delete.js", "v1"),
                ("custom-edit.js", "v1"),
                ("custom-delete.js", "v1"),
                ("both-same.js", "v1"),
            ],
            &[(1, 10), (2, 20), (3, 30), (4, 40)],
        );
        let custom = inventory(
            &[
                ("same.js", "v1"),
                ("upstream-edit.js", "v1"),
                ("upstream-delete.js", "v1"),
                ("custom-edit.js", "fusion"),
                ("both-same.js", "v2"),
                ("custom-add.js", "fusion"),
            ],
            // 2 upgraded by Fusion, 3 removed by Fusion, 9 added by Fusion.
            &[(1, 10), (2, 21), (4, 40), (9, 90)],
        );
        let upstream = inventory(
            &[
                ("same.js", "v1"),
                ("upstream-edit.js", "v2"),
                ("custom-edit.js", "v1"),
                ("custom-delete.js", "v1"),
                ("both-same.js", "v2"),
                ("upstream-add.js", "official"),
            ],
            // 1 upgraded upstream, 4 removed upstream, 8 added upstream.
            &[(1, 11), (2, 20), (3, 30), (8, 80)],
        );

        let report = three_way(&base, &custom, &upstream, &[]);

        assert_eq!(report.conflicts, 0, "{:#?}", report.items);
        assert_eq!(
            decision_for(&report, "upstream-edit.js"),
            ThreeWayDecision::TakeUpstream
        );
        assert_eq!(
            decision_for(&report, "upstream-delete.js"),
            ThreeWayDecision::TakeUpstream
        );
        assert_eq!(
            decision_for(&report, "upstream-add.js"),
            ThreeWayDecision::TakeUpstream
        );
        assert_eq!(
            decision_for(&report, "custom-edit.js"),
            ThreeWayDecision::KeepCustom
        );
        // An intentional Fusion deletion is kept deleted, not resurrected.
        assert_eq!(
            decision_for(&report, "custom-delete.js"),
            ThreeWayDecision::KeepCustom
        );
        assert_eq!(
            decision_for(&report, "custom-add.js"),
            ThreeWayDecision::KeepCustom
        );
        assert_eq!(
            decision_for(&report, "both-same.js"),
            ThreeWayDecision::SameResult
        );
        assert!(report.items.iter().all(|item| item.key != "same.js"));

        assert_eq!(
            decision_for(&report, "project 1"),
            ThreeWayDecision::TakeUpstream
        );
        assert_eq!(
            decision_for(&report, "project 2"),
            ThreeWayDecision::KeepCustom
        );
        assert_eq!(
            decision_for(&report, "project 3"),
            ThreeWayDecision::KeepCustom
        );
        assert_eq!(
            decision_for(&report, "project 4"),
            ThreeWayDecision::TakeUpstream
        );
        assert_eq!(
            decision_for(&report, "project 8"),
            ThreeWayDecision::TakeUpstream
        );
        assert_eq!(
            decision_for(&report, "project 9"),
            ThreeWayDecision::KeepCustom
        );
    }

    #[test]
    fn divergent_changes_are_conflicts_and_never_guessed() {
        let base = inventory(
            &[
                ("modify-modify.js", "v1"),
                ("delete-modify.js", "v1"),
                ("modify-delete.js", "v1"),
                ("art.png", "v1"),
            ],
            &[(1, 10)],
        );
        let custom = inventory(
            &[
                ("modify-modify.js", "fusion"),
                ("modify-delete.js", "fusion"),
                ("add-add.js", "fusion"),
                ("art.png", "fusion"),
            ],
            &[(1, 11)],
        );
        let upstream = inventory(
            &[
                ("modify-modify.js", "official"),
                ("delete-modify.js", "official"),
                ("add-add.js", "official"),
                ("art.png", "official"),
            ],
            &[(1, 12)],
        );

        let report = three_way(&base, &custom, &upstream, &[]);

        assert_eq!(report.conflicts, 6);
        let reason = |key: &str| {
            report
                .items
                .iter()
                .find(|item| item.key == key)
                .map(|item| item.reason.clone())
                .unwrap_or_default()
        };
        assert_eq!(reason("modify-modify.js"), "changed on both sides");
        assert_eq!(
            reason("delete-modify.js"),
            "deleted in the custom pack, changed upstream"
        );
        assert_eq!(
            reason("modify-delete.js"),
            "changed in the custom pack, deleted upstream"
        );
        assert_eq!(
            reason("add-add.js"),
            "added on both sides with different content"
        );
        assert_eq!(reason("art.png"), "binary file changed on both sides");
        // Conflicts lead the report, so they cannot be scrolled past.
        assert!(report.items[..6]
            .iter()
            .all(|item| item.decision == ThreeWayDecision::Conflict));
    }

    #[test]
    fn an_upstream_change_to_a_pinned_addon_needs_an_admin_decision() {
        // Only upstream moved it, which would normally be taken. The pin is the
        // custom pack's standing decision, so it is surfaced instead.
        let base = inventory(&[], &[(1530014, 8420412)]);
        let custom = inventory(&[], &[(1530014, 8420412)]);
        let upstream = inventory(&[], &[(1530014, 9000000)]);

        let pinned = three_way(&base, &custom, &upstream, &[1530014]);
        assert_eq!(
            decision_for(&pinned, "project 1530014"),
            ThreeWayDecision::Conflict
        );
        assert!(pinned.items[0].reason.contains("pinned"));

        let unpinned = three_way(&base, &custom, &upstream, &[]);
        assert_eq!(
            decision_for(&unpinned, "project 1530014"),
            ThreeWayDecision::TakeUpstream
        );
    }

    #[test]
    fn a_removed_library_and_an_absorbed_addon_are_flagged_for_review() {
        let base = inventory(&[], &[(100, 1)]);
        let mut custom = inventory(&[], &[(100, 1), (200, 2), (300, 3)]);
        // The custom addon 200 needs library 100.
        custom.addons.get_mut(&200).unwrap().requires = vec![100];
        // Upstream drops library 100 and now ships addon 300 itself.
        let upstream = inventory(&[], &[(300, 3)]);

        let report = three_way(&base, &custom, &upstream, &[]);

        assert_eq!(
            decision_for(&report, "project 100"),
            ThreeWayDecision::TakeUpstream
        );
        assert_eq!(
            decision_for(&report, "project 300"),
            ThreeWayDecision::SameResult
        );
        assert!(
            report
                .review
                .iter()
                .any(|note| note.contains("requires CurseForge project 100")),
            "got: {:?}",
            report.review
        );
        assert!(
            report
                .review
                .iter()
                .any(|note| note.contains("now includes 'Addon 300' itself")),
            "got: {:?}",
            report.review
        );
    }

    #[test]
    fn paths_differing_only_by_case_are_a_conflict() {
        let base = inventory(&[("lang/en_us.snbt", "v1")], &[]);
        let custom = inventory(&[("lang/en_us.snbt", "v1")], &[]);
        let upstream = inventory(&[("lang/en_us.snbt", "v1"), ("Lang/en_us.snbt", "v2")], &[]);

        let report = three_way(&base, &custom, &upstream, &[]);

        assert_eq!(report.conflicts, 2);
        assert!(report
            .items
            .iter()
            .all(|item| item.reason.contains("letter case")));
    }

    #[test]
    fn conflict_markers_are_recognised_only_as_a_complete_block() {
        assert!(has_conflict_markers(
            "a\n<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> upstream\nb\n"
        ));
        // A ruler of equals signs in a script is not a merge conflict.
        assert!(!has_conflict_markers("// =======\nconst a = 1\n=======\n"));
        assert!(!has_conflict_markers("<<<<<<< unfinished\nno separator\n"));
    }
}
