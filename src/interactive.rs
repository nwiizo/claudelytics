use crate::models::{SessionReport, SessionUsage};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::io::{Write, stdout};

pub struct InteractiveSelector {
    sessions: Vec<SessionUsage>,
    filtered_sessions: Vec<(usize, SessionUsage)>,
    selected_index: usize,
    query: String,
    matcher: SkimMatcherV2,
}

impl InteractiveSelector {
    pub fn new(session_report: SessionReport) -> Self {
        let sessions = session_report.sessions;
        let filtered_sessions = sessions
            .iter()
            .enumerate()
            .map(|(i, s)| (i, s.clone()))
            .collect();

        Self {
            sessions,
            filtered_sessions,
            selected_index: 0,
            query: String::new(),
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn run(&mut self) -> Result<Option<SessionUsage>> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;

        let result = self.event_loop();

        disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen)?;

        result
    }

    fn event_loop(&mut self) -> Result<Option<SessionUsage>> {
        loop {
            self.draw()?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
                    KeyCode::Enter => {
                        if !self.filtered_sessions.is_empty() {
                            let (_, session) = &self.filtered_sessions[self.selected_index];
                            return Ok(Some(session.clone()));
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if self.selected_index > 0 {
                            self.selected_index -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if self.selected_index < self.filtered_sessions.len().saturating_sub(1) {
                            self.selected_index += 1;
                        }
                    }
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.query.push(c);
                        self.filter_sessions();
                        self.selected_index = 0;
                    }
                    KeyCode::Backspace => {
                        self.query.pop();
                        self.filter_sessions();
                        self.selected_index = 0;
                    }
                    _ => {}
                }
            }
        }
    }

    fn filter_sessions(&mut self) {
        if self.query.is_empty() {
            self.filtered_sessions = self
                .sessions
                .iter()
                .enumerate()
                .map(|(i, s)| (i, s.clone()))
                .collect();
        } else {
            self.filtered_sessions = self
                .sessions
                .iter()
                .enumerate()
                .filter_map(|(i, session)| {
                    let session_path = format!("{}/{}", session.project_path, session.session_id);
                    if self
                        .matcher
                        .fuzzy_match(&session_path, &self.query)
                        .is_some()
                    {
                        Some((i, session.clone()))
                    } else {
                        None
                    }
                })
                .collect();
        }
    }

    fn draw(&self) -> Result<()> {
        print!("\x1B[2J\x1B[H"); // Clear screen and move cursor to top

        println!("📊 Claude Session Selector (ESC/q: quit, Enter: select)");
        println!("🔍 Query: {}", self.query);
        println!("{}", "─".repeat(60));

        for (i, (_, session)) in self.filtered_sessions.iter().enumerate() {
            let marker = if i == self.selected_index {
                "► "
            } else {
                "  "
            };
            let tokens_info = format!(
                "📝 In: {} | 📤 Out: {} | 💰 ${:.4}",
                session.input_tokens, session.output_tokens, session.total_cost
            );

            let session_path = format!("{}/{}", session.project_path, session.session_id);
            if i == self.selected_index {
                println!("\x1B[7m{}{}\x1B[0m", marker, session_path); // Inverted colors
                println!("  {}", tokens_info);
            } else {
                println!("{}{}", marker, session_path);
                println!("  {}", tokens_info);
            }
            println!();
        }

        if self.filtered_sessions.is_empty() {
            println!("No sessions found matching '{}'", self.query);
        }

        stdout().flush()?;
        Ok(())
    }
}
