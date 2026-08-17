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
		[
			'@nuxt/icon',
			{
				// Tauri builds must keep icons available offline instead of falling
				// back to the public Iconify API at runtime.
				provider: 'none',
				clientBundle: {
					scan: true
				}
			}
		],
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
				lang: 'en'
				// Deliberately NO `data-theme` here. The daisyUI theme pair in main.css
				// resolves light/dark from `prefers-color-scheme` on its own, so the
				// first paint is already correct for the user's OS with no flash and no
				// JavaScript. plugins/theme.client.ts sets `data-theme` only when the
				// user has chosen an explicit override.
				//
				// This also retires the F-P2-6 hydration trap: there is no longer a
				// hardcoded theme name that can drift out of sync with the store default
				// and the daisyUI `--default` theme.
			},
			link: [
				// Fonts are bundled under public/fonts and declared in main.css. A Tauri
				// app must render text offline, so nothing here may touch the network.
				{
					rel: 'preload',
					as: 'font',
					type: 'font/woff2',
					href: '/fonts/archivo-latin.woff2',
					crossorigin: 'anonymous'
				},
				{
					rel: 'preload',
					as: 'font',
					type: 'font/woff2',
					href: '/fonts/jetbrains-mono-latin.woff2',
					crossorigin: 'anonymous'
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
		'@/assets/css/main.css',
		// vue-sonner ships its own positioning, stacking and swipe-to-dismiss
		// styles. Loaded after main.css so the theme tokens are already defined
		// when its variables are overridden.
		'vue-sonner/style.css'
	],

	vue: {
		compilerOptions: {
			// Vue drops template comments in production builds. The design
			// direction contract at the top of app.vue has to survive into the
			// shipped bundle so it can be audited against what actually rendered;
			// without this it exists only in source.
			comments: true
		}
	},

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
					warningMessage.includes('@nuxt/nitro-server/dist/runtime/utils/cache-driver.')
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
			rolldownOptions: {
				// Tailwind is expected to dominate this small app's fast production
				// build, so Rolldown's relative plugin-timing warning is not actionable.
				checks: {
					pluginTimings: false
				},
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
					minify: {
						compress: {
							dropConsole: true,
							dropDebugger: true
						}
					}
				}
			},
			chunkSizeWarningLimit: 1600
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
				'@tauri-apps/api/app',
				// Must be prebundled as a single instance: toast() in the composable
				// and <Toaster/> in the layout have to share vue-sonner's internal
				// store, or toasts are pushed into a store nothing is rendering.
				'vue-sonner'
			]
		}
	}
})
