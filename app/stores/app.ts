import { defineStore } from 'pinia'

export const useAppStore = defineStore('app', () =>
{
	const mode = ref<'admin' | 'user'>('admin')
	const githubRepo = ref('') // For modpack updates (e.g., "YassaaaTU/cemm-updates")
	const appRepo = ref('YassaaaTU/cemm') // For app version updates (fixed)
	const modpackPath = ref('')

	return {
		mode,
		githubRepo,
		appRepo,
		modpackPath
	}
}, {
	persist: {
		storage: typeof window !== 'undefined' ? localStorage : undefined,
		key: 'cemm-app-store'
	}
})
