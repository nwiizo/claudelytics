use crate::models::{
    DailyReport, DailyUsage, DailyUsageMap, SessionReport, SessionUsage, SessionUsageMap,
    TokenUsage, TokenUsageTotals,
};

pub fn generate_daily_report(daily_map: DailyUsageMap) -> DailyReport {
    let mut daily_entries: Vec<DailyUsage> = daily_map
        .iter()
        .map(|(date, usage)| DailyUsage::from((*date, usage)))
        .collect();

    // Sort by date descending (most recent first)
    daily_entries.sort_by(|a, b| b.date.cmp(&a.date));

    // Calculate totals
    let totals = daily_map
        .values()
        .fold(TokenUsage::default(), |mut acc, usage| {
            acc.add(usage);
            acc
        });

    DailyReport {
        daily: daily_entries,
        totals: TokenUsageTotals::from(&totals),
    }
}

pub fn generate_session_report(session_map: SessionUsageMap) -> SessionReport {
    let mut session_entries: Vec<SessionUsage> = session_map
        .into_iter()
        .map(|(session_path, (usage, last_activity))| {
            let (project_path, session_id) = parse_session_path(&session_path);
            SessionUsage {
                project_path,
                session_id,
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                cache_creation_tokens: usage.cache_creation_tokens,
                cache_read_tokens: usage.cache_read_tokens,
                total_tokens: usage.total_tokens(),
                total_cost: usage.total_cost,
                last_activity: last_activity.date_naive().format("%Y-%m-%d").to_string(),
            }
        })
        .collect();

    // Sort by total cost descending (highest cost first)
    session_entries.sort_by(|a, b| {
        b.total_cost
            .partial_cmp(&a.total_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Calculate totals
    let totals = session_entries
        .iter()
        .fold(TokenUsage::default(), |mut acc, session| {
            acc.input_tokens += session.input_tokens;
            acc.output_tokens += session.output_tokens;
            acc.cache_creation_tokens += session.cache_creation_tokens;
            acc.cache_read_tokens += session.cache_read_tokens;
            acc.total_cost += session.total_cost;
            acc
        });

    SessionReport {
        sessions: session_entries,
        totals: TokenUsageTotals::from(&totals),
    }
}

fn parse_session_path(session_path: &str) -> (String, String) {
    let parts: Vec<&str> = session_path.split('/').collect();
    if let Some(session_id) = parts.last() {
        let project_path = if parts.len() > 1 {
            parts[..parts.len() - 1].join("/")
        } else {
            "".to_string()
        };
        (project_path, session_id.to_string())
    } else {
        ("".to_string(), session_path.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::collections::HashMap;

    #[test]
    fn test_parse_session_path() {
        let (project, session) = parse_session_path("project-name/nested/path/session-id");
        assert_eq!(project, "project-name/nested/path");
        assert_eq!(session, "session-id");

        let (project, session) = parse_session_path("session-only");
        assert_eq!(project, "");
        assert_eq!(session, "session-only");
    }

    #[test]
    fn test_daily_report_generation() {
        let mut daily_map = HashMap::new();
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 2000,
            cache_creation_tokens: 500,
            cache_read_tokens: 300,
            total_cost: 0.15,
        };
        daily_map.insert(date, usage);

        let report = generate_daily_report(daily_map);
        assert_eq!(report.daily.len(), 1);
        assert_eq!(report.daily[0].date, "2024-01-01");
        assert_eq!(report.totals.input_tokens, 1000);
        assert_eq!(report.totals.total_tokens, 3800);
    }
}
