import { invoke } from '@tauri-apps/api/core'

import type { Manifest } from '~/types'

export interface ConfigFile
{
	path: string
	content: string
}

export interface GithubProgress
{
	progress: number // 0-100
	message?: string
}

export const useGithubApi = () =>
{
	/**
	 * Uploads an update to GitHub. Accepts an options object for progress callback.
	 */
	const uploadUpdate = async (opts: {
		repo: string
		token: string
		uuid: string
		manifest: Manifest
		configFiles: ConfigFile[]
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
	 */
	const downloadUpdate = async (opts: {
		repo: string
		uuid: string
		onProgress?: (progress: number, message?: string) => void
	}): Promise<{ manifest: Manifest, configFiles: ConfigFile[] }> =>
	{
		if (typeof opts.onProgress === 'function') opts.onProgress(10, 'Contacting GitHub...')
		const result = await invoke('download_update', {
			repo: opts.repo,
			uuid: opts.uuid
		}) as { manifest: Manifest, config_files: ConfigFile[] }
		if (typeof opts.onProgress === 'function') opts.onProgress(100, 'Download complete')
		return {
			manifest: result.manifest,
			configFiles: result.config_files
		}
	}

	return { uploadUpdate, downloadUpdate }
}
