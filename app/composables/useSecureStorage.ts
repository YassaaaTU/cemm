import { deletePassword, getPassword, setPassword } from 'tauri-plugin-keyring-api'

import { usePinoLogger } from './usePinoLogger'

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
	const logger = usePinoLogger()

	/**
	 * Store a value securely
	 */
	const setSecure = async (key: string, value: string): Promise<void> =>
	{
		try
		{
			logger.info('Setting secure value', { key, valueLength: value.length })
			await setPassword('com.yasirjumaah.cemm', key, value)
			logger.info('✅ Secure value set successfully', { key })
		}
		catch (error)
		{
			logger.error('❌ Failed to set secure value', { key, error })
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
			logger.info('Getting secure value', { key })
			const value = await getPassword('com.yasirjumaah.cemm', key)
			logger.info('✅ Secure value retrieved', { key, hasValue: value !== null, valueLength: value?.length ?? 0 })
			return value
		}
		catch (error)
		{
			logger.error('❌ Failed to get secure value', { key, error })
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
			logger.info('Removing secure value', { key })
			await deletePassword('com.yasirjumaah.cemm', key)
			logger.info('✅ Secure value removed successfully', { key })
		}
		catch (error)
		{
			logger.error('❌ Failed to remove secure value', { key, error })
			throw error
		}
	}

	return {
		setSecure,
		getSecure,
		removeSecure
	}
}
