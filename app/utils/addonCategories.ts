/**
 * The four content categories a manifest carries, and the only place they are
 * named for the screen.
 *
 * The keys are the `Manifest` field names, so a row can always be traced back
 * to the array it came from, and renaming a category in Rust breaks the type
 * here rather than silently mislabelling rows. The labels are single-sourced
 * because the same addon must not be a "Resource pack" on the admin's counter
 * and a "Texture pack" on the player's.
 */
export type AddonCategory = 'mods' | 'resourcepacks' | 'shaderpacks' | 'datapacks'

/** Fixed order, everywhere a category list is rendered. */
export const ADDON_CATEGORIES: AddonCategory[] = ['mods', 'resourcepacks', 'shaderpacks', 'datapacks']

const CATEGORY_LABELS: Record<AddonCategory, string> = {
	mods: 'Mods',
	resourcepacks: 'Resource packs',
	shaderpacks: 'Shaders',
	datapacks: 'Data packs'
}

/**
 * What one addon of that category is called. A row states its own kind, so it
 * needs the singular — "Mod", not "Mods".
 */
const CATEGORY_NOUNS: Record<AddonCategory, string> = {
	mods: 'Mod',
	resourcepacks: 'Resource pack',
	shaderpacks: 'Shader',
	datapacks: 'Data pack'
}

/** Heading and pill form: "Resource packs". */
export const categoryLabel = (category: AddonCategory): string => CATEGORY_LABELS[category]

/** Row form: "Resource pack". */
export const categoryNoun = (category: AddonCategory): string => CATEGORY_NOUNS[category]

/**
 * Mid-sentence count, lowercased: "1 resource pack", "3 resource packs".
 * Used for the deletion breakdown, which is prose rather than a column.
 */
export const categoryCountPhrase = (category: AddonCategory, count: number): string =>
	`${count} ${(count === 1 ? CATEGORY_NOUNS[category] : CATEGORY_LABELS[category]).toLowerCase()}`

/**
 * "12 mods, 3 resource packs and 2 data packs" — an Oxford-comma-free list,
 * because this reads as a sentence under a headline, not as a legal clause.
 */
export const joinPhrases = (phrases: string[]): string =>
{
	if (phrases.length === 0) return ''
	if (phrases.length === 1) return phrases[0] ?? ''
	return `${phrases.slice(0, -1).join(', ')} and ${phrases[phrases.length - 1]}`
}
