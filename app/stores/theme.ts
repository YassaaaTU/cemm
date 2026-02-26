import { defineStore } from 'pinia'

export const useThemeStore = defineStore(
	'themeStore',
	() =>
	{
		const current = ref<'nord' | 'dracula'>('dracula')

		const setTheme = (theme: 'nord' | 'dracula') =>
		{
			current.value = theme
		}

		const toggleTheme = () =>
		{
			current.value = current.value === 'nord' ? 'dracula' : 'nord'
		}

		const isdark = computed(() => current.value === 'dracula')

		return {
			current,
			setTheme,
			toggleTheme,
			isdark
		}
	},
	{
		persist: true
	}
)
