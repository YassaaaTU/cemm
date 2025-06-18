/* eslint-disable pino-logger/no-console-to-pino-logger */
// bump.config.ts
/**
 * Sample bumpp config for Rust + JS/TS monorepo
 * - Bumps version in package.json and Cargo.toml
 * - Creates a git commit and tag
 * - Custom regex for Cargo.toml
 */

export default {
	files: [
		'package.json',
		'src-tauri/Cargo.toml',
		'src-tauri/tauri.conf.json',
		'./app/components/pages/SettingsPage.vue',
		'.github/workflows/release.yml',
		'.env'
	],
	cargo: {
		regex: /^version\s*=\s*"(.*?)"/m,
		replacer: (content: string, version: string) =>
			content.replace(/^version\s*=\s*"(.*?)"/m, `version = "${version}"`)
	},
	json: [
		{
			file: 'src-tauri/tauri.conf.json',
			field: 'version'
		}
	],
	commit: true,
	tag: true,
	push: false,
	changelog: true,
	replacers: [
		{
			file: './app/components/pages/SettingsPage.vue',
			regex: /(<span class="font-semibold">Version:<\/span>\s*)[0-9.]+/g,
			replacer: (content: string, version: string) =>
				content.replace(/(<span class="font-semibold">Version:<\/span>\s*)[0-9.]+/g, `$1${version}`)
		},
		{
			file: '.github/workflows/release.yml',
			regex: /(tagName:\s*app-v)[0-9.]+/g,
			replacer: (content: string, version: string) =>
				content.replace(/(tagName:\s*app-v)[0-9.]+/g, `$1${version}`)
					.replace(/(releaseName:\s*'App v)[0-9.]+/g, `$1${version}'`)
		},
		{
			file: '.env',
			regex: /(VERSION=)[0-9.]+/g,
			replacer: (content: string, version: string) =>
				content.replace(/(VERSION=)[0-9.]+/g, `$1${version}`)
		}
	],
	hooks: {
		prebump: () =>
		{
			console.info('Bumping version...')
		},
		postbump: (version: string) =>
		{
			console.info(`Bumped version to ${version}`)
		}
	}
}
