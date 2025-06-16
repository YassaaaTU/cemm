import tailwindcss from '@tailwindcss/vite'
import { defineNuxtConfig } from 'nuxt/config'

export default defineNuxtConfig({
	modules: [
		'nuxt-svgo',
		'@nuxt/eslint',
		'@vueuse/nuxt',
		'@nuxt/image',
		'@nuxt/icon',
		'@pinia/nuxt',
		'pinia-plugin-persistedstate'
	],

	// Enable static generation for Tauri
	ssr: false,

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

	future: {
		compatibilityVersion: 4
	},	experimental: {
		typedPages: true,
		payloadExtraction: false, // Better for Tauri apps
		writeEarlyHints: false,
		componentIslands: false
	},

	compatibilityDate: '2025-03-01',
	nitro: {
		preset: 'static'
	},
	vite: {
		plugins: [
			tailwindcss()
		],
		build: {
			// Code splitting for better performance
			rollupOptions: {
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
	},

	eslint: {
		checker: true,
		config: {
			stylistic: true
		}
	},

	svgo: {
		autoImportPath: '@/assets/'
	}
})
