import type { ConfigFileWithContent, Manifest } from '~/types'
import { getErrorMessage, withNetworkRetry } from '~/utils/errorHandler'
import { resolveModpackKey } from '~/utils/modpackKey'

/**
 * Composable for admin-specific API operations.
 * Extracts business logic from AdminPanel.vue for better maintainability.
 */
export function useAdminApi()
{
	const { uploadUpdate } = useGithubApi()
	const { getSecure } = useSecureStorage()
	const appStore = useAppStore()
	const manifestStore = useManifestStore()
	const { $logger: logger } = useNuxtApp()

	const {
		selectFile,
		selectSaveFile,
		selectMultipleFiles,
		readDirectoryRecursive,
		writeFile,
		parseMinecraftInstance,
		compareManifests,
		readFile,
		isBinaryFile
	} = useTauri()

	/**
   * Load a minecraftinstance.json file and convert to manifest
   */
	async function loadInstance(
		setStatus: (message: string, type: 'success' | 'error' | 'info' | 'warning') => void
	): Promise<{ success: boolean, manifest?: Manifest }>
	{
		const filePath = await selectFile()
		if (filePath == null || filePath.length === 0)
		{
			setStatus('No file selected.', 'warning')
			return { success: false }
		}

		try
		{
			const parsed = await parseMinecraftInstance(filePath)
			if (parsed == null)
			{
				setStatus('Failed to parse minecraftinstance.json. Invalid format.', 'error')
				return { success: false }
			}

			// Save previous manifest for diffing
			const currentManifest = manifestStore.manifest
			if (currentManifest != null)
			{
				manifestStore.setPreviousManifest(currentManifest)
			}
			manifestStore.setManifest(parsed)
			setStatus('Manifest generated from minecraftinstance.json.', 'success')

			// If previous manifest exists, show diff
			if (manifestStore.previousManifest != null)
			{
				const diff = await compareManifests(manifestStore.previousManifest, parsed)
				manifestStore.setUpdateInfo(diff)
			}
			else
			{
				manifestStore.setUpdateInfo(null)
			}

			return { success: true, manifest: parsed }
		}
		catch (error)
		{
			setStatus(getErrorMessage(error, 'loading instance'), 'error')
			logger.error('Failed to load instance', { error })
			return { success: false }
		}
	}

	/**
   * Save/export the generated manifest
   */
	async function saveManifest(
		manifest: Manifest | null,
		setStatus: (message: string, type: 'success' | 'error' | 'info' | 'warning') => void
	): Promise<boolean>
	{
		if (manifest == null)
		{
			return false
		}

		const filePath = await selectSaveFile()
		if (filePath == null || filePath.length === 0)
		{
			setStatus('No file selected.', 'warning')
			return false
		}

		// Check if file exists for user feedback
		let fileExists = false
		try
		{
			const existing = await readFile(filePath)
			if (typeof existing === 'string' && existing.length > 0)
			{
				fileExists = true
			}
		}
		catch
		{
			// File does not exist, proceed
		}

		if (fileExists)
		{
			setStatus('File already exists. Overwriting.', 'warning')
		}

		const ok = await writeFile(filePath, JSON.stringify(manifest, null, 2))
		if (ok)
		{
			setStatus(`Manifest saved as ${filePath}.`, 'success')
			return true
		}
		else
		{
			setStatus('Failed to save manifest.', 'error')
			return false
		}
	}

	/**
   * Select and process multiple config files
   */
	async function selectConfigFiles(
		setStatus: (message: string, type: 'success' | 'error' | 'info' | 'warning') => void
	): Promise<ConfigFileWithContent[]>
	{
		const filePaths = await selectMultipleFiles()

		if (filePaths.length === 0)
		{
			setStatus('No config files selected.', 'warning')
			return []
		}

		try
		{
			const newConfigFiles: ConfigFileWithContent[] = []

			for (const filePath of filePaths)
			{
				const isBinary = await isBinaryFile(filePath)
				const content = await readFile(filePath)

				if (content !== null && content.length > 0)
				{
					const fileName = filePath.split(/[/\\]/).pop()
					if (fileName !== undefined && fileName.length > 0)
					{
						const relativePath = calculateRelativePath(filePath, fileName, appStore.modpackPath)

						newConfigFiles.push({
							filename: fileName,
							relative_path: relativePath,
							content,
							is_binary: isBinary
						})
					}
				}
			}

			setStatus(`Added ${newConfigFiles.length} config file(s).`, 'success')
			return newConfigFiles
		}
		catch (err)
		{
			setStatus(`Failed to read config files: ${err instanceof Error ? err.message : 'Unknown error'}`, 'error')
			return []
		}
	}

	/**
   * Scan a directory for config files
   */
	async function scanDirectoryForConfigFiles(
		dirPath: string,
		setStatus: (message: string, type: 'success' | 'error' | 'info' | 'warning') => void
	): Promise<ConfigFileWithContent[]>
	{
		if (typeof dirPath !== 'string' || dirPath.trim().length === 0)
		{
			setStatus('No directory selected.', 'warning')
			return []
		}

		try
		{
			setStatus('Scanning directory for config files...', 'info')

			// Calculate the parent directory to use as base path
			const lastBackslash = dirPath.lastIndexOf('\\')
			const lastForwardslash = dirPath.lastIndexOf('/')
			const lastSeparator = Math.max(lastBackslash, lastForwardslash)

			let parentPath: string
			if (lastSeparator > 0)
			{
				parentPath = dirPath.substring(0, lastSeparator)
			}
			else
			{
				parentPath = dirPath
			}

			const configFiles = await readDirectoryRecursive(dirPath, parentPath)

			if (configFiles.length === 0)
			{
				setStatus('No config files found in the selected directory.', 'warning')
				return []
			}

			setStatus(`Added ${configFiles.length} config file(s) from directory.`, 'success')
			return configFiles
		}
		catch (err)
		{
			setStatus(`Failed to read config files from directory: ${err instanceof Error ? err.message : 'Unknown error'}`, 'error')
			return []
		}
	}

	/**
   * Upload manifest and config files to GitHub
   */
	async function uploadToGithub(
		manifest: Manifest | null,
		configFiles: ConfigFileWithContent[],
		customModpackName: string,
		onProgress: (progress: number, message?: string) => void,
		setStatus: (message: string, type: 'success' | 'error' | 'info' | 'warning') => void
	): Promise<{ success: boolean, updateReference?: string }>
	{
		if (manifest == null && configFiles.length === 0)
		{
			return { success: false }
		}

		try
		{
			const repo = appStore.githubRepo
			const modpackPath = appStore.modpackPath
			const token = await getSecure('cemm_github_token')
			if (repo.trim().length === 0 || token == null || token.trim().length === 0)
			{
				setStatus('Please configure your GitHub repository and token in settings.', 'error')
				return { success: false }
			}

			const uuid = Date.now().toString()
			let minecraftInstanceContent: string | null = null
			if (modpackPath.trim().length > 0)
			{
				minecraftInstanceContent = await readFile(`${modpackPath}/minecraftinstance.json`)
			}

			const modpackKey = resolveModpackKey({
				customName: customModpackName,
				instanceContent: minecraftInstanceContent,
				modpackPath
			})
			if (modpackKey == null)
			{
				setStatus('Unable to determine modpack name. Set modpack path or enter a custom name.', 'error')
				return { success: false }
			}

			// Create manifest (either from existing or config-only)
			let manifestWithConfig: Manifest
			if (manifest !== null)
			{
				// Filter out excluded addons
				const excludedSet = manifestStore.excludedAddons
				manifestWithConfig = {
					updateType: 'full',
					mods: manifest.mods.filter((m) => !excludedSet.has(m.addon_name)),
					resourcepacks: manifest.resourcepacks.filter((r) => !excludedSet.has(r.addon_name)),
					shaderpacks: manifest.shaderpacks.filter((s) => !excludedSet.has(s.addon_name)),
					datapacks: manifest.datapacks.filter((d) => !excludedSet.has(d.addon_name)),
					config_files: configFiles.map((cf) => ({
						filename: cf.filename,
						relative_path: cf.relative_path
					}))
				}
			}
			else
			{
				// Config-only manifest
				manifestWithConfig = {
					updateType: 'config',
					mods: [],
					resourcepacks: [],
					shaderpacks: [],
					datapacks: [],
					config_files: configFiles.map((cf) => ({
						filename: cf.filename,
						relative_path: cf.relative_path
					}))
				}
			}

			const updateReference = `${modpackKey}/${uuid}`

			await withNetworkRetry(async () =>
			{
				await uploadUpdate({
					repo,
					token,
					uuid,
					modpackKey,
					manifest: manifestWithConfig,
					configFiles,
					onProgress: (p, msg) =>
					{
						onProgress(p, msg)
					}
				})
			})

			setStatus(
				manifest !== null
					? `Upload successful! Share this update ID: ${updateReference}`
					: `Config files uploaded successfully! Share this update ID: ${updateReference}`,
				'success'
			)
			return { success: true, updateReference }
		}
		catch (error)
		{
			setStatus(getErrorMessage(error, 'GitHub upload'), 'error')
			logger.error('Upload failed', { error })
			return { success: false }
		}
	}

	return {
		loadInstance,
		saveManifest,
		selectConfigFiles,
		scanDirectoryForConfigFiles,
		uploadToGithub
	}
}

/**
 * Calculate relative path for a config file based on its location
 */
function calculateRelativePath(filePath: string, fileName: string, modpackPath: string): string
{
	// If file is within modpack directory, use actual relative path
	if (modpackPath && filePath.startsWith(modpackPath))
	{
		const normalizedModpackPath = modpackPath.replace(/\\/g, '/')
		const normalizedFilePath = filePath.replace(/\\/g, '/')
		return normalizedFilePath.substring(normalizedModpackPath.length + 1)
	}

	// File is outside modpack directory - try to infer relative path
	const normalizedFilePath = filePath.replace(/\\/g, '/')
	const pathParts = normalizedFilePath.split('/')

	// Look for common config directory patterns in the path
	const configIndex = pathParts.findIndex(
		(part) =>
			part === 'config'
			|| part === 'defaultconfigs'
			|| part === 'kubejs'
			|| part === 'resourcepacks'
			|| part === 'shaderpacks'
			|| part === 'emotes'
	)

	if (configIndex !== -1)
	{
		return pathParts.slice(configIndex).join('/')
	}

	// Special handling for known file types
	const fileExtension = fileName.toLowerCase().split('.').pop()
	if (fileExtension === 'emotecraft')
	{
		return `emotes/${fileName}`
	}

	// Fallback: use just the filename
	return fileName
}
