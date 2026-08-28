use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct Addon {
    #[ts(type = "number")]
    pub addon_file_id: u64,
    pub addon_name: String,
    #[ts(type = "number")]
    pub addon_project_id: u64,
    pub cdn_download_url: String,
    pub mod_folder_path: String,
    pub version: String,
    #[serde(rename = "webSiteURL")]
    #[ts(optional = nullable)]
    pub web_site_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub disabled: Option<bool>,
    #[serde(rename = "fileNameOnDisk")]
    pub file_name_on_disk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct ConfigFile {
    pub filename: String,
    pub relative_path: String,
}

/// Marks config content that is bytes rather than text.
///
/// A config file's content is one JSON string either way, so binary payloads
/// travel base64-encoded behind this prefix. Declared once because the encoder,
/// every decoder and the tests all have to agree on it character for character;
/// it used to be written out as a literal in six places across two modules.
pub const BINARY_CONTENT_PREFIX: &str = "data:application/octet-stream;base64,";

/// A config file carrying its contents, for upload, download and installation.
///
/// `ConfigFile` above is the manifest's own record of which config files an
/// update covers, and deliberately holds no content. This type is what moves
/// over the wire; the two used to have a third sibling in `installer.rs` that
/// was identical except for a missing `is_binary`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct ConfigFileWithContent {
    pub filename: String,
    pub relative_path: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub is_binary: Option<bool>,
}

impl ConfigFileWithContent {
    /// Whether `content` holds base64-encoded bytes rather than text.
    ///
    /// Reads the payload rather than trusting the `is_binary` flag: the flag is
    /// set by the admin side at capture time and is absent on anything a
    /// download produced, whereas the prefix is always present when it matters.
    pub fn has_binary_content(&self) -> bool {
        self.content.starts_with(BINARY_CONTENT_PREFIX)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct Manifest {
    #[serde(rename = "updateType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    // The discriminator install_update keys config-only behaviour to, so the
    // generated binding narrows it rather than exposing a bare string.
    #[ts(type = "'full' | 'config'", optional)]
    pub update_type: Option<String>,
    pub mods: Vec<Addon>,
    pub resourcepacks: Vec<Addon>,
    pub shaderpacks: Vec<Addon>,
    pub datapacks: Vec<Addon>,
    pub config_files: Vec<ConfigFile>,
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

/// In manifest order, which is the order every list in the UI is built in.
const CATEGORIES: [Category; 4] = [
    Category::Mods,
    Category::ResourcePacks,
    Category::ShaderPacks,
    Category::DataPacks,
];

impl Category {
    /// The directory under the modpack root this category installs into.
    const fn directory(self) -> &'static str {
        match self {
            Self::Mods => "mods",
            Self::ResourcePacks => "resourcepacks",
            Self::ShaderPacks => "shaderpacks",
            Self::DataPacks => "datapacks",
        }
    }

    fn addons(self, manifest: &Manifest) -> &Vec<Addon> {
        match self {
            Self::Mods => &manifest.mods,
            Self::ResourcePacks => &manifest.resourcepacks,
            Self::ShaderPacks => &manifest.shaderpacks,
            Self::DataPacks => &manifest.datapacks,
        }
    }

    fn addons_mut(self, manifest: &mut Manifest) -> &mut Vec<Addon> {
        match self {
            Self::Mods => &mut manifest.mods,
            Self::ResourcePacks => &mut manifest.resourcepacks,
            Self::ShaderPacks => &mut manifest.shaderpacks,
            Self::DataPacks => &mut manifest.datapacks,
        }
    }
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

/// The pack's actual contents, reconciled from both records CEMM keeps of them,
/// plus which of those contents CEMM did not put there itself.
///
/// Neither record is trustworthy alone, and they go stale in opposite
/// directions. `cemm-manifest.json` is what CEMM last installed, so it goes
/// stale the moment the pack is changed through CurseForge — which is the
/// admin's entire workflow, and a normal player's too. `minecraftinstance.json`
/// is CurseForge's inventory, so it goes stale the moment CEMM installs
/// anything, because CEMM writes jars into `mods/` without telling CurseForge.
/// Preferring either one outright produced a diff describing a pack nobody has:
/// deletions for files that are already gone, and "new" rows for addons sitting
/// on disk the whole time.
///
/// Disk settles it. An entry survives into the baseline only if its file is
/// really there, and an addon CurseForge knows about is folded in when CEMM's
/// own record misses it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "generated/")]
pub struct InstallBaseline {
    /// What is installed right now, as far as CEMM can tell. The diff runs
    /// against this.
    pub manifest: Manifest,
    /// Project IDs in `manifest` that CEMM did not install: present on disk and
    /// known to CurseForge, but absent from `cemm-manifest.json`. They belong in
    /// the baseline — an update that ships one of them is not installing it
    /// anew — but they are not CEMM's to delete without being asked, so the
    /// preview lists them apart from the removals it performs by default.
    #[ts(type = "number[]")]
    pub unmanaged_addon_ids: Vec<u64>,
}

/// Whether an addon's file is actually in the pack.
///
/// Both spellings count. An addon switched off in CurseForge sits on disk as
/// `X.jar.disabled` while every manifest still calls it `X.jar`, which is the
/// same reason `collect_old_file_paths` sweeps both names.
fn addon_file_present(modpack_path: &Path, category: Category, file_name: &str) -> bool {
    let directory = modpack_path.join(category.directory());
    directory.join(file_name).exists() || directory.join(format!("{file_name}.disabled")).exists()
}

/// Reads and parses `cemm-manifest.json`, distinguishing "not there" from
/// "unreadable". A pack CEMM has never installed into legitimately has no
/// manifest; one whose manifest cannot be parsed is a problem the caller has to
/// hear about rather than silently treat as a fresh install.
fn read_installed_manifest(path: &Path) -> Result<Option<Manifest>, String> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Failed to read the installed manifest at {}: {error}",
                path.display()
            ))
        }
    };

    serde_json::from_str(&content)
        .map(Some)
        .map_err(|error| format!("The installed cemm-manifest.json is not valid: {error}"))
}

/// Builds the baseline an incoming update is diffed against. See
/// [`InstallBaseline`] for why it is assembled from two records and disk rather
/// than read from one file.
///
/// `Ok(None)` means CEMM has nothing to go on — neither record exists — which is
/// a genuinely fresh install, not an empty pack.
pub fn resolve_install_baseline(modpack_path: String) -> Result<Option<InstallBaseline>, String> {
    let root = Path::new(&modpack_path);
    log::info!("resolve_install_baseline: reconciling {modpack_path}");

    let installed = read_installed_manifest(&root.join(crate::installer::INSTALLED_MANIFEST_FILE))?;

    let instance_path = root.join("minecraftinstance.json");
    let instance = if instance_path.exists() {
        Some(parse_minecraft_instance(
            instance_path.to_string_lossy().into_owned(),
        )?)
    } else {
        None
    };

    if installed.is_none() && instance.is_none() {
        log::info!("resolve_install_baseline: no CEMM or CurseForge record, treating as fresh");
        return Ok(None);
    }

    let mut manifest = Manifest {
        // A baseline is a state snapshot, not an update of either kind.
        update_type: None,
        mods: Vec::new(),
        resourcepacks: Vec::new(),
        shaderpacks: Vec::new(),
        datapacks: Vec::new(),
        config_files: installed
            .as_ref()
            .map(|installed| installed.config_files.clone())
            .unwrap_or_default(),
    };
    let mut unmanaged_addon_ids = Vec::new();
    let mut pruned = 0usize;

    for category in CATEGORIES {
        let mut present: Vec<Addon> = Vec::new();

        if let Some(installed) = installed.as_ref() {
            for addon in category.addons(installed) {
                if addon_file_present(root, category, &addon.file_name_on_disk) {
                    present.push(addon.clone());
                } else {
                    pruned += 1;
                }
            }
        }

        if let Some(instance) = instance.as_ref() {
            for addon in category.addons(instance) {
                let already_known = present
                    .iter()
                    .any(|known| known.addon_project_id == addon.addon_project_id);
                if already_known || !addon_file_present(root, category, &addon.file_name_on_disk) {
                    continue;
                }
                unmanaged_addon_ids.push(addon.addon_project_id);
                present.push(addon.clone());
            }
        }

        *category.addons_mut(&mut manifest) = present;
    }

    log::info!(
        "resolve_install_baseline: {} addons on disk ({} of them not installed by CEMM), {pruned} manifest entries dropped as missing",
        CATEGORIES
            .iter()
            .map(|category| category.addons(&manifest).len())
            .sum::<usize>(),
        unmanaged_addon_ids.len()
    );

    Ok(Some(InstallBaseline {
        manifest,
        unmanaged_addon_ids,
    }))
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

    /// The values this is actually fed: `webSiteURL` straight out of
    /// `minecraftinstance.json`, for each of the four categories the addon
    /// table renders. The allowlist is on the host alone, so a resourcepack's
    /// `/texture-packs/` path must pass exactly as a mod's `/mc-mods/` does --
    /// which is the whole reason the name-slugging route that hardcoded
    /// `/mc-mods/` was replaced by this one.
    #[test]
    fn open_url_accepts_real_website_urls_from_every_addon_category() {
        for url in [
            "https://www.curseforge.com/minecraft/mc-mods/jei",
            "https://www.curseforge.com/minecraft/texture-packs/faithful-32x",
            "https://www.curseforge.com/minecraft/shaders/complementary-shaders",
            "https://www.curseforge.com/minecraft/data-packs/terralith",
            "https://www.curseforge.com/minecraft/mc-mods/some-mod?page=files",
        ] {
            assert!(
                validate_open_url(url).is_ok(),
                "expected '{url}' to be accepted"
            );
        }
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

    /// A pack directory holding one CurseForge instance record, whatever files
    /// `on_disk` names under `mods/`, and optionally a `cemm-manifest.json`.
    fn write_pack(
        addons: serde_json::Value,
        installed: Option<&Manifest>,
        on_disk: &[&str],
    ) -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let body = serde_json::json!({ "installedAddons": addons });
        fs::write(
            temp.path().join("minecraftinstance.json"),
            serde_json::to_string(&body).expect("serialize"),
        )
        .expect("write instance");

        let mods_dir = temp.path().join("mods");
        fs::create_dir_all(&mods_dir).expect("mods dir");
        for file in on_disk {
            fs::write(mods_dir.join(file), b"").expect("plant file");
        }

        if let Some(installed) = installed {
            fs::write(
                temp.path().join("cemm-manifest.json"),
                serde_json::to_string(installed).expect("serialize"),
            )
            .expect("write installed manifest");
        }

        temp
    }

    fn baseline_of(pack: &tempfile::TempDir) -> InstallBaseline {
        resolve_install_baseline(pack.path().to_string_lossy().into_owned())
            .expect("baseline should resolve")
            .expect("pack has records, so a baseline exists")
    }

    fn installed_manifest_of(mods: Vec<Addon>) -> Manifest {
        Manifest {
            update_type: Some("full".to_string()),
            mods,
            resourcepacks: Vec::new(),
            shaderpacks: Vec::new(),
            datapacks: Vec::new(),
            config_files: Vec::new(),
        }
    }

    fn manifest_addon(project_id: u64, name: &str, file_name: &str) -> Addon {
        Addon {
            addon_file_id: project_id,
            addon_name: name.to_string(),
            addon_project_id: project_id,
            cdn_download_url: format!("https://edge.forgecdn.net/{file_name}"),
            mod_folder_path: "mods".to_string(),
            version: file_name.to_string(),
            web_site_url: None,
            disabled: None,
            file_name_on_disk: file_name.to_string(),
        }
    }

    fn instance_addon(project_id: u64, name: &str, file_name: &str) -> serde_json::Value {
        serde_json::json!({
            "addonID": project_id,
            "name": name,
            "modFolderPath": r"D:\Games\Instances\Pack\mods",
            "categorySection": { "name": "Mods" },
            "installedFile": {
                "id": project_id,
                "fileName": file_name,
                "downloadUrl": format!("https://edge.forgecdn.net/{file_name}"),
            },
        })
    }

    #[test]
    fn baseline_drops_addons_whose_files_are_gone() {
        // The admin's own workflow: CEMM installed three mods, then the pack was
        // edited through CurseForge and Lithium was removed. Trusting
        // cemm-manifest.json alone had the next update offer to delete a file
        // that is not there, which is what "removing mods no longer present"
        // looked like on screen.
        let pack = write_pack(
            serde_json::json!([]),
            Some(&installed_manifest_of(vec![
                manifest_addon(1, "Sodium", "sodium-1.jar"),
                manifest_addon(2, "Lithium", "lithium-1.jar"),
            ])),
            &["sodium-1.jar"],
        );

        let baseline = baseline_of(&pack);

        assert_eq!(baseline.manifest.mods.len(), 1);
        assert_eq!(baseline.manifest.mods[0].addon_name, "Sodium");
        assert!(baseline.unmanaged_addon_ids.is_empty());
    }

    #[test]
    fn baseline_keeps_an_addon_curseforge_has_switched_off() {
        // Its file is `X.jar.disabled` while every manifest still calls it
        // `X.jar`. Checking only the canonical name pruned every disabled addon
        // out of the baseline and then reinstalled it as new.
        let pack = write_pack(
            serde_json::json!([]),
            Some(&installed_manifest_of(vec![manifest_addon(
                1,
                "Sodium",
                "sodium-1.jar",
            )])),
            &["sodium-1.jar.disabled"],
        );

        assert_eq!(baseline_of(&pack).manifest.mods.len(), 1);
    }

    #[test]
    fn baseline_folds_in_addons_curseforge_installed_and_marks_them_unmanaged() {
        // The other half of the same drift: a mod added through CurseForge is on
        // disk but absent from cemm-manifest.json. Left out of the baseline it
        // came back as "new" in a preview of an update that already contains it.
        let pack = write_pack(
            serde_json::json!([
                instance_addon(1, "Sodium", "sodium-1.jar"),
                instance_addon(9, "Iris", "iris-1.jar"),
            ]),
            Some(&installed_manifest_of(vec![manifest_addon(
                1,
                "Sodium",
                "sodium-1.jar",
            )])),
            &["sodium-1.jar", "iris-1.jar"],
        );

        let baseline = baseline_of(&pack);

        assert_eq!(baseline.manifest.mods.len(), 2);
        // CEMM installed Sodium, so it stays CEMM's to remove; Iris does not.
        assert_eq!(baseline.unmanaged_addon_ids, vec![9]);
    }

    #[test]
    fn a_pack_curseforge_lists_but_has_not_written_yet_contributes_nothing() {
        let pack = write_pack(
            serde_json::json!([instance_addon(1, "Sodium", "sodium-1.jar")]),
            None,
            &[],
        );

        let baseline = baseline_of(&pack);

        assert!(baseline.manifest.mods.is_empty());
        assert!(baseline.unmanaged_addon_ids.is_empty());
    }

    #[test]
    fn every_addon_of_a_pack_cemm_has_never_touched_is_unmanaged() {
        // Nothing here was installed by CEMM, so a first install deletes none of
        // it by default -- including addons the admin deliberately excluded from
        // the upload, which the old baseline swept away silently.
        let pack = write_pack(
            serde_json::json!([
                instance_addon(1, "Sodium", "sodium-1.jar"),
                instance_addon(2, "Lithium", "lithium-1.jar"),
            ]),
            None,
            &["sodium-1.jar", "lithium-1.jar"],
        );

        let baseline = baseline_of(&pack);

        assert_eq!(baseline.manifest.mods.len(), 2);
        assert_eq!(baseline.unmanaged_addon_ids, vec![1, 2]);
    }

    #[test]
    fn a_folder_with_neither_record_has_no_baseline_at_all() {
        let temp = tempfile::tempdir().expect("temp dir");

        assert!(
            resolve_install_baseline(temp.path().to_string_lossy().into_owned())
                .expect("resolving should succeed")
                .is_none()
        );
    }

    #[test]
    fn an_unreadable_installed_manifest_is_reported_rather_than_ignored() {
        let temp = tempfile::tempdir().expect("temp dir");
        fs::write(temp.path().join("cemm-manifest.json"), "{ not json").expect("write");

        let error = resolve_install_baseline(temp.path().to_string_lossy().into_owned())
            .expect_err("a corrupt manifest must not pass as a fresh install");

        assert!(error.contains("cemm-manifest.json"), "got: {error}");
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
