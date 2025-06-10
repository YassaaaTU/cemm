// composables/usePinoLogger.ts
export function usePinoLogger()
{
	const { $logger } = useNuxtApp()
	return $logger
}
