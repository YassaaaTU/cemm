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

export interface ConfigFile
{
	filename: string
	relative_path: string
}

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

export interface UpdateInfo
{
	uuid: string
	timestamp: string
	addedAddons: Addon[]
	removedAddons: string[]
	configFiles: string[]
}

export interface UpdateDiff
{
	removed_addons: string[] // addon names to remove
	updated_addons: Array<[string, string]> // [old_version, new_version] pairs
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
