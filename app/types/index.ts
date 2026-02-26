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
	configFiles: string[]
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

export interface UpdatePreview
{
	oldManifest: Manifest | null
	newManifest: Manifest
	diff: UpdateDiff
	hasChanges: boolean
	configFiles?: ConfigFileWithContent[]
}
