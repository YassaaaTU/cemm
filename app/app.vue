<template>
  <!--
    THESIS: CEMM is a modpack client, a peer to the tools its users already have
    open. It refuses both the themed novelty and the anonymous grey dev-tool.
    OWN-WORLD: Dark-first game-client shell in the CurseForge / Modrinth / Steam
    family. Near-black blue-grey ground, panels one step lighter, 8px radii,
    hairline rules, flat. Violet accent — deliberately neither Modrinth green nor
    CurseForge orange — so green, blue and red mean only added, updated and
    deleted. Archivo throughout; JetBrains Mono for versions and codes.
    STORY: The player sees exactly what an update adds, changes and destroys,
    acknowledges the deletions, and installs. The admin loads an instance,
    switches off what stays home, publishes, and copies a code to hand over.
    FIRST VIEWPORT: 36px custom title bar carrying the mark, an Install/Publish
    segmented control and real window controls; a 54px icon rail; then crumb,
    heading, step rail and the work, over a pinned action bar so the commit
    button never scrolls out of reach.
    FORM: Persistent-rail game-client shell, candidate 4 of the grounded list,
    built under the user's steer (sleek, modern, game, motion) with the craft bar
    set to CurseForge, Modrinth and Steam. Seed 1a1b65bc.
    FINISH: unreviewed and undocumented is unfinished; this build ends with the
    finish review, the verdict, and DESIGN.md
  -->
  <NuxtLayout>
    <NuxtPage />
  </NuxtLayout>
</template>

<script setup lang="ts">
// Hooks for app startup
onMounted(() =>
{
	const { checkForUpdatesOnStartup } = useUpdater()

	// Check for updates automatically on startup (desktop only)
	if (import.meta.client)
	{
		checkForUpdatesOnStartup()
	}
})
</script>
