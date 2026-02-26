import { invoke } from '@tauri-apps/api/core'

import type { Addon, ConfigFileWithContent, Manifest, ManifestUpdateInfo, UpdateDiff } from '~/types'

export const useTauri = () =>
{
	const selectDirectory = async (): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('select_directory')
		}
		catch (error)
		{
			console.error('[useTauri] selectDirectory failed:', error)
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
			console.error('[useTauri] selectFile failed:', error)
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
			console.error('[useTauri] selectSaveFile failed:', error)
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
			console.error('[useTauri] selectMultipleFiles failed:', error)
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
			console.error('[useTauri] isBinaryFile failed:', { path, error })
			return false // Default to false if check fails
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
			console.error('[useTauri] readFile failed:', { path, error })
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
			console.error('[useTauri] writeFile failed:', { pathOrDir, error })
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
			console.error('[useTauri] parseMinecraftInstance failed:', { path, error })
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
			console.error('[useTauri] compareManifests failed:', error)
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
			console.error('[useTauri] openCurseforgeUrl failed:', { addonName, error })
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
			console.error('[useTauri] openUrl failed:', { url, error })
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

	const keyringTestDirect = async (): Promise<string> =>
	{
		return await invoke<string>('keyring_test_direct')
	}

	const keyringSetAndVerify = async (key: string, value: string): Promise<boolean> =>
	{
		return await invoke<boolean>('keyring_set_and_verify', { key, value })
	}
	const loadExistingManifest = async (modpackPath: string): Promise<Manifest | null> =>
	{
		try
		{
			const manifestPath = `${modpackPath}/cemm-manifest.json`
			const content = await readFile(manifestPath)
			if (content === null) return null
			return JSON.parse(content) as Manifest
		}
		catch (_error)
		{
			// No existing manifest found, this is a fresh install
			console.info('[useTauri] loadExistingManifest: No existing manifest found (fresh install)', { modpackPath })
			return null
		}
	}

	const calculateUpdateDiff = async (oldManifest: Manifest | null, newManifest: Manifest): Promise<UpdateDiff> =>
	{
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

	const downloadManifest = async (repo: string, uuid: string): Promise<Manifest | null> =>
	{
		try
		{
			return await invoke<Manifest>('download_manifest', { repo, uuid })
		}
		catch (error)
		{
			console.error('[useTauri] downloadManifest failed:', { repo, uuid, error })
			return null
		}
	}

	const downloadConfigFiles = async (repo: string, uuid: string): Promise<ConfigFileWithContent[]> =>
	{
		try
		{
			return await invoke<ConfigFileWithContent[]>('download_config_files', { repo, uuid })
		}
		catch (error)
		{
			console.error('[useTauri] downloadConfigFiles failed:', { repo, uuid, error })
			return []
		}
	}

	const selectConfigDirectory = async (): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('select_config_directory')
		}
		catch (error)
		{
			console.error('[useTauri] selectConfigDirectory failed:', error)
			return null
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
			console.error('[useTauri] readDirectoryRecursive failed:', { dirPath, basePath, error })
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
			console.error('[useTauri] validatePath failed:', { path, error })
			return {
				exists: false,
				original_path: path
			}
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
		keyringTestDirect,
		keyringSetAndVerify,
		loadExistingManifest,
		calculateUpdateDiff,
		downloadManifest,
		downloadConfigFiles,
		selectConfigDirectory,
		readDirectoryRecursive,
		validatePath
	}
}
