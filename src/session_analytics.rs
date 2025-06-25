use crate::models::{SessionUsageMap, TokenUsage};
use chrono::{DateTime, Datelike, Duration, Local, Timelike, Utc, Weekday};
use std::collections::HashMap;

/// Advanced session analytics for Claude Code usage patterns
pub struct SessionAnalytics<'a> {
    sessions: &'a SessionUsageMap,
}

/// Time of day usage analysis
#[derive(Debug, Clone)]
pub struct TimeOfDayAnalysis {
    pub hourly_usage: HashMap<u32, HourlyMetrics>,
    pub peak_hour: u32,
    pub off_peak_hour: u32,
    pub business_hours_usage: TokenUsage,
    pub after_hours_usage: TokenUsage,
}

/// Hourly usage metrics
#[derive(Debug, Clone, Default)]
pub struct HourlyMetrics {
    pub usage: TokenUsage,
    pub session_count: usize,
}

/// Day of week analysis
#[derive(Debug, Clone)]
pub struct DayOfWeekAnalysis {
    pub daily_usage: HashMap<Weekday, TokenUsage>,
    pub most_active_day: Weekday,
    pub least_active_day: Weekday,
    pub weekend_vs_weekday_ratio: f64,
}

/// Session duration analysis
#[derive(Debug, Clone)]
pub struct SessionDurationAnalysis {
    pub avg_session_duration: Duration,
    pub longest_session: SessionInfo,
    #[allow(dead_code)]
    pub shortest_session: SessionInfo,
    pub duration_distribution: DurationDistribution,
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub path: String,
    pub duration: Duration,
    pub tokens: u64,
    pub cost: f64,
}

#[derive(Debug, Clone)]
pub struct DurationDistribution {
    pub under_5_min: usize,
    pub min_5_to_30: usize,
    pub min_30_to_60: usize,
    pub hour_1_to_3: usize,
    pub over_3_hours: usize,
}

/// Session frequency analysis
#[derive(Debug, Clone)]
pub struct SessionFrequencyAnalysis {
    pub sessions_per_day: f64,
    pub sessions_per_week: f64,
    pub days_with_usage: usize,
    pub longest_streak: usize,
    pub current_streak: usize,
    pub avg_sessions_per_active_day: f64,
}

/// Cost efficiency analysis
#[derive(Debug, Clone)]
pub struct CostEfficiencyAnalysis {
    pub most_expensive_session: SessionInfo,
    pub most_efficient_session: SessionInfo,
    #[allow(dead_code)]
    pub least_efficient_session: SessionInfo,
    pub sessions_above_threshold: Vec<SessionInfo>,
    #[allow(dead_code)]
    pub cost_threshold: f64,
}

/// Model switching analysis
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ModelSwitchingAnalysis {
    pub switch_count: usize,
    pub avg_sessions_before_switch: f64,
    pub model_loyalty_score: f64, // 0-1, higher means more loyal to single model
}

impl<'a> SessionAnalytics<'a> {
    pub fn new(sessions: &'a SessionUsageMap) -> Self {
        Self { sessions }
    }

    /// Analyze usage patterns by time of day
    pub fn analyze_time_of_day(&self) -> TimeOfDayAnalysis {
        let mut hourly_usage: HashMap<u32, HourlyMetrics> = HashMap::new();
        let mut business_hours = TokenUsage::default();
        let mut after_hours = TokenUsage::default();

        // Group sessions by hour
        for (usage, timestamp) in self.sessions.values() {
            let hour = timestamp.hour();
            let metrics = hourly_usage.entry(hour).or_default();
            metrics.usage.add(usage);
            metrics.session_count += 1;

            // Business hours: 9 AM - 6 PM
            if (9..18).contains(&hour) {
                business_hours.add(usage);
            } else {
                after_hours.add(usage);
            }
        }

        // Find peak and off-peak hours
        let peak_hour = hourly_usage
            .iter()
            .max_by_key(|(_, m)| m.usage.total_tokens())
            .map(|(h, _)| *h)
            .unwrap_or(0);

        let off_peak_hour = hourly_usage
            .iter()
            .min_by_key(|(_, m)| m.usage.total_tokens())
            .map(|(h, _)| *h)
            .unwrap_or(0);

        TimeOfDayAnalysis {
            hourly_usage,
            peak_hour,
            off_peak_hour,
            business_hours_usage: business_hours,
            after_hours_usage: after_hours,
        }
    }

    /// Analyze usage patterns by day of week
    pub fn analyze_day_of_week(&self) -> DayOfWeekAnalysis {
        let mut daily_usage: HashMap<Weekday, TokenUsage> = HashMap::new();
        let mut weekend_usage = TokenUsage::default();
        let mut weekday_usage = TokenUsage::default();

        for (usage, timestamp) in self.sessions.values() {
            let weekday = timestamp.date_naive().weekday();
            daily_usage.entry(weekday).or_default().add(usage);

            match weekday {
                Weekday::Sat | Weekday::Sun => weekend_usage.add(usage),
                _ => weekday_usage.add(usage),
            }
        }

        let most_active_day = daily_usage
            .iter()
            .max_by_key(|(_, u)| u.total_tokens())
            .map(|(d, _)| *d)
            .unwrap_or(Weekday::Mon);

        let least_active_day = daily_usage
            .iter()
            .min_by_key(|(_, u)| u.total_tokens())
            .map(|(d, _)| *d)
            .unwrap_or(Weekday::Sun);

        let weekend_vs_weekday_ratio = if weekday_usage.total_tokens() > 0 {
            weekend_usage.total_tokens() as f64 / weekday_usage.total_tokens() as f64
        } else {
            0.0
        };

        DayOfWeekAnalysis {
            daily_usage,
            most_active_day,
            least_active_day,
            weekend_vs_weekday_ratio,
        }
    }

    /// Analyze session durations
    pub fn analyze_session_durations(&self) -> SessionDurationAnalysis {
        let mut session_durations: Vec<(String, Duration, &TokenUsage)> = Vec::new();
        let mut first_message_times: HashMap<String, DateTime<Utc>> = HashMap::new();

        // Calculate session durations
        for (path, (usage, timestamp)) in self.sessions {
            let session_key = extract_session_key(path);

            if let Some(first_time) = first_message_times.get(&session_key) {
                let duration = *timestamp - *first_time;
                session_durations.push((path.clone(), duration, usage));
            } else {
                first_message_times.insert(session_key, *timestamp);
                // Single message session
                session_durations.push((path.clone(), Duration::seconds(0), usage));
            }
        }

        if session_durations.is_empty() {
            return SessionDurationAnalysis {
                avg_session_duration: Duration::seconds(0),
                longest_session: SessionInfo {
                    path: String::new(),
                    duration: Duration::seconds(0),
                    tokens: 0,
                    cost: 0.0,
                },
                shortest_session: SessionInfo {
                    path: String::new(),
                    duration: Duration::seconds(0),
                    tokens: 0,
                    cost: 0.0,
                },
                duration_distribution: DurationDistribution {
                    under_5_min: 0,
                    min_5_to_30: 0,
                    min_30_to_60: 0,
                    hour_1_to_3: 0,
                    over_3_hours: 0,
                },
            };
        }

        // Calculate average duration
        let total_duration: Duration = session_durations.iter().map(|(_, d, _)| *d).sum();
        let avg_duration = total_duration / session_durations.len() as i32;

        // Find longest and shortest sessions
        let longest = session_durations
            .iter()
            .max_by_key(|(_, d, _)| d.num_seconds())
            .map(|(p, d, u)| SessionInfo {
                path: p.clone(),
                duration: *d,
                tokens: u.total_tokens(),
                cost: u.total_cost,
            })
            .unwrap();

        let shortest = session_durations
            .iter()
            .min_by_key(|(_, d, _)| d.num_seconds())
            .map(|(p, d, u)| SessionInfo {
                path: p.clone(),
                duration: *d,
                tokens: u.total_tokens(),
                cost: u.total_cost,
            })
            .unwrap();

        // Duration distribution
        let mut distribution = DurationDistribution {
            under_5_min: 0,
            min_5_to_30: 0,
            min_30_to_60: 0,
            hour_1_to_3: 0,
            over_3_hours: 0,
        };

        for (_, duration, _) in &session_durations {
            let minutes = duration.num_minutes();
            if minutes < 5 {
                distribution.under_5_min += 1;
            } else if minutes < 30 {
                distribution.min_5_to_30 += 1;
            } else if minutes < 60 {
                distribution.min_30_to_60 += 1;
            } else if minutes < 180 {
                distribution.hour_1_to_3 += 1;
            } else {
                distribution.over_3_hours += 1;
            }
        }

        SessionDurationAnalysis {
            avg_session_duration: avg_duration,
            longest_session: longest,
            shortest_session: shortest,
            duration_distribution: distribution,
        }
    }

    /// Analyze session frequency patterns
    pub fn analyze_session_frequency(&self) -> SessionFrequencyAnalysis {
        let mut daily_sessions: HashMap<chrono::NaiveDate, usize> = HashMap::new();
        let mut all_dates: Vec<chrono::NaiveDate> = Vec::new();

        // Count sessions per day
        for (_usage, timestamp) in self.sessions.values() {
            let date = timestamp.date_naive();
            *daily_sessions.entry(date).or_insert(0) += 1;
            all_dates.push(date);
        }

        all_dates.sort();
        all_dates.dedup();

        let days_with_usage = daily_sessions.len();
        let total_sessions = self.sessions.len();

        // Calculate streaks
        let (longest_streak, current_streak) = if !all_dates.is_empty() {
            let mut longest = 1;
            let mut current = 1;
            let today = Local::now().date_naive();

            for i in 1..all_dates.len() {
                if all_dates[i] - all_dates[i - 1] == Duration::days(1) {
                    current += 1;
                    longest = longest.max(current);
                } else {
                    current = 1;
                }
            }

            // Check if current streak is still active
            let last_date = all_dates.last().unwrap();
            let days_since_last = (today - *last_date).num_days();
            let current_streak = if days_since_last <= 1 { current } else { 0 };

            (longest, current_streak)
        } else {
            (0, 0)
        };

        // Calculate averages
        let first_date = all_dates
            .first()
            .cloned()
            .unwrap_or_else(|| Local::now().date_naive());
        let last_date = all_dates
            .last()
            .cloned()
            .unwrap_or_else(|| Local::now().date_naive());
        let total_days = (last_date - first_date).num_days() + 1;

        let sessions_per_day = if total_days > 0 {
            total_sessions as f64 / total_days as f64
        } else {
            0.0
        };

        let sessions_per_week = sessions_per_day * 7.0;

        let avg_sessions_per_active_day = if days_with_usage > 0 {
            total_sessions as f64 / days_with_usage as f64
        } else {
            0.0
        };

        SessionFrequencyAnalysis {
            sessions_per_day,
            sessions_per_week,
            days_with_usage,
            longest_streak,
            current_streak,
            avg_sessions_per_active_day,
        }
    }

    /// Analyze cost efficiency of sessions
    pub fn analyze_cost_efficiency(&self, cost_threshold: f64) -> CostEfficiencyAnalysis {
        let mut session_efficiencies: Vec<SessionInfo> = Vec::new();

        for (path, (usage, _)) in self.sessions {
            if usage.total_tokens() > 0 {
                session_efficiencies.push(SessionInfo {
                    path: path.clone(),
                    duration: Duration::seconds(0), // Not used here
                    tokens: usage.total_tokens(),
                    cost: usage.total_cost,
                });
            }
        }

        if session_efficiencies.is_empty() {
            return CostEfficiencyAnalysis {
                most_expensive_session: SessionInfo {
                    path: String::new(),
                    duration: Duration::seconds(0),
                    tokens: 0,
                    cost: 0.0,
                },
                most_efficient_session: SessionInfo {
                    path: String::new(),
                    duration: Duration::seconds(0),
                    tokens: 0,
                    cost: 0.0,
                },
                least_efficient_session: SessionInfo {
                    path: String::new(),
                    duration: Duration::seconds(0),
                    tokens: 0,
                    cost: 0.0,
                },
                sessions_above_threshold: Vec::new(),
                cost_threshold,
            };
        }

        // Find most expensive session
        let most_expensive = session_efficiencies
            .iter()
            .max_by(|a, b| a.cost.partial_cmp(&b.cost).unwrap())
            .unwrap()
            .clone();

        // Find most efficient (highest tokens per dollar)
        let most_efficient = session_efficiencies
            .iter()
            .max_by(|a, b| {
                let eff_a = if a.cost > 0.0 {
                    a.tokens as f64 / a.cost
                } else {
                    0.0
                };
                let eff_b = if b.cost > 0.0 {
                    b.tokens as f64 / b.cost
                } else {
                    0.0
                };
                eff_a.partial_cmp(&eff_b).unwrap()
            })
            .unwrap()
            .clone();

        // Find least efficient (lowest tokens per dollar)
        let least_efficient = session_efficiencies
            .iter()
            .filter(|s| s.cost > 0.0)
            .min_by(|a, b| {
                let eff_a = a.tokens as f64 / a.cost;
                let eff_b = b.tokens as f64 / b.cost;
                eff_a.partial_cmp(&eff_b).unwrap()
            })
            .unwrap_or(&most_expensive)
            .clone();

        // Find sessions above threshold
        let sessions_above_threshold: Vec<SessionInfo> = session_efficiencies
            .into_iter()
            .filter(|s| s.cost > cost_threshold)
            .collect();

        CostEfficiencyAnalysis {
            most_expensive_session: most_expensive,
            most_efficient_session: most_efficient,
            least_efficient_session: least_efficient,
            sessions_above_threshold,
            cost_threshold,
        }
    }
}

/// Extract session key from full path
fn extract_session_key(path: &str) -> String {
    // Extract the session identifier from the path
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 2 {
        format!("{}/{}", parts[0], parts[1])
    } else {
        path.to_string()
    }
}

/// Format duration for display
pub fn format_duration(duration: &Duration) -> String {
    let total_seconds = duration.num_seconds();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}
