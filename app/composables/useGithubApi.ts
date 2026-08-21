import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type { ConfigFileWithContent, Manifest } from '~/types'

export interface GithubProgress
{
	progress: number // 0-100
	message?: string
}

export const useGithubApi = () =>
{
	const { $logger: logger } = useNuxtApp()
	const {
		uploadUpdate: invokeUploadUpdate,
		downloadManifest: invokeDownloadManifest,
		downloadConfigFiles: invokeDownloadConfigFiles
	} = useTauri()

	/**
	 * Uploads an update to GitHub. Accepts an options object for progress callback.
	 */
	const uploadUpdate = async (opts: {
		repo: string
		token: string
		uuid: string
		modpackKey?: string
		manifest: Manifest
		configFiles: ConfigFileWithContent[]
		onProgress?: (progress: number, message?: string) => void
	}): Promise<void> =>
	{
		const startTime = performance.now()
		const operationId = globalThis.crypto.randomUUID()
		let unlisten: UnlistenFn | undefined

		try
		{
			// Listen for progress events from the Rust backend
			unlisten = await listen<{ operationId: string, progress: number, message: string }>('upload_progress', (event) =>
			{
				if (event.payload.operationId !== operationId) return
				if (typeof opts.onProgress === 'function')
				{
					opts.onProgress(event.payload.progress, event.payload.message)
				}
			})

			await invokeUploadUpdate({
				operationId,
				repo: opts.repo,
				token: opts.token,
				uuid: opts.uuid,
				modpackKey: opts.modpackKey,
				manifest: opts.manifest,
				configFiles: opts.configFiles
			})

			const duration = performance.now() - startTime
			logger.info({
				repo: opts.repo,
				modpackKey: opts.modpackKey,
				uuid: opts.uuid,
				duration: `${duration.toFixed(2)}ms`,
				manifestSize: JSON.stringify(opts.manifest).length,
				configFileCount: opts.configFiles.length
			}, 'Upload completed')
		}
		finally
		{
			// Clean up the event listener
			if (unlisten !== undefined)
			{
				unlisten()
			}
		}
	}

	/**
	 * Downloads only the manifest from GitHub (phase 1 of two-phase update).
	 */
	const downloadManifest = async (opts: {
		repo: string
		uuid: string
		modpackKey?: string
		onProgress?: (progress: number, message?: string) => void
	}): Promise<Manifest> =>
	{
		if (typeof opts.onProgress === 'function') opts.onProgress(10, 'Downloading manifest...')
		const manifest = await invokeDownloadManifest(opts.repo, opts.uuid, opts.modpackKey)
		if (typeof opts.onProgress === 'function') opts.onProgress(100, 'Manifest downloaded')
		return manifest
	}

	/**
	 * Downloads config files from GitHub (phase 2 of two-phase update).
	 */
	const downloadConfigFiles = async (opts: {
		repo: string
		uuid: string
		modpackKey?: string
		manifest: Manifest
		onProgress?: (progress: number, message?: string) => void
	}): Promise<ConfigFileWithContent[]> =>
	{
		if (typeof opts.onProgress === 'function') opts.onProgress(10, 'Downloading config files...')
		const configFiles = await invokeDownloadConfigFiles(
			opts.repo,
			opts.uuid,
			opts.manifest,
			opts.modpackKey
		)
		if (typeof opts.onProgress === 'function') opts.onProgress(100, 'Config files downloaded')
		return configFiles
	}

	return { uploadUpdate, downloadManifest, downloadConfigFiles }
}
