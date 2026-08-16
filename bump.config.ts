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
	commit: false,
	tag: false,
	push: false,
	changelog: true,
	replacers: [
		{
			// release.yml reads tagName/releaseName from the GitHub release event
			// itself (github.event.release.tag_name / .name) rather than a literal
			// version string, so it has nothing for bumpp to replace here — the
			// previous replacer targeted a `cemm-v`/`CEMM v` pattern the workflow
			// no longer contains and silently matched nothing on every run (F-P3-4).
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
