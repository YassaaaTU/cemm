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

/**
 * Interface scale, as a percentage of the browser's own base font size.
 *
 * Applied to the root font size rather than as a transform or a zoom, and
 * expressed as a *percentage* rather than a pixel value so it multiplies
 * whatever the OS and webview already decided — someone who has set their
 * system font larger keeps that, and 110% means 110% of their size, not of
 * ours. Every length in the interface is authored in rem for this reason, so
 * the whole shell scales and not just its text.
 */
export const UI_SCALE_STEPS = [90, 100, 110, 125, 150] as const

export type UiScale = typeof UI_SCALE_STEPS[number]

export const DEFAULT_UI_SCALE: UiScale = 100

/**
 * Persisted values are not trusted: a store written by a different build can
 * carry a step that no longer exists, and an unrecognised number applied to the
 * root font size would scale the entire app to something unusable with no
 * control able to express it. Anything unknown falls back to 100%.
 */
export const normaliseUiScale = (value: unknown): UiScale =>
	UI_SCALE_STEPS.includes(value as UiScale) ? value as UiScale : DEFAULT_UI_SCALE

export const useThemeStore = defineStore(
	'themeStore',
	() =>
	{
		const preference = ref<ThemePreference>('system')
		const motion = ref<MotionPreference>('full')
		const uiScale = ref<UiScale>(DEFAULT_UI_SCALE)

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

		const setUiScale = (next: UiScale) =>
		{
			uiScale.value = normaliseUiScale(next)
		}

		/**
		 * Moves one step along UI_SCALE_STEPS and stops at either end, so the
		 * keyboard shortcuts can be held down without running off the scale into
		 * a size no control can undo.
		 */
		const stepUiScale = (direction: 1 | -1) =>
		{
			const current = normaliseUiScale(uiScale.value)
			const index = UI_SCALE_STEPS.indexOf(current)
			const next = UI_SCALE_STEPS[Math.min(
				UI_SCALE_STEPS.length - 1,
				Math.max(0, index + direction)
			)]

			if (next !== undefined)
			{
				uiScale.value = next
			}
		}

		const resetUiScale = () =>
		{
			uiScale.value = DEFAULT_UI_SCALE
		}

		return {
			preference,
			motion,
			uiScale,
			systemPrefersDark,
			resolved,
			isDark,
			setPreference,
			setMotion,
			toggleMotion,
			setUiScale,
			stepUiScale,
			resetUiScale
		}
	},
	{
		persist: {
			// systemPrefersDark is an observation of the current machine, not a
			// preference, so it must not survive a restart on a different setting.
			pick: ['preference', 'motion', 'uiScale']
		}
	}
)
