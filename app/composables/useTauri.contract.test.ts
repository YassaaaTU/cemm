import { readFileSync } from 'node:fs'
import { join } from 'node:path'

import { Glob } from 'bun'
import { describe, expect, test } from 'bun:test'

/**
 * Every command the frontend invokes has to be registered with Tauri.
 *
 * Nothing else checks this. The Rust side compiles whether or not anyone calls
 * a command, and the TypeScript side compiles whatever string is passed to
 * `invoke`. A name that exists on one side only shows up at runtime, as an
 * error on the one screen that uses it — which, for a command added late, is
 * a screen nobody had clicked through yet.
 */
const root = join(import.meta.dir, '..', '..')

function registeredCommands(): Set<string>
{
	const source = readFileSync(join(root, 'src-tauri', 'src', 'lib.rs'), 'utf8')
	const handler = /generate_handler!\[([\s\S]*?)\]/.exec(source)
	if (handler === null) throw new Error('generate_handler! was not found in lib.rs')

	return new Set(
		(handler[1] ?? '')
			.split(',')
			.map((entry) => entry.trim().split('::').pop() ?? '')
			.filter((name) => name.length > 0)
	)
}

function invokedCommands(): Map<string, string>
{
	const invoked = new Map<string, string>()
	const pattern = /\binvoke(?:<[^(]*>)?\(\s*'([^']+)'/g

	for (const file of new Glob('app/**/*.{ts,vue}').scanSync({ cwd: root }))
	{
		if (file.endsWith('.test.ts')) continue
		const source = readFileSync(join(root, file), 'utf8')
		for (const match of source.matchAll(pattern))
		{
			const command = match[1]
			// Plugin commands are registered by their plugins, not by CEMM.
			if (command === undefined || command.startsWith('plugin:')) continue
			invoked.set(command, file)
		}
	}
	return invoked
}

describe('the IPC boundary', () =>
{
	test('every invoked command is registered in the Tauri handler', () =>
	{
		const registered = registeredCommands()
		const invoked = invokedCommands()

		const missing = [...invoked]
			.filter(([command]) => !registered.has(command))
			.map(([command, file]) => `${command} (called from ${file})`)

		expect(missing).toEqual([])
		// Guards the test itself: a pattern that stopped matching would pass
		// vacuously.
		expect(invoked.size).toBeGreaterThan(20)
	})

	test('the release commands are all there, on both sides', () =>
	{
		const registered = registeredCommands()
		const invoked = invokedCommands()

		for (const command of [
			'fetch_update',
			'plan_release',
			'apply_release',
			'profile_release_status',
			'read_release_spec',
			'capture_release',
			'publish_release',
			'review_upgrade'
		])
		{
			expect(registered.has(command)).toBe(true)
			expect(invoked.has(command)).toBe(true)
		}
	})
})
