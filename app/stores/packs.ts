import { defineStore } from 'pinia'

import type { PackLibrary, PackSummary } from '~/types'

/**
 * What CEMM itself remembers about a pack, as opposed to what CurseForge knows.
 * Timestamps only: nothing here can go stale in a way that misleads, which a
 * remembered update code sitting beside a pack you have since changed would.
 */
export interface PackHistoryEntry
{
	/**
	 * The path as the user's filesystem spells it. The record is keyed on a
	 * normalised, lower-cased form so two spellings of one folder cannot become
	 * two entries — but that key is unfit to show anyone, and a pack CEMM can no
	 * longer find has nothing else left to name it with.
	 */
	path?: string
	lastOpenedAt?: string
	lastPublishedAt?: string
	lastInstalledAt?: string
}

/**
 * A library row: a scanned pack, plus whatever CEMM remembers about it.
 * `missing` marks a pack CEMM has used that the scan no longer finds — moved,
 * renamed or deleted. It stays listed rather than vanishing silently.
 */
export interface PackRow extends PackSummary
{
	history: PackHistoryEntry | null
	missing: boolean
}

const normaliseKey = (path: string): string =>
	path.trim().replace(/[\\/]+$/, '').replace(/\\/g, '/').toLowerCase()

const latestTouch = (entry: PackHistoryEntry | null): number =>
{
	if (entry === null) return 0
	const stamps = [entry.lastOpenedAt, entry.lastPublishedAt, entry.lastInstalledAt]
		.filter((value): value is string => typeof value === 'string')
		.map((value) => Date.parse(value))
		.filter((value) => Number.isFinite(value))
	return stamps.length > 0 ? Math.max(...stamps) : 0
}

/**
 * CurseForge writes year 0001 rather than null for a pack that has never been
 * launched, which sorts as a real date unless it is caught here.
 */
const playedAt = (pack: PackSummary): number =>
{
	if (pack.lastPlayed === null) return 0
	const parsed = Date.parse(pack.lastPlayed)
	if (!Number.isFinite(parsed)) return 0
	return parsed < Date.parse('1970-01-01T00:00:00Z') ? 0 : parsed
}

export const usePacksStore = defineStore('packs', () =>
{
	const library = ref<PackLibrary | null>(null)
	const scanning = ref(false)
	const scanError = ref<string | null>(null)
	const scannedAt = ref(0)

	/** Keyed by normalised instance path. Persisted. */
	const history = ref<Record<string, PackHistoryEntry>>({})
	/**
	 * Overrides CurseForge auto-discovery. Set in Settings, and the only way to
	 * use the library on a machine where CurseForge is not installed where CEMM
	 * expects it — Linux, most obviously, which has no official build.
	 */
	const instancesDirOverride = ref('')

	const packs = computed<PackRow[]>(() =>
	{
		const scanned = library.value?.packs ?? []
		const seen = new Set(scanned.map((pack) => normaliseKey(pack.instancePath)))

		const rows: PackRow[] = scanned.map((pack) => ({
			...pack,
			history: history.value[normaliseKey(pack.instancePath)] ?? null,
			missing: false
		}))

		// Packs CEMM has used that the scan did not turn up. Most will be folders
		// picked by hand from outside the CurseForge library; some will have been
		// moved or deleted. Either way, dropping them would quietly lose the one
		// list that knows about them.
		for (const [key, entry] of Object.entries(history.value))
		{
			if (seen.has(key)) continue
			const displayPath = entry.path ?? key
			const folderName = displayPath
				.split(/[\\/]/)
				.filter((part) => part.length > 0)
				.pop() ?? displayPath
			rows.push({
				instancePath: displayPath,
				instanceFile: `${displayPath}/minecraftinstance.json`,
				folderName,
				name: folderName,
				gameVersion: null,
				loader: null,
				groupId: null,
				addonCount: 0,
				lastPlayed: null,
				playedCount: 0,
				icon: null,
				projectId: null,
				history: entry,
				missing: true
			})
		}

		// Packs CEMM has actually been used with lead, most recent first; the rest
		// fall back to how recently they were played, then to name. That ordering
		// is the whole "track the packs I ship" ask — expressed on the library
		// rather than as a second list that would be empty on first run.
		return rows.sort((a, b) =>
		{
			const touchDelta = latestTouch(b.history) - latestTouch(a.history)
			if (touchDelta !== 0) return touchDelta
			const playedDelta = playedAt(b) - playedAt(a)
			if (playedDelta !== 0) return playedDelta
			return a.name.localeCompare(b.name, undefined, { sensitivity: 'base' })
		})
	})

	const groups = computed(() => library.value?.groups ?? [])

	async function scan(force = false): Promise<void>
	{
		// Re-scanning on every visit costs ~400ms of disk for a list that rarely
		// changes within a session. Refresh is a button; this is the guard.
		if (!force && library.value !== null && Date.now() - scannedAt.value < 30_000) return
		if (scanning.value) return

		const { scanPackLibrary } = useTauri()
		scanning.value = true
		scanError.value = null
		try
		{
			const result = await scanPackLibrary(
				instancesDirOverride.value.trim().length > 0 ? instancesDirOverride.value : null
			)
			if (result === null)
			{
				scanError.value = 'Could not read your modpack folder. Check the location in Settings.'
				return
			}
			library.value = result
			scannedAt.value = Date.now()
		}
		finally
		{
			scanning.value = false
		}
	}

	function record(
		instancePath: string,
		field: 'lastOpenedAt' | 'lastPublishedAt' | 'lastInstalledAt'
	): void
	{
		const key = normaliseKey(instancePath)
		if (key.length === 0) return
		history.value = {
			...history.value,
			[key]: {
				...history.value[key],
				// Refreshed every time, so a folder renamed only in its casing
				// starts being shown the way it is now spelled.
				path: instancePath.trim().replace(/[\\/]+$/, ''),
				[field]: new Date().toISOString()
			}
		}
	}

	const recordOpened = (instancePath: string) => record(instancePath, 'lastOpenedAt')
	const recordPublished = (instancePath: string) => record(instancePath, 'lastPublishedAt')
	const recordInstalled = (instancePath: string) => record(instancePath, 'lastInstalledAt')

	function forget(instancePath: string): void
	{
		const key = normaliseKey(instancePath)
		const { [key]: _removed, ...rest } = history.value
		history.value = rest
	}

	function setInstancesDirOverride(next: string): void
	{
		instancesDirOverride.value = next
		// The override decides what the scan reads, so the cached result is no
		// longer an answer to the current question.
		library.value = null
		scannedAt.value = 0
	}

	return {
		library,
		packs,
		groups,
		scanning,
		scanError,
		scannedAt,
		history,
		instancesDirOverride,
		scan,
		recordOpened,
		recordPublished,
		recordInstalled,
		forget,
		setInstancesDirOverride
	}
}, {
	persist: {
		storage: typeof window !== 'undefined' ? localStorage : undefined,
		key: 'cemm-packs-store',
		// Only the parts that should outlive the session. The scan result is
		// disk state that takes 70ms to rebuild and would otherwise be served
		// stale from localStorage after the user moved a pack.
		pick: ['history', 'instancesDirOverride']
	}
})
