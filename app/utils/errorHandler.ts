import type { Ref } from 'vue'

export interface ErrorDetails
{
	code: string
	message: string
	userMessage: string
	suggestion?: string
	recoveryAction?: () => Promise<void> | void
	canRetry: boolean
	severity: 'low' | 'medium' | 'high' | 'critical'
}

export interface ErrorState
{
	error: ErrorDetails | null
	isRetrying: boolean
	retryCount: number
}

// Common error codes and their user-friendly details
export const ERROR_DEFINITIONS: Record<string, Omit<ErrorDetails, 'message'>> = {
	// File Operations
	FILE_NOT_FOUND: {
		code: 'FILE_NOT_FOUND',
		userMessage: 'The selected file could not be found.',
		suggestion: 'Please check if the file exists and try selecting it again.',
		canRetry: true,
		severity: 'medium'
	},
	FILE_READ_ERROR: {
		code: 'FILE_READ_ERROR',
		userMessage: 'Unable to read the selected file.',
		suggestion: 'Check if the file is corrupted or in use by another application.',
		canRetry: true,
		severity: 'medium'
	},
	FILE_WRITE_ERROR: {
		code: 'FILE_WRITE_ERROR',
		userMessage: 'Unable to write to the selected location.',
		suggestion: 'Check if you have write permissions and sufficient disk space.',
		canRetry: true,
		severity: 'medium'
	},
	INVALID_JSON: {
		code: 'INVALID_JSON',
		userMessage: 'The file contains invalid JSON data.',
		suggestion: 'Please select a valid cemm-manifest.json or minecraftinstance.json file.',
		canRetry: false,
		severity: 'medium'
	},
	DIRECTORY_NOT_SELECTED: {
		code: 'DIRECTORY_NOT_SELECTED',
		userMessage: 'No modpack directory selected.',
		suggestion: 'Please select your modpack directory first.',
		canRetry: true,
		severity: 'low'
	},

	// Network Operations
	NETWORK_ERROR: {
		code: 'NETWORK_ERROR',
		userMessage: 'Network connection failed.',
		suggestion: 'Check your internet connection and try again.',
		canRetry: true,
		severity: 'medium'
	},
	GITHUB_AUTH_ERROR: {
		code: 'GITHUB_AUTH_ERROR',
		userMessage: 'GitHub authentication failed.',
		suggestion: 'Check your GitHub token in settings and ensure it has the required permissions.',
		canRetry: false,
		severity: 'high'
	},
	GITHUB_REPO_NOT_FOUND: {
		code: 'GITHUB_REPO_NOT_FOUND',
		userMessage: 'GitHub repository not found.',
		suggestion: 'Verify the repository name in settings and check if it exists.',
		canRetry: false,
		severity: 'medium'
	},
	UPDATE_NOT_FOUND: {
		code: 'UPDATE_NOT_FOUND',
		userMessage: 'Update with this UUID was not found.',
		suggestion: 'Check the UUID code and ensure the update was uploaded correctly.',
		canRetry: false,
		severity: 'medium'
	},
	UPLOAD_FAILED: {
		code: 'UPLOAD_FAILED',
		userMessage: 'Failed to upload update to GitHub.',
		suggestion: 'Check your internet connection and GitHub token permissions.',
		canRetry: true,
		severity: 'high'
	},
	DOWNLOAD_FAILED: {
		code: 'DOWNLOAD_FAILED',
		userMessage: 'Failed to download update from GitHub.',
		suggestion: 'Check your internet connection and try again.',
		canRetry: true,
		severity: 'medium'
	},

	// Validation Errors
	INVALID_UUID: {
		code: 'INVALID_UUID',
		userMessage: 'Invalid UUID format.',
		suggestion: 'Please enter a valid UUID code provided by the modpack creator.',
		canRetry: false,
		severity: 'low'
	},
	INVALID_MANIFEST: {
		code: 'INVALID_MANIFEST',
		userMessage: 'Invalid manifest format.',
		suggestion: 'The manifest file is corrupted or in an unsupported format.',
		canRetry: false,
		severity: 'medium'
	},
	MISSING_GITHUB_SETTINGS: {
		code: 'MISSING_GITHUB_SETTINGS',
		userMessage: 'GitHub settings not configured.',
		suggestion: 'Please configure your GitHub repository and token in settings.',
		canRetry: false,
		severity: 'medium'
	},

	// Installation Errors
	INSTALL_FAILED: {
		code: 'INSTALL_FAILED',
		userMessage: 'Installation failed.',
		suggestion: 'Check disk space and ensure modpack directory is writable.',
		canRetry: true,
		severity: 'high'
	},
	CONFIG_INSTALL_FAILED: {
		code: 'CONFIG_INSTALL_FAILED',
		userMessage: 'Failed to install config files.',
		suggestion: 'Check if config directory is writable and try again.',
		canRetry: true,
		severity: 'medium'
	},
	ADDON_DOWNLOAD_FAILED: {
		code: 'ADDON_DOWNLOAD_FAILED',
		userMessage: 'Failed to download one or more addons.',
		suggestion: 'Check your internet connection and try again. Some addons may be temporarily unavailable.',
		canRetry: true,
		severity: 'medium'
	},

	// System Errors
	UNKNOWN_ERROR: {
		code: 'UNKNOWN_ERROR',
		userMessage: 'An unexpected error occurred.',
		suggestion: 'Please try again. If the problem persists, check the logs for more details.',
		canRetry: true,
		severity: 'high'
	}
}

export class AppError extends Error
{
	public readonly details: ErrorDetails

	constructor(code: string, originalError?: Error | string)
	{
		const message = originalError instanceof Error
			? originalError.message
			: typeof originalError === 'string'
				? originalError
				: 'Unknown error'

		super(message)
		this.name = 'AppError'

		// Get error definition with guaranteed fallback
		const errorDef = ERROR_DEFINITIONS[code] ?? ERROR_DEFINITIONS.UNKNOWN_ERROR

		// Use type assertion since we know UNKNOWN_ERROR exists
		const safeErrorDef = errorDef as Omit<ErrorDetails, 'message'>

		this.details = {
			code: safeErrorDef.code,
			message,
			userMessage: safeErrorDef.userMessage,
			suggestion: safeErrorDef.suggestion,
			recoveryAction: safeErrorDef.recoveryAction,
			canRetry: safeErrorDef.canRetry,
			severity: safeErrorDef.severity
		}
	}
}

export function createErrorHandler(
	statusMessage: Ref<string>,
	statusType: Ref<'success' | 'error' | 'info' | 'warning'>,
	logger?: { error: (message: string, data?: Record<string, unknown>) => void }
)
{
	const errorState = reactive<ErrorState>({
		error: null,
		isRetrying: false,
		retryCount: 0
	})

	const handleError = (error: Error | AppError | string, context?: string) =>
	{
		let appError: AppError

		if (error instanceof AppError)
		{
			appError = error
		}
		else if (error instanceof Error)
		{
			// Try to categorize common errors
			const message = error.message.toLowerCase()
			let code = 'UNKNOWN_ERROR'

			if (message.includes('network') || message.includes('fetch'))
			{
				code = 'NETWORK_ERROR'
			}
			else if (message.includes('file not found') || message.includes('no such file'))
			{
				code = 'FILE_NOT_FOUND'
			}
			else if (message.includes('permission') || message.includes('access denied'))
			{
				code = 'FILE_WRITE_ERROR'
			}
			else if (message.includes('json') || message.includes('parse'))
			{
				code = 'INVALID_JSON'
			}
			else if (message.includes('github') || message.includes('repository'))
			{
				code = 'GITHUB_REPO_NOT_FOUND'
			}
			else if (message.includes('auth'))
			{
				code = 'GITHUB_AUTH_ERROR'
			}

			appError = new AppError(code, error)
		}
		else
		{
			appError = new AppError('UNKNOWN_ERROR', error)
		}

		errorState.error = appError.details
		statusMessage.value = appError.details.userMessage
		statusType.value = 'error'

		if (logger !== undefined)
		{
			logger.error('Error occurred', {
				code: appError.details.code,
				context,
				originalMessage: appError.message,
				userMessage: appError.details.userMessage,
				severity: appError.details.severity,
				canRetry: appError.details.canRetry,
				retryCount: errorState.retryCount
			})
		}

		return appError
	}

	const retry = async (operation: () => Promise<void>) =>
	{
		if (errorState.error?.canRetry !== true || errorState.isRetrying)
		{
			return
		}

		errorState.isRetrying = true
		errorState.retryCount++

		try
		{
			await operation()
			clearError()
			statusMessage.value = 'Operation completed successfully'
			statusType.value = 'success'
		}
		catch (error)
		{
			handleError(error as Error, 'retry')
		}
		finally
		{
			errorState.isRetrying = false
		}
	}

	const clearError = () =>
	{
		errorState.error = null
		errorState.retryCount = 0
	}

	const executeWithRecovery = async <T>(
		operation: () => Promise<T>,
		context?: string
	): Promise<T | null> =>
	{
		try
		{
			return await operation()
		}
		catch (error)
		{
			handleError(error as Error, context)
			return null
		}
	}

	return {
		errorState: readonly(errorState),
		handleError,
		retry,
		clearError,
		executeWithRecovery
	}
}

// Network retry utility
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
