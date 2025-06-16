import { defineStore } from 'pinia'

export type PageName = 'landing' | 'dashboard' | 'settings'

export const useNavigationStore = defineStore('navigation', () =>
{
	// Current page state (no persistence)
	const currentPage = ref<PageName>('landing')

	// Navigation functions
	const navigateToLanding = () =>
	{
		currentPage.value = 'landing'
	}

	const navigateToDashboard = () =>
	{
		currentPage.value = 'dashboard'
	}

	const navigateToSettings = () =>
	{
		currentPage.value = 'settings'
	}

	const navigateTo = (page: PageName) =>
	{
		currentPage.value = page
	}

	// Computed properties for easy checking
	const isLanding = computed(() => currentPage.value === 'landing')
	const isDashboard = computed(() => currentPage.value === 'dashboard')
	const isSettings = computed(() => currentPage.value === 'settings')

	return {
		// State
		currentPage: readonly(currentPage),

		// Computed
		isLanding,
		isDashboard,
		isSettings,

		// Actions
		navigateToLanding,
		navigateToDashboard,
		navigateToSettings,
		navigateTo
	}
})
