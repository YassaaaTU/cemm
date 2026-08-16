/**
 * Matches the Rust backend's `parse_and_validate_repo` (github.rs) — every
 * value that reaches an IPC command interpolating `repo` into a GitHub API URL
 * must satisfy this same shape. Kept as a single shared check so the two entry
 * points that write `githubRepo` (GitHubSettings.vue and UserPanel.vue) can't
 * drift out of sync the way UserPanel.vue's own ad-hoc check previously did
 * (F-P2-20).
 */
const GITHUB_REPO_PATTERN = /^[a-zA-Z0-9._-]+\/[a-zA-Z0-9._-]+$/

export function isValidGithubRepo(repo: string): boolean
{
	return GITHUB_REPO_PATTERN.test(repo.trim())
}
