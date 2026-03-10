use skills_core::{AppDirs, Database};

/// MCP Server for skills-mgr.
/// Exposes skill and profile management tools.
pub struct SkillsMcpServer {
    dirs: AppDirs,
    db: Database,
}

impl SkillsMcpServer {
    pub fn new(dirs: AppDirs, db: Database) -> Self {
        Self { dirs, db }
    }

    pub fn dirs(&self) -> &AppDirs {
        &self.dirs
    }

    pub fn db(&self) -> &Database {
        &self.db
    }
}

// Note: Full MCP tool implementation will follow the rmcp #[tool] macro pattern.
// Each CLI command maps to an MCP tool with the same parameters.
// This is a structural placeholder — the actual tool implementations call
// the same skills-core functions as the CLI.

// TODO: Implement MCP tools using rmcp macros once Tauri integration is set up.
// Each tool calls into skills-core the same way the CLI does.
// The tool definitions match the MCP Server Interface table in the design spec.
