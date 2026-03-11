# skills-mgr

Cross-agent skill management tool. Manage composable skill profiles that can be activated per-project across multiple AI coding agents (Claude Code, Cursor, Windsurf, etc.).

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
└── local/
    └── skills-mgr.db   # SQLite tracking DB
```

Skills live in a central registry. Profiles compose skills into named sets. When you activate a profile, skills are copied to each agent's project directory. Deactivation cleans up only what's no longer needed.

## Quick Start

```bash
# Install (from source)
cargo install --path crates/skills-cli

# Configure your agents
skills-mgr agent add claude-code \
  --project-path ".claude/skills" \
  --global-path "~/.claude/skills"

skills-mgr agent add cursor \
  --project-path ".cursor/skills" \
  --global-path "~/.cursor/skills"

# Create skills
skills-mgr skill create rust-patterns --description "Rust development patterns"
skills-mgr skill create react-patterns --description "React development patterns"

# Build composable profiles
skills-mgr profile create rust --add rust-patterns
skills-mgr profile create fullstack --add react-patterns --include rust

# Activate for your project
cd ~/my-project
skills-mgr profile activate fullstack
# → copies rust-patterns + react-patterns to .claude/skills/ and .cursor/skills/

# Check status
skills-mgr status

# Switch profiles
skills-mgr profile switch rust
# → removes react-patterns, keeps rust-patterns

# Deactivate
skills-mgr profile deactivate rust
```

## CLI Reference

```
skills-mgr <COMMAND>

Commands:
  skill             Manage skills in the registry
  profile           Manage profiles
  agent             Manage agent configurations
  status            Show active profiles and placements
  check-conflicts   Scan for overlapping skills
  doctor            Verify placements match DB
  budget            Estimate token cost of a profile
  log               Show recent operations
```

### Skills

```bash
skills-mgr skill list                          # List all skills
skills-mgr skill create <name> --description   # Create new skill
skills-mgr skill add <source>                  # Add from git/local/registry
skills-mgr skill info <name>                   # Show details & files
skills-mgr skill files <name>                  # List skill files
skills-mgr skill remove <name>                 # Remove from registry
skills-mgr skill open <name>                   # Open in editor
```

### Profiles

```bash
skills-mgr profile list                        # List all profiles
skills-mgr profile create <name> \
  --add skill1,skill2 \
  --include base-profile                       # Create with composition
skills-mgr profile show <name>                 # Show resolved skills
skills-mgr profile activate <name>             # Activate for project
skills-mgr profile deactivate <name>           # Deactivate
skills-mgr profile switch <name>               # Switch active profile
skills-mgr profile edit <name> \
  --add new-skill --remove old-skill           # Edit existing profile
skills-mgr profile delete <name>               # Delete profile
```

### Agents

```bash
skills-mgr agent list                          # List configured agents
skills-mgr agent add <name> \
  --project-path ".claude/skills" \
  --global-path "~/.claude/skills"             # Add agent
skills-mgr agent remove <name>                 # Remove agent
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

## Architecture

Rust workspace with 4 crates:

```
crates/
├── skills-core/      # Shared library: config, DB, registry, profiles, placements
├── skills-cli/       # CLI binary (clap)
├── skills-mcp/       # MCP server for AI agent integration
└── skills-gui/       # Desktop app (Tauri 2 + React)
```

| Crate | Purpose |
|-------|---------|
| **skills-core** | Business logic, SQLite (sqlx), TOML config, SHA-256 hashing |
| **skills-cli** | Thin CLI wrapper using clap derive macros |
| **skills-mcp** | MCP protocol server for programmatic access |
| **skills-gui** | Tauri 2 desktop app with React 19, Tailwind CSS 4, shadcn/ui |

### Tech Stack

- **Language**: Rust (2024 edition)
- **Database**: SQLite with WAL mode (via sqlx)
- **Config**: TOML (serde)
- **Hashing**: SHA-256 tree hashes for content integrity
- **CLI**: clap 4 with derive macros
- **GUI**: Tauri 2 + React 19 + TypeScript + Tailwind CSS 4 + shadcn/ui
- **State**: @tanstack/react-query for frontend data fetching
- **Validation**: Zod schemas for IPC data

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

## License

MIT

