export function sanitizeModpackKey(name: string): string
{
	return name
		.trim()
		.toLowerCase()
		.replace(/[\s_]+/g, '-')
		.replace(/[^a-z0-9-]/g, '')
		.replace(/-+/g, '-')
		.replace(/^-|-$/g, '')
}

export function getFolderNameFromPath(path: string): string | null
{
	const normalized = path.replace(/\\/g, '/').replace(/\/+$/, '')
	if (normalized.length === 0)
	{
		return null
	}

	const parts = normalized.split('/')
	const last = parts[parts.length - 1]
	return last != null && last.length > 0 ? last : ''
}

export function getNameFromMinecraftInstance(content: string): string | null
{
	try
	{
		const parsed = JSON.parse(content) as { name?: unknown }
		return typeof parsed.name === 'string' && parsed.name.trim().length > 0
			? parsed.name.trim()
			: null
	}
	catch
	{
		return null
	}
}

export function resolveModpackKey(options: {
	customName?: string
	instanceContent?: string | null
	modpackPath?: string
}): string | null
{
	const custom = options.customName?.trim()
	if (custom != null && custom.length > 0)
	{
		const key = sanitizeModpackKey(custom)
		return key.length > 0 ? key : null
	}

	const fromInstance = options.instanceContent != null
		? getNameFromMinecraftInstance(options.instanceContent)
		: null
	if (fromInstance != null && fromInstance.length > 0)
	{
		const key = sanitizeModpackKey(fromInstance)
		if (key.length > 0)
		{
			return key
		}
	}

	if (typeof options.modpackPath === 'string' && options.modpackPath.trim().length > 0)
	{
		const folderName = getFolderNameFromPath(options.modpackPath)
		if (folderName != null && folderName.length > 0)
		{
			const key = sanitizeModpackKey(folderName)
			if (key.length > 0)
			{
				return key
			}
		}
	}

	return null
}
