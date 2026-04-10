import tailwindcss from '@tailwindcss/vite'
import { defineNuxtConfig } from 'nuxt/config'

// Silence stale caniuse-lite warning in environments where registry mirrors lag.
process.env.BROWSERSLIST_IGNORE_OLD_DATA ??= 'true'

export default defineNuxtConfig({
	modules: [
		[
			'nuxt-svgo',
			{
				autoImportPath: '@/assets/'
			}
		],
		[
			'@nuxt/eslint',
			{
				checker: false,
				config: {
					stylistic: true
				}
			}
		],
		'@vueuse/nuxt',
		'@nuxt/image',
		'@nuxt/icon',
		'@pinia/nuxt',
		'pinia-plugin-persistedstate'
	],

	// Enable static generation for Tauri
	ssr: false,

	components: [
		{
			path: '~/components',
			pathPrefix: false
		}
	],

	imports: {
		presets: [
			{
				from: 'pino',
				imports: ['pino']
			}
		]
	},

	devtools: {
		enabled: true
	},

	app: {
		head: {
			title: 'CEMM',
			charset: 'utf-8',
			viewport: 'width=device-width, initial-scale=1',
			meta: [
				{ name: 'format-detection', content: 'no' }
			],
			htmlAttrs: {
				'lang': 'en',
				'data-theme': 'light'
			}
		},
		pageTransition: {
			name: 'page',
			mode: 'out-in'
		},
		layoutTransition: {
			name: 'layout',
			mode: 'out-in'
		}
	},

	css: [
		'@/assets/css/main.css'
	],

	router: {
		options: {
			scrollBehaviorType: 'smooth'
		}
	},

	runtimeConfig: {
		public: {
			version: process.env.VERSION ?? 'N/A'
			// ...other public config...
		}
		// ...other private config...
	},

	// Reduce noisy source map warnings in production builds.
	sourcemap: {
		client: false,
		server: false
	},

	future: {
		compatibilityVersion: 4
	},

	experimental: {
		typedPages: true,
		payloadExtraction: false, // Better for Tauri apps
		writeEarlyHints: false,
		componentIslands: false
	},

	compatibilityDate: '2025-03-01',
	nitro: {
		preset: 'static',
		rollupConfig: {
			onwarn(warning, warn)
			{
				const warningMessage = typeof warning.message === 'string' ? warning.message : ''
				if (
					warningMessage.includes('@nuxt/nitro-server/dist/runtime/utils/cache-driver.js')
					&& warningMessage.includes('virtual:#nitro-internal-virtual/storage')
				)
				{
					return
				}

				warn(warning)
			}
		}
	},
	vite: {
		plugins: [
			tailwindcss()
		],
		build: {
			sourcemap: false,
			// Code splitting for better performance
			rollupOptions: {
				onwarn(warning, warn)
				{
					const warningMessage = typeof warning.message === 'string' ? warning.message : ''
					if (
						warningMessage.includes('Sourcemap is likely to be incorrect')
						&& (
							warningMessage.includes('nuxt:module-preload-polyfill')
							|| warningMessage.includes('@tailwindcss/vite:generate:build')
						)
					)
					{
						return
					}

					warn(warning)
				},
				output: {
					manualChunks: {
						vendor: ['vue', 'pinia'],
						tauri: ['@tauri-apps/api'],
						ui: ['tailwindcss']
					}
				}
			},
			// Optimize chunks
			chunkSizeWarningLimit: 1600,
			// Enable minification
			minify: 'terser',
			terserOptions: {
				compress: {
					drop_console: true,
					drop_debugger: true
				}
			}
		},
		// Optimize deps
		optimizeDeps: {
			include: ['pinia', '@tauri-apps/api', 'pino']
		}
	}
})
