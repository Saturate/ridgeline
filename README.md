# Ridgeline

Cross-tenant, cross-project pull request monitor for Azure DevOps. See all your PRs in one place — across organizations, projects, and repositories.

Built with [Tauri v2](https://tauri.app) (Rust backend) and React + [shadcn/ui](https://ui.shadcn.com) (frontend).

## Features

- **Cross-tenant overview** — connect multiple Azure DevOps organizations
- **Reviewing / Authored tabs** — separate views for PRs you're reviewing vs. PRs you created
- **Vote summary** — see approval progress, rejections, and "waiting for author" at a glance
- **Filters** — hide drafts, filter by provider, search across titles/repos/branches
- **Native notifications** — configurable OS notifications for new PRs, vote changes, and build failures
- **Background polling** — automatic refresh with configurable interval
- **Dark mode** — follows system preference

## Setup

```bash
pnpm install
pnpm tauri dev
```

On first launch, add your Azure DevOps provider with an organization URL and Personal Access Token.

Config is stored at `~/.config/ridgeline/config.toml` (macOS: `~/Library/Application Support/ridgeline/`).

## Build

```bash
pnpm tauri build
```

## Release

Releases use [git-cliff](https://git-cliff.org) for changelog generation. Trigger the Release workflow from GitHub Actions to auto-bump the version based on conventional commits.

## License

MIT
