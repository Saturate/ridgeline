# Changelog

## [0.6.0] - 2026-05-29


### Features

- Notify when a PR is completed or merged (#4) *(notifications)*
- Friendly error UI with auto-retry and exponential backoff

### Bug Fixes

- Don't let one provider failure block the rest (#7)

### Documentation

- Add MCP server to features list

### Miscellaneous

- List features before bug fixes in changelog
- Exclude CI commits from changelog## [0.5.0] - 2026-05-21


### Features

- Add MCP integration section to settings with setup guides
- Render PR descriptions as markdown in detail panel
- Add MCP stdio server for Claude Code integration
- Test notification opens repo, PR URLs in change events

### Bug Fixes

- Add cursor pointer to open in browser button
- Fetch full PR description and add scrollable description area
- Show team icon for group reviewers and clean group display names
- Prevent duplicate notifications on restart and re-init

### Miscellaneous

- Update Cargo.lock to v0.4.0## [0.4.0] - 2026-05-19


### Features

- Conventional commit badges and project name toggle
- Build status, age thresholds, detail refresh, animations

### Bug Fixes

- Use inline badges instead of banners in detail panel
- Replace emoji vote symbols with lucide icons, remove filter dot

### Documentation

- Add screenshot and demo mode for mock data
- Add Homebrew install instructions
- Add macOS Gatekeeper workaround to README## [0.3.0] - 2026-05-19


### Features

- Provider color indicators on PR rows

### Miscellaneous

- Add AGPL-3.0 license## [0.2.0] - 2026-05-18


### Features

- Add filters, notifications settings, icons, and CI
- Migrate from TUI to Tauri v2 desktop app

### Documentation

- Add README## [0.1.0] - 2026-03-02


### Features

- Initial ridgeline TUI for PR monitoring
