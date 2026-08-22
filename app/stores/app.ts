import { defineStore } from 'pinia'

export type AppMode = 'admin' | 'user'

export const useAppStore = defineStore('app', () =>
{
	const mode = ref<AppMode>('user')

	/**
	 * False until first-run setup has been completed or skipped.
	 *
	 * This was `modeChosen`, because setup's opening question was which counter
	 * you were on. The pack library answers that better: the action taken on a
	 * card sets the counter with a pack in hand, instead of asking in the
	 * abstract before there is anything to act on. Setup now only captures the
	 * settings with nowhere else to live, so the flag tracks setup, not a mode.
	 */
	const setupCompleted = ref(false)

	const githubRepo = ref('') // For modpack updates (e.g., "YassaaaTU/cemm-updates")
	const modpackPath = ref('')

	/**
	 * Shell navigation width. Compact is the default for the same reason the
	 * rail exists at all — it costs 54px instead of 200px — but the labels are
	 * one click away for anyone who would rather read the destination than
	 * recognise its icon.
	 */
	const railExpanded = ref(false)

	/**
	 * Deliberately does not mark setup complete. The pack library calls this on
	 * every install and publish, and a counter switch is not a statement that
	 * the repository has been configured.
	 */
	const setMode = (next: AppMode) =>
	{
		mode.value = next
	}

	const completeSetup = () =>
	{
		setupCompleted.value = true
	}

	/**
	 * Sends the app back to the first-run screen.
	 *
	 * Only the *flag* is cleared, not the configuration behind it: the setup
	 * screen pre-fills the repository and folder from this store, so re-running
	 * it reopens those fields rather than wiping them. Anyone who wanted them
	 * gone can empty them there.
	 */
	const resetSetup = () =>
	{
		setupCompleted.value = false
	}

	const setRailExpanded = (next: boolean) =>
	{
		railExpanded.value = next
	}

	const toggleRail = () =>
	{
		railExpanded.value = !railExpanded.value
	}

	return {
		mode,
		setupCompleted,
		githubRepo,
		modpackPath,
		railExpanded,
		setMode,
		completeSetup,
		resetSetup,
		setRailExpanded,
		toggleRail
	}
}, {
	persist: {
		storage: typeof window !== 'undefined' ? localStorage : undefined,
		key: 'cemm-app-store',

		/**
		 * Carries the pre-2.0 `modeChosen` flag over to `setupCompleted`.
		 * Without this every existing install would be sent back through
		 * first-run setup once, to re-confirm a repository it already has.
		 */
		afterHydrate: (ctx) =>
		{
			if (ctx.store.setupCompleted === true) return

			try
			{
				const raw = localStorage.getItem('cemm-app-store')
				if (raw === null) return

				const legacy = JSON.parse(raw) as Record<string, unknown>
				if (legacy.modeChosen !== true) return

				ctx.store.setupCompleted = true

				/**
				 * Written back by hand, and the legacy key dropped, so this runs
				 * exactly once. This hook fires before the persistence
				 * subscription is listening, so the assignment above never
				 * reaches storage on its own — and left that way, a later
				 * "Re-run setup" would be quietly undone by this migration on
				 * the next launch.
				 */
				delete legacy.modeChosen
				localStorage.setItem(
					'cemm-app-store',
					JSON.stringify({ ...legacy, setupCompleted: true })
				)
			}
			catch
			{
				// A malformed blob just means the setup screen shows once.
			}
		}
	}
})
