# CEMM: reliable Fusion updates and official ATMons upgrades

Prepared 2026-11-15. This is an implementation handoff for Claude, not a claim that these improvements already exist or have passed acceptance.

## Start here

Improve the existing CEMM workflow so friends import a CurseForge export of ATMonsFusion once, then apply reviewed Fusion releases to that same profile using CEMM codes. The admin already uses a public GitHub repository and codes successfully. Preserve that working transport and user experience.

Official ATMons updates are merged by the admin into a new Fusion release. Players install the resulting release; they do not independently merge ATMons with Fusion. CEMM must apply the reviewed result safely. A general-purpose upstream merge editor is outside scope.

Read local project instructions, verify the evidence below against current code, then implement the unchecked tickets in order. Keep each ticket runnable and end-to-end. Record evidence and remaining gates here. Do not call the work complete while required acceptance is open.

Boundaries:

- Work in `D:/Projects/Rust/cemmV3`. Preserve its Nuxt/Vue, Tauri and Rust sidecar architecture, existing dependencies, public GitHub transport, legacy codes, and transactional installer. Use Bun.
- Treat the game installations and official downloads listed below as read-only references. Test against disposable copies. Do not launch or mutate the live client/server, upload releases, push Git commits, or publish CEMM/pack updates without explicit authorization.
- Do not read `.env`, `private.key`, key stores, or other credentials. Public release content must exclude credentials and personal/runtime data.
- Do not replace the launcher, add private-repository authentication, invent an in-game synchronization mod, build a server deployment manager, or rewrite working subsystems.
- Distinguish required fixes from optional convenience work. Prefer an admin runbook and a small existing-tooling extension over a new upstream-merge GUI. No unrelated UI redesign, refactors, or broad mod/library updates.

## Verified starting point and source map

Snapshot: CEMM package version `2.1.0`, Git HEAD `4546d94663070dc20d4f6ad1b096c0ff8d62d04f`. Recheck before implementation. The worktree was clean during this assessment.

Read `AGENTS.md` first. Its Graphify-first query requirement applies; the graph exists. Query relevant concepts before targeted source reads. Existing `.plan/` documents describe other work; inspect applicable constraints without reopening unrelated tickets.

| Source under CEMM root | Relevant behavior |
| --- | --- |
| `README.md`, `PRODUCT.md` | Existing admin/player workflow and constraints; product claims can lag implementation. |
| `src-tauri/src/composables/manifest.rs` | `Manifest`, `Addon`, config/custom-datapack payloads, `parse_minecraft_instance`, `resolve_install_baseline`. |
| `src-tauri/src/composables/github.rs` | `upload_update_with_progress`, `download_manifest`, `download_config_files`; current code paths and GitHub publication. |
| `src-tauri/src/installer.rs` | `install_update_with_progress`, validation, diffs, staging, journal, recovery, tests. |
| `app/composables/useAdminApi.ts`, `useUserApi.ts` | Admin capture/publish and player preview/apply orchestration. |
| `app/utils/modpackKey.ts` | Current pack key can derive from profile/folder name. Stable lineage must survive renames and official upgrades. |
| `app/components/features/admin/ConfigFilesSection.vue` | Admin file selection. |
| `app/components/features/user/UpdatePreview.vue`, `app/components/features/packs/PackUpdateDialog.vue` | Existing review and install flows. |

Already implemented: four CurseForge addon categories; custom datapacks; text/binary file transport; selective downloads; config-only updates that retain the addon baseline; reconciliation of CEMM state, CurseForge records and actual disk files; disabled-file detection; protected unmanaged removals; staged installation with interrupted-transaction recovery. Reuse these.

Current contract has important limits:

- `Manifest` requires the four addon arrays and `config_files`; `updateType` is optional, and `custom_datapacks` defaults empty. It has no explicit schema, minimum-client, exact official-base or complete managed-file policy fields. Serde can ignore unknown fields. Merely adding `minVersion` is not an old-client safety gate.
- Addon capture reads CurseForge metadata. A physical local JAR without an installedAddon record is not automatically represented as a CurseForge addon.
- Config file bytes are distributed and replaced. Transaction rollback does not preserve player preferences by itself, nor establish raw-file deletion/ownership semantics.
- Explicit, correct synchronization of CurseForge addon records and pins was not established by this audit. Verify the code before choosing the smallest fix.
- GitHub publication already assembles blobs/tree/commit and advances the branch once with force disabled. Preserve this atomic publication boundary. Downloads currently resolve files from raw GitHub paths; freeze a consistent release revision for manifest and payloads.

## Fusion reference material

| Role | Absolute path |
| --- | --- |
| Current Fusion client | `D:/Games/Curseforge/minecraft/Instances/All the Mons Fusion` |
| Installed official 2.0.1 reference | `D:/Games/Curseforge/minecraft/Instances/All the Mons - ATMons (1)` |
| Official server download | `D:/Games/MC/ATMons-ServerFiles-2.0.1` |
| Current Fusion server | `D:/Games/MC/ATMonsFusion-Server-2.0.1` |
| Latest change evidence under client | `tools/maintenance/2026-11-15/README.md`, `selected-updates.json`, `validation.json`, `delivery.json` |
| Authored quest conventions under client | `tools/quest-design/REFERENCE.md` |
| Inventory/recovery sources under client | `mod-inventory.json`, `manifest.json`, `tools/update_mod_inventory.py` |

Official baseline identity: CurseForge project `1356598`, client file `9571494` (`All-the-Mons-2.0.1.zip`), server file `9571523`; Minecraft `1.21.1`, NeoForge `21.1.265`. The installed official reference is useful evidence, but a launched installation can contain generated settings. Use immutable source archives and recorded hashes for authoritative upstream comparisons. Keep original archive bytes outside profiles; do not assume a folder named “official” is immutable.

Current Fusion maintenance commit: `1ca20a6`. At that checkpoint: 567 active client JARs and 518 server JARs. Counts are snapshot evidence, not permanent correctness constants.

| Custom mod | CF project | Installed file ID | File |
| --- | --- | --- | --- |
| Iron's Arms 'n Artifice | 1650193 | 9618954 | `irons_artifice-1.21.1-1.2.0.jar` |
| Scorched Guns unofficial port | 1709719 | 9625849 | `scguns-1.1.1.jar` |
| Thaumaturge Caster's Gauntlet | 1775220 | 9617906 | `casters_gauntlet-1.21.1-1.2.1.jar` |
| Create: Loconautics | 1793084 | 9627028 | `loconautics-0.1.3BETA.jar` |
| TaCZ | 1028108 | 9623169 | `tacz-neoforge-1.21.1-1.2.0-hotfix.jar` |

Critical exceptions:

- Keep the Aeronautics/FTB Chunks bridge project `1530014`, file `8420412`, `create_aeronautics_ftb_chunks-1.21.1-NeoForge-1.1.1.jar`, pinned. Do not silently upgrade it to 1.2.0. A future upstream dependency that conflicts with this pin requires admin resolution.
- Local `mods/atmons-ore-hammer-fix-1.0.0.jar` needs explicit capture, integrity, ownership and replacement/removal handling. It cannot disappear because it lacks a CurseForge record. Verify whether a future official release incorporates its fix before retiring it.
- Required TaCZ Tweaks setting: JSON pointer `/general/compatibility/sableCompat` in `config/tacztweaks.json` is `false`, avoiding overlap with native TaCZ Sable support. Preserve friends' `keyActions` and unrelated personal settings in that same file.
- Current libraries satisfy the five selected updates. Do not use this work to run “Update All.”

Recent operational evidence: CurseForge detected replaced JARs and changed `installedAddons` in the background. Embedded launcher/recovery inventories had drifted, including a missing Loconautics record. Read actual files and fresh metadata; never transplant the admin's entire `minecraftinstance.json` to a friend's machine.

## Release model

Keep three separate identities: stable Fusion lineage; exact official ATMons base; custom Fusion release. A new official base changes the second identity, not the lineage. A profile rename must not create a different pack identity.

Each new-format release describes a complete reviewed target state, even when download/install only transfers differences. Store enough verified local applied state to calculate differences and removals from any supported earlier Fusion release. Players can skip releases or apply the same code twice without needing every intermediate code.

Define and validate a minimal versioned contract before implementation. It must carry:

- Stable pack identity, unique immutable custom release identity, exact official project/file ID, Minecraft/loader requirements, client/server role and minimum supported CEMM/schema capability.
- Exact selected CurseForge project/file IDs, expected filenames, enabled state, integrity data, required dependencies and intentional pins. Prefer project identity over display name or filename for matching.
- An explicit managed-file inventory: normalized relative path, content hash/size, source, policy and role. Cover locally supplied binaries as well as scripts/configs/datapacks/assets.
- Required settings or safe file patches with preconditions, and bounded deletions/retirements. Track ownership of earlier versions so skipped releases also retire obsolete owned files.
- Compatible server release reference, release notes, known manual steps and unresolved limitations. A “full update” must not silently omit required files the admin forgot to select.

Use SHA-256 for release payload checks and preserve upstream file verification where available. Hashes establish content integrity against the chosen release, not publisher trust on their own. Retain existing repository/source and URL validation. Do not add a new signing infrastructure for this ticket.

Reject attempts to reuse a release code/path for different content. Resolve manifest and payloads from one immutable publication revision; do not mix files from changing `main` across requests. Avoid a self-referential commit hash inside a manifest included in that same commit; store publication revision/receipt outside that content or resolve it after publication.

Backward compatibility has two directions: the new app must read existing codes, and old apps must reject unsupported new payloads before mutation. Test the latter against a released old binary/parser. An ignored minimum-version field is insufficient. Choose the smallest safe envelope/code/version approach after inspecting the actual legacy parser. Never make an unsupported release look like a valid empty full update that removes the old pack.

Legacy config-only codes must still leave addon state intact. They must not falsely certify a complete modern Fusion target or erase its file-ownership history. Define the legacy-to-versioned-state transition explicitly; do not silently downgrade the state schema.

## File ownership and local preferences

Maintain a distinction between release target content and local observed/applied state; they have different meanings. Keep one authoritative local applied-state record instead of duplicate frontend/backend baselines.

| Content | Policy |
| --- | --- |
| Required mods/local fixes, authored KubeJS scripts/data/assets, quest definitions and locales, required datapacks/resource packs | Managed content with explicit source, integrity and conflict rules. Shared gameplay changes must actually apply before success. |
| Mixed config files | Enforce specific pack-required keys; preserve user-owned keys. Start with the actual JSON use case, not a generic parser framework. |
| Pack defaults for personal options | Apply initially or migrate when unchanged from the previous supplied default. Preserve intentional player changes. |
| Saves/worlds, player/team/quest progress, maps/waypoints, screenshots, personal controls/video/shader choices | Never captured as routine release content, replaced, or deleted. |
| `minecraftinstance.json`, machine Java paths/JVM arguments, account/auth files, secrets | Never distribute the admin's copies. Narrow local launcher bookkeeping is a separate validated operation. |
| Logs, caches, backups, generated `.snbt_merged` fragments, auto-exported gun-pack copies | Exclude. Include authored source gun packs only where actually needed and permitted, not the entire runtime folder. |

Inventory all actual content roots, including `kubejs/startup_scripts`, `server_scripts`, `client_scripts`, `data`, `assets`, FTB quest definitions/locales, `defaultconfigs`, global pack loaders and authored schematics where used. Assign roles rather than copying everything indiscriminately. Do not blanket copy server-only files into client profiles or rely on a hard-coded count of client-only mods.

Compare the previous supplied value, the current local value and the new supplied value when migrating personal defaults. Changed-on-both-sides is a visible conflict. Required gameplay drift cannot be silently skipped while marking the required release installed. Optional overrides may be retained only with their actual local state recorded distinctly from the release target.

Delete only paths proven owned by an earlier adopted/applied release, with expected-content preconditions. A locally modified owned file is a conflict, not permission to destroy it. An unknown extra JAR is not automatically owned; preserve it and report any actual incompatibility. Handle case-insensitive aliases, renames and `.disabled` predecessors so an update leaves one intended active version per project.

Initial adoption is essential: the first complete Fusion export contains files CEMM did not install. Verify the bootstrap release's known files and adopt those exact matches as managed, including official-base mods that may be removed later. Do not claim ownership of arbitrary extras. Drift requires repair/review, not wholesale deletion or reinstallation. Validate that the exported ZIP really includes the local ore-hammer JAR and all authored overrides; neither launcher metadata nor addon counts prove this.

## Admin procedure for an official ATMons update

Use these four states outside friends' installations:

- **B**: immutable previous official release, e.g. ATMons 2.0.1 file 9571494.
- **F**: last released Fusion state, plus separately reviewed unpublished admin changes if any.
- **N**: clean newly downloaded official release, with its own exact IDs and hashes.
- **R**: candidate next Fusion release, built from N and the reviewed Fusion customizations.

Keep baseline archives, source inventories and release receipts outside runtime profiles. Git is suitable for authored text/config history; avoid placing large JAR collections in ordinary source history. Existing download references and content hashes describe binaries. Track intentional custom deletions and upstream workarounds, not just added files.

Before merging, freeze F and identify dirty/unreleased changes. Review B→F customization and B→N upstream changes; create R in an isolated working copy. Never use CurseForge “Change Version” on the live Fusion profile as the merge algorithm.

| F compared with B | N compared with B | Candidate action |
| --- | --- | --- |
| Unchanged | Changed | Take N, including an upstream deletion. |
| Changed | Unchanged | Keep F, including an intentional Fusion deletion. |
| Identical to N | Same result | Keep one copy. |
| Both changed differently | Conflicting | Admin resolves before release. |

Treat absent paths explicitly: delete/modify, independently added different files at the same path, binary replacements, renamed paths and changed arrays cannot be guessed away. Disjoint JSON key changes can merge only with tested semantics. JS, SNBT, TOML, binaries and semantic quest changes require review unless a narrow existing adapter proves correctness. A clean text merge is not proof of a correct recipe or dependency graph. No conflict markers or unresolved decisions may reach a release.

For addons, compare by project/loader identity and exact selected files. If N absorbs a custom addon, deduplicate it and decide whether its override/workaround is still necessary. If N removes a library still required by Fusion, resolve the dependency. If N changes an API used by scripts/addons, validate those integrations before promotion. Do not automatically select the higher version across incompatible forks or override explicit pins.

For quests, preserve authored IDs, links and player progress contracts. If N removes/renames upstream quest IDs referenced by custom chapters, or changes task semantics, make an explicit migration decision or hold the release. Preserve player/team data and reward claims; never replace it with admin test state. Native FTB serializers can reorder keys and omit default values: classify recognized formatting drift during admin QA without weakening byte-integrity checks for transport. Do not build a generic world/SNBT migration engine into CEMM.

Previous server assembly exposed two concrete traps: union-copying official split locales alongside Fusion consolidated locales left redundant fragments; generated `.snbt_merged` files were copied too. Windows case variants also aliased paths. Build one reviewed native quest file set, excluding derived caches, with unique normalized paths. Use current definitions as the expected set; do not permanently hard-code the earlier 172-file snapshot.

`defaultconfigs` templates are not a reliable mechanism for changing settings in an already existing world. Inspect actual NeoForge/mod config precedence for the selected version. If a required change belongs in active world SERVER configs, make a separate targeted, backed-up admin migration; do not ship or overwrite whole worlds. Keep host-local `server.properties`, EULA and Java launcher choices local.

Minecraft/loader changes need an explicit migration gate. CEMM must not pretend to install a loader by editing `baseModLoader` JSON or dropping JARs. For a same-Minecraft release requiring a different loader, provide a verified launcher-supported procedure or block with exact instructions. A major Minecraft transition may require a deliberately managed profile migration; do not promise the initial one-import workflow covers unsupported launcher changes.

Rebuild and validate the matching server candidate using its official server base and Fusion server-role changes. Never copy the entire client `mods` directory into it. Server packaging/deployment stays in the existing admin tooling. Record the compatible server release in R, then promote B to N and F to R only after required validation and deliberate release publication. Preserve previous baselines and receipts for recovery.

## Player install transaction

1. Select the existing profile and resolve one immutable release. Check lineage, role, schema/client capability, Minecraft/loader and supported starting state before planning writes. An adopted older Fusion base may migrate to R when explicitly supported; an unrelated official profile must not pass merely because its Minecraft version matches.
2. Require Minecraft to be closed and coordinate with CurseForge. Acquire a per-profile updater lock; detect concurrent metadata/file changes with fresh reads and preconditions. An advisory CEMM lock does not lock CurseForge. Do not kill either app automatically.
3. Preview additions, replacements, retirements, required setting changes, preserved preferences, pins and conflicts. Resolve required conflicts before applying. If a download needs manual action, provide a complete supported path or block before changes; never mark a missing required mod installed.
4. Stage and verify every required payload, bounded deletion, config patch and local metadata change. Preserve existing URL/path validation, including canonical containment through junctions/symlinks. Reject traversal, absolute paths, NTFS alternate streams, reserved paths and case collisions. Enforce the protected-data policy in the backend too.
5. Reuse/extend the existing journal and rollback so addon changes, managed raw files, deletions, narrow launcher metadata and applied release state commit together. Recheck expected live files immediately before promotion. Recover interrupted installs through the existing machinery. Clearly report interference/failure; never advance success state early.
6. Preserve profile identity/name/path, personal settings, JVM/memory choices and unknown launcher fields. Reconcile only the necessary installedAddon versions/enabled state and approved pins, using the current on-disk metadata. Keep the recovery manifest consistent where it is an actual consumed source. Validate with a real CurseForge reopen/rescan; if reliable pin synchronization cannot be proved, make the required manual pin step explicit and keep that acceptance gate open.
7. Verify resulting required inventory/settings, retain a usable pre-update backup and record the actual applied state. Reapplying the same code should do no unnecessary work. Show a useful completion result and compatible server release.

Keep transaction recovery separate from world rollback. Once Minecraft opens and saves a world with changed content, reverting mod files does not make downgrading that world safe. Back up valuable worlds before first launch, and restore the matching whole world backup if a downgrade is required. Never upload player worlds as CEMM release content.

## Implementation tickets

Refine details after source review and record justified changes here. Each ticket needs narrow meaningful tests and a complete supported interaction; hide unsupported flows until they work. Reuse current fixtures/tests rather than creating a parallel test framework.

- [x] **1. Release identity, compatibility and adoption.** Versioned contract, stable lineage, wrong-pack/loader checks, old-client rejection, legacy-code reads, and adoption of an already imported Fusion profile. Preview and persisted state agree; no needless re-download or new profile.
- [x] **2. Complete managed inventory and file lifecycle.** Capture all required content, including the local JAR; pinned addon identity; hashes; explicit ownership; safe rename/deletion/disabled-file handling. Full target releases support skipped versions and idempotence. Config-only legacy behavior remains correct.
- [x] **3. Required settings and personal preferences.** Implement the narrow mixed-JSON policy needed for TaCZ Tweaks plus clear whole-file/default policies. Required conflicts stop completion; personal keybinds survive. Unsupported format/merge cases receive a precise resolution path instead of guessed edits.
- [ ] **4. Transaction and launcher consistency.** Extend existing staging/journal to the new operations and applied state. Add concurrency preconditions, narrow local CF bookkeeping and pin behavior, and verified recovery. Do not introduce a second installer.
- [x] **5. Admin release workflow.** Implement only missing capture/preflight/publication support and document B/F/N→R review using existing tools. Freeze complete reviewed payloads at one immutable revision, prevent code reuse, pair server release, and provide a clear unresolved-conflict report. No automated semantic upstream merge GUI.
- [x] **6. Disposable Fusion acceptance and operator runbook.** Exercise real CEMM publish/download/install APIs against test storage or an explicitly authorized test repo, one imported fixture profile and representative local data. Document first import, routine code update, official upgrade, backup/recovery and manual gates. Do not publish to the group's production repo as a test.

Targeted checks: existing Rust tests around manifest baseline, installer transactions/config-only behavior and GitHub code handling; `bun test` for changed orchestration; `bun run typecheck`, relevant lint and Rust checks for changed code. Run builds where bindings/wiring change and one meaningful native app smoke. Give commands explicit timeouts; use mocks/fixtures for deterministic network failures. Do not substitute a web-only preview for the native sidecar/install flow. Keep runtime evidence distinct from static tests.

## Required acceptance matrix

These are behavior gates, not a requirement for a separate test per row. Combine scenarios into a few focused fixtures and one realistic roundtrip.

| Scenario | Required result |
| --- | --- |
| Import the initial exported Fusion ZIP and adopt it | Same profile retained; known files verified/owned; local JAR present; unrelated extras and personal data unchanged. Export contents checked, not inferred. |
| Five selected updates plus compatibility config | Correct IDs/hashes; bridge stays 1.1.1 and pinned; required compatibility flag false; keyActions unchanged. |
| Skip multiple releases; reapply same code | Arrive at the same complete required target, retire obsolete owned files, no duplicate active JARs, second application idempotent. |
| Existing legacy full/config-only codes | Existing behavior preserved safely; config-only never removes addons, erases ownership or claims a complete unsupported target. |
| Old released client opens new unsupported code | Clear failure before writes; not an empty-manifest install. New client handles its supported schema and gives a useful upgrade message. |
| Wrong lineage, role, unsupported base/MC/loader | Block before mutation; supported old Fusion→new Fusion transition still works. |
| Upstream-only change; Fusion-only change; both equal | Apply the three-way matrix accurately, including intentional deletions. |
| Both change differently; delete/modify; binary conflict | Admin resolution required; no guessed merge or publishable conflict markers. A synthetic N fixture tests this without pretending a future official release was tested. |
| Upstream absorbs custom addon/fix or removes needed library | One compatible resolved stack, redundant workaround explicitly reviewed, required dependency retained/replaced. |
| Player changes required file vs personal option | Required drift is a conflict/approved repair; user-owned option survives; state reports the truth. |
| Raw-file deletion/rename, disabled predecessor, case alias | Only verified owned content retired; unknown/modified file protected; no path aliases or unexpected activation. |
| Interrupted download, wrong hash, missing manual download | No partial success; old live state preserved, or clearly recoverable interruption. |
| Crash during promotion / concurrent CurseForge write | Recovery restores a consistent state; no silent lost launcher edits; exact outcome reported. |
| Reopen CurseForge after a successful install | Correct installed versions/enabled state and preserved profile identity; bridge pin remains effective. |
| Quest definitions and progress | Stable IDs/links; valid native parsing; existing team progress and claimed rewards survive; no accidental reward duplication/reset. |
| Existing-world server settings and downgrade | Explicit config migration where needed; matching world backup required; client installer never rewrites world data. |
| Rebuild candidate and compare approved target | Required inventory, dependencies, scripts/recipes, roles and release receipt agree; generated junk and secrets absent. |

For Fusion release QA, load/save/shut down a disposable matching dedicated server, launch the matching real client and join it. Check native TaCZ moving-structure collision/first-person rendering, an Artifice gun/addon tooltip, Scorched magazine reload, gauntlet search/research, Loconautics train assembly and old coloured parts. Recheck KubeJS errors, changed recipes, quest dependencies/task/reward IDs and claimed-reward persistence. A representative established-world copy is necessary where content migration is in question. Do not expand this into exhaustive unrelated gameplay testing.

At the current Fusion checkpoint the dedicated saved-world boot passed; real-client/player interaction gates remain open. Scorched's documented per-round unload/reload/casing exploit remains a known issue. Preserve that release note; CEMM work does not fix it. Do not relabel historical preflight counts as current full-pack certification.

## Completion report

Return the implemented tickets, relevant file links, focused test evidence, the immutable test release identity and before/after fixture results. State any manual gates precisely. Include a short admin/friends runbook. No public upload, production install or “flawless” claim is implied by a passing unit test. The work is ready only when the relevant native roundtrip and failure/recovery gates above pass; otherwise leave the affected ticket open.

## Implementation record — 2026-11-15

Implemented on branch `feat/release-lifecycle`, as a series of small commits ending in `chore(release): 2.2.0`. Nothing was pushed, tagged or published. The live Fusion client and server were only read: every write happened in temporary copies or throwaway profiles.

The pack itself moved during this work, by its admin: Epitaphs was replaced by Graveless, LambDynamicLights and its Lanterns addon were added, a second local fix (`fusion_waystones_fix-1.0.0.jar`) appeared, and the pack was given its own `cemm-pack.json`. The snapshot counts at the top of this document (567 jars, commit `1ca20a6`) describe the pack before that. The final acceptance run below used the pack as it stands now, and its own `cemm-pack.json`.

### Source findings

All confirmed against the code at `4546d94`. Additional findings that shaped the design:

- Every released parser (1.6.0 through 2.1.0) requires `mods`, `resourcepacks`, `shaderpacks`, `datapacks` and `config_files` as arrays and ignores unknown keys.
- No code wrote `minecraftinstance.json`. CurseForge's pin is the entry's `isLocked`; entries carry `fileLength`, a SHA-1 and a `packageFingerprint`.
- The Fusion instance has `projectID 0` and no `installedModpack`, so the official base cannot be read from the profile. The official profile carries `1356598 / 9571494`.
- 531 generated `.snbt_merged` files sit beside the quest definitions.
- No exported Fusion client ZIP exists on disk.
- `UserPanel` only downloaded payload files when `config_files` was non-empty, so a legacy update carrying only custom data packs installed none of them. Fixed.
- A legacy publish could overwrite an existing code's folder. Now refused.

### Decisions (justified changes to the plan above)

1. **Envelope.** A release folder holds `cemm-release.json` (schema 2) and a `cemm-manifest.json` tombstone whose `mods` is a sentence. Old parsers fail on it before a manifest exists and quote the sentence. A `minVersion` key alone would have been ignored.
2. **Compatibility gate.** `schema` plus a `requires` capability list, read from a lenient header before the rest is parsed. `min_cemm` is for people.
3. **Identity.** `lineage` is admin-stated in `cemm-pack.json` and is the repository folder; never derived from a profile or folder name. `base` is the exact official project/file. `release_id` is unique per lineage; `sequence` orders releases.
4. **Immutable revision.** The code is `lineage/release@commit`. Document and payloads are read at that commit. A short code is pinned to the branch head when fetched. The applied state stores the document digest, so the same id resolving to different content is refused.
5. **Payloads.** Content-addressed objects at `{lineage}/objects/{aa}/{sha256}`; only bytes a release introduces are uploaded. A per-file `hosted` flag separates "inventoried and owned" from "uploaded", so content that may not be redistributed can still be verified. Changed content must be hosted.
6. **History in the document.** `previous` (earlier hashes of a path) and `retired` (former content, by exact hash) let a profile that skipped releases, or was imported and never adopted, converge without every intermediate release.
7. **One local record.** `.cemm/state.json` is authoritative. `cemm-manifest.json` is still written, as a projection, so legacy previews keep working. A legacy config-only code on a release profile is recorded as an overlay; a legacy full code is refused there.
8. **Mixed files.** A file with required settings is shared: the pack sets the listed JSON keys and never offers the whole file for replacement. One format (JSON), one operation; anything else blocks with the value to set by hand.
9. **Launcher records.** Narrow edits to entries that already exist (file id, name, size, fingerprint, enabled, lock), removal of entries for retired addons, nothing synthesised for new addons. Real instance documents round-trip byte-for-byte, so an edit changes only its own lines.
10. **Review equals install.** The plan carries a fingerprint; apply replans from fresh reads and refuses a different one. Files about to be replaced or removed are rechecked immediately before the first move. A launcher edit made in between is kept if it does not change the plan and stops the update if it does.
11. **Interrupted installs are rolled back at review time**, not only at install time, so the review never describes a half-installed profile.
12. **Backups.** Kept whenever pack content or the launcher record changes, including when the only change is one the player approved. Three most recent.
13. **Adoption of a linked profile is the admin's call.** A profile CurseForge still links to the official pack is refused unless the pack sets `adoption.allow_official_profile`; the player is then told not to let CurseForge update it. This replaces guessing what a CurseForge import looks like.
14. **Rate limits.** Blob uploads wait out GitHub's limits for as long as GitHub says, in bounded steps, and give up with nothing published after about forty minutes.
15. **Test storage.** `CEMM_DEV_GITHUB_API`, `CEMM_DEV_GITHUB_RAW`, `CEMM_DEV_ADDON_ORIGIN` are honoured only when `debug_assertions` is on. Release builds never read them.
16. **Dependencies.** `sha2` (already in the lockfile) and `serde_json`'s `preserve_order`. No new crates.
17. **Upgrade review, not merge.** A three-way report over B, the pack and N, driven by the pack's own `cemm-pack.json`. It proposes and lists conflicts; the admin builds R.
18. **Version.** CEMM 2.2.0 is the first version that reads releases; a test keeps `Cargo.toml`, `package.json` and `tauri.conf.json` in agreement and at or above the version releases tell players they need.

### Ticket state

| Ticket | State |
| --- | --- |
| 1 Identity, compatibility, adoption | Done. Native tests, the released 2.1.0 binary, the read-only check against real GitHub, and the real-pack copy all pass. |
| 2 Inventory and file lifecycle | Done. |
| 3 Settings and preferences | Done. |
| 4 Transaction and launcher consistency | Implemented and tested, including simulated launcher writes during an update. **One gate needs a person:** restart CurseForge after an install and confirm versions, enabled state and the two pins. Left unticked for that reason, as this document asks. |
| 5 Admin workflow | Done against stand-in storage; the read side verified on real GitHub. The first real publish is the admin's to make. |
| 6 Acceptance and runbook | Done: synthetic and real-pack native roundtrips, a smoke in the real Tauri window, and `docs/releases.md`. |

### Acceptance matrix

| Scenario | Result |
| --- | --- |
| Import and adopt | Passes on fixtures and on a copy of the real pack. A real CurseForge import of a real export was not possible: no export exists. `docs/releases.md` describes how to check one in a minute. |
| Five selected updates plus compatibility config | Passes on a copy of the real pack, jars from the real CDN. |
| Skip releases; reapply | Passes (unit and native). |
| Legacy full / config-only codes | Passes (unit and native). |
| Old released client | Passes on the installed 2.1.0 binary and a frozen copy of its parser. |
| Wrong lineage, role, base, MC, loader | Passes. |
| Three-way: single-sided, both equal | Passes, including deletions. |
| Three-way: conflicts | Passes with a synthetic N. No real future official release was tested. |
| Absorbed addon / removed library | Reported for review; passes. |
| Required file vs personal option | Passes. |
| Deletion, rename, disabled predecessor, case alias | Passes. A case-only rename ends under one name; the branch for case-sensitive filesystems is reasoned and compiled only on Linux/macOS CI, not run here. |
| Interrupted download, wrong hash, manual download | Passes (unit and native). |
| Crash during promotion / concurrent CurseForge write | Recovery and both interference windows pass, with the launcher's write simulated at the exact moment. A real CurseForge writing concurrently was not staged. |
| Reopen CurseForge after install | **Needs a person.** |
| Quest definitions and progress | Definitions travel as hashed managed files; player data is protected by path. In-game check **needs a person**. |
| Existing-world server settings and downgrade | Documented; downgrade is blocked. No server work was done here. |
| Rebuild candidate and compare | Passes: a copy that matches plans zero changes; generated files and secrets are absent from what is uploaded. |

### What still needs a person

1. **CurseForge restart check.** After the first real install: restart CurseForge, open the profile, confirm the five mods show their new versions and that the bridge and Waystones are locked. Then **Check again** in CEMM: zero changes and "stays locked" means CurseForge kept every edit.
2. **Waystones is not locked in the admin's own profile.** The pack pins it; capture warns about it. Lock it in CurseForge.
3. **Check the export** before handing it out, as `docs/releases.md` describes.
4. **First publish**, to a test repository first. A first Fusion release is about 6,200 uploads and 143 MB.
5. **In-game QA** of the Fusion release, per the checklist above.
6. **Permission.** Publishing ATM-derived scripts, quests or configs to a public repository is the question in the pack's `sharing-assessment.md`. `"distribute": false` inventories content without uploading it.

### Evidence

Commands are run from `src-tauri` unless noted. All were run on the final code.

| Check | Result |
| --- | --- |
| `cargo fmt -- --check`; `cargo clippy --all-targets --all-features -- -D warnings` | Clean. |
| `cargo test` | 248 library tests, `release_roundtrip` 1, `sidecar_lifecycle` 3. All pass. |
| `bun run lint`; `vue-tsc --noEmit` under real Node; `bun test`; `bun run generate` | Clean; 20 tests pass, including one asserting every command the frontend invokes is registered; builds. |
| Each commit of the series | `cargo check --all-targets` passes on every commit; clippy and tests on the tip. |
| Released binary: `CEMM_OLD_BINARY="C:/Program Files/cemm/cemm.exe" cargo test --test release_roundtrip -- --ignored a_released_old_binary` | The installed CEMM 2.1.0 rejects the tombstone (quoting "This update needs CEMM 2.2.0 or newer") and the release document on both the preview and install paths, and writes nothing. |
| Real records: `CEMM_REFERENCE_PROFILE=<profile> cargo test --lib -- --ignored the_fingerprint_agrees a_real_instance_document` | Fingerprint and length agree with every record of the Fusion and official profiles; their `minecraftinstance.json` files render byte-for-byte. |
| Real GitHub, read-only: `cargo test --lib -- --ignored the_real_github` | On a public third-party repository: a branch resolves to one commit id; a file is readable at that commit; a missing path is 404; the tree lists recursively with a completeness flag; the contents check answers 200 and 404. |

**Native roundtrip** (`release_roundtrip::a_pack_is_published_adopted_updated_and_recovered_through_the_real_binary`). The compiled binary in sidecar mode, real HTTP to an in-process stand-in for the Git Data API, the raw host and the CDN, throwaway profiles. Publishes four releases, one of them through three rate-limit refusals; shows what a legacy client reads; adopts an import with a short code; skips a release; reapplies; fails cleanly on a wrong hash, an oversized file and a truncated download; refuses a republished id; detects a release whose content changed under its code while the pinned code still resolves to the original; refuses an unrelated profile; publishes nothing when the branch update is rejected; carries legacy codes over the same transport.

**Native app smoke.** The debug Tauri window itself, driven through WebView2's debugging port, with its browser profile redirected to a temp folder. 20 steps, all passing: the window loads; real IPC is present; a player adopts one release and updates to the next through the screen, with install gated on the acknowledgement; the result is confirmed on disk (jars, script, required flag set, keybinds kept, personal files untouched, launcher record updated and pinned, backup kept); an admin checks a release through the screen, and an unlisted jar blocks it. Publish was not clicked in the window, so the real token in the keyring was never read; the stand-in saw only the script's dummy credential.

**Real pack, in copies** (`CEMM_FUSION_SOURCE=<Fusion client> cargo test --test release_roundtrip -- --ignored --nocapture a_copy_of_the_real_pack`). Reads the client; copies 11,364 files (1.9 GB) into temp folders; uses the pack's own `cemm-pack.json`; downloads from the real CurseForge CDN.

- Test release identity: `atmons-fusion/acceptance-1@e43d6a3fd0223c2303aa0950a6ceaada7be425ea`, on the stand-in storage only. Nothing reached GitHub.
- Capture: 578 addons, 2 pins, 8,985 pack files, 531 generated quest fragments left out, 6,198 distinct payloads (143 MB). No errors. Both local jars (`atmons-ore-hammer-fix`, `fusion_waystones_fix`) are in the release as hosted local binaries. Warnings: Waystones is pinned but not locked in the admin's profile; 4 unrecorded shader variants are treated as personal; 13 addons list a dependency that is not in the pack.
- The admin copy plans zero changes against its own release.
- Friend copy **before**: the five previous jars and matching launcher records, bridge unlocked, `sableCompat` true, its own `keyActions`, plus an options file, a world and a mod of its own.
- Plan: adopt; exactly 5 downloads (90.1 MB); 11 changes; no blockers; no conflicts.
- Friend copy **after**: each of the five jars has the SHA-256 that `selected-updates.json` recorded independently, and the file id in its launcher record matches; the five previous jars are gone from `mods/` and present in the retained backup; the bridge jar is byte-identical, still file `8420412`, and now locked, as is Waystones; `sableCompat` is false and `keyActions` is unchanged; the options file, the world and the friend's own mod are byte-identical; the 570 active jars equal the admin copy's; both local jars equal the admin's; profile name and memory setting are unchanged.
- The same code again: reapply, zero changes, zero downloads.

### Files

- Contract, gate, paths, hashing, three-way: `src-tauri/src/composables/release.rs`
- Capture, preflight, upgrade review: `src-tauri/src/composables/release_capture.rs`
- State, plan, apply, settings, launcher record: `src-tauri/src/release_install.rs`
- Transaction preconditions, retained backup, profile lock: `src-tauri/src/installer.rs`
- Pinned fetch, objects, release publish, rate limits: `src-tauri/src/composables/github.rs`
- Sidecar wiring: `src-tauri/src/service/protocol.rs`, `server.rs`, `src-tauri/src/service_commands.rs`
- Player: `app/components/features/user/ReleasePreview.vue`, `UserPanel.vue`, `app/composables/useUserApi.ts`, `app/stores/release.ts`
- Admin: `app/components/features/admin/ReleasePublishSection.vue`, `AdminPanel.vue`, `app/composables/useReleaseAdmin.ts`
- Runbook: `docs/releases.md`; the pack's description as an example: `docs/examples/cemm-pack.fusion.json`
