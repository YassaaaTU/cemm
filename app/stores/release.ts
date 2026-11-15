import { defineStore } from 'pinia'

import type { ApplyReport, InstallPlan, Release } from '~/types'

/** A release as an update code resolved it: the document, and what pins it. */
export interface FetchedRelease
{
	release: Release
	/** SHA-256 of the release document as published. */
	digest: string
	/** The commit the document and every payload are read at. */
	revision: string
	/** The full pinned code, `pack/release@commit`. */
	code: string
}

/**
 * The release a player is looking at, and what applying it would do.
 *
 * Held beside the manifest store rather than inside it: a legacy manifest is
 * the payload of one update, diffed in the browser against a baseline, while a
 * release is a target state whose plan is worked out — and later re-checked —
 * by the backend against the profile itself. The two never coexist; fetching
 * either clears the other.
 *
 * Not persisted, for the same reason the fetched manifest is not: a plan
 * describes a profile at one moment, and a stale one is worse than none.
 */
export const useReleaseStore = defineStore('release', () =>
{
	const fetched = ref<FetchedRelease | null>(null)
	const plan = ref<InstallPlan | null>(null)
	const report = ref<ApplyReport | null>(null)
	/** The profile `plan` was computed against. */
	const sourcePath = ref('')

	function setFetched(next: FetchedRelease, forPath: string, nextPlan: InstallPlan)
	{
		fetched.value = next
		plan.value = nextPlan
		report.value = null
		sourcePath.value = forPath
	}

	function clear()
	{
		fetched.value = null
		plan.value = null
		report.value = null
		sourcePath.value = ''
	}

	return { fetched, plan, report, sourcePath, setFetched, clear }
})
