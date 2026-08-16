import tailwindcss from '@tailwindcss/vite'
import { defineNuxtConfig } from 'nuxt/config'

// Silence stale caniuse-lite warning in environments where registry mirrors lag.
process.env.BROWSERSLIST_IGNORE_OLD_DATA ??= 'true'

export default defineNuxtConfig({
	modules: [
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
				// Must match the theme store's default (stores/theme.ts) and the
				// daisyUI `--default` theme in main.css — otherwise the page ships
				// with a data-theme value the compiled CSS has no rules for at all,
				// leaving every daisyUI color variable undefined until
				// plugins/theme.client.ts runs post-hydration (F-P2-6).
				'data-theme': 'dracula'
			},
			link: [
				{
					rel: 'preconnect',
					href: 'https://fonts.googleapis.com'
				},
				{
					rel: 'preconnect',
					href: 'https://fonts.gstatic.com',
					crossorigin: ''
				},
				{
					rel: 'stylesheet',
					href: 'https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=Space+Grotesk:wght@500;600;700&display=swap'
				}
			]
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
						tauri: ['@tauri-apps/api']
						// tailwindcss was listed here too, but it's a build-time Vite/PostCSS
						// plugin, never present in the client module graph — the chunk it
						// produced was always empty (F-P3-5, confirmed via build output).
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
			include: [
				'pinia',
				'@tauri-apps/api',
				'pino', // CJS
				'@vue/devtools-core',
				'@vue/devtools-kit',
				'@tauri-apps/plugin-process',
				'@tauri-apps/plugin-updater',
				'@tauri-apps/api/app'
			]
		}
	}
})
