import pino from 'pino'

const logger = pino({
	browser: { asObject: true },
	level: import.meta.dev ? 'debug' : 'info'
})

export default logger
