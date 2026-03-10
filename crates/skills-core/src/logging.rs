use anyhow::Result;
use crate::db::Database;

#[derive(Debug, Clone, Copy)]
pub enum Source {
    Cli,
    Mcp,
    Gui,
}

impl Source {
    pub fn as_str(&self) -> &'static str {
        match self {
            Source::Cli => "cli",
            Source::Mcp => "mcp",
            Source::Gui => "gui",
        }
    }
}

pub async fn log(
    db: &Database,
    source: Source,
    agent_name: Option<&str>,
    operation: &str,
    params: Option<&serde_json::Value>,
    project_path: Option<&str>,
    result: &str,
    details: &str,
) -> Result<()> {
    let params_str = params.map(|p| p.to_string());
    db.log_operation(
        source.as_str(),
        agent_name,
        operation,
        params_str.as_deref(),
        project_path,
        result,
        Some(details),
    ).await
}
