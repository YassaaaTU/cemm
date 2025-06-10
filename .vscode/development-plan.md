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

### Phase 6: Polish & Error Handling
**Objective**: Make app production-ready

**Tasks**:
1. **Comprehensive Error Handling**:
   - User-friendly error messages
   - Recovery suggestions
   - Logging system

2. **UI Polish**:
   - Loading states
   - Better visual feedback
   - Responsive design
   - Dark mode support

3. **Final Testing**:
   - End-to-end workflow testing
   - Error scenario testing
   - Performance optimization

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

## Success Criteria
- Admin can create and upload updates with UUID
- User can install updates using UUID code
- Both modes have clean, intuitive interfaces
- All operations provide clear feedback and error handling
- Settings are properly persisted

**Start with Phase 1** - build the complete UI shell first, then implement backend functionality phase by phase. This approach lets you visualize the entire workflow before writing complex backend logic.

---

**Recent Architectural Changes (June 2025):**
- Switched to a single-page, component-driven UI for mode selection and workflow.
- Mode switching is handled by `ModeSelector` and dynamic rendering of `AdminPanel`/`UserPanel` components.
- Removed separate `/admin` and `/user` pages for a more seamless, app-like experience.
- Settings remain accessible at all times.