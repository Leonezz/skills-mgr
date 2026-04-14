---
name: skills-mgr-guide
description: Guide for using skills-mgr to manage AI agent skills
---

# skills-mgr Guide

## What is skills-mgr?

skills-mgr is a cross-agent skill management tool. It manages composable skill profiles that can be activated per-project across multiple AI agents (Claude Code, Cursor, Windsurf, etc.).

## Core Concepts

- **Skills**: Reusable instruction sets (SKILL.md + supporting files) stored in a central registry
- **Profiles**: Named collections of skills that can be composed via `includes`
- **Agents**: AI coding assistants (each with project-level and global skill directories)
- **Projects**: Registered directories where profiles can be linked and activated
- **Placements**: The act of copying skills from the registry into agent directories
- **Global Skills**: Machine-level skills placed into every agent's global path

## Quick Start

```bash
# Add your agents (or use --all for all known presets)
skills-mgr agent add --all

# Register a project
skills-mgr project add /path/to/my-project

# Create and import skills
skills-mgr skill create rust-patterns --description "Rust development patterns"
skills-mgr skill add anthropics/skills/skills/pdf

# Build composable profiles
skills-mgr profile create rust --add rust-patterns
skills-mgr profile create fullstack --add react-patterns --include rust

# Link a profile to your project and activate
cd /path/to/my-project
skills-mgr project link fullstack
skills-mgr profile activate fullstack

# Check status and token budget
skills-mgr status
skills-mgr budget fullstack
```

## How Profiles Work

Profiles support composition via `includes`. When you activate a profile:

1. The profile's skills are resolved (including transitive includes)
2. Base skills (always-on) are added
3. All resolved skills are copied to each agent's project directory
4. Placements are tracked in a local database

When you deactivate a profile, only skills unique to that profile are removed. Shared skills (used by other active profiles) are kept.

## CLI Command Reference

### Skills

```bash
skills-mgr skill list                          # List all skills
skills-mgr skill create <name> --description   # Create new skill scaffold
skills-mgr skill add <source>                  # Add from GitHub/local/hub
skills-mgr skill info <name>                   # Show metadata and files
skills-mgr skill read <name>                   # Display SKILL.md content
skills-mgr skill files <name>                  # List skill files
skills-mgr skill remove <name>                 # Remove from registry
skills-mgr skill open <name>                   # Open directory in editor
skills-mgr skill update <name>                 # Update from remote source
skills-mgr skill update --all                  # Update all remote-sourced skills
skills-mgr skill sync                          # Re-fetch all git-sourced skills
skills-mgr skill browse <source>               # Browse skills in a remote repo
skills-mgr skill browse --hub <name>           # Browse a configured skill hub
skills-mgr skill hubs                          # List configured skill hubs
skills-mgr skill discover                      # Find unmanaged skills in agent paths
skills-mgr skill discover --global-only        # Scan only global paths
skills-mgr skill discover --delegate <profile> # Import discovered skills to a profile
skills-mgr skill link-remote <name> --url <url> --git-ref main  # Link to remote
skills-mgr skill unlink-remote <name>          # Unlink from remote
```

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

# Activation (defaults to current directory)
skills-mgr profile activate <name>             # Activate for current project
skills-mgr profile activate <name> --project /path  # Activate for specific project
skills-mgr profile activate <name> --dry-run   # Preview without changes
skills-mgr profile activate <name> --force     # Overwrite conflicts
skills-mgr profile activate --global           # Activate global skills
skills-mgr profile deactivate <name>           # Deactivate
skills-mgr profile deactivate --global         # Deactivate global skills
skills-mgr profile switch <name>               # Switch from current active profile
skills-mgr profile switch <name> --from old    # Switch from specific profile
```

### Projects

```bash
skills-mgr project list                        # List registered projects
skills-mgr project add <path>                  # Register a project directory
skills-mgr project add <path> --name myproj    # Register with custom name
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
skills-mgr check-conflicts                     # Check for overlapping skills
skills-mgr doctor                              # Verify registry/placement integrity
```

## MCP Server

skills-mgr includes an MCP (Model Context Protocol) server for programmatic access by AI agents. Available tools:

- `list_skills`, `create_skill`, `remove_skill`, `add_skill` — Skill registry
- `list_profiles`, `create_profile`, `delete_profile` — Profile management
- `activate_profile`, `deactivate_profile`, `switch_profile` — Deployment
- `global_status`, `activate_global`, `deactivate_global`, `edit_global_skills` — Global skills
- `list_agents`, `add_agent`, `list_agent_presets` — Agent configuration
- `get_status` — Project status
- `discover_skills` — Skill discovery
- `link_remote`, `unlink_remote`, `update_skill`, `sync_skills` — Remote management
