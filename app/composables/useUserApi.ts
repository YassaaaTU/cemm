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
	const { writeFile, readFile, parseMinecraftInstance, installUpdate: installUpdateTauri } = useTauri()
	const { $logger: logger } = useNuxtApp()

	/**
   * Download manifest from GitHub
   */
	async function downloadFromGithub(
		uuid: string,
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
			const modpackKey = await resolveDownloadModpackKey()
			if (modpackKey == null)
			{
				setStatus('Could not determine modpack name. Select a valid modpack path first.', 'error')
				return { success: false }
			}

			onProgress(10, 'Downloading manifest...')

			const downloadedManifest = await withNetworkRetry(
				async () => await downloadManifest({
					repo,
					uuid: uuid.trim(),
					modpackKey,
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

			// Load existing manifest for comparison if modpack path is selected
			const modpackPath = appStore.modpackPath
			if (modpackPath && modpackPath.trim().length > 0)
			{
				await generatePreviousManifest(modpackPath, onProgress)
				await writeNewManifest(modpackPath, downloadedManifest)
			}

			setStatus('Manifest ready for preview. Config files will be downloaded after confirmation.', 'success')
			return { success: true, manifest: downloadedManifest }
		}
		catch (err)
		{
			setStatus(getErrorMessage(err, 'download'), 'error')
			logger.error('Download failed', { error: err })
			return { success: false }
		}
	}

	/**
   * Download config files from GitHub
   */
	async function downloadConfigFiles(
		uuid: string,
		manifest: Manifest,
		onProgress: (progress: number, message?: string) => void,
		setStatus: (message: string, type: 'success' | 'error' | 'info' | 'warning') => void
	): Promise<{ success: boolean, configFiles: ConfigFileWithContent[] }>
	{
		try
		{
			const repo = appStore.githubRepo
			const modpackKey = await resolveDownloadModpackKey()
			if (modpackKey == null)
			{
				setStatus('Could not determine modpack name. Select a valid modpack path first.', 'error')
				return { success: false, configFiles: [] }
			}

			const configFiles = await apiDownloadConfigFiles({
				repo,
				uuid: uuid.trim(),
				modpackKey,
				manifest,
				onProgress: (p, msg) =>
				{
					onProgress(p, msg)
				}
			})

			// Write config files to disk if modpack path is selected
			const modpackPath = appStore.modpackPath
			if (modpackPath && modpackPath.trim().length > 0 && configFiles.length > 0)
			{
				const filesToWrite: Array<[string, string]> = []

				for (const configFile of configFiles)
				{
					filesToWrite.push([configFile.relative_path, configFile.content])
				}

				const writeSuccess = await writeFile(modpackPath, filesToWrite)
				if (!writeSuccess)
				{
					setStatus('Config files downloaded but failed to write to disk.', 'warning')
					return { success: false, configFiles: [] }
				}

				setStatus(`Config files downloaded and written to ${modpackPath}`, 'success')
			}
			else
			{
				setStatus(
					configFiles.length > 0
						? 'Config files downloaded (no modpack path selected)'
						: 'No config files to download',
					'success'
				)
			}

			return { success: true, configFiles }
		}
		catch (err)
		{
			const errorMessage = err instanceof Error ? err.message : 'Failed to download config files'
			setStatus(errorMessage, 'error')
			logger.error('Failed to download config files', { error: err, uuid, repo: appStore.githubRepo })
			return { success: false, configFiles: [] }
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
			logger.error('Installation failed', { error: err })
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
			onProgress(60, 'Generating cemm-manifest_old.json from current installation...')

			const minecraftInstancePath = `${modpackPath}/minecraftinstance.json`
			const minecraftInstanceContent = await readFile(minecraftInstancePath)

			if (minecraftInstanceContent !== null && minecraftInstanceContent.trim().length > 0)
			{
				const parsedManifest = await parseMinecraftInstance(minecraftInstancePath)

				if (parsedManifest !== null)
				{
					const oldManifestPath = `${modpackPath}/cemm-manifest_old.json`
					const manifestContent = JSON.stringify(parsedManifest, null, 2)
					const writeSuccess = await writeFile(oldManifestPath, manifestContent)

					if (!writeSuccess)
					{
						const errorMsg = 'Failed to write cemm-manifest_old.json from minecraftinstance.json'
						logger.error(errorMsg)
						manifestStore.loadInstalledManifest(null)
						return { success: false, error: errorMsg }
					}

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
			logger.error('Failed to generate cemm-manifest_old.json', { error: errorMsg })
			manifestStore.loadInstalledManifest(null)
			return { success: false, error: errorMsg }
		}
	}

	/**
   * Write new manifest to disk
   */
	async function writeNewManifest(modpackPath: string, newManifest: Manifest): Promise<boolean>
	{
		try
		{
			const manifestPath = `${modpackPath}/cemm-manifest.json`
			const writeSuccess = await writeFile(manifestPath, JSON.stringify(newManifest, null, 2))

			if (!writeSuccess)
			{
				throw new Error('Failed to write new cemm-manifest.json')
			}

			logger.info('Successfully wrote new cemm-manifest.json')
			return true
		}
		catch (err)
		{
			logger.error('Failed to write new cemm-manifest.json', { error: err })
			throw err
		}
	}

	return {
		downloadFromGithub,
		downloadConfigFiles,
		installUpdate,
		generatePreviousManifest,
		writeNewManifest
	}
}
