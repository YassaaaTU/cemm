import { appDataDir } from '@tauri-apps/api/path'
import { Stronghold } from '@tauri-apps/plugin-stronghold'

const VAULT_PASSWORD = 'cemm-vault' // In production, prompt user or use OS secret
const CLIENT_NAME = 'cemm-client'
const STORE_NAME = 'cemm-store'

let stronghold: Stronghold | null = null
let client: Awaited<ReturnType<Stronghold['createClient']>> | null = null
let initPromise: Promise<void> | null = null

async function initStronghold()
{
	if (client !== null) return
	if (initPromise !== null) return initPromise
	initPromise = (async () =>
	{
		const vaultPath = `${await appDataDir()}cemm-vault.hold`
		stronghold = await Stronghold.load(vaultPath, VAULT_PASSWORD)
		try
		{
			client = await stronghold.loadClient(CLIENT_NAME)
		}
		catch
		{
			client = await stronghold.createClient(CLIENT_NAME)
		}
	})()
	await initPromise
}

export const useSecureStorage = () =>
{
	const setSecure = async (key: string, value: string) =>
	{
		await initStronghold()
		if (client === null) throw new Error('Stronghold client not initialized')
		const store = client.getStore()
		await store.insert(key, Array.from(new TextEncoder().encode(value)))
		await stronghold?.save()
	}
	const getSecure = async (key: string): Promise<string | null> =>
	{
		await initStronghold()
		if (client === null) throw new Error('Stronghold client not initialized')
		const store = client.getStore()
		try
		{
			const data = await store.get(key)
			if (data == null) return null
			return new TextDecoder().decode(new Uint8Array(data))
		}
		catch
		{
			return null
		}
	}
	const removeSecure = async (key: string) =>
	{
		await initStronghold()
		if (client === null) throw new Error('Stronghold client not initialized')
		const store = client.getStore()
		await store.remove(key)
		await stronghold?.save()
	}
	return { setSecure, getSecure, removeSecure }
}
