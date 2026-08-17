import { defineStore } from 'pinia'

/**
 * Appearance preference.
 *
 * `system` is the default and the desktop-native behaviour: no `data-theme`
 * attribute is written at all, which lets the `prefersdark` flag on the
 * `cemm-dark` daisyUI theme resolve light/dark purely in CSS. That means the
 * very first paint already matches the OS with no flash and no JavaScript.
 *
 * The two explicit values are overrides, and only those write `data-theme`.
 */
export type ThemePreference = 'system' | 'cemm-light' | 'cemm-dark'

/** User-level motion control, independent of the OS `prefers-reduced-motion`. */
export type MotionPreference = 'full' | 'reduced'

export const useThemeStore = defineStore(
	'themeStore',
	() =>
	{
		const preference = ref<ThemePreference>('system')
		const motion = ref<MotionPreference>('full')

		/**
		 * Tracks the OS setting so the UI can show which theme `system` currently
		 * resolves to. Populated by plugins/theme.client.ts; on the server and
		 * before hydration it stays `false` and nothing depends on it for painting.
		 */
		const systemPrefersDark = ref(false)

		/** The theme actually in effect, with `system` resolved. */
		const resolved = computed<'cemm-light' | 'cemm-dark'>(() =>
		{
			if (preference.value === 'system')
			{
				return systemPrefersDark.value ? 'cemm-dark' : 'cemm-light'
			}
			return preference.value
		})

		const isDark = computed(() => resolved.value === 'cemm-dark')

		const setPreference = (next: ThemePreference) =>
		{
			preference.value = next
		}

		const setMotion = (next: MotionPreference) =>
		{
			motion.value = next
		}

		const toggleMotion = () =>
		{
			motion.value = motion.value === 'full' ? 'reduced' : 'full'
		}

		return {
			preference,
			motion,
			systemPrefersDark,
			resolved,
			isDark,
			setPreference,
			setMotion,
			toggleMotion
		}
	},
	{
		persist: {
			// systemPrefersDark is an observation of the current machine, not a
			// preference, so it must not survive a restart on a different setting.
			pick: ['preference', 'motion']
		}
	}
)
