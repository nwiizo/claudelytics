use crate::display::{display_daily_report_table, print_info};
use crate::parser::UsageParser;
use crate::reports::generate_daily_report_sorted;
use anyhow::Result;
use chrono::Local;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

pub struct UsageWatcher {
    parser: UsageParser,
    watcher: RecommendedWatcher,
    receiver: mpsc::Receiver<notify::Result<notify::Event>>,
}

impl UsageWatcher {
    pub fn new(parser: UsageParser) -> Result<Self> {
        let (sender, receiver) = mpsc::channel();

        let watcher = RecommendedWatcher::new(
            move |res| {
                if let Err(e) = sender.send(res) {
                    eprintln!("Watch error: {}", e);
                }
            },
            Config::default(),
        )?;

        Ok(Self {
            parser,
            watcher,
            receiver,
        })
    }

    pub fn watch(&mut self, claude_dir: &Path) -> Result<()> {
        print_info("🔍 Starting watch mode...");
        print_info(&format!("👀 Monitoring: {}", claude_dir.display()));
        print_info("Press Ctrl+C to stop");

        self.watcher.watch(claude_dir, RecursiveMode::Recursive)?;

        // Initial report
        self.display_current_usage()?;

        loop {
            match self.receiver.recv_timeout(Duration::from_secs(5)) {
                Ok(Ok(event)) => {
                    if self.is_relevant_event(&event) {
                        print_info(&format!("📄 File changed: {:?}", event.paths));
                        self.display_current_usage()?;
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("Watch error: {}", e);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Periodic refresh every 5 seconds
                    self.display_current_usage()?;
                }
                Err(e) => {
                    eprintln!("Receiver error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    fn is_relevant_event(&self, event: &notify::Event) -> bool {
        event.paths.iter().any(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "jsonl")
                .unwrap_or(false)
        })
    }

    fn display_current_usage(&self) -> Result<()> {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        println!("\n🕐 Last updated: {}", timestamp);
        println!("{}", "─".repeat(80));

        let (daily_map, _, _) = self.parser.parse_all()?;

        if !daily_map.is_empty() {
            let report = generate_daily_report_sorted(daily_map, None, None);
            display_daily_report_table(&report);
        } else {
            print_info("No usage data found");
        }

        println!("{}", "─".repeat(80));
        Ok(())
    }
}
