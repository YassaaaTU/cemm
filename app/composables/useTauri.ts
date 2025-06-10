import { invoke } from '@tauri-apps/api/core'

export const useTauri = () =>
{
	const selectDirectory = async (): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('select_directory')
		}
		catch (e)
		{
			return null
		}
	}

	const selectFile = async (): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('select_file')
		}
		catch (e)
		{
			return null
		}
	}

	const readFile = async (path: string): Promise<string | null> =>
	{
		try
		{
			return await invoke<string>('read_file', { path })
		}
		catch (e)
		{
			return null
		}
	}

	const writeFile = async (path: string, content: string): Promise<boolean> =>
	{
		try
		{
			await invoke('write_file', { path, content })
			return true
		}
		catch (e)
		{
			return false
		}
	}

	return {
		selectDirectory,
		selectFile,
		readFile,
		writeFile
	}
}
