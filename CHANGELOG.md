# Changelog

## [0.5.0] - 2026-05-21

### Bug Fixes

- Use GitHub App token instead of PAT for all workflows *(ci)*
- Let git-cliff auto-detect version bump from commits *(ci)*
- Add cursor pointer to open in browser button
- Fetch full PR description and add scrollable description area
- Show team icon for group reviewers and clean group display names
- Use PAT for release to trigger homebrew workflow and strip quarantine in cask template *(ci)*
- Prevent duplicate notifications on restart and re-init
- Use PAT for release push to trigger build workflow *(ci)*

### Features

- Add MCP integration section to settings with setup guides
- Render PR descriptions as markdown in detail panel
- Add MCP stdio server for Claude Code integration
- Test notification opens repo, PR URLs in change events

### Miscellaneous

- Update Cargo.lock to v0.4.0
## [0.4.0] - 2026-05-19

### Bug Fixes

- Use inline badges instead of banners in detail panel
- Replace emoji vote symbols with lucide icons, remove filter dot
- Add workflow_dispatch to homebrew and winget workflows *(ci)*

### Documentation

- Add screenshot and demo mode for mock data
- Add Homebrew install instructions
- Add macOS Gatekeeper workaround to README

### Features

- Conventional commit badges and project name toggle
- Build status, age thresholds, detail refresh, animations
- Auto-update Homebrew tap and WinGet on release *(ci)*

### Miscellaneous

- V0.4.0 *(release)*
## [0.3.0] - 2026-05-19

### Bug Fixes

- Fix release workflow startup failure *(ci)*
- Add workflow_call trigger to build workflow *(ci)*

### Features

- Provider color indicators on PR rows

### Miscellaneous

- V0.3.0 *(release)*
- Add AGPL-3.0 license
## [0.2.0] - 2026-05-18

### Bug Fixes

- Split build vs release steps, fix concurrency for tags *(ci)*
- Use tauri-action built-in release instead of manual artifacts *(ci)*
- Add @types/node for vite.config.ts build *(ci)*
- Approve esbuild build scripts in pnpm-workspace.yaml *(ci)*
- Allow esbuild builds via pnpm-workspace.yaml *(ci)*
- Use pnpm 11 in build workflow *(ci)*
- Remove pnpm-workspace.yaml that broke CI *(ci)*

### Documentation

- Add README

### Features

- Add filters, notifications settings, icons, and CI
- Migrate from TUI to Tauri v2 desktop app

### Miscellaneous

- V0.2.0 *(release)*
## [0.1.0] - 2026-03-02

### Features

- Initial ridgeline TUI for PR monitoring

