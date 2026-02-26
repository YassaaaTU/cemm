/**
 * Simple error handling utilities
 * Provides basic error state management without over-engineering
 */

export interface ErrorState
{
	error: {
		message: string
		userMessage: string
		suggestion?: string
		canRetry: boolean
	} | null
	isRetrying: boolean
	retryCount: number
}

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

			// Check if it's a network-related error worth retrying
			const errorMessage = error instanceof Error ? error.message.toLowerCase() : ''
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
	if (error instanceof Error)
	{
		const message = error.message.toLowerCase()

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

		return error.message
	}

	if (typeof error === 'string')
	{
		return error
	}

	if (context !== undefined && context.trim().length > 0)
	{
		return `An error occurred during ${context}`
	}

	return 'An unexpected error occurred'
}
