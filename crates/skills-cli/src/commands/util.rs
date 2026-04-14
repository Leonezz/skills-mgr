use anyhow::Result;
use skills_core::Database;

pub async fn show_log(
    db: &Database,
    limit: i64,
    project: Option<&str>,
    source: Option<&str>,
) -> Result<()> {
    let logs = if let Some(project_path) = project {
        let canonical = std::fs::canonicalize(project_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| project_path.to_string());
        db.get_logs_for_project(&canonical, limit).await?
    } else {
        db.get_recent_logs(limit).await?
    };

    let filtered: Vec<_> = if let Some(source_filter) = source {
        logs.into_iter()
            .filter(|e| e.source.eq_ignore_ascii_case(source_filter))
            .collect()
    } else {
        logs
    };

    if filtered.is_empty() {
        println!("No operations logged yet.");
        return Ok(());
    }
    for entry in &filtered {
        let agent = entry.agent_name.as_deref().unwrap_or("");
        let details = entry.details.as_deref().unwrap_or("");
        println!(
            "{} | {} ({}) | {} | {} | {}",
            entry.timestamp, entry.operation, entry.source, agent, entry.result, details
        );
    }
    Ok(())
}
