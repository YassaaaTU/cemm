import { deletePassword, getPassword, setPassword } from 'tauri-plugin-keyring-api'

/**
 * Composable for secure storage using tauri-plugin-keyring
 *
 * This uses the OS-native credential manager:
 * - Windows: Windows Credential Manager
 * - macOS: Keychain
 * - Linux: Secret Service/KWallet
 */
export const useSecureStorage = () =>
{
	const { $logger: logger } = useNuxtApp()

	/**
	 * Simple hash function to obscure key names in logs for security
	 */
	const hashKey = (key: string): string =>
	{
		let hash = 0
		for (let i = 0; i < key.length; i++)
		{
			const char = key.charCodeAt(i)
			hash = ((hash << 5) - hash) + char
			hash = hash & hash // Convert to 32-bit integer
		}
		return Math.abs(hash).toString(16).padStart(8, '0')
	}

	/**
	 * Store a value securely
	 */
	const setSecure = async (key: string, value: string): Promise<void> =>
	{
		try
		{
			logger.debug('Secure storage operation', { operation: 'set', keyHash: hashKey(key), valueLength: value.length })
			await setPassword('com.yasirjumaah.cemm', key, value)
			logger.debug('Secure storage operation completed', { operation: 'set', keyHash: hashKey(key) })
		}
		catch (error)
		{
			logger.error('Secure storage operation failed', { operation: 'set', keyHash: hashKey(key), error })
			throw error
		}
	}

	/**
	 * Get a value from secure storage
	 */
	const getSecure = async (key: string): Promise<string | null> =>
	{
		try
		{
			logger.debug('Secure storage operation', { operation: 'get', keyHash: hashKey(key) })
			const value = await getPassword('com.yasirjumaah.cemm', key)
			logger.debug('Secure storage operation completed', { operation: 'get', keyHash: hashKey(key), hasValue: value !== null })
			return value
		}
		catch (error)
		{
			logger.error('Secure storage operation failed', { operation: 'get', keyHash: hashKey(key), error })
			return null
		}
	}

	/**
	 * Remove a value from secure storage
	 */
	const removeSecure = async (key: string): Promise<void> =>
	{
		try
		{
			logger.debug('Secure storage operation', { operation: 'remove', keyHash: hashKey(key) })
			await deletePassword('com.yasirjumaah.cemm', key)
			logger.debug('Secure storage operation completed', { operation: 'remove', keyHash: hashKey(key) })
		}
		catch (error)
		{
			logger.error('Secure storage operation failed', { operation: 'remove', keyHash: hashKey(key), error })
			throw error
		}
	}

	return {
		setSecure,
		getSecure,
		removeSecure
	}
}
