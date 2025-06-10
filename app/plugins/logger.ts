import logger from '~/utils/logger'

export default defineNuxtPlugin(() =>
{
	return {
		provide: {
			logger
		}
	}
})
