import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type { PublishReceipt, Release, ReleaseDraft, ThreeWayReport } from '~/types'

type Status = (message: string, type: 'success' | 'error' | 'info' | 'warning') => void

/**
 * The admin side of the release workflow: check a profile against its
 * `cemm-pack.json`, publish the result, and review an official base upgrade.
 *
 * Deliberately thin. Every decision that matters — what a release contains,
 * what blocks it, what is uploaded — is made in the backend against the files
 * themselves, so this only carries inputs in and reports out.
 */
export function useReleaseAdmin()
{
	const appStore = useAppStore()
	const { getSecure } = useSecureStorage()
	const { fetchUpdate, captureRelease, publishRelease, reviewUpgrade } = useTauri()
	const { $logger: logger } = useNuxtApp()

	/**
	 * Reads the profile into a release draft.
	 *
	 * `previousCode` names the release this one follows. It is fetched from the
	 * repository rather than remembered locally, because the published document
	 * is the only version of it every player also has: its history is what lets
	 * someone who skipped it still be brought up to date.
	 */
	async function checkRelease(
		instanceDir: string,
		releaseId: string,
		notes: string,
		previousCode: string,
		setStatus: Status
	): Promise<ReleaseDraft | null>
	{
		let previous: Release | null = null
		if (previousCode.trim().length > 0)
		{
			const repo = appStore.githubRepo
			if (repo.trim().length === 0)
			{
				setStatus('Set your GitHub repository in settings to look up the previous release.', 'error')
				return null
			}
			try
			{
				const fetched = await fetchUpdate(repo, previousCode.trim())
				if (fetched.kind !== 'release')
				{
					setStatus('That previous code is a legacy update, not a release of this pack.', 'error')
					return null
				}
				previous = fetched.release
			}
			catch (error)
			{
				logger.error({ error }, 'Previous release lookup failed')
				setStatus(
					`Could not load the previous release: ${typeof error === 'string' ? error : 'check the code and your connection'}`,
					'error'
				)
				return null
			}
		}

		const captured = await captureRelease(instanceDir, releaseId.trim(), notes, previous)
		if (!captured.ok)
		{
			setStatus(captured.message, 'error')
			return null
		}
		return captured.value
	}

	/**
	 * Publishes a checked draft. Not retried on failure: like a legacy upload it
	 * creates blobs, a tree and a commit, and the backend already guarantees a
	 * failed attempt leaves the repository unchanged.
	 */
	async function publishDraft(
		instanceDir: string,
		release: Release,
		onProgress: (progress: number, message?: string) => void,
		setStatus: Status
	): Promise<PublishReceipt | null>
	{
		const repo = appStore.githubRepo
		const token = await getSecure('cemm_github_token')
		if (repo.trim().length === 0 || token == null || token.trim().length === 0)
		{
			setStatus('Please configure your GitHub repository and token in settings.', 'error')
			return null
		}

		const operationId = globalThis.crypto.randomUUID()
		let unlisten: UnlistenFn | undefined
		try
		{
			unlisten = await listen<{ operationId: string, progress: number, message: string }>('upload_progress', (event) =>
			{
				if (event.payload.operationId !== operationId) return
				onProgress(event.payload.progress, event.payload.message)
			})

			const published = await publishRelease({
				operationId,
				repo,
				token,
				modpackPath: instanceDir,
				release
			})
			if (!published.ok)
			{
				setStatus(published.message, 'error')
				return null
			}
			return published.value
		}
		finally
		{
			if (unlisten !== undefined) unlisten()
		}
	}

	async function reviewOfficialUpgrade(
		basePath: string,
		customPath: string,
		upstreamPath: string,
		setStatus: Status
	): Promise<ThreeWayReport | null>
	{
		const reviewed = await reviewUpgrade(basePath, customPath, upstreamPath)
		if (!reviewed.ok)
		{
			setStatus(reviewed.message, 'error')
			return null
		}
		return reviewed.value
	}

	return { checkRelease, publishDraft, reviewOfficialUpgrade }
}
