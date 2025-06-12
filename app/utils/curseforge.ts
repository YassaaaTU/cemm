/* eslint-disable no-useless-escape */
// Utility to generate CurseForge mod page URL from addon name
export function getCurseforgeUrl(addonName: string): string
{
	const slug = addonName.toLowerCase()
		.replace(/[ _]+/g, '-')
		.replace(/[^a-z0-9-\[\]]+/g, '') // no unnecessary escapes
		.replace(/--+/g, '-')
		.replace(/^-+|-+$/g, '')
	return `https://www.curseforge.com/minecraft/mc-mods/${slug}`
}
