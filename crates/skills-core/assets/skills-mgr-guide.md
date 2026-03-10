---
name: skills-mgr-guide
description: Guide for using skills-mgr to manage AI agent skills
---

# skills-mgr Guide

## What is skills-mgr?

skills-mgr is a cross-agent skill management tool. It manages composable skill profiles that can be activated per-project across multiple AI agents (Claude Code, Cursor, etc.).

## Core Concepts

- **Skills**: Reusable instruction sets (SKILL.md + supporting files) stored in a central registry
- **Profiles**: Named collections of skills that can be composed via `includes`
- **Agents**: AI coding assistants (each with project-level and global skill directories)
- **Placements**: The act of copying skills from the registry into agent directories

## Quick Start

```bash
# Add your agents
skills-mgr agent add claude-code --project-path ".claude/skills" --global-path "~/.claude/skills"
skills-mgr agent add cursor --project-path ".cursor/skills" --global-path "~/.cursor/skills"

# Create some skills
skills-mgr skill create rust-patterns --description "Rust development patterns"
skills-mgr skill create react-patterns --description "React development patterns"

# Create profiles
skills-mgr profile create rust --add rust-patterns
skills-mgr profile create fullstack --add react-patterns --include rust

# Activate for your project
skills-mgr profile activate rust

# Check status
skills-mgr status
```

## How Profiles Work

Profiles support composition via `includes`. When you activate a profile:

1. The profile's skills are resolved (including transitive includes)
2. Base skills (always-on) are added
3. All resolved skills are copied to each agent's project directory
4. Placements are tracked in a local database

When you deactivate a profile, only skills unique to that profile are removed. Shared skills (used by other active profiles) are kept.

## Available Commands

- `skills-mgr skill list|add|remove|create|info|files` — Manage the skill registry
- `skills-mgr profile list|create|edit|show|activate|deactivate|switch` — Manage profiles
- `skills-mgr agent list|add|remove` — Configure agents
- `skills-mgr status` — Show current state
- `skills-mgr log` — View operation history
