# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

<!-- Rationale: CEMM ships as a Tauri 2 desktop app for Windows and Linux, but the
     UI is Nuxt 4 + Vue 3 rendered in a webview. Per init.md step 2, a native
     wrapper around a web UI does not make the design language native — so the
     platform value stays `web`. The *target feel* is a native desktop
     application; that is a design goal recorded below, not a platform switch. -->

## Users

Two audiences, both to be treated as **non-technical** (confirmed by the user):

- **Players ("user mode")** — friends of the admin who already have a CurseForge
  modpack installed and want to receive the admin's custom modifications. Their
  situation: they were handed an "update code" over chat and want their game to
  match everyone else's before the group plays. Their job: paste the code, point
  at the right folder, understand what is about to change, and install it without
  breaking their existing modpack.
- **Admin ("admin mode")** — in practice a single maintainer (the project author)
  who curates the group's modpack. Their situation: they have locally modified a
  downloaded CurseForge modpack and need to publish that delta. Their job: load
  the instance, decide which addons and config files to share, publish to GitHub,
  and hand out the resulting update code.

The user explicitly chose to design for **both audiences as non-technical**:
neither side is assumed to read documentation, so guidance, plain language, and
confirmation of destructive actions apply to admin flows as well as player flows.

## Product Purpose

CEMM (ChillEcke Modpack Manager) distributes *modifications to an existing
CurseForge modpack* among a small group of friends, without republishing the
whole modpack and without anyone hand-copying files.

Success is a player going from "admin sent me a code" to "my game matches the
group's" in one pass, with no corrupted modpack and no manual file surgery — and
the admin publishing a change set without touching the GitHub web UI.

## Positioning

CurseForge and Modrinth distribute whole modpacks; CEMM distributes the *diff* on
top of one. The mechanism a neighboring product could not truthfully copy: CEMM
reads a real local `minecraftinstance.json`, computes an addon-level and
config-file-level delta against the previously installed manifest, publishes that
delta to a GitHub repository the admin owns, and reduces the entire distribution
step to a short shareable code (`modpack-key/uuid`). Distribution runs on
infrastructure the group already has, with no CEMM-operated server.

## Operating Context

**Admin workflow**
1. Select the modpack directory containing `minecraftinstance.json`.
2. Load the instance → CEMM generates `manifest.json` (mods, resourcepacks,
   shaderpacks, datapacks, each with project id, file id, version, CDN URL, and
   the exact on-disk filename).
3. Optionally exclude individual addons from the upload while keeping them
   locally (server-side or platform-specific mods).
4. Optionally select config files to distribute (text or binary).
5. Optionally set a custom modpack name (becomes the `modpack-key`).
6. Upload to GitHub → receive an update reference to share.

**Player workflow**
1. Set the GitHub repository to pull updates from.
2. Point at the modpack directory to update.
3. Paste the update code (`modpack-key/uuid`, or a bare UUID for older updates).
4. Download the manifest and preview the diff: new / updated / removed /
   unchanged addons, plus incoming config files.
5. Confirm — explicitly acknowledging destructive changes — and install.

**Environment:** a desktop app running beside the Minecraft launcher and a chat
client. Sessions are short and goal-directed. Filesystem access, GitHub HTTP
calls, and the install itself run in Rust via Tauri commands; install progress
streams back over a Tauri `install-progress` event.

## Capabilities and Constraints

**Confirmed functionality**
- Addon categories: mods, resourcepacks, shaderpacks, datapacks.
- Addon exclusion, persisted per session, with bulk exclude and clear-all.
- Config file distribution, including binary files (base64 data URIs).
- Two update types: `full` (addons + config) and `config` (config only).
- Diff computation matches updated addons by `addon_project_id`, removals by
  addon name, and resolves files on disk by `fileNameOnDisk`.
- Detection of `.disabled` addon files.
- GitHub token stored in the OS keyring via `tauri-plugin-keyring-api`; only the
  admin needs one.
- Self-update via `@tauri-apps/plugin-updater` against the GitHub releases feed.

**Technical constraints**
- Nuxt 4 with `ssr: false` and the `static` Nitro preset; Vue 3, TypeScript,
  Tailwind CSS v4, Pinia (persisted to `localStorage`), Tauri 2, Rust backend.
- **DaisyUI 5 is a binding constraint** — see Brand Commitments.
- The app must work fully offline apart from the GitHub calls it explicitly
  makes. Icons are already bundled locally (`@nuxt/icon` with `provider: 'none'`).
  Fonts currently load from Google Fonts over the network and must be bundled to
  satisfy this constraint.
- `data-theme` on `<html>` in `nuxt.config.ts` must agree with the theme store
  default and the DaisyUI `--default` theme, or the app ships a theme the
  compiled CSS has no rules for (recorded as finding F-P2-6).
- Tauri window: 1200×800 default, 800×600 minimum, resizable, centered.
- Installs are destructive and irreversible: removed addon files are deleted from
  disk. Any UI that triggers one must say so before it happens.

**Undecided / explicitly open**
- macOS is untested and unclaimed. Windows and Linux are the supported targets.

## Brand Commitments

- **Name:** CEMM — ChillEcke Modpack Manager. Author: YassaaaTU (YasirJumaah).
  MIT licensed.
- **DaisyUI 5 must remain the component layer.** The user stated this twice and
  it overrides any recommendation to replace it. Impeccable-grade craft is to be
  implemented *with* DaisyUI 5, not instead of it.
- **The app must read as a native desktop application on Windows and Linux**, not
  as a website in a window. Confirmed decisions serving this:
  - Custom frameless window chrome on **both** operating systems, with a real
    title bar carrying window controls and drag regions.
  - Light/dark follows the **OS preference** by default, with a manual override
    available in settings.
- **Craft bar: CurseForge, Modrinth and Steam.** The user named these as the
  products CEMM should sit alongside. They set the quality standard and the
  interaction vocabulary: dark-first, dense content rows with icons and
  filenames, a narrow icon rail, toggles for enable/disable, pill filters.
  This is a standing preference, not a one-off styling note.
- **No literal metaphors or themes, and nothing retro.** Four themed directions
  were dealt and rejected; the user's stated objection was that dressing a
  utility as an object reads as costume. CEMM should look like good software.
  The accent is violet specifically so it is neither Modrinth's green nor
  CurseForge's orange — a peer, not a clone.
- **The pre-1.7.0 visual design is anti-reference, not a starting point.** The
  user explicitly rejected incremental refinement of the old look.

## Evidence on Hand

- Working product at v1.7.0 with a completed full-stack audit
  (`plans/CEMM_FULLSTACK_AUDIT.md`); only the 1.7.0 release ticket is outstanding.
- Real domain types in `app/types/index.ts` and a real Rust backend in
  `src-tauri/`.
- Accessibility work already landed and must not regress: a single `<main>`
  landmark with a skip-link target, `sr-only` page headings, `aria-current` on
  navigation, `role="alert"` on status regions, and two measured contrast fixes
  documented in `app/assets/css/design-tokens.css` (finding F-P3-7).
- Motion tokens, a `prefers-reduced-motion` media override, and a user-level
  `.motion-reduced` class override already exist and are to be reused.
- No testimonials, usage metrics, customer names, or press exist. Future work
  must not fabricate any.

## Product Principles

1. **Never let a destructive install surprise anyone.** The diff is the product's
   most important screen; deletion must be legible, specific, and acknowledged
   before it runs.
2. **The code is the handshake.** Producing, sharing, and redeeming the update
   code is the spine of the product — every surface should make the current step
   of that handshake obvious.
3. **Assume no documentation was read.** Both roles get plain language and
   in-context guidance; nothing depends on the README.
4. **Offline-first, local-first.** The filesystem and the OS keyring are the
   sources of truth; the network is used only for explicit GitHub operations.
5. **Respect the desktop.** Behave like an installed application — OS theme, real
   window controls, keyboard access, and no web-page idioms.

## Accessibility & Inclusion

Existing commitments are binding and must not regress: WCAG AA contrast (with two
specific ratios already measured and corrected), a single `<main>` landmark with
a skip-link target, `aria-current` navigation state, `role="alert"` status
messaging, and both media-query and user-level reduced-motion support.
