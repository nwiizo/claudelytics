use crate::models::{
    DailyReport, DailyUsage, DailyUsageMap, MonthlyReport, MonthlyUsage, SessionReport,
    SessionUsage, SessionUsageMap, TokenUsage, TokenUsageTotals,
};
use chrono::Datelike;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub enum SortField {
    Date,
    Cost,
    Tokens,
    Efficiency,
    Project,
}

#[derive(Clone, Copy, Debug)]
pub enum SortOrder {
    Asc,
    Desc,
}

pub fn generate_daily_report_sorted(
    daily_map: DailyUsageMap,
    sort_field: Option<SortField>,
    sort_order: Option<SortOrder>,
) -> DailyReport {
    let mut daily_entries: Vec<DailyUsage> = daily_map
        .iter()
        .map(|(date, usage)| DailyUsage::from((*date, usage)))
        .collect();

    // Apply sorting
    sort_daily_entries(&mut daily_entries, sort_field, sort_order);

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

pub fn generate_session_report_sorted(
    session_map: SessionUsageMap,
    sort_field: Option<SortField>,
    sort_order: Option<SortOrder>,
) -> SessionReport {
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

    // Apply sorting
    sort_session_entries(&mut session_entries, sort_field, sort_order);

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

pub fn generate_monthly_report_sorted(
    daily_map: DailyUsageMap,
    sort_field: Option<SortField>,
    sort_order: Option<SortOrder>,
) -> MonthlyReport {
    // Group by year-month
    let mut monthly_map: HashMap<(u32, u32), (TokenUsage, u32)> = HashMap::new();

    for (date, usage) in daily_map.iter() {
        let year = date.year() as u32;
        let month = date.month();

        let entry = monthly_map
            .entry((year, month))
            .or_insert((TokenUsage::default(), 0));
        entry.0.add(usage);
        entry.1 += 1; // Count active days
    }

    // Convert to MonthlyUsage entries
    let mut monthly_entries: Vec<MonthlyUsage> = monthly_map
        .into_iter()
        .map(|((year, month), (usage, days_active))| {
            let month_name = match month {
                1 => "January",
                2 => "February",
                3 => "March",
                4 => "April",
                5 => "May",
                6 => "June",
                7 => "July",
                8 => "August",
                9 => "September",
                10 => "October",
                11 => "November",
                12 => "December",
                _ => "Unknown",
            };

            MonthlyUsage {
                month: month_name.to_string(),
                year,
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                cache_creation_tokens: usage.cache_creation_tokens,
                cache_read_tokens: usage.cache_read_tokens,
                total_tokens: usage.total_tokens(),
                total_cost: usage.total_cost,
                days_active,
                avg_daily_cost: if days_active > 0 {
                    usage.total_cost / days_active as f64
                } else {
                    0.0
                },
            }
        })
        .collect();

    // Apply sorting
    sort_monthly_entries(&mut monthly_entries, sort_field, sort_order);

    // Calculate totals
    let totals = daily_map
        .values()
        .fold(TokenUsage::default(), |mut acc, usage| {
            acc.add(usage);
            acc
        });

    MonthlyReport {
        monthly: monthly_entries,
        totals: TokenUsageTotals::from(&totals),
    }
}

fn month_to_num(month: &str) -> u32 {
    match month {
        "January" => 1,
        "February" => 2,
        "March" => 3,
        "April" => 4,
        "May" => 5,
        "June" => 6,
        "July" => 7,
        "August" => 8,
        "September" => 9,
        "October" => 10,
        "November" => 11,
        "December" => 12,
        _ => 0,
    }
}

fn sort_daily_entries(
    entries: &mut [DailyUsage],
    sort_field: Option<SortField>,
    sort_order: Option<SortOrder>,
) {
    let field = sort_field.unwrap_or(SortField::Date);
    let order = sort_order.unwrap_or(SortOrder::Desc);

    entries.sort_by(|a, b| {
        let cmp = match field {
            SortField::Date => a.date.cmp(&b.date),
            SortField::Cost => a
                .total_cost
                .partial_cmp(&b.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortField::Tokens => a.total_tokens.cmp(&b.total_tokens),
            _ => a.date.cmp(&b.date), // Default to date for unsupported fields
        };

        match order {
            SortOrder::Asc => cmp,
            SortOrder::Desc => cmp.reverse(),
        }
    });
}

fn sort_session_entries(
    entries: &mut [SessionUsage],
    sort_field: Option<SortField>,
    sort_order: Option<SortOrder>,
) {
    let field = sort_field.unwrap_or(SortField::Cost);
    let order = sort_order.unwrap_or(SortOrder::Desc);

    entries.sort_by(|a, b| {
        let cmp = match field {
            SortField::Date => a.last_activity.cmp(&b.last_activity),
            SortField::Cost => a
                .total_cost
                .partial_cmp(&b.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortField::Tokens => a.total_tokens.cmp(&b.total_tokens),
            SortField::Efficiency => {
                let eff_a = if a.total_cost > 0.0 {
                    a.total_tokens as f64 / a.total_cost
                } else {
                    0.0
                };
                let eff_b = if b.total_cost > 0.0 {
                    b.total_tokens as f64 / b.total_cost
                } else {
                    0.0
                };
                eff_a
                    .partial_cmp(&eff_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
            SortField::Project => a.project_path.cmp(&b.project_path),
        };

        match order {
            SortOrder::Asc => cmp,
            SortOrder::Desc => cmp.reverse(),
        }
    });
}

fn sort_monthly_entries(
    entries: &mut [MonthlyUsage],
    sort_field: Option<SortField>,
    sort_order: Option<SortOrder>,
) {
    let field = sort_field.unwrap_or(SortField::Date);
    let order = sort_order.unwrap_or(SortOrder::Desc);

    entries.sort_by(|a, b| {
        let cmp = match field {
            SortField::Date => {
                let month_num_a = month_to_num(&a.month);
                let month_num_b = month_to_num(&b.month);
                match a.year.cmp(&b.year) {
                    std::cmp::Ordering::Equal => month_num_a.cmp(&month_num_b),
                    other => other,
                }
            }
            SortField::Cost => a
                .total_cost
                .partial_cmp(&b.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortField::Tokens => a.total_tokens.cmp(&b.total_tokens),
            _ => {
                let month_num_a = month_to_num(&a.month);
                let month_num_b = month_to_num(&b.month);
                match a.year.cmp(&b.year) {
                    std::cmp::Ordering::Equal => month_num_a.cmp(&month_num_b),
                    other => other,
                }
            } // Default to date
        };

        match order {
            SortOrder::Asc => cmp,
            SortOrder::Desc => cmp.reverse(),
        }
    });
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

        let report = generate_daily_report_sorted(daily_map, None, None);
        assert_eq!(report.daily.len(), 1);
        assert_eq!(report.daily[0].date, "2024-01-01");
        assert_eq!(report.totals.input_tokens, 1000);
        assert_eq!(report.totals.total_tokens, 3800);
    }
}
