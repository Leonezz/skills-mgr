/// Well-known agent path presets for common AI coding agents.
pub struct AgentPreset {
    pub name: &'static str,
    pub project_path: &'static str,
    pub global_path: &'static str,
}

pub const KNOWN_AGENTS: &[AgentPreset] = &[
    AgentPreset {
        name: "claude-code",
        project_path: ".claude/skills",
        global_path: "~/.claude/skills",
    },
    AgentPreset {
        name: "cursor",
        project_path: ".cursor/skills",
        global_path: "~/.cursor/skills",
    },
    AgentPreset {
        name: "windsurf",
        project_path: ".windsurf/skills",
        global_path: "~/.windsurf/skills",
    },
    AgentPreset {
        name: "codex",
        project_path: ".codex/skills",
        global_path: "~/.codex/skills",
    },
    AgentPreset {
        name: "copilot",
        project_path: ".github/skills",
        global_path: "~/.github/skills",
    },
    AgentPreset {
        name: "gemini",
        project_path: ".gemini/skills",
        global_path: "~/.gemini/skills",
    },
];

pub fn lookup_preset(name: &str) -> Option<&'static AgentPreset> {
    KNOWN_AGENTS.iter().find(|p| p.name == name)
}

pub fn all_presets() -> &'static [AgentPreset] {
    KNOWN_AGENTS
}
