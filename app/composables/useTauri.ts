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
		configFiles: ConfigFile[]
	): Promise<void> =>
	{
		return await invoke('install_update', {
			modpackPath,
			manifest,
			configFiles
		})
	}
	const installUpdateWithCleanup = async (
		modpackPath: string,
		oldManifest: Manifest | null,
		newManifest: Manifest,
		configFiles: ConfigFile[]
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

	return {
		selectDirectory,
		selectFile,
		selectSaveFile,
		readFile,
		writeFile,
		parseMinecraftInstance,
		compareManifests,
		openCurseforgeUrl,
		openUrl,
		installUpdate,
		installUpdateWithCleanup,
		keyringTestDirect,
		keyringSetAndVerify
	}
}
