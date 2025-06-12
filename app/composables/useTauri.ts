import { invoke } from '@tauri-apps/api/core'

import type { Manifest, UpdateInfo } from '~/types'

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

	const writeFile = async (path: string, content: string): Promise<boolean> =>
	{
		try
		{
			await invoke('write_file', { path, content })
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

	return {
		selectDirectory,
		selectFile,
		selectSaveFile,
		readFile,
		writeFile,
		parseMinecraftInstance,
		compareManifests,
		openCurseforgeUrl,
		openUrl
	}
}
