// app/plugins/logger.ts
import pino from 'pino'

export default defineNuxtPlugin(() =>
{
	const logger = pino({
		browser: {
			asObject: true,
			formatters: {
				level: (label) => ({ level: label }),
				log: (object) =>
				{
					// Extract msg and merge the rest
					const { msg, ...rest } = object
					let msgObj: unknown
					if (typeof msg === 'string')
					{
						msgObj = { text: msg }
					}
					else if (msg !== undefined)
					{
						msgObj = msg
					}
					return {
						...rest,
						...(msgObj !== undefined ? { msg: msgObj } : {}),
						time: new Date().toISOString()
					}
				}
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
