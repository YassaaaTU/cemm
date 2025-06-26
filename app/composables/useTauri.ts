import { invoke } from '@tauri-apps/api/core'

import type { Addon, ConfigFileWithContent, Manifest, UpdateDiff, UpdateInfo } from '~/types'

export const useTauri = () =>
{
	const selectDirectory = async (): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('select_directory')
		}
		catch (_e)
		{
			return null
		}
	}

	const selectFile = async (): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('select_file')
		}
		catch (_e)
		{
			return null
		}
	}

	const selectSaveFile = async (): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('select_save_file')
		}
		catch (_e)
		{
			return null
		}
	}

	const selectMultipleFiles = async (): Promise<string[]> =>
	{
		try
		{
			return await invoke<string[]>('select_multiple_files')
		}
		catch (_e)
		{
			return []
		}
	}

	const isBinaryFile = async (path: string): Promise<boolean> =>
	{
		try
		{
			return await invoke<boolean>('is_binary_file', { path })
		}
		catch (_e)
		{
			return false // Default to false if check fails
		}
	}

	const readFile = async (path: string): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('read_file', { path })
		}
		catch (_e)
		{
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
		catch (_e)
		{
			return false
		}
	}

	const parseMinecraftInstance = async (path: string): Promise<Manifest | null> =>
	{
		try
		{
			return await invoke<Manifest>('parse_minecraft_instance', { path })
		}
		catch (_e)
		{
			return null
		}
	}

	const compareManifests = async (oldManifest: Manifest, newManifest: Manifest): Promise<UpdateInfo | null> =>
	{
		try
		{
			return await invoke<UpdateInfo>('compare_manifests', { old: oldManifest, new: newManifest })
		}
		catch (_e)
		{
			return null
		}
	}

	const openCurseforgeUrl = async (addonName: string): Promise<void> =>
	{
		try
		{
			await invoke('open_curseforge_url', { addonName })
		}
		catch (_e)
		{
			// Optionally log error
		}
	}

	const openUrl = async (url: string): Promise<void> =>
	{
		try
		{
			await invoke('open_url', { url })
		}
		catch (_e)
		{
			// Optionally log error
		}
	}
	const installUpdate = async (
		modpackPath: string,
		manifest: Manifest,
		configFiles: ConfigFileWithContent[]
	): Promise<void> =>
	{
		return await invoke('install_update', {
			modpackPath,
			manifest,
			configFiles })
	}

	const installUpdateWithCleanup = async (
		modpackPath: string,
		oldManifest: Manifest | null,
		newManifest: Manifest,
		configFiles: ConfigFileWithContent[]
	): Promise<void> =>
	{
		return await invoke('install_update_with_cleanup', {
			modpackPath,
			oldManifest,
			newManifest,
			configFiles
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
			const manifestPath = `${modpackPath}/manifest.json`
			const content = await readFile(manifestPath)
			if (content === null) return null
			return JSON.parse(content) as Manifest
		}
		catch (_e)
		{
			// No existing manifest found, this is a fresh install
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
				updated_addons: [],
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
			updated_addons: [],
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
			for (const oldAddon of oldAddons)
			{
				const newAddon = newAddons.find((addon) => addon.addon_project_id === oldAddon.addon_project_id)
				if (newAddon !== undefined && oldAddon.version !== newAddon.version)
				{
					diff.updated_addons.push([oldAddon.version, newAddon.version])
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
		catch (_e)
		{
			return null
		}
	}

	const downloadConfigFiles = async (repo: string, uuid: string): Promise<ConfigFileWithContent[]> =>
	{
		try
		{
			return await invoke<ConfigFileWithContent[]>('download_config_files', { repo, uuid })
		}
		catch (_e)
		{
			return []
		}
	}

	const selectConfigDirectory = async (): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('select_config_directory')
		}
		catch (_e)
		{
			return null
		}
	}

	const readDirectoryRecursive = async (dirPath: string, basePath: string): Promise<ConfigFileWithContent[]> =>
	{
		try
		{
			return await invoke<ConfigFileWithContent[]>('read_directory_recursive', { dirPath, basePath })
		}
		catch (_e)
		{
			return []
		}
	}

	const installUpdateOptimized = async (
		modpackPath: string,
		oldManifest: Manifest | null,
		newManifest: Manifest,
		configFiles: ConfigFileWithContent[]
	): Promise<void> =>
	{
		return await invoke('install_update_optimized', {
			modpackPath,
			oldManifest,
			newManifest,
			configFiles
		})
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
		installUpdateWithCleanup,
		installUpdateOptimized,
		keyringTestDirect,
		keyringSetAndVerify,
		loadExistingManifest,
		calculateUpdateDiff,
		downloadManifest,
		downloadConfigFiles,
		selectConfigDirectory,
		readDirectoryRecursive
	}
}
