import { fileURLToPath } from 'node:url'

import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vitest/config'

// Deliberately a plain Vitest setup rather than a full Nuxt runtime. What needs
// covering first is the pure logic that decides what happens to a user's files;
// that logic takes plain arguments and returns plain values, so booting Nuxt to
// exercise it would only add startup cost and a second way for the suite to
// break. Add @nuxt/test-utils the day a test genuinely needs to mount a page.
export default defineConfig({
	plugins: [vue()],
	resolve: {
		alias: {
			'~': fileURLToPath(new URL('./app', import.meta.url)),
			'@': fileURLToPath(new URL('./app', import.meta.url))
		}
	},
	test: {
		environment: 'happy-dom',
		include: ['app/**/*.{test,spec}.ts'],
		reporters: 'dot'
	}
})
