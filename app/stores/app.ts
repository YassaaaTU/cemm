import { defineStore } from 'pinia'

export const useAppStore = defineStore('app', () =>
{
	const mode = ref<'admin' | 'user'>('admin')
	const githubRepo = ref('')
	const modpackPath = ref('')

	return {
		mode,
		githubRepo,
		modpackPath
	}
}, {
	persist: {
		storage: typeof window !== 'undefined' ? localStorage : undefined,
		key: 'cemm-app-store'
	}
})
