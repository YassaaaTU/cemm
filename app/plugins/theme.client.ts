import { useThemeStore } from '~/stores/theme'

/**
 * Binds the persisted appearance preference to the document.
 *
 * Deliberately minimal: when the preference is `system` the attribute is
 * REMOVED rather than set to a resolved value, so daisyUI's `prefersdark`
 * media query stays in charge. Writing a resolved theme name here would pin the
 * app to whatever the OS happened to be at startup and stop it following a
 * live theme change.
 */
export default defineNuxtPlugin(() =>
{
	const themeStore = useThemeStore()
	const darkQuery = window.matchMedia('(prefers-color-scheme: dark)')

	themeStore.systemPrefersDark = darkQuery.matches

	const onSystemChange = (event: MediaQueryListEvent) =>
	{
		themeStore.systemPrefersDark = event.matches
	}

	darkQuery.addEventListener('change', onSystemChange)

	watch(
		() => themeStore.preference,
		(preference) =>
		{
			if (preference === 'system')
			{
				document.documentElement.removeAttribute('data-theme')
				return
			}

			document.documentElement.setAttribute('data-theme', preference)
		},
		{ immediate: true }
	)

	watch(
		() => themeStore.motion,
		(motion) =>
		{
			if (motion === 'reduced')
			{
				document.documentElement.setAttribute('data-motion', 'reduced')
				return
			}

			document.documentElement.removeAttribute('data-motion')
		},
		{ immediate: true }
	)
})
