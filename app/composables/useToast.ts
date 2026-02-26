/**
 * Simple toast notification composable
 * Replaces the over-engineered error handler with a basic toast system
 * Uses useState to persist toast state across navigation
 */
export const useToast = () =>
{
	const message = useState('toast-message', () => '')
	const type = useState<'success' | 'error' | 'info' | 'warning'>('toast-type', () => 'info')

	const show = (msg: string, t: 'success' | 'error' | 'info' | 'warning' = 'info') =>
	{
		message.value = msg
		type.value = t
	}

	const error = (msg: string) => show(msg, 'error')
	const success = (msg: string) => show(msg, 'success')
	const info = (msg: string) => show(msg, 'info')
	const warning = (msg: string) => show(msg, 'warning')
	const clear = () =>
	{
		message.value = ''
		type.value = 'info'
	}

	return {
		message,
		type,
		show,
		error,
		success,
		info,
		warning,
		clear
	}
}
