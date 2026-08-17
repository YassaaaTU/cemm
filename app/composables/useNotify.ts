import { toast } from 'vue-sonner'

export type NotifyType = 'success' | 'error' | 'info' | 'warning'

/**
 * Transient notifications.
 *
 * Replaces the inline alert bars that used to sit under each workspace. Those
 * pushed the content down when they appeared, competed with the diff for
 * attention, and had to be dismissed by hand even for routine successes.
 *
 * The one rule that matters here: **progress commentary is not a notification.**
 * The Tauri install and upload paths call their status callback repeatedly while
 * work is in flight, so routing all of it here would fire dozens of toasts per
 * install. Callers keep that text inline next to the progress bar and only send
 * outcomes to this composable.
 */
export const useNotify = () =>
{
	/** Errors linger — they usually name a path or a failed request. */
	const DURATIONS: Record<NotifyType, number> = {
		success: 3000,
		info: 3500,
		warning: 6000,
		error: 9000
	}

	const notify = (message: string, type: NotifyType = 'info', description?: string) =>
	{
		const options = {
			duration: DURATIONS[type],
			description
		}

		if (type === 'success') return toast.success(message, options)
		if (type === 'error') return toast.error(message, options)
		if (type === 'warning') return toast.warning(message, options)
		return toast.info(message, options)
	}

	return {
		notify,
		success: (message: string, description?: string) => notify(message, 'success', description),
		error: (message: string, description?: string) => notify(message, 'error', description),
		warning: (message: string, description?: string) => notify(message, 'warning', description),
		info: (message: string, description?: string) => notify(message, 'info', description),
		dismiss: (id?: number | string) => toast.dismiss(id)
	}
}
