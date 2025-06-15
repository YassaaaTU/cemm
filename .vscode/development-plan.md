# CEMM (ChillEcke Modpack Manager) - Simplified Development Plan

## Project Overview

**CEMM** is a lightweight desktop application with two modes:
- **Admin Mode**: Generate manifest.json from minecraftinstance.json → Upload to GitHub with UUID
- **User Mode**: Input UUID code → Download and apply changes from GitHub repo

## Core Workflow
1. **Admin**: Creates modpack modifications → Generates manifest → Uploads to GitHub with UUID
2. **User**: Receives UUID + repo name → Downloads and installs changes

## Technology Stack
- **Frontend**: Nuxt 3 + TypeScript + TailwindCSS + DaisyUI + Pinia + Pinia Persist
- **Backend**: Tauri + Rust
- **Storage**: Public GitHub repository

## UI Architecture Update (June 2025)
- The application now uses a **single-page, component-driven UI** for mode selection and workflow.
- The landing page (`index.vue`) displays a `ModeSelector` and dynamically renders either the `AdminPanel` or `UserPanel` component based on the selected mode.
- There are no longer separate `/admin` or `/user` pages; all workflow is handled via components and state.
- This approach provides a seamless, app-like experience and reduces navigation friction.
- The `ModeSelector` is the primary way to switch between Admin and User workflows.
- The Settings page remains accessible via a persistent link.

## Simplified Development Phases

### Phase 1: UI Foundation & Core Interface
**Objective**: Build complete user interface first to establish workflow

**Tasks**:
1. **Main Layout & Navigation** (`app/layouts/default.vue`):
   ```vue
   <!-- Simple layout with mode toggle (Admin/User) and settings access -->
   ```

2. **Core Page** (`app/pages/`):
   - `index.vue` - Main dashboard with mode selection and dynamic workflow panels
   - `settings.vue` - GitHub configuration

3. **Key Components** (`app/components/`):
   - `ModeSelector.vue` - Toggle between Admin/User modes
   - `AdminPanel.vue` - Admin workflow UI
   - `UserPanel.vue` - User workflow UI
   - `FileSelector.vue` - Directory/file picker interface
   - `ManifestPreview.vue` - Show manifest contents
   - `AddonList.vue` - Display mods/resourcepacks/shaderpacks
   - `GitHubSettings.vue` - Repository configuration
   - `ProgressBar.vue` - Operation progress
   - `StatusAlert.vue` - Success/error messages

4. **Basic Stores** (`app/stores/`):
   ```typescript
   // app/stores/app.ts
   export const useAppStore = defineStore('app', () => {
     const mode = ref<'admin' | 'user'>('admin')
     const githubRepo = ref('')
     const modpackPath = ref('')
   })
   
   // app/stores/manifest.ts  
   export const useManifestStore = defineStore('manifest', () => {
     const manifest = ref<Manifest | null>(null)
     const selectedAddons = ref<string[]>([])
     const removedAddons = ref<string[]>([])
   })
   ```

5. **TypeScript Types** (`app/types/index.ts`):
   ```typescript
   export interface Addon {
     addon_file_id: number
     addon_name: string
     addon_project_id: number
     cdn_download_url: string
     mod_folder_path: string
     version: string
   }
   
   export interface Manifest {
     mods: Addon[]
     resourcepacks: Addon[]
     shaderpacks: Addon[]
   }
   
   export interface UpdateInfo {
     uuid: string
     timestamp: string
     addedAddons: Addon[]
     removedAddons: string[]
     configFiles: string[]
   }
   ```

### Phase 2: Basic Tauri Integration
**Objective**: Connect UI to basic file operations

**Tasks**:
1. **Essential Tauri Commands** (`src-tauri/src/lib.rs`):
   ```rust
   #[tauri::command]
   fn select_directory() -> Result<String, String>
   
   #[tauri::command]
   fn select_file() -> Result<String, String>
   
   #[tauri::command]
   fn read_file(path: String) -> Result<String, String>
   
   #[tauri::command]
   fn write_file(path: String, content: String) -> Result<(), String>
   ```

2. **Frontend Integration** (`app/composables/useTauri.ts`):
   ```typescript
   export const useTauri = () => {
     const selectDirectory = async () => {
       return await invoke<string>('select_directory')
     }
     
     const readFile = async (path: string) => {
       return await invoke<string>('read_file', { path })
     }
   }
   ```

3. **Working File Operations**:
   - Directory selection for modpack path
   - Read minecraftinstance.json
   - Basic file validation

### Phase 3: Manifest Generation & Management
**Objective**: Core manifest creation and addon selection

**Tasks**:
1. **Manifest Generation** (`src-tauri/src/manifest.rs`):
   ```rust
   pub fn parse_minecraft_instance(content: &str) -> Result<Manifest, Error>
   pub fn generate_manifest(instance_path: &str) -> Result<Manifest, Error>
   pub fn compare_manifests(old: &Manifest, new: &Manifest) -> UpdateInfo
   ```

2. **Admin Interface Logic**:
   - Load existing modpack structure
   - Display current addons
   - Allow selection/deselection of addons
   - Show changes from previous version
   - Generate update UUID

3. **VCS Functionality**:
   - Track addon additions/removals
   - Show diff between versions
   - Maintain update history

### Phase 3.5: Secure Storage & Settings Polish
**Objective**: Enhance security for GitHub token storage and improve settings management

**Tasks**:
1. **Secure Storage Integration**:
   - Implement Stronghold plugin for secure GitHub token storage
   - Ensure tokens are never exposed in the frontend code or logs
   - Provide a fallback for development environments

2. **Settings Management Improvements**:
   - Refactor `GitHubSettings.vue` for better reactivity and performance
   - Add debug logging and timing for settings load/save operations
   - Resolve any performance issues related to settings management

3. **Pinia Persistence**:
   - Configure Pinia to persist non-sensitive state (e.g., GitHub repo name)
   - Ensure sensitive data is never persisted or exposed

### Phase 4: GitHub Integration
**Objective**: Upload/download functionality with GitHub

**Tasks**:
1. **GitHub Client** (`src-tauri/src/github.rs`):
   ```rust
   pub async fn upload_update(
       repo: &str, 
       token: &str, 
       uuid: &str, 
       manifest: &Manifest,
       config_files: Vec<&str>
   ) -> Result<(), Error>
   
   pub async fn download_update(
       repo: &str, 
       uuid: &str
   ) -> Result<(Manifest, Vec<ConfigFile>), Error>
   ```

2. **Settings Management**:
   - GitHub token storage (secure)
   - Repository configuration
   - Connection validation

3. **Upload/Download UI**:
   - Progress tracking
   - Error handling
   - Success confirmation

### Phase 5: User Installation System
**Objective**: Complete user workflow for applying updates

**Tasks**:
1. **Installation Logic** (`src-tauri/src/installer.rs`):
   ```rust
   pub async fn install_update(
       modpack_path: &str,
       manifest: &Manifest,
       config_files: Vec<ConfigFile>
   ) -> Result<(), Error>
   ```

2. **CurseForge Integration**:
   - Download addons from CDN URLs
   - Handle download failures
   - Verify file integrity

3. **User Interface**:
   - UUID input validation
   - Installation preview
   - Progress tracking
   - Rollback capability

### Phase 5.5: Update Management System
**Objective**: Handle updating existing installations by removing old files and managing version changes

**Tasks**:
1. **Manifest Comparison Logic**:
   - Compare current installed manifest with new manifest
   - Identify removed addons that need deletion
   - Identify updated addons where old versions should be removed
   - Identify completely new addons to install

2. **File Management System**:
   - Scan existing mod/resourcepack/shaderpack/datapack directories
   - Match installed files to manifest entries (by addon name/project ID)
   - Remove old versions before installing new ones
   - Remove completely deleted addons

3. **Update Preview Enhancement**:
   - Show users what will be removed, updated, and added
   - Display before/after version comparisons
   - Allow users to see the full update diff

4. **Backup & Safety**:
   - Create backup of removed files before deletion
   - Implement safer update process with rollback capability
   - Handle edge cases (file conflicts, permission issues)

5. **Update Installation Flow**:
   - Extend backend `install_update` to accept "previous manifest"
   - Implement cleanup phase before installation phase
   - Update UI to show removal progress alongside installation progress

### Phase 6: Polish & Error Handling
**Objective**: Make app production-ready

**Tasks**:
1. **Config File Selection UI** (High Priority):
   - Add file selector to AdminPanel for config files
   - Allow users to set relative paths within modpack
   - Show config files in UpdatePreview component
   - Basic file validation and upload functionality

2. **Comprehensive Error Handling**:
   - User-friendly error messages for all operations
   - Recovery suggestions for common failure scenarios
   - Logging system improvements
   - Network error handling with retry logic

3. **UI Polish & User Experience**:
   - Loading states for all async operations
   - Better visual feedback and animations
   - Responsive design for different screen sizes
   - Dark mode support and theme consistency
   - Improved accessibility (keyboard navigation, screen readers)

4. **Performance Optimization**:
   - Optimize large manifest handling
   - Implement proper caching strategies
   - Reduce bundle size and startup time
   - Memory usage optimization

5. **Final Testing & Quality Assurance**:
   - End-to-end workflow testing
   - Error scenario testing
   - Performance benchmarking
   - Cross-platform compatibility testing

6. **Documentation & Help**:
   - In-app help system
   - User guide and troubleshooting
   - Developer documentation

## Key Features Per Mode

### Admin Mode Interface
- **File Selection**: Choose modpack directory
- **Addon Management**: View, select, and remove addons
- **VCS View**: See changes from previous versions
- **Upload**: Generate UUID and push to GitHub

### User Mode Interface  
- **Repository Settings**: Configure GitHub repo
- **UUID Input**: Enter update code
- **Preview**: Show what will be installed/removed
- **Install**: Apply changes with progress tracking

## Development Guidelines

1. **UI First**: Build complete interface before backend logic
2. **Simple Start**: Focus on core workflow, add features incrementally  
3. **Real Data**: Use actual manifest.json structure from your example
4. **Error Handling**: Every operation should have proper error states
5. **User Feedback**: Always show what's happening during operations

## Logging & Diagnostics (Pino)

### Logging Guidelines
- Use [Pino](https://getpino.io/) for all application logging in the Nuxt frontend. Do not use `console.log`, `console.error`, etc. in production or development code.
- The logger is provided globally via a Nuxt plugin as `$logger` (see `app/plugins/logger.ts`).
- Use structured logging: always log objects or context, not just strings.
- Log at appropriate levels: `logger.info`, `logger.warn`, `logger.error`, `logger.debug`.
- Example usage:
  ```ts
  const logger = useNuxtApp().$logger
  logger.info('Manifest loaded', { manifest })
  logger.error('Failed to read file', { error })
  ```
- For composables, stores, and utilities, import the logger directly from `~/utils/logger` if needed.
- Do not log sensitive data (e.g., GitHub tokens, user secrets).
- Use logging for:
  - File operation results (success/failure)
  - API/network errors
  - User actions (mode switch, manifest load/save, etc.)
  - Unexpected conditions or recoverable errors
- For Tauri backend (Rust), continue to use `tauri-plugin-log` for backend logging.

### Pino Integration Steps
1. Install Pino with Bun: `bun add pino`
2. Create a logger utility at `app/utils/logger.ts`:
   ```ts
   import pino from 'pino'
   const logger = pino({ browser: { asObject: true }, level: import.meta.dev ? 'debug' : 'info' })
   export default logger
   ```
3. Add a Nuxt plugin at `app/plugins/logger.ts` to provide `$logger` globally.
4. Add type augmentation in `app/types/global.d.ts` for `$logger`.
5. Replace all `console.log` usage with `$logger` calls.

## Success Criteria
- Admin can create and upload updates with UUID
- User can install updates using UUID code
- Both modes have clean, intuitive interfaces
- All operations provide clear feedback and error handling
- Settings are properly persisted

**Start with Phase 1** - build the complete UI shell first, then implement backend functionality phase by phase. This approach lets you visualize the entire workflow before writing complex backend logic.

---

## Development Progress Status

### ✅ Completed Features (June 2025)

#### Phase 1: UI Foundation & Core Interface - **COMPLETED**
- [x] Main Layout & Navigation: `app/layouts/default.vue` with mode toggle
- [x] Core Pages: `index.vue` (dashboard), `settings.vue` (GitHub config)
- [x] Key Components: All core UI components implemented and functional
- [x] Pinia stores for app, manifest, and theme
- [x] TypeScript types complete

#### Phase 2: Basic Tauri Integration - **COMPLETED**
- [x] Tauri commands for file operations (select, read, write)
- [x] Frontend composable for Tauri integration
- [x] Directory selection, file reading, and validation working

#### Phase 3: Manifest Generation & Management - **COMPLETED**
- [x] Rust backend parses minecraftinstance.json and generates manifest
- [x] Addon management, diffing, and UUID generation working
- [x] Addon links open in browser via Tauri opener
- [x] Manifest store preserves webSiteURL

#### Phase 3.5: Secure Storage & Settings Polish - **COMPLETED**
- [x] Stronghold plugin for secure GitHub token storage (frontend + backend)
- [x] Pinia store persistence for GitHub repo name (non-sensitive)
- [x] GitHubSettings.vue refactored for security, reactivity, and performance
- [x] Debug logging and timing for settings load/save
- [x] Performance issues resolved (slow dev only, fast in production)

#### Phase 4: GitHub Integration - **COMPLETED**
- [x] Backend GitHub client: `upload_update()` and `download_update()` implemented and tested
- [x] Frontend upload/download UI fully connected to backend
- [x] Progress tracking, error handling, and success confirmation in UI
- [x] User mode: download from GitHub and write to disk works reliably (handles manifest-only and manifest+config files)
- [x] All edge cases (empty config files, missing directory, etc.) handled gracefully

### 🚧 Current Status (June 15, 2025)
- **Phase 6: Polish & Error Handling - Config File System Redesign** - ✅ **COMPLETE**
- New config file workflow with manifest integration is fully implemented and working
- All frontend and backend types updated to support the new `ConfigFileWithContent` structure
- AdminPanel now builds config files into manifest before upload
- UserPanel uses manifest-driven config file downloads and installation
- UpdatePreview component enhanced to show config file details with relative paths
- Ready to proceed with remaining Phase 6 tasks (comprehensive error handling, UI polish)

### **Phase 6: Config File System Redesign** - ✅ **COMPLETE**
- ✅ Updated TypeScript types to include `config_files` array in `Manifest` interface
- ✅ Updated Rust backend types and functions to use new `ConfigFileWithContent` structure
- ✅ Redesigned GitHub upload/download logic to use manifest-driven config file handling
- ✅ Updated AdminPanel.vue to include config files in manifest before upload
- ✅ Updated UserPanel.vue to use manifest for config file download and installation
- ✅ Updated UpdatePreview.vue to display config files with new structure
- ✅ Enhanced config file selection to support directory scanning with proper relative paths
- ✅ All TypeScript and Rust compilation checks pass successfully

### 📋 Next Steps (Phase 6: Remaining Tasks)
### 📋 Next Steps (Phase 6: Polish & Error Handling)
**Objective**: Make app production-ready with comprehensive error handling and UI polish

**Tasks**:

1. **Config File System Redesign** - ✅ **COMPLETED**:
   - ✅ **Problem Solved**: Config files now stored in manifest for reliable, atomic operations
   - ✅ **New Architecture**: Config files included directly in manifest.json with explicit paths
   
   **Implemented Config File Architecture**:
   ```json
   {
     "mods": [...],
     "resourcepacks": [...], 
     "shaderpacks": [...],
     "datapacks": [...],
     "config_files": [
       {
         "filename": "attributefix.json",
         "relative_path": "config/attributefix.json"
       },
       {
         "filename": "sort-order.json", 
         "relative_path": "config/jade/sort-order.json"
       },
       {
         "filename": "biomesoplenty.json",
         "relative_path": "defaultconfigs/biomesoplenty.json"
       }
     ]
   }
   ```
   
   **✅ Implementation Changes Completed**:
   - ✅ Updated TypeScript types to include `config_files` array in `Manifest` interface
   - ✅ Modified AdminPanel config selection to capture relative paths from modpack root
   - ✅ Rewrote GitHub upload logic to store config files with directory structure: `{uuid}/{relative_path}`
   - ✅ Rewrote download logic to get config files from manifest list, not directory scanning
   - ✅ Updated install logic to write config files to correct relative paths before addon installation
   - ✅ Enhanced UpdatePreview to show config file changes with full paths
   
   **✅ Benefits Achieved**:
   - ✅ Manifest becomes single source of truth for entire update
   - ✅ Atomic operations - all files or none
   - ✅ Better user transparency in preview
   - ✅ Supports any directory structure, not just `/config/`
   - ✅ Eliminates race conditions and partial uploads

2. **Comprehensive Error Handling**:
   - User-friendly error messages for all operations
   - Recovery suggestions for common failure scenarios
   - Logging system improvements
   - Network error handling with retry logic

3. **UI Polish & User Experience**:
   - Loading states for all async operations
   - Better visual feedback and animations
   - Responsive design for different screen sizes
   - Dark mode support and theme consistency
   - Improved accessibility (keyboard navigation, screen readers)

4. **Performance Optimization**:
   - Optimize large manifest handling
   - Implement proper caching strategies
   - Reduce bundle size and startup time
   - Memory usage optimization

5. **Final Testing & Quality Assurance**:
   - End-to-end workflow testing
   - Error scenario testing
   - Performance benchmarking
   - Cross-platform compatibility testing

6. **Documentation & Help**:
   - In-app help system
   - User guide and troubleshooting
   - Developer documentation