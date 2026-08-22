/**
 * Sends a configured install straight to its packs.
 *
 * This lives in middleware rather than in the setup page's `onMounted` for a
 * reason that only shows up on an upgrade. The pre-2.0 `modeChosen` flag is
 * migrated to `setupCompleted` during store hydration, which lands *after* a
 * page has mounted — so a redirect fired from `onMounted` changed the URL to
 * /packs while leaving the setup screen rendered underneath it. Middleware runs
 * before the page resolves, and reading the store here is what forces hydration
 * (and the migration with it), so the flag is always settled before it is read.
 */
export default defineNuxtRouteMiddleware((to) =>
{
	if (to.path !== '/') return

	const appStore = useAppStore()
	if (!appStore.setupCompleted) return

	return navigateTo('/packs', { replace: true })
})
