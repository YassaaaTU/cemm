/**
 * Simple TTL-only cache composable for CEMM
 * Uses Vue's useState for reactivity
 */
export const useCache = <T>(key: string, ttlMs = 300000) =>
{
	const cache = useState<Map<string, { data: T, expires: number }>>(`cache-${key}`, () => new Map())

	return {
		get: (k: string): T | null =>
		{
			const entry = cache.value.get(k)
			if (entry === undefined) return null
			if (Date.now() >= entry.expires)
			{
				cache.value.delete(k)
				return null
			}
			return entry.data
		},
		set: (k: string, data: T): void =>
		{
			cache.value.set(k, { data, expires: Date.now() + ttlMs })
		}
	}
}
