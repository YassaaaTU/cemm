import { normaliseUiScale, useThemeStore } from '~/stores/theme'

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

	/**
	 * Interface scale is applied to the ROOT FONT SIZE, which is why every
	 * length in the interface is authored in rem — the rail, the title bar, the
	 * icon boxes and the active markers included. Scaling the root is the one
	 * mechanism that moves all of them together without a transform, so nothing
	 * blurs and hit targets stay exactly where they are drawn.
	 *
	 * At 100% the inline style is REMOVED rather than set to `100%`, on the same
	 * principle as `data-theme`: absent means "whatever the browser and the OS
	 * already agreed", which is the accessible default and the one a user who
	 * has enlarged their system font is relying on.
	 */
	watch(
		() => themeStore.uiScale,
		(scale) =>
		{
			const resolved = normaliseUiScale(scale)

			// A store written by another build can carry a step this one dropped.
			// Writing it back keeps the control in Settings in sync with what the
			// document is actually rendering at.
			if (resolved !== scale)
			{
				themeStore.uiScale = resolved
			}

			if (resolved === 100)
			{
				document.documentElement.style.removeProperty('font-size')
				return
			}

			document.documentElement.style.fontSize = `${resolved}%`
		},
		{ immediate: true }
	)

	/**
	 * Ctrl/Cmd +, − and 0, because this is a desktop app and that is the shortcut
	 * every other window on the machine uses for exactly this. Bound on keydown
	 * with preventDefault so the webview's own zoom — which would scale the
	 * frameless chrome and the window controls along with the content — never
	 * gets the event.
	 *
	 * `code` is read rather than `key` so the shortcut lands on the physical key
	 * regardless of layout, and both the main row and the numpad are accepted.
	 */
	const onKeydown = (event: KeyboardEvent) =>
	{
		if (!event.ctrlKey && !event.metaKey) return
		if (event.altKey || event.shiftKey) return

		switch (event.code)
		{
			case 'Equal':
			case 'NumpadAdd':
				event.preventDefault()
				themeStore.stepUiScale(1)
				break

			case 'Minus':
			case 'NumpadSubtract':
				event.preventDefault()
				themeStore.stepUiScale(-1)
				break

			case 'Digit0':
			case 'Numpad0':
				event.preventDefault()
				themeStore.resetUiScale()
				break

			default:
				break
		}
	}

	window.addEventListener('keydown', onKeydown)
})
