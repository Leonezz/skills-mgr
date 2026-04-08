use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use skills_core::config::{AgentDef, AgentsConfig, ProfileDef, ProfilesConfig};
use skills_core::logging::{self, LogEntry, Source};
use skills_core::profiles;
use skills_core::{AppDirs, Database, ProviderRegistry, Registry};

/// MCP Server for skills-mgr.
/// Implements the Model Context Protocol over stdio (JSON-RPC 2.0).
pub struct SkillsMcpServer {
    dirs: AppDirs,
    db: Database,
    providers: ProviderRegistry,
}

#[derive(Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

impl SkillsMcpServer {
    pub fn new(dirs: AppDirs, db: Database, providers: ProviderRegistry) -> Self {
        Self {
            dirs,
            db,
            providers,
        }
    }

    /// Run the MCP server on stdio.
    pub async fn run_stdio(&self) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break; // EOF
            }

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let response = match serde_json::from_str::<JsonRpcRequest>(line) {
                Ok(req) => self.handle_request(req).await,
                Err(e) => JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                    }),
                },
            };

            let mut out = serde_json::to_string(&response)?;
            out.push('\n');
            stdout.write_all(out.as_bytes()).await?;
            stdout.flush().await?;
        }

        Ok(())
    }

    async fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let id = req.id.unwrap_or(Value::Null);
        let _ = req.jsonrpc; // validate if needed

        match req.method.as_str() {
            "initialize" => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": {
                        "name": "skills-mgr",
                        "version": "0.1.0"
                    }
                })),
                error: None,
            },
            "tools/list" => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id,
                result: Some(json!({
                    "tools": Self::tool_definitions()
                })),
                error: None,
            },
            "tools/call" => {
                let params = req.params.unwrap_or(Value::Null);
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                match self.call_tool(tool_name, &args).await {
                    Ok(result) => JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        id,
                        result: Some(json!({
                            "content": [{ "type": "text", "text": result }]
                        })),
                        error: None,
                    },
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        id,
                        result: Some(json!({
                            "content": [{ "type": "text", "text": e.to_string() }],
                            "isError": true
                        })),
                        error: None,
                    },
                }
            }
            _ => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", req.method),
                }),
            },
        }
    }

    fn tool_definitions() -> Value {
        json!([
            {
                "name": "list_skills",
                "description": "List all skills in the registry",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "create_skill",
                "description": "Create a new skill with a scaffold SKILL.md",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Skill name" },
                        "description": { "type": "string", "description": "Skill description" }
                    },
                    "required": ["name", "description"]
                }
            },
            {
                "name": "remove_skill",
                "description": "Remove a skill from the registry",
                "inputSchema": {
                    "type": "object",
                    "properties": { "name": { "type": "string" } },
                    "required": ["name"]
                }
            },
            {
                "name": "add_skill",
                "description": "Add a skill from a local path or remote URL (GitHub, ClawHub, or other hub)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": { "type": "string", "description": "Local path, GitHub URL/shorthand, or hub URL (e.g. https://clawhub.ai/owner/skill)" }
                    },
                    "required": ["source"]
                }
            },
            {
                "name": "list_profiles",
                "description": "List all profiles and base skills",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "create_profile",
                "description": "Create a new profile",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "skills": { "type": "array", "items": { "type": "string" } },
                        "includes": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["name"]
                }
            },
            {
                "name": "delete_profile",
                "description": "Delete a profile",
                "inputSchema": {
                    "type": "object",
                    "properties": { "name": { "type": "string" } },
                    "required": ["name"]
                }
            },
            {
                "name": "list_agents",
                "description": "List all configured agents",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "add_agent",
                "description": "Add a new agent configuration. If project_path/global_path are omitted, uses built-in presets for known agents (claude-code, cursor, windsurf, codex, copilot, gemini).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Agent name (e.g. claude-code, cursor)" },
                        "project_path": { "type": "string", "description": "Relative project skill path (optional for known agents)" },
                        "global_path": { "type": "string", "description": "Absolute global skill path (optional for known agents)" }
                    },
                    "required": ["name"]
                }
            },
            {
                "name": "list_agent_presets",
                "description": "List all known agent presets with their default paths",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "activate_profile",
                "description": "Activate a profile for a project",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "project_path": { "type": "string" },
                        "force": { "type": "boolean" },
                        "dry_run": { "type": "boolean", "description": "Preview without making changes" }
                    },
                    "required": ["name", "project_path"]
                }
            },
            {
                "name": "deactivate_profile",
                "description": "Deactivate a profile for a project",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "project_path": { "type": "string" },
                        "dry_run": { "type": "boolean", "description": "Preview without making changes" }
                    },
                    "required": ["name", "project_path"]
                }
            },
            {
                "name": "switch_profile",
                "description": "Atomically switch from current active profile(s) to a new one using diff-based semantics",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "New profile to activate" },
                        "project_path": { "type": "string" },
                        "from": { "type": "string", "description": "Explicit old profile to switch from (optional, default: all active)" },
                        "force": { "type": "boolean" },
                        "dry_run": { "type": "boolean", "description": "Preview without making changes" }
                    },
                    "required": ["name", "project_path"]
                }
            },
            {
                "name": "global_status",
                "description": "Get global skills status",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "activate_global",
                "description": "Activate global skills (place into agent global paths)",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "deactivate_global",
                "description": "Deactivate global skills (remove from agent global paths)",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "edit_global_skills",
                "description": "Set the global skills list",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "skills": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["skills"]
                }
            },
            {
                "name": "get_status",
                "description": "Get active profiles and placements for a project",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "project_path": { "type": "string" }
                    },
                    "required": ["project_path"]
                }
            },
            {
                "name": "discover_skills",
                "description": "Scan agent paths for unmanaged skills not in the registry",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "global_only": { "type": "boolean", "description": "If true, only scan global agent paths" }
                    }
                }
            },
            {
                "name": "link_remote",
                "description": "Link a local skill to a remote git repository",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Skill name" },
                        "url": { "type": "string", "description": "Remote git URL" },
                        "subpath": { "type": "string", "description": "Subpath within the repo" },
                        "git_ref": { "type": "string", "description": "Git ref (default: main)" }
                    },
                    "required": ["name", "url"]
                }
            },
            {
                "name": "update_skill",
                "description": "Update a remote-sourced skill (git or hub) and refresh all placements",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Skill name to update" }
                    },
                    "required": ["name"]
                }
            },
            {
                "name": "sync_skills",
                "description": "Sync all remote-sourced skills (git and hub) from their tracked remotes",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "unlink_remote",
                "description": "Unlink a skill from its remote repository",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Skill name" }
                    },
                    "required": ["name"]
                }
            }
        ])
    }

    async fn call_tool(&self, name: &str, args: &Value) -> Result<String> {
        match name {
            "list_skills" => {
                let registry = Registry::new(self.dirs.clone());
                let skills = registry.list()?;
                let items: Vec<Value> = skills
                    .iter()
                    .map(|s| {
                        json!({
                            "name": s.name,
                            "description": s.description,
                            "files": s.files,
                            "total_bytes": s.total_bytes,
                            "token_estimate": s.token_estimate,
                            "metadata_token_estimate": s.metadata_token_estimate,
                        })
                    })
                    .collect();
                Ok(serde_json::to_string_pretty(&items)?)
            }
            "create_skill" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .context("name required")?;
                let desc = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .context("description required")?;
                let registry = Registry::new(self.dirs.clone());
                registry.create(name, desc)?;
                let _ = logging::log(
                    &self.db,
                    LogEntry {
                        source: Source::Mcp,
                        agent_name: None,
                        operation: "skill_create",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!("Created skill '{}'", name),
                    },
                )
                .await;
                Ok(format!("Created skill '{}'", name))
            }
            "remove_skill" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .context("name required")?;
                let registry = Registry::new(self.dirs.clone());
                registry.remove(name)?;
                let _ = logging::log(
                    &self.db,
                    LogEntry {
                        source: Source::Mcp,
                        agent_name: None,
                        operation: "skill_remove",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!("Removed skill '{}'", name),
                    },
                )
                .await;
                Ok(format!("Removed skill '{}'", name))
            }
            "add_skill" => {
                let source = args
                    .get("source")
                    .and_then(|v| v.as_str())
                    .context("source required")?;
                let registry = Registry::new(self.dirs.clone());
                let path = std::path::Path::new(source);
                let name = if path.exists() {
                    registry.add_from_local(path)?
                } else if let Some(provider) = self.providers.detect(source) {
                    // GitHub uses add_from_remote for richer source metadata
                    if provider.provider_type() == "github" {
                        registry.add_from_remote(source).await?
                    } else {
                        registry.add_from_provider(source, provider).await?
                    }
                } else {
                    anyhow::bail!(
                        "Source '{}' is not a local path or recognized remote URL",
                        source
                    );
                };
                let _ = logging::log(
                    &self.db,
                    LogEntry {
                        source: Source::Mcp,
                        agent_name: None,
                        operation: "skill_add",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!("Added skill '{}' from {}", name, source),
                    },
                )
                .await;
                Ok(format!("Added skill '{}' from {}", name, source))
            }
            "list_profiles" => {
                let config = ProfilesConfig::load(&self.dirs.profiles_toml())?;
                Ok(serde_json::to_string_pretty(&config)?)
            }
            "create_profile" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .context("name required")?;
                let skills: Vec<String> = args
                    .get("skills")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let includes: Vec<String> = args
                    .get("includes")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let mut config = ProfilesConfig::load(&self.dirs.profiles_toml())?;
                config.profiles.insert(
                    name.to_string(),
                    ProfileDef {
                        description: None,
                        skills,
                        includes,
                    },
                );
                profiles::validate_no_cycles(&config)?;
                config.save(&self.dirs.profiles_toml())?;
                let _ = logging::log(
                    &self.db,
                    LogEntry {
                        source: Source::Mcp,
                        agent_name: None,
                        operation: "profile_create",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!("Created profile '{}'", name),
                    },
                )
                .await;
                Ok(format!("Created profile '{}'", name))
            }
            "delete_profile" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .context("name required")?;
                let mut config = ProfilesConfig::load(&self.dirs.profiles_toml())?;
                if config.profiles.remove(name).is_none() {
                    anyhow::bail!("Profile '{}' not found", name);
                }
                config.save(&self.dirs.profiles_toml())?;
                let _ = logging::log(
                    &self.db,
                    LogEntry {
                        source: Source::Mcp,
                        agent_name: None,
                        operation: "profile_delete",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!("Deleted profile '{}'", name),
                    },
                )
                .await;
                Ok(format!("Deleted profile '{}'", name))
            }
            "list_agents" => {
                let config = AgentsConfig::load(&self.dirs.agents_toml())?;
                Ok(serde_json::to_string_pretty(&config)?)
            }
            "add_agent" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .context("name required")?;
                let explicit_pp = args.get("project_path").and_then(|v| v.as_str());
                let explicit_gp = args.get("global_path").and_then(|v| v.as_str());

                let (project_path, global_path) = match (explicit_pp, explicit_gp) {
                    (Some(pp), Some(gp)) => (pp.to_string(), gp.to_string()),
                    (pp, gp) => {
                        if let Some(preset) = skills_core::lookup_preset(name) {
                            (
                                pp.map(String::from)
                                    .unwrap_or_else(|| preset.project_path.to_string()),
                                gp.map(String::from)
                                    .unwrap_or_else(|| preset.global_path.to_string()),
                            )
                        } else {
                            anyhow::bail!(
                                "Unknown agent '{}'. Provide project_path and global_path, \
                                 or use a known agent: {}",
                                name,
                                skills_core::KNOWN_AGENTS
                                    .iter()
                                    .map(|p| p.name)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            );
                        }
                    }
                };

                let mut config = AgentsConfig::load(&self.dirs.agents_toml())?;
                config.agents.insert(
                    name.to_string(),
                    AgentDef {
                        project_path,
                        global_path,
                        enabled: true,
                    },
                );
                config.save(&self.dirs.agents_toml())?;
                let _ = logging::log(
                    &self.db,
                    LogEntry {
                        source: Source::Mcp,
                        agent_name: None,
                        operation: "agent_add",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!("Added agent '{}'", name),
                    },
                )
                .await;
                Ok(format!("Added agent '{}'", name))
            }
            "list_agent_presets" => {
                let presets: Vec<Value> = skills_core::KNOWN_AGENTS
                    .iter()
                    .map(|p| {
                        json!({
                            "name": p.name,
                            "project_path": p.project_path,
                            "global_path": p.global_path,
                        })
                    })
                    .collect();
                Ok(serde_json::to_string_pretty(&presets)?)
            }
            "activate_profile" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .context("name required")?;
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .context("project_path required")?;
                let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
                let dry_run = args
                    .get("dry_run")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let profiles_config = ProfilesConfig::load(&self.dirs.profiles_toml())?;
                let agents_config = AgentsConfig::load(&self.dirs.agents_toml())?;

                if dry_run {
                    let result = skills_core::placements::dry_run_activate(
                        &self.dirs,
                        &self.db,
                        &profiles_config,
                        &agents_config,
                        name,
                        project_path,
                        force,
                    )
                    .await?;
                    let ops: Vec<Value> = result
                        .operations
                        .iter()
                        .map(|op| {
                            json!({
                                "skill": op.skill_name,
                                "agent": op.agent_name,
                                "target": op.target_path,
                                "action": format!("{:?}", op.action),
                            })
                        })
                        .collect();
                    Ok(serde_json::to_string_pretty(&json!({
                        "dry_run": true,
                        "profile": result.profile_name,
                        "skills": result.skills_resolved,
                        "agents": result.agents_used,
                        "operations": ops,
                    }))?)
                } else {
                    let result = skills_core::placements::activate(
                        &self.dirs,
                        &self.db,
                        &profiles_config,
                        &agents_config,
                        name,
                        project_path,
                        force,
                    )
                    .await?;
                    let _ = logging::log(
                        &self.db,
                        LogEntry {
                            source: Source::Mcp,
                            agent_name: None,
                            operation: "profile_activate",
                            params: None,
                            project_path: Some(project_path),
                            result: "success",
                            details: &format!(
                                "Activated '{}': {} placements",
                                name, result.total_placements
                            ),
                        },
                    )
                    .await;
                    Ok(format!(
                        "Activated '{}': {} skills, {} placements",
                        result.profile_name, result.skills_placed, result.total_placements
                    ))
                }
            }
            "deactivate_profile" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .context("name required")?;
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .context("project_path required")?;
                let dry_run = args
                    .get("dry_run")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if dry_run {
                    let result =
                        skills_core::placements::dry_run_deactivate(&self.db, name, project_path)
                            .await?;
                    let format_ops =
                        |ops: &[skills_core::placements::PlannedOperation]| -> Vec<Value> {
                            ops.iter()
                                .map(|op| {
                                    json!({
                                        "skill": op.skill_name,
                                        "agent": op.agent_name,
                                        "target": op.target_path,
                                    })
                                })
                                .collect()
                        };
                    Ok(serde_json::to_string_pretty(&json!({
                        "dry_run": true,
                        "profile": result.profile_name,
                        "would_remove": format_ops(&result.would_remove),
                        "would_keep": format_ops(&result.would_keep),
                    }))?)
                } else {
                    let result =
                        skills_core::placements::deactivate(&self.db, name, project_path).await?;
                    let _ = logging::log(
                        &self.db,
                        LogEntry {
                            source: Source::Mcp,
                            agent_name: None,
                            operation: "profile_deactivate",
                            params: None,
                            project_path: Some(project_path),
                            result: "success",
                            details: &format!("Deactivated '{}'", name),
                        },
                    )
                    .await;
                    Ok(format!(
                        "Deactivated '{}': {} removed, {} kept",
                        result.profile_name, result.files_removed, result.files_kept
                    ))
                }
            }
            "switch_profile" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .context("name required")?;
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .context("project_path required")?;
                let from = args.get("from").and_then(|v| v.as_str());
                let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
                let dry_run = args
                    .get("dry_run")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let profiles_config = ProfilesConfig::load(&self.dirs.profiles_toml())?;
                let agents_config = AgentsConfig::load(&self.dirs.agents_toml())?;

                if dry_run {
                    let result = skills_core::placements::dry_run_switch(
                        &self.dirs,
                        &self.db,
                        &profiles_config,
                        &agents_config,
                        name,
                        project_path,
                        from,
                    )
                    .await?;
                    Ok(serde_json::to_string_pretty(&json!({
                        "dry_run": true,
                        "old_profiles": result.old_profiles,
                        "new_profile": result.new_profile,
                        "to_add": result.to_add,
                        "to_remove": result.to_remove,
                        "to_keep": result.to_keep,
                    }))?)
                } else {
                    let result = skills_core::placements::switch_profile(
                        &self.dirs,
                        &self.db,
                        &profiles_config,
                        &agents_config,
                        name,
                        project_path,
                        from,
                        force,
                    )
                    .await?;
                    let _ = logging::log(
                        &self.db,
                        LogEntry {
                            source: Source::Mcp,
                            agent_name: None,
                            operation: "profile_switch",
                            params: None,
                            project_path: Some(project_path),
                            result: "success",
                            details: &format!(
                                "Switched to '{}': +{} -{} ~{}",
                                name,
                                result.skills_added,
                                result.skills_removed,
                                result.skills_kept
                            ),
                        },
                    )
                    .await;
                    Ok(format!(
                        "Switched to '{}': +{} added, ~{} kept, -{} removed",
                        result.new_profile,
                        result.skills_added,
                        result.skills_kept,
                        result.skills_removed
                    ))
                }
            }
            "global_status" => {
                let config = ProfilesConfig::load(&self.dirs.profiles_toml())?;
                let status = skills_core::placements::global_status(&self.db, &config).await?;
                Ok(serde_json::to_string_pretty(&json!({
                    "configured_skills": status.configured_skills,
                    "placed_skills": status.placed_skills,
                    "is_active": status.is_active,
                }))?)
            }
            "activate_global" => {
                let config = ProfilesConfig::load(&self.dirs.profiles_toml())?;
                let agents_config = AgentsConfig::load(&self.dirs.agents_toml())?;
                let result = skills_core::placements::activate_global(
                    &self.dirs,
                    &self.db,
                    &config,
                    &agents_config,
                )
                .await?;
                let _ = logging::log(
                    &self.db,
                    LogEntry {
                        source: Source::Mcp,
                        agent_name: None,
                        operation: "global_activate",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!(
                            "Activated global skills: {} placements",
                            result.total_placements
                        ),
                    },
                )
                .await;
                Ok(format!(
                    "Activated {} global skills ({} placements)",
                    result.skills_placed, result.total_placements
                ))
            }
            "deactivate_global" => {
                let result = skills_core::placements::deactivate_global(&self.db).await?;
                let _ = logging::log(
                    &self.db,
                    LogEntry {
                        source: Source::Mcp,
                        agent_name: None,
                        operation: "global_deactivate",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!(
                            "Deactivated global skills: {} removed",
                            result.files_removed
                        ),
                    },
                )
                .await;
                Ok(format!(
                    "Deactivated global skills: {} removed",
                    result.files_removed
                ))
            }
            "edit_global_skills" => {
                let skills: Vec<String> = args
                    .get("skills")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let mut config = ProfilesConfig::load(&self.dirs.profiles_toml())?;
                config.global.skills = skills.clone();
                config.save(&self.dirs.profiles_toml())?;
                let _ = logging::log(
                    &self.db,
                    LogEntry {
                        source: Source::Mcp,
                        agent_name: None,
                        operation: "global_edit",
                        params: None,
                        project_path: None,
                        result: "success",
                        details: &format!("Updated global skills: {}", skills.join(", ")),
                    },
                )
                .await;
                Ok(format!("Updated global skills: {}", skills.join(", ")))
            }
            "get_status" => {
                let project_path = args
                    .get("project_path")
                    .and_then(|v| v.as_str())
                    .context("project_path required")?;
                let profiles_config = ProfilesConfig::load(&self.dirs.profiles_toml())?;
                let s = skills_core::placements::status(&self.db, &profiles_config, project_path)
                    .await?;
                Ok(serde_json::to_string_pretty(&json!({
                    "project_path": s.project_path,
                    "base_skills": s.base_skills,
                    "active_profiles": s.active_profiles,
                    "placement_count": s.placement_count,
                }))?)
            }
            "discover_skills" => {
                let params = args;
                let global_only = params
                    .get("global_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let registry = Registry::new(self.dirs.clone());
                let agents_config = AgentsConfig::load(&self.dirs.agents_toml())?;
                let all_projects = self.db.list_all_projects().await?;
                let project_paths = if global_only {
                    vec![]
                } else {
                    all_projects
                        .iter()
                        .filter(|p| p.path != skills_core::placements::GLOBAL_PROJECT_PATH)
                        .map(|p| p.path.clone())
                        .collect()
                };
                let placed_paths = self.db.collect_placed_paths().await?;
                let discovered = skills_core::discovery::scan_all_agents(
                    &self.dirs,
                    &registry,
                    &agents_config,
                    &project_paths,
                    &placed_paths,
                )?;
                let result: Vec<serde_json::Value> = discovered
                    .iter()
                    .map(|d| {
                        serde_json::json!({
                            "name": d.name,
                            "description": d.description,
                            "agent_name": d.agent_name,
                            "found_path": d.found_path.to_string_lossy(),
                            "scope": match &d.scope {
                                skills_core::discovery::DiscoveryScope::Global => "global".to_string(),
                                skills_core::discovery::DiscoveryScope::Project(p) => p.clone(),
                            },
                            "files": d.files,
                            "total_bytes": d.total_bytes,
                            "token_estimate": d.token_estimate,
                            "exists_in_registry": d.exists_in_registry,
                        })
                    })
                    .collect();
                Ok(serde_json::to_string_pretty(&result)?)
            }
            "update_skill" => {
                let name = args["name"].as_str().context("name required")?;
                let registry = Registry::new(self.dirs.clone());
                let result = registry
                    .update_from_remote(name, Some(&self.providers))
                    .await?;
                match &result {
                    skills_core::registry::SkillUpdateResult::Updated {
                        name, new_hash, ..
                    } => {
                        let replaced =
                            skills_core::placements::replace_skill(&self.dirs, &self.db, name)
                                .await?;
                        let _ = logging::log(
                            &self.db,
                            LogEntry {
                                source: Source::Mcp,
                                agent_name: None,
                                operation: "skill_update",
                                params: None,
                                project_path: None,
                                result: "success",
                                details: &format!(
                                    "Updated '{}', {} placements refreshed",
                                    name, replaced
                                ),
                            },
                        )
                        .await;
                        Ok(format!(
                            "Updated '{}' (new hash: {}…, {} placements refreshed)",
                            name,
                            &new_hash[..20.min(new_hash.len())],
                            replaced
                        ))
                    }
                    skills_core::registry::SkillUpdateResult::AlreadyUpToDate { name } => {
                        Ok(format!("'{}' is already up to date", name))
                    }
                    skills_core::registry::SkillUpdateResult::Skipped { name, reason } => {
                        anyhow::bail!("'{}' skipped: {}", name, reason)
                    }
                    skills_core::registry::SkillUpdateResult::Failed { name, error } => {
                        anyhow::bail!("'{}' failed: {}", name, error)
                    }
                }
            }
            "sync_skills" => {
                let registry = Registry::new(self.dirs.clone());
                let results = registry.sync_all(Some(&self.providers)).await?;
                let mut summary = Vec::new();
                let mut updated_count = 0;
                for result in &results {
                    match result {
                        skills_core::registry::SkillUpdateResult::Updated { name, .. } => {
                            let replaced =
                                skills_core::placements::replace_skill(&self.dirs, &self.db, name)
                                    .await
                                    .unwrap_or(0);
                            summary.push(format!("{}: updated ({} refreshed)", name, replaced));
                            updated_count += 1;
                        }
                        skills_core::registry::SkillUpdateResult::AlreadyUpToDate { name } => {
                            summary.push(format!("{}: up to date", name));
                        }
                        skills_core::registry::SkillUpdateResult::Skipped { name, reason } => {
                            summary.push(format!("{}: skipped ({})", name, reason));
                        }
                        skills_core::registry::SkillUpdateResult::Failed { name, error } => {
                            summary.push(format!("{}: FAILED ({})", name, error));
                        }
                    }
                }
                let failed_count = results
                    .iter()
                    .filter(|r| {
                        matches!(r, skills_core::registry::SkillUpdateResult::Failed { .. })
                    })
                    .count();
                if updated_count > 0 {
                    let _ = logging::log(
                        &self.db,
                        LogEntry {
                            source: Source::Mcp,
                            agent_name: None,
                            operation: "skill_sync",
                            params: None,
                            project_path: None,
                            result: "success",
                            details: &format!("Synced {} skills", updated_count),
                        },
                    )
                    .await;
                }
                let msg = format!(
                    "Sync complete: {} updated, {} failed\n{}",
                    updated_count,
                    failed_count,
                    summary.join("\n")
                );
                if failed_count > 0 {
                    anyhow::bail!("{}", msg)
                } else {
                    Ok(msg)
                }
            }
            "link_remote" => {
                let params = args;
                let name = params["name"].as_str().context("name required")?;
                let url = params["url"].as_str().context("url required")?;
                let subpath = params.get("subpath").and_then(|v| v.as_str());
                let git_ref = params
                    .get("git_ref")
                    .and_then(|v| v.as_str())
                    .unwrap_or("main");
                let registry = Registry::new(self.dirs.clone());
                registry.link_remote(name, url, subpath, git_ref)?;
                Ok(format!(
                    "Linked '{}' to remote: {} (ref: {})",
                    name, url, git_ref
                ))
            }
            "unlink_remote" => {
                let params = args;
                let name = params["name"].as_str().context("name required")?;
                let registry = Registry::new(self.dirs.clone());
                registry.unlink_remote(name)?;
                Ok(format!("Unlinked '{}' from remote", name))
            }
            _ => anyhow::bail!("Unknown tool: {}", name),
        }
    }
}
