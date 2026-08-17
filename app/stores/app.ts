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

	const setMode = (next: AppMode) =>
	{
		mode.value = next
		modeChosen.value = true
	}

	return {
		mode,
		modeChosen,
		githubRepo,
		modpackPath,
		setMode
	}
}, {
	persist: {
		storage: typeof window !== 'undefined' ? localStorage : undefined,
		key: 'cemm-app-store'
	}
})
