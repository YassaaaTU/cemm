import type { ConfigFileWithContent, Manifest } from '~/types'
import { getErrorMessage, withNetworkRetry } from '~/utils/errorHandler'
import { resolveModpackKey } from '~/utils/modpackKey'

/**
 * Composable for user-specific API operations.
 * Extracts business logic from UserPanel.vue for better maintainability.
 */
export function useUserApi()
{
	const { downloadManifest, downloadConfigFiles: apiDownloadConfigFiles } = useGithubApi()
	const appStore = useAppStore()
	const manifestStore = useManifestStore()
	const { readFile, parseMinecraftInstance, installUpdate: installUpdateTauri } = useTauri()
	const { $logger: logger } = useNuxtApp()

	/**
   * Download manifest from GitHub
   */
	async function downloadFromGithub(
		updateInput: string,
		onProgress: (progress: number, message?: string) => void,
		setStatus: (message: string, type: 'success' | 'error' | 'info' | 'warning') => void
	): Promise<{ success: boolean, manifest?: Manifest }>
	{
		const repo = appStore.githubRepo
		if (repo.trim().length === 0)
		{
			setStatus('Please configure your GitHub repository in settings.', 'error')
			return { success: false }
		}

		try
		{
			const resolvedQuery = await resolveUpdateQuery(updateInput)

			onProgress(10, 'Downloading manifest...')

			const downloadedManifest = await withNetworkRetry(
				async () => await downloadManifest({
					repo,
					uuid: resolvedQuery.updateId,
					modpackKey: resolvedQuery.modpackKey,
					onProgress: (p, msg) =>
					{
						onProgress(Math.min(p / 2, 50), msg)
					}
				}),
				3, // maxRetries
				1000 // backoffMs
			)

			manifestStore.setManifest(downloadedManifest)
			onProgress(50, 'Manifest downloaded. Ready to preview update.')

			// Load existing manifest for comparison if modpack path is selected.
			// cemm-manifest.json itself is NOT written here — only once the user
			// actually confirms the install (see installUpdate below). Writing it
			// at download time meant cancelling after seeing a scary preview still
			// left disk state describing an update that was never applied (F-P1-5).
			const modpackPath = appStore.modpackPath
			if (modpackPath && modpackPath.trim().length > 0)
			{
				await generatePreviousManifest(modpackPath, onProgress)
				// The pack this manifest is about. Recorded because the deletion set
				// is derived from that folder's own inventory, so anything later
				// pairing the two has to be able to tell whether they still match.
				manifestStore.sourcePath = modpackPath
			}

			setStatus('Manifest ready for preview. Config files will be downloaded after confirmation.', 'success')
			return { success: true, manifest: downloadedManifest }
		}
		catch (err)
		{
			setStatus(getErrorMessage(err, 'download'), 'error')
			logger.error({ error: err }, 'Download failed')
			return { success: false }
		}
	}

	/**
   * Download config files from GitHub
   */
	async function downloadConfigFiles(
		updateInput: string,
		manifest: Manifest,
		onProgress: (progress: number, message?: string) => void,
		setStatus: (message: string, type: 'success' | 'error' | 'info' | 'warning') => void
	): Promise<{ success: boolean, configFiles: ConfigFileWithContent[] }>
	{
		try
		{
			const repo = appStore.githubRepo
			const resolvedQuery = await resolveUpdateQuery(updateInput)

			const configFiles = await apiDownloadConfigFiles({
				repo,
				uuid: resolvedQuery.updateId,
				modpackKey: resolvedQuery.modpackKey,
				manifest,
				onProgress: (p, msg) =>
				{
					onProgress(p, msg)
				}
			})

			// Config file content is held in memory and written to disk later by
			// installUpdate, which validates every relative_path against the modpack
			// root before writing (see installer.rs). Writing here too — before that
			// validation, and before the user has confirmed the install — let a
			// traversing path in a downloaded manifest reach disk unchecked (F-P0-4).
			setStatus(
				configFiles.length > 0
					? 'Config files downloaded. They will be written to disk when you confirm the install.'
					: 'No config files to download',
				'success'
			)

			return { success: true, configFiles }
		}
		catch (err)
		{
			const errorMessage = err instanceof Error ? err.message : 'Failed to download config files'
			setStatus(errorMessage, 'error')
			logger.error({ error: err, updateInput, repo: appStore.githubRepo }, 'Failed to download config files')
			return { success: false, configFiles: [] }
		}
	}

	async function resolveUpdateQuery(updateInput: string): Promise<{ updateId: string, modpackKey?: string }>
	{
		const trimmed = updateInput.trim().replace(/\\/g, '/')
		if (trimmed.includes('/'))
		{
			// Full repo-relative update reference: modpackKey/uuid
			return { updateId: trimmed }
		}

		const modpackKey = await resolveDownloadModpackKey()
		return {
			updateId: trimmed,
			modpackKey: modpackKey ?? undefined
		}
	}

	async function resolveDownloadModpackKey(): Promise<string | null>
	{
		const modpackPath = appStore.modpackPath
		if (modpackPath.trim().length === 0)
		{
			return null
		}

		const minecraftInstancePath = `${modpackPath}/minecraftinstance.json`
		const instanceContent = await readFile(minecraftInstancePath)

		return resolveModpackKey({
			instanceContent,
			modpackPath
		})
	}

	/**
	  * Check if a path contains path traversal patterns
	  */
	function hasPathTraversal(path: string): boolean
	{
		// Check for parent directory references
		if (path.includes('..')) return true

		// Check for absolute paths (Unix and Windows)
		if (path.startsWith('/')) return true
		if (/^[A-Za-z]:/.test(path)) return true

		// Check for home directory expansion
		if (path.startsWith('~')) return true

		return false
	}

	/**
	  * Install the update
	  */
	async function installUpdate(
		operationId: string,
		manifest: Manifest,
		configFiles: ConfigFileWithContent[],
		previousManifest: Manifest | null,
		onProgress: (progress: number, message?: string) => void,
		setStatus: (message: string, type: 'success' | 'error' | 'info' | 'warning') => void
	): Promise<boolean>
	{
		try
		{
			// Validate config files structure and paths
			for (const configFile of configFiles)
			{
				if (!configFile.filename || !configFile.relative_path || typeof configFile.content !== 'string')
				{
					throw new Error(`Invalid config file structure: ${JSON.stringify(configFile)}`)
				}

				// Check for path traversal attempts (defense in depth)
				if (hasPathTraversal(configFile.relative_path))
				{
					throw new Error(`Invalid config file path: path traversal detected in ${configFile.relative_path}`)
				}
			}

			await installUpdateTauri(
				operationId,
				appStore.modpackPath,
				manifest,
				configFiles,
				{
					oldManifest: previousManifest,
					cleanupOld: previousManifest !== null
				}
			)

			setStatus(
				previousManifest !== null ? 'Update installation complete!' : 'Fresh installation complete!',
				'success'
			)
			return true
		}
		catch (err)
		{
			setStatus(err instanceof Error ? err.message : 'Installation failed', 'error')
			logger.error({ error: err }, 'Installation failed')
			return false
		}
	}

	/**
   * Generate previous manifest from minecraftinstance.json
   */
	async function generatePreviousManifest(
		modpackPath: string,
		onProgress: (progress: number, message?: string) => void
	): Promise<{ success: boolean, error?: string }>
	{
		try
		{
			onProgress(60, 'Reading the current installation...')

			const minecraftInstancePath = `${modpackPath}/minecraftinstance.json`
			const minecraftInstanceContent = await readFile(minecraftInstancePath)

			if (minecraftInstanceContent !== null && minecraftInstanceContent.trim().length > 0)
			{
				const parsedManifest = await parseMinecraftInstance(minecraftInstancePath)

				if (parsedManifest !== null)
				{
					manifestStore.loadInstalledManifest(parsedManifest)
					return { success: true }
				}
				else
				{
					const errorMsg = 'Invalid minecraftinstance.json format - failed to parse'
					logger.error(errorMsg)
					manifestStore.loadInstalledManifest(null)
					return { success: false, error: errorMsg }
				}
			}
			else
			{
				logger.info('No minecraftinstance.json found, treating as fresh install')
				manifestStore.loadInstalledManifest(null)
				return { success: false, error: 'No previous installation found - will perform fresh install' }
			}
		}
		catch (err)
		{
			const errorMsg = err instanceof Error ? err.message : 'Unknown error generating previous manifest'
			logger.error({ error: errorMsg }, 'Failed to generate cemm-manifest_old.json')
			manifestStore.loadInstalledManifest(null)
			return { success: false, error: errorMsg }
		}
	}

	return {
		downloadFromGithub,
		downloadConfigFiles,
		installUpdate,
		generatePreviousManifest
	}
}
