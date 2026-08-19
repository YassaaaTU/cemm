export interface Addon
{
	addon_file_id: number
	addon_name: string
	addon_project_id: number
	cdn_download_url: string
	mod_folder_path: string
	version: string
	thumbnailUrl?: string // optional, for UI only
	webSiteURL?: string // optional, CurseForge or homepage URL (always preserved)
	disabled?: boolean // optional, true if .disabled file detected
	fileNameOnDisk: string // exact filename on disk for reliable removal
}

/**
 * Configuration file metadata without content.
 *
 * This type is mirrored in multiple locations across the codebase:
 * - TypeScript: app/types/index.ts (this file)
 * - Rust: src-tauri/src/installer.rs (ConfigFile struct with content)
 * - Rust: src-tauri/src/composables/github.rs (ConfigFileWithContent struct)
 *
 * When modifying this type, ensure all definitions remain consistent.
 */
export interface ConfigFile
{
	filename: string
	relative_path: string
}

/**
 * Configuration file with content for upload/download operations.
 *
 * Extends ConfigFile with file content and binary flag.
 *
 * This type is mirrored in:
 * - TypeScript: app/types/index.ts (this file)
 * - Rust: src-tauri/src/composables/github.rs (ConfigFileWithContent struct)
 *
 * When modifying this type, ensure all definitions remain consistent.
 */
export interface ConfigFileWithContent extends ConfigFile
{
	content: string
	is_binary?: boolean // true if this is a binary file (content will be base64 data URI)
}

export interface Manifest
{
	updateType?: 'full' | 'config' // 'full' = addons + config, 'config' = config only
	mods: Addon[]
	resourcepacks: Addon[]
	shaderpacks: Addon[]
	datapacks: Addon[]
	config_files: ConfigFile[]
}

export interface ManifestUpdateInfo
{
	uuid: string
	timestamp: string
	addedAddons: Addon[]
	removedAddons: string[]
	updatedAddonIds: number[]
}

/**
 * Represents the difference between two manifest versions during an update.
 *
 * This type is mirrored in:
 * - TypeScript: app/types/index.ts (this file)
 * - Rust: src-tauri/src/installer.rs (UpdateDiff struct)
 *
 * When modifying this type, ensure all definitions remain consistent.
 */
export interface UpdateDiff
{
	removed_addons: string[] // addon names to remove
	updated_addon_ids: number[] // project IDs of addons that were updated (matched by project_id for reliability)
	new_addons: string[] // completely new addon names
}

/**
 * One of CurseForge's instance groups.
 *
 * Mirrored in Rust: src-tauri/src/composables/instances.rs (PackGroup).
 */
export interface PackGroup
{
	id: string
	name: string
}

/**
 * A modpack as the library lists it — enough to recognise and choose one, and
 * deliberately not a Manifest. Loading a pack for real still goes through
 * parse_minecraft_instance.
 *
 * Mirrored in Rust: src-tauri/src/composables/instances.rs (PackSummary).
 */
export interface PackSummary
{
	/** Folder holding minecraftinstance.json. The identity key everywhere. */
	instancePath: string
	instanceFile: string
	/**
	 * The directory's own name, which is not always the pack's: a real library
	 * holds a folder called `All the Mods 10 - ATM10 (2)` containing a pack
	 * named `Aeronautics`. Both are shown so a card is never ambiguous.
	 */
	folderName: string
	name: string
	gameVersion: string | null
	/** Loader family only — `NeoForge`, not `neoforge-21.1.228`. */
	loader: string | null
	groupId: string | null
	addonCount: number
	/** RFC 3339. CurseForge writes year 0001 for "never played". */
	lastPlayed: string | null
	playedCount: number
	/** A `data:` URI, or null. Never a remote URL. */
	icon: string | null
	/**
	 * The pack's artwork on CurseForge's CDN, for a pack installed from there
	 * whose image is not cached yet. Fetched separately from the scan so the
	 * library still opens instantly and offline.
	 */
	iconUrl: string | null
	projectId: number | null
}

/**
 * Mirrored in Rust: src-tauri/src/composables/instances.rs (CachedIcon).
 */
export interface CachedIcon
{
	url: string
	/** A `data:` URI, or null when the fetch failed. */
	icon: string | null
}

/**
 * Mirrored in Rust: src-tauri/src/composables/instances.rs (PackLibrary).
 */
export interface PackLibrary
{
	instancesDir: string | null
	/**
	 * `curseforge` when found from CurseForge's own settings, `manual` when the
	 * folder was supplied, `none` when there was nothing to scan. "You have no
	 * packs" and "I could not find CurseForge" are different problems.
	 */
	source: 'curseforge' | 'manual' | 'none'
	packs: PackSummary[]
	groups: PackGroup[]
	/** Instances that could not be read. One bad pack must not cost the rest. */
	warnings: string[]
}
