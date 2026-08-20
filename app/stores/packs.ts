import { defineStore } from 'pinia'

import type { PackLibrary, PackSummary } from '~/types'
import { normaliseInstancePath } from '~/utils/instancePath'

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
 * Where a pack CEMM remembers actually is.
 *
 * - `library`   — the scan returned it.
 * - `outside`   — not in the scanned folder, but the folder is there on disk.
 *                 An ordinary thing to have: anything picked by hand through
 *                 the install destination or "Choose a file instead" gets
 *                 recorded, and none of those live in the CurseForge library.
 * - `missing`   — CEMM looked, and the folder is not there.
 * - `unchecked` — absent from the scan, and not looked for yet. Also what a
 *                 failed scan leaves behind, because a scan that could not read
 *                 the library is in no position to say what is missing from it.
 */
export type PackPresence = 'library' | 'outside' | 'missing' | 'unchecked'

/**
 * A library row: a scanned pack, plus whatever CEMM remembers about it. A pack
 * the scan no longer returns stays listed rather than vanishing silently.
 */
export interface PackRow extends PackSummary
{
	history: PackHistoryEntry | null
	presence: PackPresence
}

/**
 * Shared with the manifest store, which asks the same question of the same
 * paths: two spellings of one folder must not become two packs.
 */
const normaliseKey = normaliseInstancePath

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
	 * CurseForge artwork fetched this session, keyed by its CDN URL. Not
	 * persisted: the real cache is the one Rust keeps on disk, which every scan
	 * reads without touching the network. This is only here so the cards that are
	 * already on screen can fill in without a rescan.
	 */
	const fetchedIcons = ref<Record<string, string>>({})
	/**
	 * Overrides CurseForge auto-discovery. Set in Settings, and the only way to
	 * use the library on a machine where CurseForge is not installed where CEMM
	 * expects it — Linux, most obviously, which has no official build.
	 */
	const instancesDirOverride = ref('')
	/**
	 * What became of the packs CEMM remembers but the scan did not return, once
	 * their folders have actually been looked at. Keyed by normalised path, and
	 * only ever written after a scan that worked — absent means unchecked, which
	 * is a different thing from gone and must not be shown as one. Not persisted:
	 * it describes the disk as it was this session.
	 */
	const absence = ref<Record<string, 'outside' | 'missing'>>({})

	const packs = computed<PackRow[]>(() =>
	{
		const scanned = library.value?.packs ?? []
		const seen = new Set(scanned.map((pack) => normaliseKey(pack.instancePath)))

		const rows: PackRow[] = scanned.map((pack) => ({
			...pack,
			// The scan's own icon wins — it is either the user's local image or an
			// already-cached thumbnail. This only fills the gap for artwork that
			// arrived after the grid rendered.
			icon: pack.icon ?? (pack.iconUrl !== null ? fetchedIcons.value[pack.iconUrl] ?? null : null),
			history: history.value[normaliseKey(pack.instancePath)] ?? null,
			presence: 'library'
		}))

		// Packs CEMM has used that the scan did not turn up. Most will be folders
		// picked by hand from outside the CurseForge library; some will have been
		// moved or deleted. Either way, dropping them would quietly lose the one
		// list that knows about them — and calling them all gone would be a claim
		// about a filesystem nobody had looked at, so which of the two it is comes
		// from `absence`, filled in by a real check after the scan.
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
				iconUrl: null,
				projectId: null,
				history: entry,
				presence: absence.value[key] ?? 'unchecked'
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
			// Deliberately not awaited. The grid is already correct without any
			// artwork, and a slow or unreachable CDN must never be something the
			// user waits behind to choose a modpack.
			void fetchMissingIcons()
			// Only from here, inside the success path: a scan that could not read
			// the library has no standing to say what is absent from it.
			void verifyRemembered()
		}
		finally
		{
			scanning.value = false
		}
	}

	/**
	 * Fill in CurseForge artwork for packs whose thumbnail is not cached yet.
	 *
	 * Rust writes each one to disk as it arrives, so this happens once per pack
	 * ever — every later launch, online or not, gets it straight from the scan.
	 */
	async function fetchMissingIcons(): Promise<void>
	{
		const wanted = (library.value?.packs ?? [])
			.filter((pack) => pack.icon === null && pack.iconUrl !== null)
			.map((pack) => pack.iconUrl as string)
			.filter((url) => fetchedIcons.value[url] === undefined)

		if (wanted.length === 0) return

		const { cachePackIcons } = useTauri()
		const results = await cachePackIcons([...new Set(wanted)])

		const merged = { ...fetchedIcons.value }
		for (const result of results)
		{
			if (result.icon !== null) merged[result.url] = result.icon
		}
		fetchedIcons.value = merged
	}

	/**
	 * Look at the packs CEMM remembers but the scan did not return.
	 *
	 * Being absent from the scan was previously reported to the user as "not
	 * found on disk", and the card then dropped both its actions on the strength
	 * of it. That was a claim about a filesystem CEMM had never looked at, and it
	 * was wrong for the commonest case: a pack picked by hand from outside the
	 * CurseForge library is recorded in history and is never in the scan, yet is
	 * sitting there perfectly intact.
	 *
	 * So the folders get checked. One `validate_path` per remembered pack that
	 * the scan did not account for, which on a real library is a handful.
	 */
	async function verifyRemembered(): Promise<void>
	{
		const scanned = new Set((library.value?.packs ?? []).map((pack) => normaliseKey(pack.instancePath)))
		const pending = Object.entries(history.value).filter(([key]) => !scanned.has(key))
		if (pending.length === 0) return

		const { validatePath } = useTauri()
		const checked: Record<string, 'outside' | 'missing'> = {}
		for (const [key, entry] of pending)
		{
			const path = entry.path ?? key
			const result = await validatePath(path)
			checked[key] = result.exists ? 'outside' : 'missing'
		}

		// Replaced wholesale rather than merged: a pack that has come back, or one
		// the scan now accounts for, should not keep an answer from last time.
		absence.value = checked
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
		fetchedIcons,
		instancesDirOverride,
		absence,
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
