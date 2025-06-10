// app/plugins/logger.server.ts
import pino from 'pino'

export default defineNuxtPlugin(() =>
{
	const logger = pino({
		browser: {
			asObject: true,
			formatters: {
				level: (label) => ({ level: label }),
				log: (object) => ({ ...object, time: new Date().toISOString() })
			}
		},
		level: import.meta.dev ? 'debug' : 'info'
	})

	return {
		provide: {
			logger
		}
	}
})
