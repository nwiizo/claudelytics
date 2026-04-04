use crate::models::{DailyReport, MonthlyReport, SessionReport, WeeklyReport};

pub fn display_report_json<T: serde::Serialize>(report: &T) {
    match serde_json::to_string_pretty(report) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error serializing report to JSON: {}", e),
    }
}

pub fn display_daily_report_json(report: &DailyReport) {
    display_report_json(report);
}

pub fn display_session_report_json(report: &SessionReport) {
    display_report_json(report);
}

pub fn display_monthly_report_json(report: &MonthlyReport) {
    display_report_json(report);
}

pub fn display_weekly_report_json(report: &WeeklyReport) {
    display_report_json(report);
}
