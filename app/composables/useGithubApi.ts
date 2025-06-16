// eslint-disable-next-line simple-import-sort/imports
import { invoke } from '@tauri-apps/api/core'

import { useCache } from './useCache'

import type { ConfigFileWithContent, Manifest } from '~/types'

export interface GithubProgress
{
	progress: number // 0-100
	message?: string
}

interface CachedGitHubData
{
	manifest: Manifest
	configFiles: ConfigFileWithContent[]
	uploadedAt?: number
	downloadedAt?: number
}

export const useGithubApi = () =>
{
	const cache = useCache<CachedGitHubData>('github', {
		ttl: 600000, // 10 minutes
		persistent: true,
		maxSize: 50
	})
	const logger = usePinoLogger()

	/**
	 * Uploads an update to GitHub. Accepts an options object for progress callback.
	 */
	const uploadUpdate = async (opts: {
		repo: string
		token: string
		uuid: string
		manifest: Manifest
		configFiles: ConfigFileWithContent[]
		onProgress?: (progress: number, message?: string) => void
	}): Promise<void> =>
	{
		const startTime = performance.now()

		if (typeof opts.onProgress === 'function') opts.onProgress(10, 'Preparing upload...')

		await invoke('upload_update', {
			repo: opts.repo,
			token: opts.token,
			uuid: opts.uuid,
			manifest: opts.manifest,
			configFiles: opts.configFiles
		})

		// Cache the uploaded manifest for potential re-use
		const cacheKey = `${opts.repo}-${opts.uuid}`
		cache.set(cacheKey, {
			manifest: opts.manifest,
			configFiles: opts.configFiles,
			uploadedAt: Date.now()
		})

		const duration = performance.now() - startTime
		logger.info('Upload completed', {
			repo: opts.repo,
			uuid: opts.uuid,
			duration: `${duration.toFixed(2)}ms`,
			manifestSize: JSON.stringify(opts.manifest).length,
			configFileCount: opts.configFiles.length
		})

		if (typeof opts.onProgress === 'function') opts.onProgress(100, 'Upload complete')
	}	/**
	 * Downloads an update from GitHub. Accepts an options object for progress callback.
	 */
	const downloadUpdate = async (opts: {
		repo: string
		uuid: string
		onProgress?: (progress: number, message?: string) => void
	}): Promise<{ manifest: Manifest, configFiles: ConfigFileWithContent[] }> =>
	{
		const cacheKey = `${opts.repo}-${opts.uuid}`
		const startTime = performance.now()

		// Check cache first
		const cached = cache.get(cacheKey)
		if (cached !== null)
		{
			logger.info('Using cached download', { repo: opts.repo, uuid: opts.uuid })
			if (typeof opts.onProgress === 'function') opts.onProgress(100, 'Using cached data')
			return {
				manifest: cached.manifest,
				configFiles: cached.configFiles
			}
		}

		if (typeof opts.onProgress === 'function') opts.onProgress(10, 'Contacting GitHub...')

		const result = await invoke('download_update', {
			repo: opts.repo,
			uuid: opts.uuid
		}) as { manifest: Manifest, config_files: ConfigFileWithContent[] }

		const downloadResult = {
			manifest: result.manifest,
			configFiles: result.config_files
		}

		// Cache the result
		cache.set(cacheKey, {
			manifest: result.manifest,
			configFiles: result.config_files,
			downloadedAt: Date.now()
		})

		const duration = performance.now() - startTime
		logger.info('Download completed', {
			repo: opts.repo,
			uuid: opts.uuid,
			duration: `${duration.toFixed(2)}ms`,
			manifestSize: JSON.stringify(result.manifest).length,
			configFileCount: result.config_files.length
		})

		if (typeof opts.onProgress === 'function') opts.onProgress(100, 'Download complete')
		return downloadResult
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
