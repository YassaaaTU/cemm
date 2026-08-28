import type { Addon as WireAddon } from './generated/Addon'
import type { Manifest as WireManifest } from './generated/Manifest'

export type { ConfigFile } from './generated/ConfigFile'
export type { ConfigFileWithContent } from './generated/ConfigFileWithContent'
export type { CustomDatapack } from './generated/CustomDatapack'
export type { CustomDatapackWithContent } from './generated/CustomDatapackWithContent'
export type { InstallBaseline } from './generated/InstallBaseline'
export type { UpdateDiff } from './generated/UpdateDiff'

/**
 * Everything under `./generated` is written from the Rust structs by ts-rs
 * (`bun run types:generate`) and checked in CI, so these shapes cannot drift
 * from the backend the way the hand-maintained copies did. Edit the Rust
 * definition, not the generated file.
 *
 * Only two types are restated here, and only to add fields that exist purely in
 * the UI and are never sent to Rust.
 */

/** The wire shape, plus the artwork the UI attaches after a manifest loads. */
export interface Addon extends WireAddon
{
	/** UI only — resolved client-side, dropped when the addon goes back to Rust. */
	thumbnailUrl?: string
}

/**
 * The wire shape over the UI-extended Addon.
 *
 * Written as an Omit so it still inherits any field Rust adds, and so renaming
 * a category in Rust breaks this line rather than silently going unnoticed.
 */
export type Manifest = Omit<WireManifest, 'mods' | 'resourcepacks' | 'shaderpacks' | 'datapacks'> & {
	mods: Addon[]
	resourcepacks: Addon[]
	shaderpacks: Addon[]
	datapacks: Addon[]
}

/**
 * A backend call whose failure is worth repeating to the user.
 *
 * Most `useTauri` wrappers collapse failure to `null` and keep the detail in
 * the log, because "it did not work" is all the caller can act on. These
 * cannot: the local service now refuses a request outright while a publish or
 * an install is running, and reporting that as a malformed instance file or an
 * unreadable modpack folder would be a lie about the user's disk.
 */
export type TauriOutcome<T> = { ok: true, value: T } | { ok: false, message: string }

/**
 * One of CurseForge's instance groups.
 *
 * Mirrored in Rust: src-tauri/src/composables/instances.rs (PackGroup).
 */
export interface PackGroup
{
	id: string
	name: string
}

/**
 * A modpack as the library lists it — enough to recognise and choose one, and
 * deliberately not a Manifest. Loading a pack for real still goes through
 * parse_minecraft_instance.
 *
 * Mirrored in Rust: src-tauri/src/composables/instances.rs (PackSummary).
 */
export interface PackSummary
{
	/** Folder holding minecraftinstance.json. The identity key everywhere. */
	instancePath: string
	instanceFile: string
	/**
	 * The directory's own name, which is not always the pack's: a real library
	 * holds a folder called `All the Mods 10 - ATM10 (2)` containing a pack
	 * named `Aeronautics`. Both are shown so a card is never ambiguous.
	 */
	folderName: string
	name: string
	gameVersion: string | null
	/** Loader family only — `NeoForge`, not `neoforge-21.1.228`. */
	loader: string | null
	groupId: string | null
	addonCount: number
	/** RFC 3339. CurseForge writes year 0001 for "never played". */
	lastPlayed: string | null
	playedCount: number
	/** A `data:` URI, or null. Never a remote URL. */
	icon: string | null
	/**
	 * The pack's artwork on CurseForge's CDN, for a pack installed from there
	 * whose image is not cached yet. Fetched separately from the scan so the
	 * library still opens instantly and offline.
	 */
	iconUrl: string | null
	projectId: number | null
}

/**
 * Mirrored in Rust: src-tauri/src/composables/instances.rs (CachedIcon).
 */
export interface CachedIcon
{
	url: string
	/** A `data:` URI, or null when the fetch failed. */
	icon: string | null
}

/**
 * Mirrored in Rust: src-tauri/src/composables/instances.rs (PackLibrary).
 */
export interface PackLibrary
{
	instancesDir: string | null
	/**
	 * `curseforge` when found from CurseForge's own settings, `manual` when the
	 * folder was supplied, `none` when there was nothing to scan. "You have no
	 * packs" and "I could not find CurseForge" are different problems.
	 */
	source: 'curseforge' | 'manual' | 'none'
	packs: PackSummary[]
	groups: PackGroup[]
	/** Instances that could not be read. One bad pack must not cost the rest. */
	warnings: string[]
}
