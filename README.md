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

## Install

Download the latest release from the [Releases page](https://github.com/Saturate/ridgeline/releases).

### macOS

macOS may show "Ridgeline is damaged and can't be opened" because the app is not notarized. To fix this, run after installing:

```bash
xattr -cr /Applications/Ridgeline.app
```

On first launch, add your Azure DevOps provider with an organization URL and Personal Access Token.

Config is stored at `~/Library/Application Support/ridgeline/` (macOS) or `~/.config/ridgeline/` (Linux).

## Development

```bash
pnpm install
pnpm tauri dev
```

## Build

```bash
pnpm tauri build
```

## Release

Releases use [git-cliff](https://git-cliff.org) for changelog generation. Trigger the Release workflow from GitHub Actions to auto-bump the version based on conventional commits.

## License

[AGPL-3.0](LICENSE)
