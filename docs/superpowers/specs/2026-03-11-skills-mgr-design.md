# Skills-Mgr Design Spec

## Overview

Skills-mgr is a cross-agent skill management tool that provides a GUI (Tauri 2 + React) for humans and a CLI + MCP server for AI agents. It manages a central registry of skills following the [Agent Skills](https://agentskills.io) open standard, organizes them into composable profiles, and places them into each agent's discovery path per project.

## Problem Statement

With 17+ plugins and 151+ skills installed across AI coding agents, users face:

- **Context window bloat**: 100+ skills consume 3,000-5,000 tokens in metadata alone
- **No profile/scenario system**: No way to say "activate Rust+React skills" for a project
- **Per-skill granular control**: Plugins are all-or-nothing, cannot disable individual skills
- **Update friction**: Stale caches, unreliable update commands, no version tracking
- **Cross-agent fragmentation**: Skills scattered across agent-specific directories

## Key Insight

Agent Skills is an open standard adopted by 30+ agents (Claude Code, Cursor, Copilot, Gemini CLI, Codex, and many more). The skill format (`SKILL.md` with YAML frontmatter) is universal. The only difference between agents is the **discovery path** where they look for skills. This means no format adapters are needed — just placing skill directories in the right locations.

## Architecture

### Rust Workspace

```
skills-mgr/
├── Cargo.toml                     # workspace root
├── crates/
│   ├── skills-core/               # shared library
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── registry.rs        # skill CRUD, hash computation
│   │       ├── profiles.rs        # profile resolution, base + layers
│   │       ├── sources.rs         # pull, update, hash verification
│   │       ├── placements.rs      # activate/deactivate, copy to agent paths
│   │       ├── agents.rs          # agent config, discovery paths
│   │       ├── projects.rs        # project detection, scoping
│   │       ├── db.rs              # SQLite via sqlx
│   │       ├── config.rs          # TOML parsing
│   │       └── logging.rs         # operation log
│   │
│   ├── skills-cli/                # standalone CLI binary (clap)
│   │   └── src/main.rs
│   │
│   ├── skills-mcp/                # MCP server module (rmcp)
│   │   └── src/lib.rs
│   │
│   └── skills-gui/                # Tauri 2 app
│       ├── src-tauri/
│       │   └── src/
│       │       ├── main.rs        # Tauri setup, embeds MCP server
│       │       └── commands.rs    # Tauri IPC commands
│       └── src/                   # React frontend
│           └── ...

```

### Component Relationships

```
┌─────────────────────────────────────────┐
│              skills-gui (Tauri 2)       │
│  ┌──────────────┐  ┌────────────────┐   │
│  │ React UI     │  │ MCP Server     │   │
│  │ (frontend)   │  │ (HTTP/SSE)     │   │
│  └──────┬───────┘  └───────┬────────┘   │
│         │ IPC              │            │
│         ▼                  ▼            │
│  ┌──────────────────────────────────┐   │
│  │        skills-core (lib)         │   │
│  │  registry │ profiles │ placements│   │
│  └──────────────┬───────────────────┘   │
│                 │                       │
│         ┌───────┴───────┐               │
│         ▼               ▼               │
│    ~/.skills-mgr/   skills-mgr.db       │
└─────────────────────────────────────────┘

┌─────────────────┐
│   skills-cli    │──► skills-core (lib)
│   (standalone)  │         │
└─────────────────┘   ┌────┴────┐
                      ▼         ▼
                 ~/.skills-mgr/  skills-mgr.db
```

### Key Design Decisions

1. **`skills-core` owns all logic** — GUI, CLI, and MCP are thin wrappers
2. **MCP server embedded in GUI app** — starts HTTP/SSE endpoint on launch
3. **CLI is a Tauri sidecar** — bundled with the app, installed to PATH
4. **SQLite (WAL mode)** — handles concurrent access from GUI and CLI
5. **No format adapters** — Agent Skills standard is universal, only discovery paths differ
6. **Copy-based activation** — copies skill directories to agent paths (no symlinks, avoids cross-platform issues)

## Data Model

### Central Registry (git-tracked)

```
~/.skills-mgr/
├── registry/                      # skill directories (Agent Skills standard)
│   ├── rust-engineer/
│   │   ├── SKILL.md
│   │   └── scripts/
│   ├── code-review/
│   │   └── SKILL.md
│   └── ...
├── sources.toml                   # provenance + update tracking
├── profiles.toml                  # base layer + profile definitions
├── agents.toml                    # configured agent discovery paths
└── .gitignore                     # excludes local/
```

### Local State (gitignored)

```
~/.skills-mgr/
└── local/
    ├── skills-mgr.db              # SQLite
    └── cache/                     # temp files during pull/update
```

### sources.toml

Tracks where each skill came from and its current hash:

```toml
[skills.rust-engineer]
type = "git"                       # git | registry | local
url = "https://github.com/anthropics/skills"
path = "rust-engineer"             # subdirectory within repo
ref = "main"                       # branch/tag
hash = "sha256:abc123..."          # tree hash of entire skill directory
updated_at = "2026-03-10T12:00:00Z"

[skills.my-custom-skill]
type = "local"
hash = "sha256:def456..."
updated_at = "2026-03-11T08:00:00Z"
```

### profiles.toml

Composable profiles with base layer:

```toml
[base]
skills = ["code-review", "obsidian", "git-workflow"]

[profiles.rust]
description = "Rust development"
skills = ["rust-engineer", "cargo-patterns"]

[profiles.react]
description = "React development"
skills = ["react-specialist", "frontend-patterns"]

[profiles.rust-react]
description = "Rust + React full-stack"
includes = ["rust", "react"]
skills = ["api-design"]
```

### agents.toml

Agent discovery paths:

```toml
[agents.claude-code]
project_path = ".claude/skills"
global_path = "~/.claude/skills"

[agents.cursor]
project_path = ".cursor/skills"
global_path = "~/.cursor/skills"

[agents.copilot]
project_path = ".github/skills"
global_path = "~/.copilot/skills"

[agents.gemini-cli]
project_path = ".gemini/skills"
global_path = "~/.gemini/skills"

[agents.universal]
project_path = ".agents/skills"
global_path = "~/.agents/skills"
```

### SQLite Schema

All timestamps are ISO 8601 UTC with millisecond precision (e.g., `2026-03-11T14:30:45.123Z`).

```sql
CREATE TABLE projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    name TEXT
);

CREATE TABLE project_profiles (
    project_id INTEGER NOT NULL REFERENCES projects(id),
    profile_name TEXT NOT NULL,
    activated_at TEXT NOT NULL,
    PRIMARY KEY (project_id, profile_name)
);

CREATE TABLE project_agents (
    project_id INTEGER NOT NULL REFERENCES projects(id),
    agent_name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (project_id, agent_name)
);

-- One placement per (project, skill, agent). Deduplicated: if two profiles
-- both include the same skill, only one placement exists with links to both.
CREATE TABLE placements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    skill_name TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    target_path TEXT NOT NULL,
    placed_at TEXT NOT NULL,
    UNIQUE (project_id, skill_name, agent_name)
);

-- Junction table: a placement can belong to multiple profiles.
-- A placement is only removed when no profile references it.
CREATE TABLE placement_profiles (
    placement_id INTEGER NOT NULL REFERENCES placements(id) ON DELETE CASCADE,
    profile_name TEXT NOT NULL,
    PRIMARY KEY (placement_id, profile_name)
);

CREATE TABLE operation_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,           -- ISO 8601 UTC (e.g. 2026-03-11T14:30:45.123Z)
    source TEXT NOT NULL,              -- cli, mcp, gui
    agent_name TEXT,                   -- which AI agent invoked (nullable)
    operation TEXT NOT NULL,
    params TEXT,                       -- JSON
    project_path TEXT,
    result TEXT NOT NULL,              -- success, error, conflict
    details TEXT
);
```

## Behavioral Rules

### Profile Composition

- **Transitive expansion**: If profile A includes B, and B includes C, then activating A also activates skills from B and C.
- **Circular reference prevention**: Detected at profile creation/edit time. `skills-core` builds a dependency graph and rejects cycles with a clear error: "Circular include detected: A -> B -> A".
- **Deduplication**: Skills are deduplicated by name across base + all included profiles. A skill appears once regardless of how many profiles reference it.
- **Precedence**: Not applicable — skills are either present or absent. There is no override or priority between profiles for the same skill.

### Global vs Project Scope

- **Additive model**: Global and project scopes are combined. Global base/profiles provide a foundation; project-level profiles add on top.
- **No shadowing**: If the same skill is active both globally and per-project, it's deduplicated — only one copy exists at the target path.
- **Deactivation is scoped**: Deactivating a project-level profile only removes project-level placements. Global placements are untouched. If a skill was placed by both global and project profiles, the file remains (the global placement still references it).
- **Global placements use `global_path`**: e.g., `~/.claude/skills/`. Project placements use `project_path` resolved relative to the project root: e.g., `/path/to/project/.claude/skills/`.

### Conflict Resolution During Activation

- **Same skill, same agent, same project**: Deduplicated. One placement, multiple profile links in `placement_profiles`.
- **Different skill targeting same filename**: Rejected. The system checks `target_path` uniqueness. If another skill already occupies the path, activation fails with: "Conflict: target path already occupied by skill X from profile Y. Use `--force` to overwrite."
- **`--force` flag**: Overwrites the existing placement and updates `placement_profiles` to reflect the new owner.

### Error Recovery

- **Partial placement failure**: If copying fails for one skill (e.g., disk full, permission denied), already-placed skills in this batch are rolled back (deleted). The operation is atomic per activation — all or nothing.
- **Partial update failure**: During `skill update`, if re-copy fails for one placement, the system continues with remaining placements and reports partial success with the list of failures.
- **Database consistency**: All placement writes are wrapped in a SQLite transaction. If the transaction fails, no records are written.

### Gitignore Behavior

When placing skills into a project directory, the system checks if the agent's skills directory pattern is in the project's root `.gitignore`. If not:
- **Interactive (CLI/GUI)**: Prompt "Add `.claude/skills/` to .gitignore? [Y/n]"
- **Non-interactive (MCP)**: Warn in the response but proceed without modifying `.gitignore`
- If `.gitignore` doesn't exist, create it with the pattern
- Each agent's pattern is added separately (e.g., `.claude/skills/`, `.cursor/skills/`)

### Concurrent Access

- SQLite runs in WAL mode with `PRAGMA busy_timeout = 5000` (5 second retry on lock)
- All placement writes use `BEGIN IMMEDIATE` transactions to prevent write conflicts
- Reads are non-blocking (WAL allows concurrent readers during writes)
- If two clients activate simultaneously for the same project, one will wait up to 5 seconds; if still locked, fails with a clear error

## Activation Flow

### Activate

```
skills-mgr profile activate rust-react --project /path/to/my-app
```

1. **Resolve skills**: expand profile (including transitive `includes`), merge with base, deduplicate by skill name
2. **Resolve agents**: query `project_agents` for enabled agents (or all configured agents if none specified)
3. **Compute placements**: for each (skill, agent) pair, compute `target_path = project_root / agent.project_path / skill_name`
4. **Check conflicts**: query SQLite for existing placements at computed target paths. If conflict with a different skill, fail (or overwrite with `--force`)
5. **Check gitignore**: warn/prompt if agent skills directories not in `.gitignore`
6. **Execute (atomic)**: begin SQLite transaction, copy each skill directory from `~/.skills-mgr/registry/<name>/` to target path. On any copy failure, rollback all copies and abort.
7. **Record**: insert/update placements with UNIQUE constraint `(project_id, skill_name, agent_name)`, link in `placement_profiles`
8. **Log**: write operation to `operation_log`

### Deactivate

1. Query `placements` + `placement_profiles` for this project + profile
2. For each placement: remove the profile link from `placement_profiles`
3. If a placement has **no remaining profile links**: delete the skill directory at `target_path` and remove the placement record
4. If a placement still has other profile links: leave the file in place
5. Remove `project_profiles` record
6. Log operation

### Profile Switch

1. Deactivate all current profile-layer placements (base layer untouched)
2. Activate new profile
3. Log as a single "switch" operation

### Skill Update

1. Check `sources.toml` for source type, URL, and current hash
2. If `type = "local"`: warn "local skill, edit directly in registry" and exit
3. Pull latest from source (git fetch/clone, registry download) into `local/cache/`
4. Compute new tree hash (SHA-256 of sorted concatenation of all file hashes in the skill directory)
5. Compare with stored hash — if unchanged, report "already up to date" and exit
6. Update registry copy (replace directory contents), update hash in `sources.toml`
7. Query `placements` for all active placements of this skill across all projects
8. Re-copy to each target path. If a copy fails, log the failure but continue with remaining placements (partial success is acceptable for updates)
9. Report results: "Updated rust-engineer (sha256:abc→def), re-placed in 3 projects (2 succeeded, 1 failed)"

## CLI Interface

```
skills-mgr <command> [options]

SKILL MANAGEMENT
  skill list                          List all skills in registry
  skill add <source>                  Pull skill directory from git URL, registry, or local path
  skill remove <name>                 Remove skill directory from registry
  skill update <name>                 Re-pull skill from its remote source
  skill update --all                  Re-pull all remotely-sourced skills
  skill info <name>                   Show skill details: files, provenance, placements
  skill create <name>                 Scaffold new skill directory with SKILL.md template
  skill open <name>                   Open skill directory in file manager / editor
  skill files <name>                  List all files in a skill directory

PROFILE MANAGEMENT
  profile list                        List all profiles
  profile create <name>               Create a new profile
    --add <skill>[,skill,...]
    --include <profile>
  profile delete <name>               Delete a profile
  profile show <name>                 Show skills in a profile
  profile edit <name>                 Add/remove skills from a profile
    --add <skill>[,skill,...]
    --remove <skill>[,skill,...]
    --include <profile>

ACTIVATION (project-scoped by default)
  profile activate <name>             Activate profile for current project
  profile deactivate <name>           Deactivate profile for current project
  profile switch <name>               Deactivate all profiles, activate this one
  status                              Show active base + profiles for current project

  --project <path>                    Target a specific project (default: cwd)
  --global                            Apply to global scope instead of project

AGENT MANAGEMENT
  agent list                          List configured agents
  agent add <name>                    Add an agent with its discovery paths
  agent remove <name>                 Remove an agent
  agent enable <name>                 Enable agent for current project
  agent disable <name>                Disable agent for current project

SYNC (post-MVP)
  sync init                           Initialize git tracking for ~/.skills-mgr/
  sync push                           Push registry + config to remote git
  sync pull                           Pull latest from remote git

UTILITIES
  check-conflicts                     Scan for overlapping skills across active profiles
  doctor                              Verify placements match DB, check for orphans
  budget [profile]                    Estimate token cost of active or specified profile
  log                                 Show recent operations
    --project <path>
    --source <cli|mcp|gui>
    --limit <n>
```

## MCP Server Interface

Full parity with CLI. Embedded in GUI app, served via HTTP/SSE.

| Tool | Parameters | Description |
|------|-----------|-------------|
| `skill_list` | `tags?`, `search?` | List skills in registry |
| `skill_info` | `name` | Skill details, files, provenance, placements |
| `skill_add` | `source` | Pull skill from git URL, registry, or local path |
| `skill_remove` | `name` | Remove skill from registry |
| `skill_update` | `name?`, `all?` | Re-pull from remote source |
| `skill_create` | `name`, `description`, `content` | Scaffold new skill |
| `skill_files` | `name` | List all files in a skill directory |
| `profile_list` | | List all profiles |
| `profile_create` | `name`, `skills?`, `includes?` | Create a new profile |
| `profile_delete` | `name` | Delete a profile |
| `profile_show` | `name` | Show skills in a profile |
| `profile_edit` | `name`, `add?`, `remove?`, `include?` | Modify profile membership |
| `profile_activate` | `name`, `project?`, `global?` | Activate profile |
| `profile_deactivate` | `name`, `project?`, `global?` | Deactivate profile |
| `profile_switch` | `name`, `project?` | Deactivate all, activate this one |
| `status` | `project?` | Active base + profiles |
| `agent_list` | | List configured agents |
| `agent_add` | `name`, `project_path`, `global_path` | Add agent config |
| `agent_remove` | `name` | Remove agent config |
| `agent_enable` | `name`, `project?` | Enable agent for project |
| `agent_disable` | `name`, `project?` | Disable agent for project |
| `check_conflicts` | `project?` | Scan for overlapping skills |
| `doctor` | | Verify placements match DB |
| `budget_estimate` | `profile?`, `project?` | Estimate token cost |
| `sync_init` | `remote?` | Initialize git tracking |
| `sync_push` | | Push to remote |
| `sync_pull` | | Pull from remote |
| `operation_log` | `project?`, `source?`, `limit?` | Query operation history |

### MCP Settings (in GUI Settings view)

| Setting | Default | Description |
|---------|---------|-------------|
| MCP server enabled | `true` | Start MCP server when GUI launches |
| Port | `auto` | Port for HTTP/SSE endpoint |
| Allowed origins | `["localhost"]` | Which hosts can connect |
| Auth token | | Optional bearer token |
| Auto-register | `false` | Write MCP server entry into agent configs on startup |
| Connection status | | Shows connected agents, last ping |

## Teaching Skill

Ships with the tool as `skills-mgr-guide` in the registry:

```yaml
---
name: skills-mgr-guide
description: How to use the skills-mgr tool for managing AI agent skills,
  profiles, and activation. Use when the user asks about managing skills,
  switching profiles, or configuring agent skill sets.
user-invocable: false
---

You have access to skills-mgr via MCP tools or CLI (`skills-mgr <command>`).

## Quick Reference

### Check Current State
- `status` - see active base + profiles for current project
- `skill_list` - browse all skills in the registry
- `profile_list` - see all available profiles
- `agent_list` - see configured agents
- `check_conflicts` - detect overlapping skills
- `budget_estimate` - estimate token cost of a profile

### Skill Operations
- `skill_info <name>` - details, files, provenance, placements
- `skill_files <name>` - list all files in a skill
- `skill_add <source>` - pull from git URL, registry, or local path
- `skill_remove <name>` - remove from registry
- `skill_update <name>` - re-pull from remote source
- `skill_update --all` - re-pull all remote skills
- `skill_create <name>` - scaffold a new skill

### Profile Operations
- `profile_show <name>` - see skills in a profile
- `profile_create <name>` - create profile, optionally with skills
- `profile_delete <name>` - delete a profile
- `profile_edit <name>` - add/remove skills or compose other profiles

### Activation (project-scoped by default)
- `profile_activate <name>` - activate (composable, multiple allowed)
- `profile_deactivate <name>` - deactivate one profile
- `profile_switch <name>` - deactivate all, activate this one
- Use `--global` for global scope instead of project

### Agent Configuration
- `agent_add <name>` - register an agent with discovery paths
- `agent_remove <name>` - unregister an agent
- `agent_enable/disable <name>` - toggle agent per project

### Sync (git-backed config)
- `sync_init` - initialize git tracking for config
- `sync_push` / `sync_pull` - sync with remote

## Guidelines
- Always `status` first to understand current state
- When you detect project type (Cargo.toml -> rust, package.json -> react),
  suggest activating the matching profile
- When the user asks to "remember" a workflow, use `skill_create`
- Profiles are composable - activate multiple if needed
- Use `budget_estimate` before suggesting large profile additions
- Use `doctor` if placements seem inconsistent
```

## GUI

### Tech Stack

| Library | Purpose |
|---------|---------|
| React 19 | UI framework |
| Tailwind CSS | Styling |
| shadcn/ui | Component library (copy-paste, built on Radix UI) |
| `@tauri-apps/api` | IPC to Rust backend |
| `@tanstack/react-query` | Data fetching / cache |
| `react-router` | View navigation |
| `sonner` | Toast notifications (bundled with shadcn/ui) |
| `zod` | Schema validation |

### Views

1. **Dashboard** - Overview stats, recent activity, quick actions
2. **Skills Registry** - Grid/list of all skills, search, filter, CRUD
3. **Profiles** - Base + profile management, composition, drag-and-drop
4. **Projects** - Per-project profile activation, agent configuration
5. **Agents** - Agent discovery path configuration
6. **Activity Log** - Full operation history, filterable
7. **Settings** - Git sync config, MCP server config, defaults, theme

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `sqlx` + SQLite | Database |
| `clap` | CLI argument parsing |
| `rmcp` | MCP server (official Rust SDK) |
| `toml` / `serde` | Config file parsing |
| `sha2` | Content hashing |
| `git2` | Git operations for pulling skills |
| `notify` | File watching (registry changes) |
| `tauri` | GUI framework |

## MVP Scope

The MVP focuses on **profile/scenario activation** as the killer feature:

### In MVP
- Central skill registry with sources tracking
- Composable profiles (base + layers)
- Project-scoped and global activation/deactivation
- CLI with full command set
- SQLite placement tracking + operation logging
- GUI with all 7 views
- MCP server with full tool parity
- Teaching skill for AI agents
- Conflict detection during activation (same target path)
- Context budget estimation (token counting based on file sizes)
- `doctor` command for DB/filesystem consistency checks

### Post-MVP
- Smart activation (auto-detect project type from Cargo.toml, package.json, etc.)
- Git sync for portable config (sync init/push/pull)
- Auto-register MCP server in agent configs
- Skill marketplace / registry integration
- Bulk import from existing agent directories
- Skill diff/changelog on updates
