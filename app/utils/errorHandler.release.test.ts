import { describe, expect, test } from 'bun:test'

import { getErrorMessage, withNetworkRetry } from './errorHandler'

/**
 * What a client from before releases existed shows when it is handed a
 * release code.
 *
 * Such a client fetches `cemm-manifest.json`, which a release publishes as a
 * tombstone whose `mods` is a sentence instead of an array. Its Rust side
 * fails to parse that and returns serde's message, quoting the sentence — the
 * exact string below was captured from the installed 2.1.0 binary. This
 * function has not changed since 2.1.0, so what it does with that string here
 * is what that client's screen says.
 */
const OLD_CLIENT_ERROR = 'invalid type: string "This update needs CEMM 2.2.0 or newer. Update CEMM, then paste the code again.", expected a sequence at line 3 column 92'

describe('a release code in a client that predates releases', () =>
{
	test('the upgrade instruction reaches the screen unrewritten', () =>
	{
		const shown = getErrorMessage(OLD_CLIENT_ERROR, 'download')

		expect(shown).toBe(OLD_CLIENT_ERROR)
		expect(shown).toContain('This update needs CEMM 2.2.0 or newer')
	})

	test('it is not mistaken for a network failure and retried', async () =>
	{
		let attempts = 0
		const failing = withNetworkRetry(async () =>
		{
			attempts += 1
			return await Promise.reject(OLD_CLIENT_ERROR)
		}, 3, 1)

		await expect(failing).rejects.toBe(OLD_CLIENT_ERROR)
		expect(attempts).toBe(1)
	})
})
