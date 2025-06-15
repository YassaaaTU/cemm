import { invoke } from '@tauri-apps/api/core'

import type { ConfigFileWithContent, Manifest } from '~/types'

export interface GithubProgress
{
	progress: number // 0-100
	message?: string
}

export const useGithubApi = () =>
{
	/**
	 * Uploads an update to GitHub. Accepts an options object for progress callback.
	 */	const uploadUpdate = async (opts: {
		repo: string
		token: string
		uuid: string
		manifest: Manifest
		configFiles: ConfigFileWithContent[]
		onProgress?: (progress: number, message?: string) => void
	}): Promise<void> =>
	{
		if (typeof opts.onProgress === 'function') opts.onProgress(10, 'Preparing upload...')
		await invoke('upload_update', {
			repo: opts.repo,
			token: opts.token,
			uuid: opts.uuid,
			manifest: opts.manifest,
			configFiles: opts.configFiles
		})
		if (typeof opts.onProgress === 'function') opts.onProgress(100, 'Upload complete')
	}
	/**
	 * Downloads an update from GitHub. Accepts an options object for progress callback.
	 */	const downloadUpdate = async (opts: {
		repo: string
		uuid: string
		onProgress?: (progress: number, message?: string) => void
	}): Promise<{ manifest: Manifest, configFiles: ConfigFileWithContent[] }> =>
	{
		if (typeof opts.onProgress === 'function') opts.onProgress(10, 'Contacting GitHub...')
		const result = await invoke('download_update', {
			repo: opts.repo,
			uuid: opts.uuid
		}) as { manifest: Manifest, config_files: ConfigFileWithContent[] }
		if (typeof opts.onProgress === 'function') opts.onProgress(100, 'Download complete')
		return {
			manifest: result.manifest,
			configFiles: result.config_files
		}
	}
	/**
	 * Downloads only the manifest from GitHub (phase 1 of two-phase update).
	 */
	const downloadManifest = async (opts: {
		repo: string
		uuid: string
		onProgress?: (progress: number, message?: string) => void
	}): Promise<Manifest> =>
	{
		if (typeof opts.onProgress === 'function') opts.onProgress(10, 'Downloading manifest...')
		const manifest = await invoke<Manifest>('download_manifest', {
			repo: opts.repo,
			uuid: opts.uuid
		})
		if (typeof opts.onProgress === 'function') opts.onProgress(100, 'Manifest downloaded')
		return manifest
	}
	/**
	 * Downloads config files from GitHub (phase 2 of two-phase update).
	 */
	const downloadConfigFiles = async (opts: {
		repo: string
		uuid: string
		manifest: Manifest
		onProgress?: (progress: number, message?: string) => void
	}): Promise<ConfigFileWithContent[]> =>
	{
		if (typeof opts.onProgress === 'function') opts.onProgress(10, 'Downloading config files...')
		const configFiles = await invoke<ConfigFileWithContent[]>('download_config_files', {
			repo: opts.repo,
			uuid: opts.uuid,
			manifest: opts.manifest
		})
		if (typeof opts.onProgress === 'function') opts.onProgress(100, 'Config files downloaded')
		return configFiles
	}

	return { uploadUpdate, downloadUpdate, downloadManifest, downloadConfigFiles }
}
