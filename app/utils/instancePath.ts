/**
 * One spelling for one folder.
 *
 * Windows hands the same instance back as `D:\Packs\ATM10`, `D:/Packs/ATM10/`
 * and any casing of either. CEMM has to treat all of them as the same pack in
 * two places that must agree: the key its history is stored under, and the test
 * for whether a loaded manifest is about the pack in front of it.
 */
export function normaliseInstancePath(path: string): string
{
	return path.trim().replace(/[\\/]+$/, '').replace(/\\/g, '/').toLowerCase()
}

/**
 * Whether two paths name the same instance folder. An empty path matches
 * nothing — "unknown" is not the same as "the same".
 */
export function isSameInstance(a: string, b: string): boolean
{
	const left = normaliseInstancePath(a)
	const right = normaliseInstancePath(b)
	return left.length > 0 && left === right
}
