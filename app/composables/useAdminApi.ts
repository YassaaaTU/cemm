import type { ConfigFileWithContent, Manifest } from '~/types'
import { getErrorMessage } from '~/utils/errorHandler'
import { resolveModpackKey } from '~/utils/modpackKey'

function buildUpdateManifest(
	manifest: Manifest | null,
	configFiles: readonly ConfigFileWithContent[],
	excludedAddons: readonly string[]
): Manifest
{
	const configMetadata = configFiles.map((configFile) => ({
		filename: configFile.filename,
		relative_path: configFile.relative_path
	}))

	if (manifest === null)
	{
		return {
			updateType: 'config',
			mods: [],
			resourcepacks: [],
			shaderpacks: [],
			datapacks: [],
			config_files: configMetadata
		}
	}

	const excluded = new Set(excludedAddons)
	return {
		updateType: 'full',
		mods: manifest.mods.filter((addon) => !excluded.has(addon.addon_name)),
		resourcepacks: manifest.resourcepacks.filter((addon) => !excluded.has(addon.addon_name)),
		shaderpacks: manifest.shaderpacks.filter((addon) => !excluded.has(addon.addon_name)),
		datapacks: manifest.datapacks.filter((addon) => !excluded.has(addon.addon_name)),
		config_files: configMetadata
	}
}

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
	 * The instance folder this publish is about.
	 *
	 * `appStore.modpackPath` is the *player's* install target, and `loadInstance`
	 * says as much where it deliberately declines to write it. Reading it here
	 * anyway only worked while a session meant one pack. Load a pack from the
	 * library while the destination still points at another and the update is
	 * published under the wrong pack's key — the half of the update code friends
	 * paste — and its config files get relative paths measured from the wrong
	 * root. The manifest records the folder it came from, so ask it, and fall
	 * back to the destination only when nothing is loaded to ask.
	 */
	const instanceRoot = (): string =>
	{
		const loaded = manifestStore.sourcePath.trim()
		return loaded.length > 0 ? loaded : appStore.modpackPath
	}

	/**
   * Load a minecraftinstance.json file and convert to manifest.
   *
   * `knownPath` skips the native file dialog — that is how the pack library
   * loads a card the user has already chosen. Without it the dialog opens, as
   * before.
   */
	async function loadInstance(
		setStatus: (message: string, type: 'success' | 'error' | 'info' | 'warning') => void,
		knownPath?: string
	): Promise<{ success: boolean, manifest?: Manifest, instanceDir?: string }>
	{
		const filePath = knownPath !== undefined && knownPath.trim().length > 0
			? knownPath
			: await selectFile()
		if (filePath == null || filePath.length === 0)
		{
			// Backing out of the native file dialog is a decision, not a failure.
			// Toasting a warning for it meant every accidental Escape produced a
			// six-second notification reporting that nothing had happened.
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

			// The folder holding minecraftinstance.json identifies the pack. Returned
			// rather than written to the shared store, because appStore.modpackPath is
			// the player's install target and must not be clobbered by a publish.
			// Strip the trailing path segment on either separator, so this works
			// for Windows paths as well as POSIX ones.
			const instanceDir = filePath.replace(/[\\/][^\\/]*$/, '')
			// Recorded on the manifest itself so any surface can name what is
			// loaded, including one mounted after the load happened — which is
			// exactly the case when the pack library loads a card and navigates.
			manifestStore.sourcePath = instanceDir

			return { success: true, manifest: parsed, instanceDir }
		}
		catch (error)
		{
			setStatus(getErrorMessage(error, 'loading instance'), 'error')
			logger.error({ error }, 'Failed to load instance')
			return { success: false }
		}
	}

	/**
   * Save/export the generated manifest
   */
	async function saveManifest(
		manifest: Manifest | null,
		configFiles: ConfigFileWithContent[],
		setStatus: (message: string, type: 'success' | 'error' | 'info' | 'warning') => void
	): Promise<boolean>
	{
		if (manifest == null && configFiles.length === 0)
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

		const updateManifest = buildUpdateManifest(
			manifest,
			configFiles,
			manifestStore.excludedAddons
		)
		const ok = await writeFile(filePath, JSON.stringify(updateManifest, null, 2))
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
						const relativePath = calculateRelativePath(filePath, fileName, instanceRoot())

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
			// The pack that was loaded, not the one the player side is aimed at.
			const sourceRoot = instanceRoot()
			const token = await getSecure('cemm_github_token')
			if (repo.trim().length === 0 || token == null || token.trim().length === 0)
			{
				setStatus('Please configure your GitHub repository and token in settings.', 'error')
				return { success: false }
			}

			const uuid = Date.now().toString()
			let minecraftInstanceContent: string | null = null
			if (sourceRoot.trim().length > 0)
			{
				minecraftInstanceContent = await readFile(`${sourceRoot}/minecraftinstance.json`)
			}

			const modpackKey = resolveModpackKey({
				customName: customModpackName,
				instanceContent: minecraftInstanceContent,
				modpackPath: sourceRoot
			})
			if (modpackKey == null)
			{
				setStatus('Unable to determine modpack name. Set modpack path or enter a custom name.', 'error')
				return { success: false }
			}

			const updateManifest = buildUpdateManifest(
				manifest,
				configFiles,
				manifestStore.excludedAddons
			)

			const updateReference = `${modpackKey}/${uuid}`

			// Not wrapped in withNetworkRetry: upload_update is not idempotent —
			// it creates blobs/trees/commits and moves the branch ref forward, so
			// retrying after a partial success would create duplicate commits, and
			// "force": false on the ref update makes a naive retry fail anyway
			// once the ref has already moved (F-P1-7).
			await uploadUpdate({
				repo,
				token,
				uuid,
				modpackKey,
				manifest: updateManifest,
				configFiles,
				onProgress: (p, msg) =>
				{
					onProgress(p, msg)
				}
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
			logger.error({ error }, 'Upload failed')
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
	if (modpackPath)
	{
		const normalizedModpackPath = modpackPath.replace(/\\/g, '/').replace(/\/+$/, '')
		const normalizedFilePath = filePath.replace(/\\/g, '/')
		const compareModpackPath = normalizedModpackPath.toLowerCase()
		const compareFilePath = normalizedFilePath.toLowerCase()

		if (compareFilePath === compareModpackPath || compareFilePath.startsWith(`${compareModpackPath}/`))
		{
			const relativePath = normalizedFilePath.slice(normalizedModpackPath.length).replace(/^\/+/, '')
			if (relativePath.length > 0)
			{
				return relativePath
			}
		}
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
