import tailwindcss from '@tailwindcss/vite'
import { defineNuxtConfig } from 'nuxt/config'

export default defineNuxtConfig({
	// Enable static generation for Tauri

	modules: [
		'nuxt-svgo',
		'@nuxt/eslint',
		'@vueuse/nuxt',
		'@nuxt/image',
		'@nuxt/icon',
		'@pinia/nuxt',
		'pinia-plugin-persistedstate'
	],	ssr: false,

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
	},

	experimental: {
		typedPages: true
	},

	compatibilityDate: '2025-03-01',
	nitro: {
		preset: 'static'
	},

	vite: {
		plugins: [
			tailwindcss()
		]
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
