/**
 * Performance-optimized addon list with virtual scrolling and lazy loading
 * Handles large lists efficiently by only rendering visible items
 */

interface LazyListOptions
{
	itemHeight: number
	bufferSize: number
	threshold: number
}

export const useLazyList = <T>(
	items: Ref<T[]>,
	options: LazyListOptions = {
		itemHeight: 60,
		bufferSize: 5,
		threshold: 50
	}
) =>
{
	const { itemHeight, bufferSize, threshold } = options
	const logger = usePinoLogger()

	// Container refs
	const containerRef = ref<HTMLElement>()
	const scrollTop = ref(0)
	const containerHeight = ref(0)

	// Computed visible range
	const visibleRange = computed(() =>
	{
		if (containerHeight.value === 0)
		{
			return { start: 0, end: Math.min(threshold, items.value.length) }
		}

		const visibleItemCount = Math.ceil(containerHeight.value / itemHeight)
		const start = Math.max(0, Math.floor(scrollTop.value / itemHeight) - bufferSize)
		const end = Math.min(
			items.value.length,
			start + visibleItemCount + bufferSize * 2
		)

		return { start, end }
	})

	// Visible items
	const visibleItems = computed(() =>
	{
		const { start, end } = visibleRange.value
		return items.value.slice(start, end).map((item, index) => ({
			item,
			index: start + index,
			key: `item-${start + index}`
		}))
	})

	// Total height for virtual scrolling
	const totalHeight = computed(() => items.value.length * itemHeight)

	// Offset for visible items
	const offsetY = computed(() => visibleRange.value.start * itemHeight)

	// Scroll handler with throttling
	const handleScroll = useDebounceFn((event: Event) =>
	{
		const target = event.target as HTMLElement
		scrollTop.value = target.scrollTop

		logger.debug('Virtual scroll', {
			scrollTop: scrollTop.value,
			visibleRange: visibleRange.value,
			visibleCount: visibleItems.value.length
		})
	}, 16) // ~60fps

	// Resize observer
	const resizeObserver = ref<ResizeObserver>()

	// Initialize on mount
	onMounted(() =>
	{
		if (containerRef.value !== undefined)
		{
			containerHeight.value = containerRef.value.clientHeight

			// Set up resize observer
			resizeObserver.value = new ResizeObserver((entries) =>
			{
				const entry = entries[0]
				if (entry !== undefined)
				{
					containerHeight.value = entry.contentRect.height
				}
			})

			resizeObserver.value.observe(containerRef.value)
		}
	})

	// Cleanup on unmount
	onUnmounted(() =>
	{
		resizeObserver.value?.disconnect()
	})

	// Performance stats
	const getStats = () => ({
		totalItems: items.value.length,
		visibleItems: visibleItems.value.length,
		renderRatio: visibleItems.value.length / items.value.length,
		memoryUsage: visibleItems.value.length * 50 // Rough estimate in bytes
	})

	return {
		// Refs
		containerRef,

		// Computed
		visibleItems,
		totalHeight,
		offsetY,
		visibleRange,

		// Methods
		handleScroll,
		getStats
	}
}

/**
 * Optimized search and filtering for large datasets
 */
export const useSearchOptimized = <T>(
	items: Ref<T[]>,
	searchFields: Array<keyof T>,
	options: {
		debounceMs?: number
		minLength?: number
		maxResults?: number
	} = {}
) =>
{
	const {
		debounceMs = 300,
		minLength = 2,
		maxResults = 100
	} = options

	const searchTerm = ref('')
	const isSearching = ref(false)
	const logger = usePinoLogger()

	// Memoized search function
	const searchFn = computed(() =>
	{
		const term = searchTerm.value.toLowerCase().trim()

		if (term.length < minLength)
		{
			return items.value
		}

		const start = performance.now()
		const results = items.value.filter((item) =>
		{
			return searchFields.some((field) =>
			{
				const value = item[field]
				if (typeof value === 'string')
				{
					return value.toLowerCase().includes(term)
				}
				return false
			})
		}).slice(0, maxResults)

		const duration = performance.now() - start
		logger.debug('Search completed', {
			term,
			resultCount: results.length,
			duration: `${duration.toFixed(2)}ms`,
			totalItems: items.value.length
		})

		return results
	})

	// Debounced search
	const debouncedSearch = useDebounceFn(() =>
	{
		isSearching.value = false
	}, debounceMs)

	// Watch search term
	watch(searchTerm, () =>
	{
		isSearching.value = true
		debouncedSearch()
	})

	return {
		searchTerm,
		isSearching,
		filteredItems: searchFn
	}
}

/**
 * Optimized batch operations for large datasets
 */
export const useBatchOperations = <T>(
	batchSize: number = 100,
	delayMs: number = 10
) =>
{
	const isProcessing = ref(false)
	const progress = ref(0)
	const logger = usePinoLogger()

	const processBatch = async <R>(
		items: T[],
		processor: (item: T, index: number) => Promise<R> | R,
		onProgress?: (processed: number, total: number) => void
	): Promise<R[]> =>
	{
		isProcessing.value = true
		progress.value = 0
		const results: R[] = []

		try
		{
			for (let i = 0; i < items.length; i += batchSize)
			{
				const batch = items.slice(i, i + batchSize)
				const batchResults = await Promise.all(
					batch.map(async (item, batchIndex) =>
					{
						return await processor(item, i + batchIndex)
					})
				)

				results.push(...batchResults)
				progress.value = Math.min(100, ((i + batch.length) / items.length) * 100)

				onProgress?.(i + batch.length, items.length)

				// Allow UI updates between batches
				if (i + batchSize < items.length)
				{
					await new Promise((resolve) => setTimeout(resolve, delayMs))
				}
			}

			logger.debug('Batch processing completed', {
				totalItems: items.length,
				batchSize,
				batches: Math.ceil(items.length / batchSize)
			})

			return results
		}
		finally
		{
			isProcessing.value = false
			progress.value = 0
		}
	}

	return {
		isProcessing: readonly(isProcessing),
		progress: readonly(progress),
		processBatch
	}
}
