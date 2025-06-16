/**
 * Performance-optimized caching composable for CEMM
 * Provides memory and localStorage caching with TTL support
 */

interface CacheEntry<T>
{
	data: T
	timestamp: number
	ttl: number
}

interface CacheOptions
{
	ttl?: number // Time to live in milliseconds
	maxSize?: number // Maximum cache entries
	persistent?: boolean // Use localStorage for persistence
}

export const useCache = <T>(key: string, options: CacheOptions = {}) =>
{
	const {
		ttl = 300000, // 5 minutes default
		maxSize = 100,
		persistent = false
	} = options

	const logger = usePinoLogger()

	// Memory cache
	const memoryCache = useState<Map<string, CacheEntry<T>>>(`cache-${key}`, () => new Map())

	// Get cache key with namespace
	const getCacheKey = (subKey: string) => `cemm-cache-${key}-${subKey}`

	// Check if entry is expired
	const isExpired = (entry: CacheEntry<T>): boolean =>
	{
		return Date.now() - entry.timestamp > entry.ttl
	}

	// Clean expired entries
	const cleanExpired = () =>
	{
		const cache = memoryCache.value
		const now = Date.now()

		for (const [key, entry] of cache.entries())
		{
			if (now - entry.timestamp > entry.ttl)
			{
				cache.delete(key)
				logger.debug('Cache entry expired', { key, age: now - entry.timestamp })
			}
		}

		// Enforce max size
		if (cache.size > maxSize)
		{
			const entries = Array.from(cache.entries())
			entries.sort((a, b) => a[1].timestamp - b[1].timestamp)

			const toRemove = entries.slice(0, cache.size - maxSize)
			toRemove.forEach(([key]) =>
			{
				cache.delete(key)
				logger.debug('Cache entry evicted due to size limit', { key })
			})
		}
	}

	// Get from localStorage if persistent
	const getFromPersistent = (subKey: string): T | null =>
	{
		if (!persistent || !import.meta.client) return null
		try
		{
			const stored = localStorage.getItem(getCacheKey(subKey))
			if (stored === null || stored.length === 0) return null

			const entry: CacheEntry<T> = JSON.parse(stored)
			if (isExpired(entry))
			{
				localStorage.removeItem(getCacheKey(subKey))
				return null
			}

			return entry.data
		}
		catch (error)
		{
			logger.warn('Failed to read from persistent cache', { error, key: subKey })
			return null
		}
	}

	// Save to localStorage if persistent
	const saveToPersistent = (subKey: string, entry: CacheEntry<T>) =>
	{
		if (!persistent || !import.meta.client) return

		try
		{
			localStorage.setItem(getCacheKey(subKey), JSON.stringify(entry))
		}
		catch (error)
		{
			logger.warn('Failed to save to persistent cache', { error, key: subKey })
		}
	}
	// Get cached value
	const get = (subKey: string): T | null =>
	{
		// Check memory cache first
		const memEntry = memoryCache.value.get(subKey)
		if (memEntry !== undefined && !isExpired(memEntry))
		{
			logger.debug('Cache hit (memory)', { key: subKey })
			return memEntry.data
		}

		// Check persistent cache
		const persistentData = getFromPersistent(subKey)
		if (persistentData !== null)
		{
			// Restore to memory cache
			const entry: CacheEntry<T> = {
				data: persistentData,
				timestamp: Date.now(),
				ttl
			}
			memoryCache.value.set(subKey, entry)
			logger.debug('Cache hit (persistent)', { key: subKey })
			return persistentData
		}

		logger.debug('Cache miss', { key: subKey })
		return null
	}

	// Set cached value
	const set = (subKey: string, data: T, customTtl?: number): void =>
	{
		const entry: CacheEntry<T> = {
			data,
			timestamp: Date.now(),
			ttl: customTtl !== undefined ? customTtl : ttl
		}

		// Clean expired entries before adding new one
		cleanExpired()

		// Set in memory
		memoryCache.value.set(subKey, entry)

		// Set in persistent storage
		saveToPersistent(subKey, entry)

		logger.debug('Cache set', {
			key: subKey,
			ttl: entry.ttl,
			cacheSize: memoryCache.value.size
		})
	}

	// Remove cached value
	const remove = (subKey: string): void =>
	{
		memoryCache.value.delete(subKey)

		if (persistent && import.meta.client)
		{
			localStorage.removeItem(getCacheKey(subKey))
		}

		logger.debug('Cache removed', { key: subKey })
	}

	// Clear all cache entries for this key
	const clear = (): void =>
	{
		memoryCache.value.clear()

		if (persistent && import.meta.client)
		{
			const keys = Object.keys(localStorage)
			const prefix = getCacheKey('')
			keys.forEach((key) =>
			{
				if (key.startsWith(prefix))
				{
					localStorage.removeItem(key)
				}
			})
		}

		logger.debug('Cache cleared', { key })
	}

	// Get cache statistics
	const getStats = () =>
	{
		const cache = memoryCache.value
		const total = cache.size
		let expired = 0
		const now = Date.now()

		for (const entry of cache.values())
		{
			if (now - entry.timestamp > entry.ttl)
			{
				expired++
			}
		}

		return {
			total,
			active: total - expired,
			expired,
			maxSize
		}
	}

	// Auto-cleanup on unmount
	onUnmounted(() =>
	{
		cleanExpired()
	})

	return {
		get,
		set,
		remove,
		clear,
		getStats,
		cleanExpired
	}
}

/**
 * Specialized cache for GitHub API responses
 */
export const useGitHubCache = () =>
{
	return useCache<Record<string, unknown>>('github', {
		ttl: 600000, // 10 minutes for GitHub API
		persistent: true,
		maxSize: 50
	})
}

/**
 * Specialized cache for manifest data
 */
export const useManifestCache = () =>
{
	return useCache<Record<string, unknown>>('manifest', {
		ttl: 1800000, // 30 minutes for manifests
		persistent: true,
		maxSize: 20
	})
}

/**
 * Specialized cache for config files
 */
export const useConfigCache = () =>
{
	return useCache<string>('config', {
		ttl: 900000, // 15 minutes for config files
		persistent: false, // Don't persist config files
		maxSize: 30
	})
}
