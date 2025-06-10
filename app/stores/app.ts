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
})
