import { invoke } from '@tauri-apps/api/core'

import type { Addon, CachedIcon, ConfigFileWithContent, Manifest, ManifestUpdateInfo, PackLibrary, UpdateDiff } from '~/types'

export const useTauri = () =>
{
	// drop_console strips every console.* call in production builds
	// (nuxt.config.ts), which previously meant every failure at this IPC
	// boundary vanished silently in release builds — the one place callers
	// most need a diagnostic trail (F-P2-18). $logger (pino) survives that.
	const { $logger: logger } = useNuxtApp()

	const selectDirectory = async (): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('select_directory')
		}
		catch (error)
		{
			logger.error({ error }, '[useTauri] selectDirectory failed')
			return null
		}
	}

	const selectFile = async (): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('select_file')
		}
		catch (error)
		{
			logger.error({ error }, '[useTauri] selectFile failed')
			return null
		}
	}

	const selectSaveFile = async (): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('select_save_file')
		}
		catch (error)
		{
			logger.error({ error }, '[useTauri] selectSaveFile failed')
			return null
		}
	}

	const selectMultipleFiles = async (): Promise<string[]> =>
	{
		try
		{
			return await invoke<string[]>('select_multiple_files')
		}
		catch (error)
		{
			logger.error({ error }, '[useTauri] selectMultipleFiles failed')
			return []
		}
	}

	const isBinaryFile = async (path: string): Promise<boolean> =>
	{
		try
		{
			return await invoke<boolean>('is_binary_file', { path })
		}
		catch (error)
		{
			logger.error({ path, error }, '[useTauri] isBinaryFile failed')
			return false
		}
	}

	const readFile = async (path: string): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('read_file', { path })
		}
		catch (error)
		{
			logger.error({ path, error }, '[useTauri] readFile failed')
			return null
		}
	}

	const writeFile = async (
		pathOrDir: string,
		contentOrFiles: string | Array<[string, string]>
	): Promise<boolean> =>
	{
		try
		{
			if (typeof contentOrFiles === 'string')
			{
				await invoke('write_file', { path: pathOrDir, content: contentOrFiles })
			}
			else
			{
				await invoke('write_file', { dir: pathOrDir, files: contentOrFiles })
			}
			return true
		}
		catch (error)
		{
			logger.error({ pathOrDir, error }, '[useTauri] writeFile failed')
			return false
		}
	}

	const parseMinecraftInstance = async (path: string): Promise<Manifest | null> =>
	{
		try
		{
			return await invoke<Manifest>('parse_minecraft_instance', { path })
		}
		catch (error)
		{
			logger.error({ path, error }, '[useTauri] parseMinecraftInstance failed')
			return null
		}
	}

	const compareManifests = async (oldManifest: Manifest, newManifest: Manifest): Promise<ManifestUpdateInfo | null> =>
	{
		try
		{
			return await invoke<ManifestUpdateInfo>('compare_manifests', { old: oldManifest, new: newManifest })
		}
		catch (error)
		{
			logger.error({ error }, '[useTauri] compareManifests failed')
			return null
		}
	}

	const openCurseforgeUrl = async (addonName: string): Promise<void> =>
	{
		try
		{
			await invoke('open_curseforge_url', { addonName })
		}
		catch (error)
		{
			logger.error({ addonName, error }, '[useTauri] openCurseforgeUrl failed')
		}
	}

	const openUrl = async (url: string): Promise<void> =>
	{
		try
		{
			await invoke('open_url', { url })
		}
		catch (error)
		{
			logger.error({ url, error }, '[useTauri] openUrl failed')
		}
	}

	const installUpdate = async (
		modpackPath: string,
		manifest: Manifest,
		configFiles: ConfigFileWithContent[],
		options?: {
			oldManifest?: Manifest | null
			cleanupOld?: boolean
		}
	): Promise<void> =>
	{
		return await invoke('install_update', {
			modpackPath,
			manifest,
			configFiles,
			options: options !== undefined
				? {
					old_manifest: options.oldManifest ?? null,
					cleanup_old: options.cleanupOld ?? (options.oldManifest !== null && options.oldManifest !== undefined)
				}
				: undefined
		})
	}

	const downloadManifest = async (repo: string, uuid: string, modpackKey?: string): Promise<Manifest | null> =>
	{
		try
		{
			return await invoke<Manifest>('download_manifest', { repo, uuid, modpackKey })
		}
		catch (error)
		{
			logger.error({ repo, uuid, modpackKey, error }, '[useTauri] downloadManifest failed')
			return null
		}
	}

	const downloadConfigFiles = async (repo: string, uuid: string, modpackKey?: string): Promise<ConfigFileWithContent[]> =>
	{
		try
		{
			return await invoke<ConfigFileWithContent[]>('download_config_files', { repo, uuid, modpackKey })
		}
		catch (error)
		{
			logger.error({ repo, uuid, modpackKey, error }, '[useTauri] downloadConfigFiles failed')
			return []
		}
	}

	const readDirectoryRecursive = async (dirPath: string, basePath: string): Promise<ConfigFileWithContent[]> =>
	{
		try
		{
			return await invoke<ConfigFileWithContent[]>('read_directory_recursive', { dirPath, basePath })
		}
		catch (error)
		{
			logger.error({ dirPath, basePath, error }, '[useTauri] readDirectoryRecursive failed')
			return []
		}
	}

	const validatePath = async (path: string): Promise<{
		exists: boolean
		is_directory?: boolean
		is_file?: boolean
		can_read?: boolean
		has_minecraft_instance?: boolean
		has_mods_folder?: boolean
		has_config_folder?: boolean
		is_likely_modpack?: boolean
		is_valid_config?: boolean
		extension?: string
		absolute_path?: string
		original_path: string
	}> =>
	{
		try
		{
			return await invoke('validate_path', { path })
		}
		catch (error)
		{
			logger.error({ path, error }, '[useTauri] validatePath failed')
			return {
				exists: false,
				original_path: path
			}
		}
	}

	/**
	 * Read the local CurseForge library.
	 *
	 * `instancesDir` overrides discovery; omit it to let Rust find the folder
	 * from CurseForge's own settings. Failure is returned rather than thrown so
	 * the library can show its own empty state — not finding CurseForge is an
	 * ordinary outcome on a machine that does not have it.
	 */
	const scanPackLibrary = async (instancesDir?: string | null): Promise<PackLibrary | null> =>
	{
		try
		{
			return await invoke<PackLibrary>('scan_pack_library', {
				instancesDir: instancesDir ?? null
			})
		}
		catch (error)
		{
			logger.error({ instancesDir, error }, '[useTauri] scanPackLibrary failed')
			return null
		}
	}

	/**
	 * Fetch pack artwork from CurseForge's CDN and keep it on disk.
	 *
	 * Separate from the scan on purpose: the library must open instantly and
	 * work offline, so it renders first and the pictures arrive afterwards.
	 * Returns an empty list on failure — no artwork is a cosmetic outcome, not
	 * something worth interrupting the user for.
	 */
	const cachePackIcons = async (urls: string[]): Promise<CachedIcon[]> =>
	{
		if (urls.length === 0) return []
		try
		{
			return await invoke<CachedIcon[]>('cache_pack_icons', { urls })
		}
		catch (error)
		{
			logger.error({ count: urls.length, error }, '[useTauri] cachePackIcons failed')
			return []
		}
	}

	return {
		selectDirectory,
		selectFile,
		selectSaveFile,
		selectMultipleFiles,
		readFile,
		writeFile,
		isBinaryFile,
		parseMinecraftInstance,
		compareManifests,
		openCurseforgeUrl,
		openUrl,
		installUpdate,
		downloadManifest,
		downloadConfigFiles,
		readDirectoryRecursive,
		validatePath,
		scanPackLibrary,
		cachePackIcons
	}
}

/**
 * Calculate the difference between two manifests.
 * Exported as a standalone function for use in components without composable overhead.
 */
export function calculateUpdateDiff(oldManifest: Manifest | null, newManifest: Manifest): UpdateDiff
{
	// Config-only updates never change addons. Key this behavior to the explicit
	// discriminator so a legitimate full update can still empty a category.
	if (newManifest.updateType === 'config')
	{
		return {
			removed_addons: [],
			updated_addon_ids: [],
			new_addons: []
		}
	}

	// If no old manifest, everything is new
	if (oldManifest === null)
	{
		return {
			removed_addons: [],
			updated_addon_ids: [],
			new_addons: [
				...newManifest.mods.map((addon) => addon.addon_name),
				...newManifest.resourcepacks.map((addon) => addon.addon_name),
				...newManifest.shaderpacks.map((addon) => addon.addon_name),
				...newManifest.datapacks.map((addon) => addon.addon_name)
			]
		}
	}

	const diff: UpdateDiff = {
		removed_addons: [],
		updated_addon_ids: [],
		new_addons: []
	}

	// Helper function to process addon categories
	const processCategory = (oldAddons: Addon[], newAddons: Addon[]) =>
	{
		// Find removed addons (in old but not in new)
		for (const oldAddon of oldAddons)
		{
			const stillExists = newAddons.some((newAddon) => newAddon.addon_project_id === oldAddon.addon_project_id)
			if (!stillExists)
			{
				diff.removed_addons.push(oldAddon.addon_name)
			}
		}

		// Find updated addons (same project ID, different version)
		// Store project_id for reliable matching during removal
		for (const oldAddon of oldAddons)
		{
			const newAddon = newAddons.find((addon) => addon.addon_project_id === oldAddon.addon_project_id)
			if (newAddon !== undefined && oldAddon.version !== newAddon.version)
			{
				diff.updated_addon_ids.push(oldAddon.addon_project_id)
			}
		}

		// Find new addons (in new but not in old)
		for (const newAddon of newAddons)
		{
			const isNew = !oldAddons.some((oldAddon) => oldAddon.addon_project_id === newAddon.addon_project_id)
			if (isNew)
			{
				diff.new_addons.push(newAddon.addon_name)
			}
		}
	}

	// Process each category
	processCategory(oldManifest.mods, newManifest.mods)
	processCategory(oldManifest.resourcepacks, newManifest.resourcepacks)
	processCategory(oldManifest.shaderpacks, newManifest.shaderpacks)
	processCategory(oldManifest.datapacks, newManifest.datapacks)

	return diff
}
