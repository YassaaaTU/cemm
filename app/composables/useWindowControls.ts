import type { UnlistenFn } from '@tauri-apps/api/event'

/**
 * Window chrome for the frameless shell.
 *
 * `tauri.conf.json` sets `decorations: false` on both Windows and Linux, so the
 * app draws its own title bar. Everything here degrades to a no-op in a plain
 * browser (`nuxt dev` on localhost), where `isDesktop` stays false and the
 * title bar hides its controls rather than rendering buttons that do nothing.
 */
export const useWindowControls = () =>
{
	const { $logger: logger } = useNuxtApp()

	const isDesktop = ref(false)
	const isMaximized = ref(false)
	let unlistenResize: UnlistenFn | null = null

	/**
	 * Tauri 2 exposes `window.isTauri` from 2.1 onward; `__TAURI_INTERNALS__` is
	 * the older marker. Checking both keeps this working if the app is pinned to
	 * an earlier 2.x, and both are absent in a normal browser tab.
	 */
	const detectDesktop = (): boolean =>
	{
		if (!import.meta.client) return false
		const runtime = window as unknown as Record<string, unknown>
		return runtime.isTauri === true || runtime.__TAURI_INTERNALS__ !== undefined
	}

	const getWindow = async () =>
	{
		const { getCurrentWindow } = await import('@tauri-apps/api/window')
		return getCurrentWindow()
	}

	const syncMaximized = async () =>
	{
		if (!isDesktop.value) return
		try
		{
			const appWindow = await getWindow()
			isMaximized.value = await appWindow.isMaximized()
		}
		catch (error)
		{
			logger.error({ error }, '[useWindowControls] isMaximized failed')
		}
	}

	const minimize = async () =>
	{
		if (!isDesktop.value) return
		try
		{
			const appWindow = await getWindow()
			await appWindow.minimize()
		}
		catch (error)
		{
			logger.error({ error }, '[useWindowControls] minimize failed')
		}
	}

	const toggleMaximize = async () =>
	{
		if (!isDesktop.value) return
		try
		{
			const appWindow = await getWindow()
			await appWindow.toggleMaximize()
			await syncMaximized()
		}
		catch (error)
		{
			logger.error({ error }, '[useWindowControls] toggleMaximize failed')
		}
	}

	const close = async () =>
	{
		if (!isDesktop.value) return
		try
		{
			const appWindow = await getWindow()
			await appWindow.close()
		}
		catch (error)
		{
			logger.error({ error }, '[useWindowControls] close failed')
		}
	}

	/**
	 * Double-clicking a title bar maximises/restores on both Windows and the
	 * common Linux desktops, and losing that is one of the clearest tells that a
	 * window is not really native. `-webkit-app-region: drag` handles the drag
	 * itself but does not give us this, so it is wired explicitly.
	 */
	const onTitleBarDoubleClick = (event: MouseEvent) =>
	{
		// Ignore double-clicks that land on a real control inside the bar.
		if ((event.target as HTMLElement | null)?.closest('[data-no-drag]') !== null) return
		void toggleMaximize()
	}

	onMounted(async () =>
	{
		isDesktop.value = detectDesktop()
		if (!isDesktop.value) return

		await syncMaximized()

		try
		{
			const appWindow = await getWindow()
			unlistenResize = await appWindow.onResized(() =>
			{
				void syncMaximized()
			})
		}
		catch (error)
		{
			logger.error({ error }, '[useWindowControls] onResized subscription failed')
		}
	})

	onUnmounted(() =>
	{
		if (typeof unlistenResize === 'function')
		{
			unlistenResize()
			unlistenResize = null
		}
	})

	return {
		isDesktop,
		isMaximized,
		minimize,
		toggleMaximize,
		close,
		onTitleBarDoubleClick
	}
}
