use crate::db::Database;
use anyhow::Result;

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

pub struct LogEntry<'a> {
    pub source: Source,
    pub agent_name: Option<&'a str>,
    pub operation: &'a str,
    pub params: Option<&'a serde_json::Value>,
    pub project_path: Option<&'a str>,
    pub result: &'a str,
    pub details: &'a str,
}

pub async fn log(db: &Database, entry: LogEntry<'_>) -> Result<()> {
    let params_str = entry.params.map(|p| p.to_string());
    db.log_operation(
        entry.source.as_str(),
        entry.agent_name,
        entry.operation,
        params_str.as_deref(),
        entry.project_path,
        entry.result,
        Some(entry.details),
    )
    .await
}
