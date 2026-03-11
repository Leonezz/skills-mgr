use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use skills_core::config::{AgentDef, AgentsConfig, ProfileDef, ProfilesConfig};
use skills_core::logging::{self, Source};
use skills_core::profiles;
use skills_core::{AppDirs, Database, Registry};

/// MCP Server for skills-mgr.
/// Implements the Model Context Protocol over stdio (JSON-RPC 2.0).
pub struct SkillsMcpServer {
    dirs: AppDirs,
    db: Database,
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
    pub fn new(dirs: AppDirs, db: Database) -> Self {
        Self { dirs, db }
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
                "description": "Add a new agent configuration",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "project_path": { "type": "string" },
                        "global_path": { "type": "string" }
                    },
                    "required": ["name", "project_path", "global_path"]
                }
            },
            {
                "name": "activate_profile",
                "description": "Activate a profile for a project",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "project_path": { "type": "string" },
                        "force": { "type": "boolean" }
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
                        "project_path": { "type": "string" }
                    },
                    "required": ["name", "project_path"]
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
            }
        ])
    }

    async fn call_tool(&self, name: &str, args: &Value) -> Result<String> {
        match name {
            "list_skills" => {
                let registry = Registry::new(self.dirs.clone());
                let skills = registry.list()?;
                let items: Vec<Value> = skills.iter().map(|s| json!({
                    "name": s.name,
                    "description": s.description,
                    "files": s.files,
                })).collect();
                Ok(serde_json::to_string_pretty(&items)?)
            }
            "create_skill" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let desc = args.get("description").and_then(|v| v.as_str()).unwrap_or("");
                let registry = Registry::new(self.dirs.clone());
                registry.create(name, desc)?;
                let _ = logging::log(&self.db, Source::Mcp, None, "skill_create", None, None, "success", &format!("Created skill '{}'", name)).await;
                Ok(format!("Created skill '{}'", name))
            }
            "remove_skill" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let registry = Registry::new(self.dirs.clone());
                registry.remove(name)?;
                let _ = logging::log(&self.db, Source::Mcp, None, "skill_remove", None, None, "success", &format!("Removed skill '{}'", name)).await;
                Ok(format!("Removed skill '{}'", name))
            }
            "list_profiles" => {
                let config = ProfilesConfig::load(&self.dirs.profiles_toml())?;
                Ok(serde_json::to_string_pretty(&config)?)
            }
            "create_profile" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let skills: Vec<String> = args.get("skills")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let includes: Vec<String> = args.get("includes")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let mut config = ProfilesConfig::load(&self.dirs.profiles_toml())?;
                config.profiles.insert(name.to_string(), ProfileDef {
                    description: None,
                    skills,
                    includes,
                });
                profiles::validate_no_cycles(&config)?;
                config.save(&self.dirs.profiles_toml())?;
                let _ = logging::log(&self.db, Source::Mcp, None, "profile_create", None, None, "success", &format!("Created profile '{}'", name)).await;
                Ok(format!("Created profile '{}'", name))
            }
            "delete_profile" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let mut config = ProfilesConfig::load(&self.dirs.profiles_toml())?;
                config.profiles.remove(name);
                config.save(&self.dirs.profiles_toml())?;
                let _ = logging::log(&self.db, Source::Mcp, None, "profile_delete", None, None, "success", &format!("Deleted profile '{}'", name)).await;
                Ok(format!("Deleted profile '{}'", name))
            }
            "list_agents" => {
                let config = AgentsConfig::load(&self.dirs.agents_toml())?;
                Ok(serde_json::to_string_pretty(&config)?)
            }
            "add_agent" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let project_path = args.get("project_path").and_then(|v| v.as_str()).unwrap_or("");
                let global_path = args.get("global_path").and_then(|v| v.as_str()).unwrap_or("");
                let mut config = AgentsConfig::load(&self.dirs.agents_toml())?;
                config.agents.insert(name.to_string(), AgentDef {
                    project_path: project_path.to_string(),
                    global_path: global_path.to_string(),
                });
                config.save(&self.dirs.agents_toml())?;
                let _ = logging::log(&self.db, Source::Mcp, None, "agent_add", None, None, "success", &format!("Added agent '{}'", name)).await;
                Ok(format!("Added agent '{}'", name))
            }
            "activate_profile" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let project_path = args.get("project_path").and_then(|v| v.as_str()).unwrap_or("");
                let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
                let profiles_config = ProfilesConfig::load(&self.dirs.profiles_toml())?;
                let agents_config = AgentsConfig::load(&self.dirs.agents_toml())?;
                let result = skills_core::placements::activate(&self.dirs, &self.db, &profiles_config, &agents_config, name, project_path, force).await?;
                let _ = logging::log(&self.db, Source::Mcp, None, "profile_activate", None, Some(project_path), "success", &format!("Activated '{}': {} placements", name, result.total_placements)).await;
                Ok(format!("Activated '{}': {} skills, {} placements", result.profile_name, result.skills_placed, result.total_placements))
            }
            "deactivate_profile" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let project_path = args.get("project_path").and_then(|v| v.as_str()).unwrap_or("");
                let result = skills_core::placements::deactivate(&self.db, name, project_path).await?;
                let _ = logging::log(&self.db, Source::Mcp, None, "profile_deactivate", None, Some(project_path), "success", &format!("Deactivated '{}'", name)).await;
                Ok(format!("Deactivated '{}': {} removed, {} kept", result.profile_name, result.files_removed, result.files_kept))
            }
            "get_status" => {
                let project_path = args.get("project_path").and_then(|v| v.as_str()).unwrap_or("");
                let profiles_config = ProfilesConfig::load(&self.dirs.profiles_toml())?;
                let s = skills_core::placements::status(&self.db, &profiles_config, project_path).await?;
                Ok(serde_json::to_string_pretty(&json!({
                    "project_path": s.project_path,
                    "base_skills": s.base_skills,
                    "active_profiles": s.active_profiles,
                    "placement_count": s.placement_count,
                }))?)
            }
            _ => anyhow::bail!("Unknown tool: {}", name),
        }
    }
}
