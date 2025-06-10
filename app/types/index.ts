export interface Addon
{
	addon_file_id: number
	addon_name: string
	addon_project_id: number
	cdn_download_url: string
	mod_folder_path: string
	version: string
}

export interface Manifest
{
	mods: Addon[]
	resourcepacks: Addon[]
	shaderpacks: Addon[]
}

export interface UpdateInfo
{
	uuid: string
	timestamp: string
	addedAddons: Addon[]
	removedAddons: string[]
	configFiles: string[]
}
