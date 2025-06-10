import type { Logger } from 'pino'

declare module '#app'
{
	interface NuxtApp
	{
		$logger: Logger
	}
}

declare module 'vue'
{
	interface ComponentCustomProperties
	{
		$logger: Logger
	}
}
