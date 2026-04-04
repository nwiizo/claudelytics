use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Tabs, Wrap},
};

use super::{AppMode, ExportFormat, SortMode, Tab, TimeFilter, TuiApp};

impl TuiApp {
    pub(crate) fn ui(&mut self, f: &mut Frame) {
        match self.current_mode {
            AppMode::CommandPalette => {
                self.render_main_ui(f);
                self.render_command_palette(f);
            }
            AppMode::ExportDialog => {
                self.render_main_ui(f);
                self.render_export_dialog(f);
            }
            _ => {
                self.render_main_ui(f);
            }
        }

        if self.show_help_popup {
            self.render_help_popup(f);
        }

        // Render visual effects on top
        self.visual_effects.render_all(f);

        // Render loading animations if any
        if !self.visual_effects.loading_animations.is_empty() {
            let loading_area = Rect {
                x: f.area().width / 2 - 15,
                y: f.area().height / 2,
                width: 30,
                height: 3,
            };
            for anim in &self.visual_effects.loading_animations {
                anim.render(f, loading_area);
            }
        }
    }

    pub(crate) fn render_main_ui(&mut self, f: &mut Frame) {
        // Update status bar information
        self.visual_effects.status_bar.mode = match self.current_mode {
            AppMode::Normal => {
                if self.search_mode {
                    "Search"
                } else {
                    "Normal"
                }
            }
            AppMode::CommandPalette => "Command",
            AppMode::Search => "Search",
            AppMode::Visual => "Visual",
            AppMode::ExportDialog => "Export",
        }
        .to_string();

        self.visual_effects.status_bar.filter = match self.time_filter {
            TimeFilter::All => "All",
            TimeFilter::Today => "Today",
            TimeFilter::LastWeek => "Week",
            TimeFilter::LastMonth => "Month",
        }
        .to_string();

        self.visual_effects.status_bar.sort = match self.sort_mode {
            SortMode::Date => "Date",
            SortMode::Cost => "Cost",
            SortMode::Tokens => "Tokens",
            SortMode::Efficiency => "Efficiency",
            SortMode::Project => "Project",
        }
        .to_string();

        self.visual_effects.status_bar.items_count = match self.current_tab {
            Tab::Daily => self.daily_report.daily.len(),
            Tab::Weekly => self
                .weekly_report
                .as_ref()
                .map(|r| r.weekly.len())
                .unwrap_or(0),
            Tab::Sessions => self.session_report.sessions.len(),
            Tab::Cache => self
                .cache_analysis
                .as_ref()
                .map(|a| a.sessions.len())
                .unwrap_or(0),
            _ => 0,
        };

        self.visual_effects.status_bar.selected_index = match self.current_tab {
            Tab::Daily => self.daily_table_state.selected(),
            Tab::Weekly => self.weekly_table_state.selected(),
            Tab::Sessions => self.session_table_state.selected(),
            Tab::Cache => self.cache_table_state.selected(),
            _ => None,
        };

        // Set key hints based on current tab
        let hints = match self.current_tab {
            Tab::Overview => vec![
                ("Tab".to_string(), "Switch".to_string()),
                ("/".to_string(), "Search".to_string()),
                ("r".to_string(), "Refresh".to_string()),
            ],
            Tab::Sessions => vec![
                ("b".to_string(), "Bookmark".to_string()),
                ("x".to_string(), "Compare".to_string()),
                ("s".to_string(), "Sort".to_string()),
                ("f".to_string(), "Filter".to_string()),
            ],
            _ => vec![],
        };
        self.visual_effects.status_bar.set_key_hints(hints);

        let main_chunks = if self.status_message.is_some() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                    Constraint::Length(1), // Status bar
                ])
                .split(f.area())
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(1), // Status bar
                ])
                .split(f.area())
        };

        let main_area = main_chunks[1];

        // Tab bar with enhanced titles
        let tab_titles = vec![
            "\u{1f4ca} Overview",
            "\u{1f4c5} Daily",
            "\u{1f4c6} Weekly",
            "\u{1f4cb} Sessions",
            "\u{1f504} Cache",
            "\u{23f0} Billing",
            "\u{2753} Help",
        ];
        let tabs = Tabs::new(tab_titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Claudelytics Enhanced (Ctrl+P: Command Palette)"),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .select(self.current_tab as usize);
        f.render_widget(tabs, main_chunks[0]);

        // Main content
        match self.current_tab {
            Tab::Overview => self.render_overview(f, main_area),
            Tab::Daily => self.render_daily(f, main_area),
            Tab::Weekly => self.render_weekly(f, main_area),
            Tab::Sessions => self.render_sessions(f, main_area),
            Tab::Cache => self.render_cache(f, main_area),
            Tab::BillingBlocks => self.render_billing_blocks(f, main_area),
            Tab::Help => self.render_help(f, main_area),
        }

        // Status message
        if let Some(ref message) = self.status_message
            && main_chunks.len() > 2
        {
            let status_paragraph = Paragraph::new(message.clone())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Status")
                        .border_style(Style::default().fg(Color::Green)),
                )
                .style(Style::default().fg(Color::Green))
                .wrap(Wrap { trim: true });
            f.render_widget(status_paragraph, main_chunks[2]);
        }

        // Render enhanced status bar at the bottom
        let status_bar_area = main_chunks[main_chunks.len() - 1];
        self.visual_effects.status_bar.render(f, status_bar_area);
    }

    pub(crate) fn render_help_popup(&mut self, f: &mut Frame) {
        let area = f.area();
        let popup_area = Rect {
            x: area.width / 4,
            y: area.height / 4,
            width: area.width / 2,
            height: area.height / 2,
        };

        f.render_widget(ratatui::widgets::Clear, popup_area);

        let help_text = vec![
            Line::from("Quick Help"),
            Line::from(""),
            Line::from("/ - Search"),
            Line::from("s - Sort"),
            Line::from("f - Filter"),
            Line::from("r - Refresh"),
            Line::from("e - Export"),
            Line::from("? - Toggle this popup"),
            Line::from(""),
            Line::from("Press ? again to close"),
        ];

        let popup = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Quick Help")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().bg(Color::Black))
            .wrap(Wrap { trim: true });

        f.render_widget(popup, popup_area);
    }

    pub(crate) fn render_command_palette(&mut self, f: &mut Frame) {
        let area = f.area();
        let popup_area = Rect {
            x: area.width / 6,
            y: area.height / 6,
            width: (area.width * 2) / 3,
            height: (area.height * 2) / 3,
        };

        f.render_widget(Clear, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(popup_area);

        // Search input
        let search_text = if self.command_palette_query.is_empty() {
            "Type to search commands..."
        } else {
            &self.command_palette_query
        };

        let search_paragraph = Paragraph::new(search_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("\u{1f50d} Command Palette")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().bg(Color::Black));
        f.render_widget(search_paragraph, chunks[0]);

        // Commands list
        let header_cells = ["Command", "Description", "Shortcut", "Category"]
            .iter()
            .map(|h| {
                Cell::from(*h).style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            });
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = self.filtered_commands.iter().map(|cmd| {
            let shortcut = cmd.shortcut.as_deref().unwrap_or("");
            Row::new(vec![
                Cell::from(cmd.name.clone()),
                Cell::from(cmd.description.clone()),
                Cell::from(shortcut),
                Cell::from(cmd.category.clone()),
            ])
            .height(1)
        });

        let commands_table = Table::new(
            rows,
            [
                Constraint::Percentage(25),
                Constraint::Percentage(45),
                Constraint::Length(8),
                Constraint::Percentage(20),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    "\u{1f4cb} Commands ({} found)",
                    self.filtered_commands.len()
                ))
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("\u{25ba} ")
        .style(Style::default().bg(Color::Black));

        f.render_stateful_widget(commands_table, chunks[1], &mut self.command_table_state);
    }

    pub(crate) fn render_export_dialog(&mut self, f: &mut Frame) {
        let area = f.area();
        let popup_area = Rect {
            x: area.width / 4,
            y: area.height / 3,
            width: area.width / 2,
            height: 10,
        };

        f.render_widget(Clear, popup_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title("\u{1f4c1} Export Data")
            .border_style(Style::default().fg(Color::Cyan));

        let inner_area = block.inner(popup_area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Min(0),
            ])
            .split(inner_area);

        f.render_widget(block, popup_area);

        // Data type indicator
        let data_type = match self.current_tab {
            Tab::Daily => "Daily Report",
            Tab::Weekly => "Weekly Report",
            Tab::Sessions => "Sessions Report",
            Tab::BillingBlocks => "Billing Blocks",
            Tab::Cache => "Cache Analysis",
            Tab::Overview => "Summary",
            _ => "Current View",
        };

        let data_info = Paragraph::new(vec![Line::from(vec![
            Span::styled("Exporting: ", Style::default().fg(Color::White)),
            Span::styled(
                data_type,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ])]);
        f.render_widget(data_info, chunks[0]);

        // Format selection
        let csv_style = if self.export_dialog_state.selected_format == ExportFormat::Csv {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(Color::White)
        };

        let json_style = if self.export_dialog_state.selected_format == ExportFormat::Json {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(Color::White)
        };

        let format_selection = Paragraph::new(vec![Line::from(vec![
            Span::styled("Format: ", Style::default().fg(Color::Cyan)),
            Span::styled("  CSV  ", csv_style),
            Span::styled("    ", Style::default()),
            Span::styled("  JSON  ", json_style),
        ])])
        .block(Block::default().borders(Borders::NONE));
        f.render_widget(format_selection, chunks[1]);

        // Instructions
        let instructions = Paragraph::new(vec![Line::from(vec![
            Span::styled(
                "\u{2190}\u{2192}/Tab",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Switch format  ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Export  ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Esc",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Cancel", Style::default().fg(Color::Gray)),
        ])]);
        f.render_widget(instructions, chunks[2]);

        // Show success or error message if any
        if self.export_dialog_state.show_success_message {
            let success_msg = Paragraph::new(vec![Line::from(vec![
                Span::styled("\u{2705} ", Style::default().fg(Color::Green)),
                Span::styled(
                    &self.export_dialog_state.success_message,
                    Style::default().fg(Color::Green),
                ),
            ])]);
            f.render_widget(success_msg, chunks[3]);
        } else if let Some(error) = &self.export_dialog_state.error_message {
            let error_msg = Paragraph::new(vec![Line::from(vec![
                Span::styled("\u{274c} ", Style::default().fg(Color::Red)),
                Span::styled(error, Style::default().fg(Color::Red)),
            ])]);
            f.render_widget(error_msg, chunks[3]);
        }
    }
}
