<template>
  <div class="container mx-auto px-4 py-6">
    <div class="mx-auto">
      <div class="flex items-center gap-4 mb-6">
        <h1 class="text-3xl font-bold">
          Settings
        </h1>
      </div>

      <!-- GitHub Settings -->
      <div class="card">
        <div class="card-header">
          <h2 class="card-title">
            <Icon
              name="mdi:github"
              size="1.5rem"
              class="inline-block mr-2"
            />
            GitHub Configuration
          </h2>
        </div>
        <div class="card-body">
          <GitHubSettings />
        </div>
      </div>

      <!-- Theme Settings -->
      <div class="card mt-6">
        <div class="card-header">
          <h2 class="card-title">
            <Icon
              name="solar:palette-bold"
              size="1.5rem"
              class="inline-block mr-2"
            />
            Theme
          </h2>
        </div>        <div class="card-body">
          <fieldset class="fieldset">
            <legend class="fieldset-legend">
              Application Theme
            </legend>
            <select
              v-model="themeStore.current"
              class="select select-bordered"
            >
              <option value="nord">
                Nord
              </option>
              <option value="dracula">
                Dracula
              </option>
            </select>
            <p class="label">
              Choose between Nord (light) and Dracula (dark) themes
            </p>
          </fieldset>
        </div>
      </div>

      <!-- App Information -->
      <div class="card mt-6">
        <div class="card-header">
          <h2 class="card-title">
            <Icon
              name="mdi:information"
              size="1.5rem"
              class="inline-block mr-2"
            />
            About CEMM
          </h2>
        </div>
        <div class="card-body">
          <div class="space-y-2">
            <p>
              <span class="font-semibold">Version:</span> 1.0.0
            </p>
            <p>
              <span class="font-semibold">Built with:</span> Nuxt 3 + Tauri + Rust
            </p>
            <p>
              <span class="font-semibold">Purpose:</span> ChillEcke Modpack Manager
            </p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const themeStore = useThemeStore()
</script>
