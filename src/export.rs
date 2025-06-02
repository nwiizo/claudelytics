use crate::models::{DailyReport, SessionReport};
use anyhow::Result;
use csv::Writer;
use std::fs::File;
use std::path::Path;

pub fn export_daily_to_csv(report: &DailyReport, path: &Path) -> Result<()> {
    let mut wtr = Writer::from_writer(File::create(path)?);

    // Write header
    wtr.write_record([
        "Date",
        "Input Tokens",
        "Output Tokens",
        "Cache Creation Tokens",
        "Cache Read Tokens",
        "Total Tokens",
        "Cost USD",
    ])?;

    // Write data
    for daily in &report.daily {
        wtr.write_record(&[
            daily.date.to_string(),
            daily.input_tokens.to_string(),
            daily.output_tokens.to_string(),
            daily.cache_creation_tokens.to_string(),
            daily.cache_read_tokens.to_string(),
            daily.total_tokens.to_string(),
            format!("{:.6}", daily.total_cost),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

pub fn export_sessions_to_csv(report: &SessionReport, path: &Path) -> Result<()> {
    let mut wtr = Writer::from_writer(File::create(path)?);

    // Write header
    wtr.write_record([
        "Session Path",
        "Last Activity",
        "Input Tokens",
        "Output Tokens",
        "Cache Creation Tokens",
        "Cache Read Tokens",
        "Total Tokens",
        "Cost USD",
    ])?;

    // Write data
    for session in &report.sessions {
        wtr.write_record(&[
            format!("{}/{}", session.project_path, session.session_id),
            session.last_activity.clone(),
            session.input_tokens.to_string(),
            session.output_tokens.to_string(),
            session.cache_creation_tokens.to_string(),
            session.cache_read_tokens.to_string(),
            session.total_tokens.to_string(),
            format!("{:.6}", session.total_cost),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

pub fn export_summary_to_csv(
    daily_report: &DailyReport,
    session_report: &SessionReport,
    path: &Path,
) -> Result<()> {
    let mut wtr = Writer::from_writer(File::create(path)?);

    // Write summary header
    wtr.write_record(["Metric", "Value"])?;

    // Daily summary
    wtr.write_record(["Total Days", &daily_report.daily.len().to_string()])?;
    wtr.write_record([
        "Total Input Tokens",
        &daily_report.totals.input_tokens.to_string(),
    ])?;
    wtr.write_record([
        "Total Output Tokens",
        &daily_report.totals.output_tokens.to_string(),
    ])?;
    wtr.write_record([
        "Total Cache Creation Tokens",
        &daily_report.totals.cache_creation_tokens.to_string(),
    ])?;
    wtr.write_record([
        "Total Cache Read Tokens",
        &daily_report.totals.cache_read_tokens.to_string(),
    ])?;
    wtr.write_record([
        "Total Cost (USD)",
        &format!("{:.6}", daily_report.totals.total_cost),
    ])?;

    // Session summary
    wtr.write_record(["Total Sessions", &session_report.sessions.len().to_string()])?;

    wtr.flush()?;
    Ok(())
}
