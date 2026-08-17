/**
 * Eases a progress value toward its target.
 *
 * Install and upload progress arrives from Tauri as discrete events, so the raw
 * value teleports — 20% then 60% then 95% — which reads as a broken bar rather
 * than work happening. This smooths the *value* in JavaScript rather than
 * transitioning the bar in CSS, because styling a native <progress> element's
 * fill requires vendor pseudo-element selectors this project does not author.
 *
 * Honours reduced motion by snapping straight to the target: the smoothing is
 * decoration over a number that is already correct.
 */
export const useSmoothProgress = (target: Ref<number>) =>
{
	const displayed = ref(target.value)
	let frame: number | null = null

	const prefersReducedMotion = () =>
	{
		if (!import.meta.client) return true
		return (
			window.matchMedia('(prefers-reduced-motion: reduce)').matches
			|| document.documentElement.getAttribute('data-motion') === 'reduced'
		)
	}

	const stop = () =>
	{
		if (frame !== null)
		{
			cancelAnimationFrame(frame)
			frame = null
		}
	}

	const step = () =>
	{
		const distance = target.value - displayed.value

		// Close enough: land exactly on the target so the bar can reach 100%.
		if (Math.abs(distance) < 0.4)
		{
			displayed.value = target.value
			frame = null
			return
		}

		// Exponential approach — fast at first, settling as it arrives.
		displayed.value += distance * 0.18
		frame = requestAnimationFrame(step)
	}

	watch(target, (next) =>
	{
		if (!import.meta.client || prefersReducedMotion())
		{
			displayed.value = next
			return
		}

		// A reset to zero is a new operation starting, not progress running
		// backwards, so it snaps rather than animating down from 100.
		if (next < displayed.value)
		{
			stop()
			displayed.value = next
			return
		}

		if (frame === null)
		{
			frame = requestAnimationFrame(step)
		}
	})

	onUnmounted(stop)

	return { displayed }
}
