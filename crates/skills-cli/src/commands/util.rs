use anyhow::Result;
use skills_core::Database;

pub async fn show_log(db: &Database, limit: i64) -> Result<()> {
    let logs = db.get_recent_logs(limit).await?;
    if logs.is_empty() {
        println!("No operations logged yet.");
        return Ok(());
    }
    for entry in &logs {
        let agent = entry.agent_name.as_deref().unwrap_or("");
        let _project = entry.project_path.as_deref().unwrap_or("");
        let details = entry.details.as_deref().unwrap_or("");
        println!(
            "{} | {} ({}) | {} | {} | {}",
            entry.timestamp, entry.operation, entry.source, agent, entry.result, details
        );
    }
    Ok(())
}
