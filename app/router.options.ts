import type { RouterOptions } from '@nuxt/schema'

export default {
	scrollBehavior(to, _from, savedPosition)
	{
		// The 100ms setTimeout this used to wrap every resolution in didn't
		// correspond to anything — the page transition runs on --duration-smooth
		// (300ms), so this was just added latency on every navigation (F-P3-11).
		if (savedPosition != null)
		{
			return savedPosition
		}
		if (to.hash)
		{
			return { el: to.hash, top: 0 }
		}
		return { top: 0 }
	}
} satisfies RouterOptions
