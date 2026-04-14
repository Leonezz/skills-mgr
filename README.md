# skills-mgr

Cross-agent skill management tool. Manage composable skill profiles that can be activated per-project across multiple AI coding agents (Claude Code, Cursor, Windsurf, etc.).

Built on the [Agent Skills open standard](https://agentskills.io) (`SKILL.md` with YAML frontmatter), adopted by Claude Code, GitHub Copilot, Cursor, Windsurf, Gemini CLI, and others.

> **Platform support**: This application is developed and partially tested on **macOS ARM64 (Apple Silicon)** only. Windows and Linux builds are produced by CI but have **not been tested**. If you encounter platform-specific issues, please open an issue.

## Problem

AI coding agents use skill/instruction files to customize behavior, but managing them is painful:

- **Context bloat** — Loading all skills wastes 3,000–5,000 tokens per conversation
- **No profiles** — No way to activate different skill sets per project
- **Update friction** — Manual copy-paste across agents and directories
- **No composition** — Can't build profiles from reusable skill sets

## How It Works

```
~/.skills-mgr/
├── registry/           # Central skill store
│   ├── rust-patterns/
│   │   └── SKILL.md
│   └── react-patterns/
│       └── SKILL.md
├── sources.toml        # Skill provenance & hashes
├── profiles.toml       # Profile definitions
├── agents.toml         # Agent directory mappings
├── settings.toml       # App settings (MCP, git sync, auto-scan)
├── logs/               # Rolling daily log files
└── local/
    └── skills-mgr.db   # SQLite tracking DB
```

Skills live in a central registry. Profiles compose skills into named sets. When you activate a profile, skills are copied to each agent's project directory. Deactivation cleans up only what's no longer needed.

## Features

- **Skill Registry** — Central store for all your skills with SHA-256 content hashing
- **Composable Profiles** — Named skill sets with `includes` for profile inheritance
- **Multi-Agent Support** — Deploy skills to Claude Code, Cursor, Windsurf, and more simultaneously
- **GitHub Import** — Import skills directly from GitHub repos with `owner/repo` shorthand
- **Multi-Skill Repo Browsing** — Discover and selectively import from collection repos like `anthropics/skills`
- **Skill Discovery** — Scan agent paths to find unmanaged skills and delegate them to the registry
- **Remote Linking** — Link local skills to GitHub repos for upstream sync
- **Global Skills** — Machine-level skills placed into each agent's global path
- **Token Budgeting** — Estimate token cost per skill and profile
- **Project Management** — Link profiles to projects, activate/deactivate per-project
- **MCP Server** — Programmatic access via Model Context Protocol
- **Desktop App** — Tauri 2 GUI with dark/light theme, activity logging, and auto-updater
- **Structured Logging** — `tracing` with stderr + rolling daily log files

## Installation

### From Source (CLI)

```bash
cargo install --path crates/skills-cli
```

### Desktop App (macOS)

Download the latest `.dmg` from [GitHub Releases](https://github.com/Leonezz/skills-mgr/releases).

#### macOS Gatekeeper workaround

The app is not code-signed with an Apple Developer certificate. macOS will block it on first launch. To work around this:

**Option 1 — Remove the quarantine attribute (recommended):**

```bash
# After mounting the .dmg and copying to /Applications:
xattr -cr /Applications/skills-mgr.app
```

**Option 2 — Allow via System Settings:**

1. Open the `.dmg` and drag the app to `/Applications`
2. Try to open the app — macOS will show "cannot be opened because the developer cannot be verified"
3. Go to **System Settings → Privacy & Security**
4. Scroll down to find the blocked app message and click **Open Anyway**
5. Click **Open** in the confirmation dialog

**Option 3 — Right-click to open:**

1. Right-click (or Control-click) the app in `/Applications`
2. Select **Open** from the context menu
3. Click **Open** in the dialog — this bypasses Gatekeeper for this app

After any of these methods, the app will open normally on subsequent launches.

## Quick Start

```bash
# One-shot bootstrap for any project
cd ~/my-project
skills-mgr init
# → registers project, sets up agents, activates global skills, prints guide

# Or step-by-step:
skills-mgr agent add --all               # Add all known agent presets
skills-mgr project add .                 # Register current directory

# Import skills
skills-mgr skill add anthropics/skills/skills/pdf
skills-mgr skill create rust-patterns --description "Rust development patterns"

# Build composable profiles
skills-mgr profile create rust --add rust-patterns
skills-mgr profile create fullstack --add react-patterns --include rust

# Activate for your project
skills-mgr profile activate fullstack
# → copies rust-patterns + react-patterns to .claude/skills/ and .cursor/skills/

# Check status and token budget
skills-mgr status
skills-mgr budget fullstack

# Switch profiles
skills-mgr profile switch rust
# → removes react-patterns, keeps rust-patterns

# Deactivate
skills-mgr profile deactivate rust
```

### AI Agent Bootstrap

To let an AI agent manage skills in your project, add one line to your project instructions (e.g., `CLAUDE.md`):

```
Run `skills-mgr guide` to learn how to manage skills for this project.
```

The agent runs `skills-mgr guide`, gets the full usage manual, and can operate autonomously from there.

## CLI Reference

```
skills-mgr <COMMAND>

Commands:
  init              Initialize skills-mgr for the current project
  guide             Print the usage guide (for AI agents and humans)
  skill             Manage skills in the registry
  profile           Manage profiles
  project           Manage projects (register, link profiles)
  agent             Manage agent configurations
  global            Manage global skills (machine-level)
  status            Show active profiles and placements
  check-conflicts   Scan for overlapping skills
  doctor            Verify placements match DB
  budget            Estimate token cost of a profile
  log               Show recent operations
```

### Skills

```bash
skills-mgr skill list                          # List all skills
skills-mgr skill create <name> --description   # Create new skill scaffold
skills-mgr skill add <source>                  # Add from GitHub/local/hub
skills-mgr skill add anthropics/skills/skills/pdf  # Import from GitHub subdirectory
skills-mgr skill add owner/repo               # Import entire repo as skill
skills-mgr skill info <name>                   # Show metadata & files
skills-mgr skill read <name>                   # Display SKILL.md content
skills-mgr skill files <name>                  # List skill files
skills-mgr skill remove <name>                 # Remove from registry
skills-mgr skill open <name>                   # Open directory in editor
skills-mgr skill update <name>                 # Update from remote source
skills-mgr skill update --all                  # Update all remote-sourced skills
skills-mgr skill sync                          # Re-fetch all git-sourced skills
skills-mgr skill discover                      # Find unmanaged skills in agent paths
skills-mgr skill discover --global-only        # Scan only global paths
skills-mgr skill discover --delegate <profile> # Import and assign to a profile
skills-mgr skill browse <source>               # Browse skills in a remote repo
skills-mgr skill browse --hub <name>           # Browse a configured skill hub
skills-mgr skill hubs                          # List configured skill hubs
skills-mgr skill link-remote <name> \
  --url https://github.com/owner/repo \
  --subpath path/to/skill \
  --git-ref main                               # Link local skill to remote
skills-mgr skill unlink-remote <name>          # Unlink from remote
```

Supported GitHub import formats:
- `https://github.com/owner/repo/tree/main/path/to/skill` — Full URL with branch and subpath
- `owner/repo` — Shorthand, defaults to `main` branch
- `owner/repo/path/to/skill` — Shorthand with subpath

The GUI and CLI also support **browsing multi-skill repos** — `skill browse owner/repo` discovers all skills in the repo, letting you select which ones to import. Use `--hub <name>` to browse configured skill hubs.

### Profiles

```bash
skills-mgr profile list                        # List all profiles
skills-mgr profile show <name>                 # Show resolved skill list
skills-mgr profile create <name> \
  --add skill1,skill2 \
  --include base-profile                       # Create with composition
skills-mgr profile edit <name> \
  --add new-skill --remove old-skill \
  --include other-profile                      # Edit skills and includes
skills-mgr profile duplicate <source> <new>    # Clone a profile
skills-mgr profile delete <name>               # Delete profile
skills-mgr profile activate <name>             # Activate for current project
skills-mgr profile activate <name> --project /path  # Activate for specific project
skills-mgr profile activate <name> --dry-run   # Preview without making changes
skills-mgr profile activate <name> --force     # Overwrite conflicts
skills-mgr profile activate --global           # Activate global skills
skills-mgr profile deactivate <name>           # Deactivate
skills-mgr profile deactivate --global         # Deactivate global skills
skills-mgr profile switch <name>               # Switch from current active profile
skills-mgr profile switch <name> --from old    # Switch from a specific profile
```

### Projects

```bash
skills-mgr project list                        # List registered projects
skills-mgr project add <path>                  # Register a project directory
skills-mgr project add <path> --name myproj    # Register with custom display name
skills-mgr project remove <path>               # Unregister a project
skills-mgr project link <profile>              # Link profile to current project
skills-mgr project link <profile> --project /path  # Link to specific project
skills-mgr project unlink <profile>            # Unlink profile from current project
```

### Global Skills

```bash
skills-mgr global status                       # Show global skills status
skills-mgr global add <skill1> <skill2>        # Add skills to global config
skills-mgr global remove <skill1>              # Remove from global config
skills-mgr global activate                     # Place into agent global paths
skills-mgr global deactivate                   # Remove from agent global paths
```

### Agents

```bash
skills-mgr agent list                          # List configured agents
skills-mgr agent add --all                     # Add all known agent presets
skills-mgr agent add <name> \
  --project-path ".claude/skills" \
  --global-path "~/.claude/skills"             # Add specific agent
skills-mgr agent remove <name>                 # Remove agent
skills-mgr agent enable <name>                 # Enable agent for project
skills-mgr agent disable <name>                # Disable agent for project
```

### Utilities

```bash
skills-mgr status                              # Active profiles and placements
skills-mgr status --project /path              # Status for specific project
skills-mgr budget <profile>                    # Token cost for a profile
skills-mgr budget --project /path              # Token cost for active project profiles
skills-mgr log                                 # Recent operations (default: 20)
skills-mgr log --project /path                 # Filter logs by project
skills-mgr log --source cli                    # Filter by source (cli/gui/mcp)
skills-mgr log --limit 50                      # Show more entries
skills-mgr check-conflicts                     # Scan for overlapping skills
skills-mgr doctor                              # Verify registry/placement integrity
```

## Core Concepts

### Skills

A skill is a directory containing a `SKILL.md` file (with YAML frontmatter) and optional supporting files. Skills are stored in the central registry at `~/.skills-mgr/registry/`.

```
my-skill/
├── SKILL.md          # Instructions with name/description frontmatter
├── examples.md       # Optional supporting files
└── templates/
    └── component.tsx
```

### Profiles

Profiles are named collections of skills defined in `profiles.toml`. They support composition via `includes`:

```toml
[base]
skills = ["code-review", "git-workflow"]    # Always-on skills

[profiles.rust]
description = "Rust development"
skills = ["rust-patterns", "cargo-best-practices"]

[profiles.fullstack]
description = "Full-stack Rust + React"
skills = ["react-patterns", "api-design"]
includes = ["rust"]                         # Inherits rust profile skills
```

When `fullstack` is activated, it resolves to: `code-review` + `git-workflow` (base) + `rust-patterns` + `cargo-best-practices` (from rust) + `react-patterns` + `api-design`.

### Agents

Agents are AI coding assistants with known skill directory paths:

```toml
[agents.claude-code]
project_path = ".claude/skills"
global_path = "~/.claude/skills"

[agents.cursor]
project_path = ".cursor/skills"
global_path = "~/.cursor/skills"
```

### Placements

When a profile is activated, skills are copied from the registry into each agent's project directory. The SQLite database tracks what's placed where, so deactivation only removes skills not shared with other active profiles.

### Skill Discovery

The discovery engine scans configured agent paths for `SKILL.md` files that exist outside the registry. This lets you find skills that were manually placed or installed by other tools, and delegate them to skills-mgr for centralized management.

Discovered skills can be:
- **Delegated** — imported into the registry and assigned to a profile
- **Linked to remote** — connected to a GitHub repo URL for upstream sync

## MCP Server

The MCP (Model Context Protocol) server provides programmatic access for AI agents. It exposes 19 tools:

| Tool | Purpose |
|------|---------|
| `list_skills` | List all skills in the registry |
| `create_skill` | Create a new skill scaffold |
| `remove_skill` | Remove a skill from the registry |
| `add_skill` | Import a skill from a source URL |
| `update_skill` | Update a remote-sourced skill |
| `sync_skills` | Re-fetch all git-sourced skills |
| `link_remote` | Link a local skill to a GitHub repo |
| `unlink_remote` | Unlink a skill from its remote |
| `list_profiles` | List all profiles with resolved skills |
| `create_profile` | Create a new profile |
| `delete_profile` | Delete a profile |
| `activate_profile` | Activate a profile for a project |
| `deactivate_profile` | Deactivate a profile |
| `switch_profile` | Switch active profile |
| `global_status` | Show global skills status |
| `activate_global` | Activate global skills |
| `deactivate_global` | Deactivate global skills |
| `edit_global_skills` | Set the global skills list |
| `list_agents` | List configured agents |
| `add_agent` | Add an agent |
| `list_agent_presets` | List known agent presets |
| `get_status` | Get project status |
| `discover_skills` | Scan for unmanaged skills |

Enable the MCP server in `~/.skills-mgr/settings.toml`:

```toml
mcp_enabled = true
mcp_port = 3100
mcp_transport = "stdio"
```

## Architecture

Rust workspace with 4 crates:

```
crates/
├── skills-core/      # Shared library: config, DB, registry, profiles, placements, discovery
├── skills-cli/       # CLI binary (clap)
├── skills-mcp/       # MCP server for AI agent integration
└── skills-gui/       # Desktop app (Tauri 2 + React)
```

| Crate | Purpose |
|-------|---------|
| **skills-core** | Business logic, SQLite (sqlx), TOML config, SHA-256 hashing, GitHub tarball download, skill discovery |
| **skills-cli** | Thin CLI wrapper using clap derive macros |
| **skills-mcp** | MCP protocol server for programmatic access |
| **skills-gui** | Tauri 2 desktop app with React 19, Tailwind CSS 4, shadcn/ui, auto-updater |

### Tech Stack

- **Language**: Rust (2024 edition)
- **Database**: SQLite with WAL mode (via sqlx)
- **Config**: TOML (serde)
- **Hashing**: SHA-256 tree hashes for content integrity
- **Logging**: tracing + tracing-subscriber (stderr + rolling daily files)
- **CLI**: clap 4 with derive macros
- **GUI**: Tauri 2 + React 19 + TypeScript + Tailwind CSS 4 + shadcn/ui
- **Updates**: tauri-plugin-updater with signed releases via GitHub Actions
- **State**: @tanstack/react-query for frontend data fetching
- **Validation**: Zod schemas for IPC data
- **CI/CD**: GitHub Actions with tauri-apps/tauri-action for cross-platform builds

## Building

### Prerequisites

- Rust 1.85+ (2024 edition)
- Node.js 22+
- pnpm 9+

### CLI only

```bash
cargo build --release
# Binary at target/release/skills-mgr
```

### GUI (Tauri desktop app)

```bash
cd crates/skills-gui
pnpm install
cargo tauri build
```

### Run tests

```bash
cargo test --workspace
```

## Releases

Releases are built automatically via GitHub Actions when a version tag is pushed:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The CD pipeline builds for macOS (arm64 + x64), Linux, and Windows, signs the update artifacts, and publishes a GitHub Release with a `latest.json` manifest for the auto-updater.

## License

MIT
