import { defineStore } from 'pinia'

export type AppMode = 'admin' | 'user'

export const useAppStore = defineStore('app', () =>
{
	const mode = ref<AppMode>('user')

	/**
	 * False until the first launch picks a counter. The old build showed a
	 * full-screen mode picker on every cold start, which put an interstitial
	 * between the user and their task forever. Now the choice is made once and
	 * the switch lives in the app shell.
	 */
	const modeChosen = ref(false)

	const githubRepo = ref('') // For modpack updates (e.g., "YassaaaTU/cemm-updates")
	const modpackPath = ref('')

	/**
	 * Shell navigation width. Compact is the default for the same reason the
	 * rail exists at all — it costs 54px instead of 200px — but the labels are
	 * one click away for anyone who would rather read the destination than
	 * recognise its icon.
	 */
	const railExpanded = ref(false)

	const setMode = (next: AppMode) =>
	{
		mode.value = next
		modeChosen.value = true
	}

	const setRailExpanded = (next: boolean) =>
	{
		railExpanded.value = next
	}

	const toggleRail = () =>
	{
		railExpanded.value = !railExpanded.value
	}

	return {
		mode,
		modeChosen,
		githubRepo,
		modpackPath,
		railExpanded,
		setMode,
		setRailExpanded,
		toggleRail
	}
}, {
	persist: {
		storage: typeof window !== 'undefined' ? localStorage : undefined,
		key: 'cemm-app-store'
	}
})
