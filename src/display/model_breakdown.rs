use super::helpers::{format_currency, format_number, print_warning};
use crate::terminal::Terminal;
use chrono::Local;
use colored::*;
use comfy_table::{Cell, Color, Table};

// Create a simple structure for family aggregation
#[derive(Debug, Clone, Default)]
struct FamilyUsage {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_tokens: u64,
    cache_read_tokens: u64,
    total_cost: f64,
}

impl FamilyUsage {
    fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_creation_tokens + self.cache_read_tokens
    }

    fn add_usage(&mut self, usage: &crate::models::TokenUsage) {
        self.input_tokens += usage.input_tokens;
        self.output_tokens += usage.output_tokens;
        self.cache_creation_tokens += usage.cache_creation_tokens;
        self.cache_read_tokens += usage.cache_read_tokens;
        self.total_cost += usage.total_cost;
    }
}

pub fn display_model_breakdown_report(
    daily_map: &std::collections::HashMap<chrono::NaiveDate, crate::models::TokenUsage>,
    _session_map: &std::collections::HashMap<
        String,
        (crate::models::TokenUsage, chrono::DateTime<chrono::Utc>),
    >,
) {
    use std::collections::HashMap;

    // Check for display format preference
    let display_format = std::env::var("CLAUDELYTICS_DISPLAY_FORMAT")
        .unwrap_or_else(|_| "default".to_string())
        .to_lowercase();

    // Check if user wants table format (based on FIX_SUMMARY.md documentation)
    if std::env::var("CLAUDELYTICS_TABLE_FORMAT").is_ok() || display_format == "table" {
        // Calculate total cost and tokens for the table display
        let mut total_cost = 0.0;
        let mut total_tokens = 0u64;
        let mut family_usage: HashMap<String, FamilyUsage> = HashMap::new();

        if let Ok(model_breakdown) = parse_usage_by_model() {
            for (family, usage_data) in model_breakdown {
                let fu = family_usage.entry(family.clone()).or_default();
                fu.add_usage(&usage_data);
                total_cost += usage_data.total_cost;
                total_tokens += usage_data.input_tokens
                    + usage_data.output_tokens
                    + usage_data.cache_creation_tokens
                    + usage_data.cache_read_tokens;
            }
        }

        return display_model_breakdown_as_table(&family_usage, total_cost, total_tokens);
    }

    let _registry = crate::models_registry::ModelsRegistry::new();
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

    // Header
    println!("{}", Terminal::separator('═').bright_black());
    println!(
        "{}  {}",
        "📊 Claude Usage by Model Family".bright_blue().bold(),
        format!("Generated {}", timestamp).dimmed()
    );
    println!("{}", Terminal::separator('═').bright_black());
    println!();

    // Group usage by model family
    let mut family_usage: HashMap<String, FamilyUsage> = HashMap::new();

    // Parse raw JSONL files to extract model information
    if let Ok(model_breakdown) = parse_usage_by_model() {
        for (family, usage_data) in model_breakdown {
            let family_usage_entry = FamilyUsage {
                input_tokens: usage_data.input_tokens,
                output_tokens: usage_data.output_tokens,
                cache_creation_tokens: usage_data.cache_creation_tokens,
                cache_read_tokens: usage_data.cache_read_tokens,
                total_cost: usage_data.total_cost,
            };

            family_usage.insert(family, family_usage_entry);
        }
    } else {
        // Fallback to aggregated data if parsing fails
        print_warning(
            "Unable to parse model data from JSONL files, showing aggregated data as 'Unknown'",
        );
        let mut unknown_usage = FamilyUsage::default();

        // Process daily data
        for usage in daily_map.values() {
            unknown_usage.add_usage(usage);
        }

        if unknown_usage.total_tokens() > 0 {
            family_usage.insert("Unknown".to_string(), unknown_usage);
        }
    }

    if family_usage.is_empty() {
        print_warning("No model usage data found");
        return;
    }

    // Sort families by cost (highest first)
    let mut sorted_families: Vec<_> = family_usage.iter().collect();
    sorted_families.sort_by(|a, b| {
        b.1.total_cost
            .partial_cmp(&a.1.total_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Calculate totals
    let total_cost: f64 = family_usage.values().map(|u| u.total_cost).sum();
    let total_tokens: u64 = family_usage.values().map(|u| u.total_tokens()).sum();

    // Display overall summary using ASCII table
    println!("{}", "💰 OVERALL USAGE SUMMARY".bright_yellow().bold());
    println!("{}", "=".repeat(80));

    println!(
        "Total Cost: {}  |  Total Tokens: {}  |  Model Families: {}",
        format_currency(total_cost).bright_green().bold(),
        format_number(total_tokens).bright_magenta().bold(),
        family_usage.len().to_string().bright_blue().bold()
    );

    println!("{}", "=".repeat(80));
    println!();

    // Display breakdown by family
    println!("{}", "📋 USAGE BY MODEL FAMILY".bright_green().bold());
    println!();

    for (family, usage) in sorted_families {
        let cost_str = format_currency(usage.total_cost);
        let tokens_str = format_number(usage.total_tokens());
        let input_str = format_number(usage.input_tokens);
        let output_str = format_number(usage.output_tokens);
        let cache_str = format_number(usage.cache_creation_tokens + usage.cache_read_tokens);

        // Calculate metrics
        let cost_percentage = if total_cost > 0.0 {
            (usage.total_cost / total_cost) * 100.0
        } else {
            0.0
        };

        let token_percentage = if total_tokens > 0 {
            (usage.total_tokens() as f64 / total_tokens as f64) * 100.0
        } else {
            0.0
        };

        let efficiency = if usage.total_cost > 0.0 {
            usage.total_tokens() as f64 / usage.total_cost
        } else {
            0.0
        };

        let output_input_ratio = if usage.input_tokens > 0 {
            usage.output_tokens as f64 / usage.input_tokens as f64
        } else {
            0.0
        };

        // Family icon and display name
        let (family_icon, family_display) = match family.to_lowercase().as_str() {
            "opus" => ("🔥", "Opus"),
            "sonnet" => ("🎵", "Sonnet"),
            "haiku" => ("🌸", "Haiku"),
            _ => ("❓", family.as_str()),
        };

        // Use comfy_table for proper alignment
        let mut _model_table = Table::new();
        _model_table.load_preset(comfy_table::presets::ASCII_FULL);

        println!(
            "{} {} Model Family",
            family_icon,
            family_display.bright_cyan().bold()
        );
        println!("{}", "-".repeat(70));

        // Display metrics in a clean, aligned format
        println!(
            "  Cost:         {:>12} ({:>5.1}%)",
            cost_str.bright_green(),
            cost_percentage
        );
        println!(
            "  Tokens:       {:>12} ({:>5.1}%)",
            tokens_str.bright_magenta(),
            token_percentage
        );
        println!("  Input:        {:>12}", input_str.green());
        println!("  Output:       {:>12}", output_str.blue());
        println!("  Cache:        {:>12}", cache_str.yellow());
        println!(
            "  Efficiency:   {:>12} tok/$",
            format!("{:.0}", efficiency).bright_cyan()
        );
        println!(
            "  O/I Ratio:    {:>12}",
            format!("{:.1}:1", output_input_ratio).bright_yellow()
        );
        println!();
    }

    // Footer
    println!("{}", Terminal::separator('═').bright_black());
}

/// Display model breakdown as a proper aligned table
fn display_model_breakdown_as_table(
    family_usage: &std::collections::HashMap<String, FamilyUsage>,
    total_cost: f64,
    _total_tokens: u64,
) {
    let mut table = Table::new();
    table.load_preset(comfy_table::presets::ASCII_FULL);

    table.set_header(vec![
        Cell::new("Model").fg(Color::Cyan),
        Cell::new("Cost").fg(Color::Green),
        Cell::new("Cost %").fg(Color::Green),
        Cell::new("Tokens").fg(Color::Magenta),
        Cell::new("Input").fg(Color::Blue),
        Cell::new("Output").fg(Color::Yellow),
        Cell::new("Efficiency").fg(Color::Cyan),
    ]);

    // Sort families by cost
    let mut sorted_families: Vec<_> = family_usage.iter().collect();
    sorted_families.sort_by(|a, b| {
        b.1.total_cost
            .partial_cmp(&a.1.total_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for (family, usage) in sorted_families {
        let cost_pct = if total_cost > 0.0 {
            (usage.total_cost / total_cost) * 100.0
        } else {
            0.0
        };
        let efficiency = if usage.total_cost > 0.0 {
            usage.total_tokens() as f64 / usage.total_cost
        } else {
            0.0
        };

        table.add_row(vec![
            Cell::new(family),
            Cell::new(format_currency(usage.total_cost)),
            Cell::new(format!("{:.1}%", cost_pct)),
            Cell::new(format_number(usage.total_tokens())),
            Cell::new(format_number(usage.input_tokens)),
            Cell::new(format_number(usage.output_tokens)),
            Cell::new(format!("{:.0} tok/$", efficiency)),
        ]);
    }

    println!("{}", table);
}

/// Parse raw JSONL files to extract usage by model family
fn parse_usage_by_model()
-> Result<std::collections::HashMap<String, crate::models::TokenUsage>, Box<dyn std::error::Error>>
{
    use crate::models::{TokenUsage, UsageRecord};
    use crate::models_registry::ModelsRegistry;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::PathBuf;
    use walkdir::WalkDir;

    let registry = ModelsRegistry::new();
    let mut family_usage: HashMap<String, TokenUsage> = HashMap::new();

    // Get Claude directory paths (legacy + XDG)
    let home = std::env::var("HOME").map_err(|_| "Unable to determine home directory")?;
    let claude_dirs = vec![
        PathBuf::from(&home).join(".claude"),
        PathBuf::from(&home).join(".config").join("claude"),
    ];

    // Find all JSONL files across all directories
    let mut jsonl_files: Vec<PathBuf> = Vec::new();
    for claude_dir in &claude_dirs {
        let projects_dir = claude_dir.join("projects");
        if !projects_dir.exists() {
            continue;
        }

        let files: Vec<PathBuf> = WalkDir::new(projects_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "jsonl")
                    .unwrap_or(false)
            })
            .map(|entry| entry.path().to_path_buf())
            .collect();
        jsonl_files.extend(files);
    }

    if jsonl_files.is_empty() {
        return Err("Claude projects directory not found".into());
    }

    // Parse each file
    for file_path in jsonl_files {
        if let Ok(file) = File::open(&file_path) {
            let reader = BufReader::new(file);

            for line in reader.lines().map_while(Result::ok) {
                if line.trim().is_empty() {
                    continue;
                }

                if let Ok(record) = serde_json::from_str::<UsageRecord>(&line)
                    && let Some(model_name) = record.get_model_name()
                    && record
                        .message
                        .as_ref()
                        .and_then(|m| m.usage.as_ref())
                        .is_some()
                {
                    let family = registry
                        .get_model_family(model_name)
                        .unwrap_or_else(|| "Unknown".to_string());

                    let family = capitalize_family_name(&family);

                    let usage = TokenUsage::from(&record);
                    family_usage.entry(family).or_default().add(&usage);
                }
            }
        }
    }

    Ok(family_usage)
}

/// Capitalize family name for display
fn capitalize_family_name(family: &str) -> String {
    match family.to_lowercase().as_str() {
        "opus" => "Opus".to_string(),
        "sonnet" => "Sonnet".to_string(),
        "haiku" => "Haiku".to_string(),
        _ => family.to_string(),
    }
}
