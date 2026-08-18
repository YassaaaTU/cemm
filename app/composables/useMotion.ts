/**
 * Whether decorative motion should play, and the transition classes to use.
 *
 * Combines the OS `prefers-reduced-motion` setting with the in-app switch in
 * Settings. Gating happens in JavaScript and the motion itself is expressed as
 * Tailwind utility classes handed to Vue's <Transition>, so the stylesheet
 * stays inside the token/theme boundary and no keyframes are authored.
 *
 * Reports no motion during SSR/prerender so the first paint never animates.
 */
export const useMotion = () =>
{
	const themeStore = useThemeStore()
	const systemAllows = ref(false)

	let query: MediaQueryList | null = null

	const onChange = (event: MediaQueryListEvent) =>
	{
		systemAllows.value = !event.matches
	}

	onMounted(() =>
	{
		query = window.matchMedia('(prefers-reduced-motion: reduce)')
		systemAllows.value = !query.matches
		query.addEventListener('change', onChange)
	})

	onUnmounted(() =>
	{
		query?.removeEventListener('change', onChange)
		query = null
	})

	const motionOk = computed(() => systemAllows.value && themeStore.motion === 'full')

	const anim = (className: string) => (motionOk.value ? className : '')

	/**
	 * Transition class sets. Passed straight to <Transition>; when motion is
	 * off every class is empty, so the element simply appears.
	 */
	const paneTransition = computed(() => ({
		enterActiveClass: anim('transition duration-200 ease-out-quick'),
		enterFromClass: anim('translate-x-2 opacity-0'),
		enterToClass: anim('translate-x-0 opacity-100')
	}))

	return { motionOk, anim, paneTransition }
}
