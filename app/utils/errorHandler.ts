/**
 * Simple error handling utilities
 * Provides basic error state management without over-engineering
 */

/**
 * Simple network retry utility
 */
export async function withNetworkRetry<T>(
	operation: () => Promise<T>,
	maxRetries = 3,
	backoffMs = 1000
): Promise<T>
{
	for (let attempt = 0; attempt <= maxRetries; attempt++)
	{
		try
		{
			return await operation()
		}
		catch (error)
		{
			if (attempt === maxRetries)
			{
				throw error
			}

			// Check if it's a network-related error worth retrying. Tauri's invoke()
			// rejects with the *string* returned by the Rust Err variant, not an
			// Error instance — checking only `error instanceof Error` meant every
			// Tauri rejection fell through to '' here and never matched, so no
			// retry in this codebase ever actually retried anything (F-P1-7).
			const errorMessage = error instanceof Error
				? error.message.toLowerCase()
				: typeof error === 'string'
					? error.toLowerCase()
					: ''
			const isNetworkError = (
				errorMessage.includes('network')
				|| errorMessage.includes('fetch')
				|| errorMessage.includes('timeout')
				|| errorMessage.includes('connection')
			)

			if (!isNetworkError)
			{
				throw error // Don't retry non-network errors
			}

			// Exponential backoff
			const delay = backoffMs * Math.pow(2, attempt)
			await new Promise((resolve) => setTimeout(resolve, delay))
		}
	}

	throw new Error('Max retries exceeded')
}

/**
 * Get user-friendly error message from an error
 */
export function getErrorMessage(error: unknown, context?: string): string
{
	// Tauri's invoke() rejects with the *string* Err value from Rust, not an
	// Error instance — the contextual rewrites below used to only run for
	// `error instanceof Error`, so every real Tauri failure skipped straight to
	// the raw string fallback further down and never got a friendly message.
	const rawMessage = error instanceof Error
		? error.message
		: typeof error === 'string'
			? error
			: null

	if (rawMessage !== null)
	{
		const message = rawMessage.toLowerCase()

		// Network errors
		if (message.includes('network') || message.includes('fetch') || message.includes('connection'))
		{
			return 'Network connection failed. Please check your internet connection and try again.'
		}

		// File errors
		if (message.includes('file not found') || message.includes('no such file'))
		{
			return 'The selected file could not be found.'
		}

		if (message.includes('permission') || message.includes('access denied'))
		{
			return 'Permission denied. Please check your file permissions.'
		}

		if (message.includes('json') || message.includes('parse'))
		{
			return 'Invalid file format. Please select a valid JSON file.'
		}

		// GitHub errors
		if (message.includes('github') || message.includes('repository'))
		{
			return 'GitHub repository not found. Please verify the repository name.'
		}

		if (message.includes('auth') || message.includes('token'))
		{
			return 'GitHub authentication failed. Please check your token in settings.'
		}

		return rawMessage
	}

	if (context !== undefined && context.trim().length > 0)
	{
		return `An error occurred during ${context}`
	}

	return 'An unexpected error occurred'
}
