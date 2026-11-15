import { invoke } from '@tauri-apps/api/core'

import type { AppliedState, ApplyReport, CachedIcon, ConfigFileWithContent, CustomDatapackWithContent, FetchedUpdate, InstallBaseline, InstallPlan, Manifest, PackLibrary, PublishReceipt, Release, ReleaseDraft, ReleaseSpec, TauriOutcome, ThreeWayReport, UpdateDiff } from '~/types'
import { getErrorMessage } from '~/utils/errorHandler'

export const useTauri = () =>
{
	// drop_console strips every console.* call in production builds
	// (nuxt.config.ts), which previously meant every failure at this IPC
	// boundary vanished silently in release builds — the one place callers
	// most need a diagnostic trail (F-P2-18). $logger (pino) survives that.
	const { $logger: logger } = useNuxtApp()

	const selectDirectory = async (): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('select_directory')
		}
		catch (error)
		{
			logger.error({ error }, '[useTauri] selectDirectory failed')
			return null
		}
	}

	const selectFile = async (): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('select_file')
		}
		catch (error)
		{
			logger.error({ error }, '[useTauri] selectFile failed')
			return null
		}
	}

	const selectSaveFile = async (): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('select_save_file')
		}
		catch (error)
		{
			logger.error({ error }, '[useTauri] selectSaveFile failed')
			return null
		}
	}

	const selectMultipleFiles = async (): Promise<string[]> =>
	{
		try
		{
			return await invoke<string[]>('select_multiple_files')
		}
		catch (error)
		{
			logger.error({ error }, '[useTauri] selectMultipleFiles failed')
			return []
		}
	}

	const isBinaryFile = async (path: string): Promise<boolean> =>
	{
		try
		{
			return await invoke<boolean>('is_binary_file', { path })
		}
		catch (error)
		{
			logger.error({ path, error }, '[useTauri] isBinaryFile failed')
			return false
		}
	}

	const readFile = async (path: string): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('read_file', { path })
		}
		catch (error)
		{
			logger.error({ path, error }, '[useTauri] readFile failed')
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
		catch (error)
		{
			logger.error({ pathOrDir, error }, '[useTauri] writeFile failed')
			return false
		}
	}

	/**
	 * Returns the failure rather than swallowing it: a refusal from the local
	 * service ("CEMM is publishing an update…") must not reach the user dressed
	 * up as "invalid minecraftinstance.json".
	 */
	const parseMinecraftInstance = async (path: string): Promise<TauriOutcome<Manifest>> =>
	{
		try
		{
			return { ok: true, value: await invoke<Manifest>('parse_minecraft_instance', { path }) }
		}
		catch (error)
		{
			logger.error({ path, error }, '[useTauri] parseMinecraftInstance failed')
			return { ok: false, message: getErrorMessage(error) }
		}
	}

	/**
	 * What the pack currently holds, for an incoming update to be diffed against.
	 *
	 * Deliberately not `cemm-manifest.json`, which is only what CEMM last
	 * installed: it goes stale the moment the pack is edited through CurseForge,
	 * and preferring it produced previews that offered to delete files already
	 * gone and to install addons already on disk. Rust reconciles both records
	 * against the files that are actually there — see `resolve_install_baseline`.
	 *
	 * `null` inside a successful outcome means CEMM has no record of this pack
	 * at all, which is a genuinely fresh install.
	 */
	const resolveInstallBaseline = async (modpackPath: string): Promise<TauriOutcome<InstallBaseline | null>> =>
	{
		try
		{
			return {
				ok: true,
				value: await invoke<InstallBaseline | null>('resolve_install_baseline', { modpackPath })
			}
		}
		catch (error)
		{
			logger.error({ modpackPath, error }, '[useTauri] resolveInstallBaseline failed')
			return { ok: false, message: getErrorMessage(error) }
		}
	}

	/**
	 * Data packs in the instance that CurseForge did not install, grouped by
	 * pack and carrying their content, ready to publish.
	 *
	 * An addon entry is a CurseForge project id plus a CDN URL, so a data pack
	 * from anywhere else — Vanilla Tweaks, Modrinth, a hand-written one — cannot
	 * be described by one. It gets its own manifest section instead, and its
	 * files travel by relative path the way config files do.
	 *
	 * Grouped because a pack is the unit a person sees and toggles: one row for
	 * `vanillatweaks`, whether that is a single zip or a folder of forty files.
	 */
	const collectCustomDatapacks = async (modpackPath: string): Promise<TauriOutcome<CustomDatapackWithContent[]>> =>
	{
		try
		{
			return {
				ok: true,
				value: await invoke<CustomDatapackWithContent[]>('collect_custom_datapacks', { modpackPath })
			}
		}
		catch (error)
		{
			logger.error({ modpackPath, error }, '[useTauri] collectCustomDatapacks failed')
			return { ok: false, message: getErrorMessage(error) }
		}
	}

	/**
	 * The diff behind the update preview.
	 *
	 * Computed in Rust by the same function the installer uses to decide what to
	 * delete. This used to be a third implementation living in this file, and it
	 * had drifted: it skipped addons disabled in the old manifest when detecting
	 * version changes, so an addon disabled-then-updated never reached the
	 * preview even though the installer removed its files.
	 *
	 * Returns null on failure so callers can hold the preview back rather than
	 * show an empty diff, which would read as "nothing will change".
	 */
	const getUpdateDiff = async (oldManifest: Manifest | null, newManifest: Manifest): Promise<UpdateDiff | null> =>
	{
		try
		{
			return await invoke<UpdateDiff>('get_update_diff', { old: oldManifest, new: newManifest })
		}
		catch (error)
		{
			logger.error({ error }, '[useTauri] getUpdateDiff failed')
			return null
		}
	}

	const openUrl = async (url: string): Promise<void> =>
	{
		try
		{
			await invoke('open_url', { url })
		}
		catch (error)
		{
			logger.error({ url, error }, '[useTauri] openUrl failed')
		}
	}

	const installUpdate = async (
		operationId: string,
		modpackPath: string,
		manifest: Manifest,
		configFiles: ConfigFileWithContent[],
		options?: {
			oldManifest?: Manifest | null
			cleanupOld?: boolean
		}
	): Promise<void> =>
	{
		return await invoke('install_update', {
			operationId,
			modpackPath,
			manifest,
			configFiles,
			options: options !== undefined
				? {
					old_manifest: options.oldManifest ?? null,
					cleanup_old: options.cleanupOld ?? (options.oldManifest !== null && options.oldManifest !== undefined)
				}
				: undefined
		})
	}

	const uploadUpdate = async (params: {
		operationId: string
		repo: string
		token: string
		uuid: string
		modpackKey?: string
		manifest: Manifest
		configFiles: ConfigFileWithContent[]
	}): Promise<void> =>
	{
		await invoke('upload_update', params)
	}

	const downloadManifest = async (repo: string, uuid: string, modpackKey?: string): Promise<Manifest> =>
	{
		return await invoke<Manifest>('download_manifest', { repo, uuid, modpackKey })
	}

	const downloadConfigFiles = async (
		repo: string,
		uuid: string,
		manifest: Manifest,
		modpackKey?: string
	): Promise<ConfigFileWithContent[]> =>
	{
		return await invoke<ConfigFileWithContent[]>('download_config_files', { repo, uuid, modpackKey, manifest })
	}

	const readDirectoryRecursive = async (dirPath: string, basePath: string): Promise<ConfigFileWithContent[]> =>
	{
		try
		{
			return await invoke<ConfigFileWithContent[]>('read_directory_recursive', { dirPath, basePath })
		}
		catch (error)
		{
			logger.error({ dirPath, basePath, error }, '[useTauri] readDirectoryRecursive failed')
			return []
		}
	}

	const validatePath = async (path: string): Promise<{
		exists: boolean
		is_directory?: boolean
		is_file?: boolean
		can_read?: boolean
		has_minecraft_instance?: boolean
		has_mods_folder?: boolean
		has_config_folder?: boolean
		is_likely_modpack?: boolean
		is_valid_config?: boolean
		extension?: string
		absolute_path?: string
		original_path: string
	}> =>
	{
		try
		{
			return await invoke('validate_path', { path })
		}
		catch (error)
		{
			logger.error({ path, error }, '[useTauri] validatePath failed')
			return {
				exists: false,
				original_path: path
			}
		}
	}

	/**
	 * Read the local CurseForge library.
	 *
	 * `instancesDir` overrides discovery; omit it to let Rust find the folder
	 * from CurseForge's own settings. Not finding CurseForge is an ordinary
	 * outcome on a machine that does not have it, and comes back as a successful
	 * empty library — so an actual failure here always has something specific to
	 * say, and the message is carried out rather than dropped.
	 */
	const scanPackLibrary = async (instancesDir?: string | null): Promise<TauriOutcome<PackLibrary>> =>
	{
		try
		{
			return {
				ok: true,
				value: await invoke<PackLibrary>('scan_pack_library', {
					instancesDir: instancesDir ?? null
				})
			}
		}
		catch (error)
		{
			logger.error({ instancesDir, error }, '[useTauri] scanPackLibrary failed')
			return { ok: false, message: getErrorMessage(error) }
		}
	}

	/**
	 * Fetch pack artwork from CurseForge's CDN and keep it on disk.
	 *
	 * Separate from the scan on purpose: the library must open instantly and
	 * work offline, so it renders first and the pictures arrive afterwards.
	 * Returns an empty list on failure — no artwork is a cosmetic outcome, not
	 * something worth interrupting the user for.
	 */
	const cachePackIcons = async (urls: string[]): Promise<CachedIcon[]> =>
	{
		if (urls.length === 0) return []
		try
		{
			return await invoke<CachedIcon[]>('cache_pack_icons', { urls })
		}
		catch (error)
		{
			logger.error({ count: urls.length, error }, '[useTauri] cachePackIcons failed')
			return []
		}
	}

	/**
	 * Wraps a backend call whose failure message is the thing to show.
	 *
	 * Every release operation refuses in words meant for the person at the
	 * keyboard — "close Minecraft", "this code is for another pack" — so none of
	 * them may be collapsed to null the way the cosmetic calls above are.
	 */
	const outcome = async <T>(what: string, call: () => Promise<T>): Promise<TauriOutcome<T>> =>
	{
		try
		{
			return { ok: true, value: await call() }
		}
		catch (error)
		{
			logger.error({ error }, `[useTauri] ${what} failed`)
			// Deliberately not getErrorMessage: its keyword rewrites turn a precise
			// refusal into "check your internet connection".
			return {
				ok: false,
				message: typeof error === 'string'
					? error
					: error instanceof Error ? error.message : `Could not ${what}.`
			}
		}
	}

	/**
	 * Resolves an update code of either kind.
	 *
	 * Throws rather than returning an outcome, because the caller wraps it in
	 * the same network retry the legacy manifest download always had.
	 */
	const fetchUpdate = async (repo: string, code: string, modpackKey?: string): Promise<FetchedUpdate> =>
	{
		return await invoke<FetchedUpdate>('fetch_update', { repo, code, modpackKey: modpackKey ?? null })
	}

	/** What applying a release to a profile would do. Changes nothing. */
	const planRelease = (modpackPath: string, release: Release, releaseDigest: string) =>
		outcome('plan the update', () =>
			invoke<InstallPlan>('plan_release', { modpackPath, release, releaseDigest }))

	const applyRelease = (params: {
		operationId: string
		modpackPath: string
		repo: string
		release: Release
		releaseDigest: string
		revision: string
		planFingerprint: string
		approved: string[]
	}) => outcome('install the update', () => invoke<ApplyReport>('apply_release', params))

	/** Which release a profile follows, or null when CEMM has not adopted it. */
	const profileReleaseStatus = (modpackPath: string) =>
		outcome('read the profile', () =>
			invoke<AppliedState | null>('profile_release_status', { modpackPath }))

	/** The pack's `cemm-pack.json`, or null when it publishes legacy updates. */
	const readReleaseSpec = (modpackPath: string) =>
		outcome('read cemm-pack.json', () =>
			invoke<ReleaseSpec | null>('read_release_spec', { modpackPath }))

	const captureRelease = (modpackPath: string, releaseId: string, notes: string, previous: Release | null) =>
		outcome('check the release', () =>
			invoke<ReleaseDraft>('capture_release', { modpackPath, releaseId, notes, previous }))

	const publishRelease = (params: {
		operationId: string
		repo: string
		token: string
		modpackPath: string
		release: Release
	}) => outcome('publish the release', () => invoke<PublishReceipt>('publish_release', params))

	const reviewUpgrade = (basePath: string, customPath: string, upstreamPath: string) =>
		outcome('compare the three packs', () =>
			invoke<ThreeWayReport>('review_upgrade', { basePath, customPath, upstreamPath }))

	return {
		fetchUpdate,
		planRelease,
		applyRelease,
		profileReleaseStatus,
		readReleaseSpec,
		captureRelease,
		publishRelease,
		reviewUpgrade,
		selectDirectory,
		selectFile,
		selectSaveFile,
		selectMultipleFiles,
		readFile,
		writeFile,
		isBinaryFile,
		parseMinecraftInstance,
		resolveInstallBaseline,
		collectCustomDatapacks,
		getUpdateDiff,
		openUrl,
		installUpdate,
		uploadUpdate,
		downloadManifest,
		downloadConfigFiles,
		readDirectoryRecursive,
		validatePath,
		scanPackLibrary,
		cachePackIcons
	}
}
