//! Visual enhancement components for the TUI
//!
//! This module provides visual feedback components including:
//! - Loading animations with Unicode braille patterns
//! - Smooth progress bars for cost/token usage
//! - Visual feedback for key presses
//! - Toast notifications
//! - Enhanced status bar with real-time information

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
};
use std::time::{Duration, Instant};

/// Loading animation states using Unicode braille patterns
pub const LOADING_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Additional loading animation styles
pub const DOTS_FRAMES: &[&str] = &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
pub const SPINNER_FRAMES: &[&str] = &["◐", "◓", "◑", "◒"];
pub const PROGRESS_FRAMES: &[&str] = &["▱▱▱", "▰▱▱", "▰▰▱", "▰▰▰", "▱▰▰", "▱▱▰"];

/// Loading animation state
#[derive(Debug, Clone)]
pub struct LoadingAnimation {
    frames: Vec<&'static str>,
    current_frame: usize,
    last_update: Instant,
    frame_duration: Duration,
    message: String,
}

impl LoadingAnimation {
    #[allow(dead_code)]
    pub fn new(message: String) -> Self {
        Self {
            frames: LOADING_FRAMES.to_vec(),
            current_frame: 0,
            last_update: Instant::now(),
            frame_duration: Duration::from_millis(80),
            message,
        }
    }

    pub fn with_style(message: String, style: AnimationStyle) -> Self {
        let frames = match style {
            AnimationStyle::Braille => LOADING_FRAMES.to_vec(),
            AnimationStyle::Dots => DOTS_FRAMES.to_vec(),
            AnimationStyle::Spinner => SPINNER_FRAMES.to_vec(),
            AnimationStyle::Progress => PROGRESS_FRAMES.to_vec(),
        };

        Self {
            frames,
            current_frame: 0,
            last_update: Instant::now(),
            frame_duration: Duration::from_millis(80),
            message,
        }
    }

    pub fn tick(&mut self) {
        if self.last_update.elapsed() >= self.frame_duration {
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            self.last_update = Instant::now();
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let text = vec![Line::from(vec![
            Span::styled(
                self.frames[self.current_frame],
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(&self.message, Style::default().fg(Color::White)),
        ])];

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AnimationStyle {
    #[allow(dead_code)]
    Braille,
    Dots,
    Spinner,
    #[allow(dead_code)]
    Progress,
}

/// Enhanced progress bar with smooth animations
#[derive(Debug, Clone)]
pub struct SmoothProgressBar {
    current: f64,
    target: f64,
    max: f64,
    label: String,
    color_scheme: ProgressColorScheme,
    animation_speed: f64,
}

#[derive(Debug, Clone, Copy)]
pub enum ProgressColorScheme {
    CostBased,  // Green -> Yellow -> Red based on cost
    TokenBased, // Blue gradient
    #[allow(dead_code)]
    Efficiency, // Cyan gradient
    #[allow(dead_code)]
    Success, // Green
    #[allow(dead_code)]
    Warning, // Yellow
    #[allow(dead_code)]
    Error, // Red
}

impl SmoothProgressBar {
    pub fn new(label: String, max: f64) -> Self {
        Self {
            current: 0.0,
            target: 0.0,
            max,
            label,
            color_scheme: ProgressColorScheme::CostBased,
            animation_speed: 0.1,
        }
    }

    pub fn set_value(&mut self, value: f64) {
        self.target = value.min(self.max);
    }

    pub fn set_color_scheme(&mut self, scheme: ProgressColorScheme) {
        self.color_scheme = scheme;
    }

    pub fn tick(&mut self) {
        if (self.current - self.target).abs() > 0.001 {
            let diff = self.target - self.current;
            self.current += diff * self.animation_speed;
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let ratio = (self.current / self.max).clamp(0.0, 1.0);

        let color = match self.color_scheme {
            ProgressColorScheme::CostBased => {
                if ratio < 0.33 {
                    Color::Green
                } else if ratio < 0.66 {
                    Color::Yellow
                } else {
                    Color::Red
                }
            }
            ProgressColorScheme::TokenBased => Color::Blue,
            ProgressColorScheme::Efficiency => Color::Cyan,
            ProgressColorScheme::Success => Color::Green,
            ProgressColorScheme::Warning => Color::Yellow,
            ProgressColorScheme::Error => Color::Red,
        };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(color)),
            )
            .gauge_style(Style::default().fg(color).bg(Color::Black))
            .ratio(ratio)
            .label(format!(
                "{}: {:.2}/{:.2}",
                self.label, self.current, self.max
            ));

        f.render_widget(gauge, area);
    }
}

/// Visual feedback for key presses
#[derive(Debug, Clone)]
pub struct KeyPressEffect {
    key: String,
    position: Rect,
    start_time: Instant,
    duration: Duration,
    style: KeyPressStyle,
}

#[derive(Debug, Clone, Copy)]
pub enum KeyPressStyle {
    Flash,
    #[allow(dead_code)]
    Ripple,
    #[allow(dead_code)]
    Fade,
}

impl KeyPressEffect {
    pub fn new(key: String, position: Rect, style: KeyPressStyle) -> Self {
        Self {
            key,
            position,
            start_time: Instant::now(),
            duration: Duration::from_millis(300),
            style,
        }
    }

    pub fn is_active(&self) -> bool {
        self.start_time.elapsed() < self.duration
    }

    pub fn render(&self, f: &mut Frame) {
        let elapsed = self.start_time.elapsed().as_millis() as f32;
        let progress = (elapsed / self.duration.as_millis() as f32).min(1.0);

        let (text, style) = match self.style {
            KeyPressStyle::Flash => {
                let alpha = 1.0 - progress;
                let color = if alpha > 0.5 {
                    Color::Yellow
                } else {
                    Color::Gray
                };
                (
                    format!(" {} ", self.key),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )
            }
            KeyPressStyle::Ripple => {
                let size = (progress * 3.0) as usize;
                let border = "◯".repeat(size.max(1));
                (border, Style::default().fg(Color::Cyan))
            }
            KeyPressStyle::Fade => {
                let alpha = 1.0 - progress;
                let color = if alpha > 0.7 {
                    Color::White
                } else if alpha > 0.3 {
                    Color::Gray
                } else {
                    Color::DarkGray
                };
                (format!("[{}]", self.key), Style::default().fg(color))
            }
        };

        let paragraph = Paragraph::new(text)
            .style(style)
            .alignment(Alignment::Center);

        f.render_widget(paragraph, self.position);
    }
}

/// Toast notification system
#[derive(Debug, Clone)]
pub struct ToastNotification {
    pub message: String,
    pub toast_type: ToastType,
    pub created_at: Instant,
    pub duration: Duration,
    pub position: ToastPosition,
}

#[derive(Debug, Clone, Copy)]
pub enum ToastType {
    Success,
    Info,
    #[allow(dead_code)]
    Warning,
    #[allow(dead_code)]
    Error,
}

#[derive(Debug, Clone, Copy)]
pub enum ToastPosition {
    TopRight,
    #[allow(dead_code)]
    TopLeft,
    #[allow(dead_code)]
    BottomRight,
    #[allow(dead_code)]
    BottomLeft,
    #[allow(dead_code)]
    Center,
}

impl ToastNotification {
    pub fn success(message: String) -> Self {
        Self {
            message,
            toast_type: ToastType::Success,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
            position: ToastPosition::TopRight,
        }
    }

    pub fn info(message: String) -> Self {
        Self {
            message,
            toast_type: ToastType::Info,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
            position: ToastPosition::TopRight,
        }
    }

    #[allow(dead_code)]
    pub fn warning(message: String) -> Self {
        Self {
            message,
            toast_type: ToastType::Warning,
            created_at: Instant::now(),
            duration: Duration::from_secs(4),
            position: ToastPosition::TopRight,
        }
    }

    #[allow(dead_code)]
    pub fn error(message: String) -> Self {
        Self {
            message,
            toast_type: ToastType::Error,
            created_at: Instant::now(),
            duration: Duration::from_secs(5),
            position: ToastPosition::TopRight,
        }
    }

    pub fn is_active(&self) -> bool {
        self.created_at.elapsed() < self.duration
    }

    pub fn get_progress(&self) -> f32 {
        let elapsed = self.created_at.elapsed().as_millis() as f32;
        let total = self.duration.as_millis() as f32;
        (elapsed / total).min(1.0)
    }

    pub fn render(&self, f: &mut Frame, screen_area: Rect) {
        let progress = self.get_progress();
        let fade_start = 0.8;

        let alpha = if progress < fade_start {
            1.0
        } else {
            1.0 - ((progress - fade_start) / (1.0 - fade_start))
        };

        let (icon, color, title) = match self.toast_type {
            ToastType::Success => ("✅", Color::Green, "Success"),
            ToastType::Info => ("ℹ️", Color::Blue, "Info"),
            ToastType::Warning => ("⚠️", Color::Yellow, "Warning"),
            ToastType::Error => ("❌", Color::Red, "Error"),
        };

        // Calculate toast dimensions
        let width = self.message.len().min(40) + 4;
        let height = 3;

        // Calculate position
        let (x, y) = match self.position {
            ToastPosition::TopRight => (
                screen_area
                    .width
                    .saturating_sub(width as u16)
                    .saturating_sub(2),
                2,
            ),
            ToastPosition::TopLeft => (2, 2),
            ToastPosition::BottomRight => (
                screen_area
                    .width
                    .saturating_sub(width as u16)
                    .saturating_sub(2),
                screen_area.height.saturating_sub(height).saturating_sub(2),
            ),
            ToastPosition::BottomLeft => (
                2,
                screen_area.height.saturating_sub(height).saturating_sub(2),
            ),
            ToastPosition::Center => (
                (screen_area.width.saturating_sub(width as u16)) / 2,
                (screen_area.height.saturating_sub(height)) / 2,
            ),
        };

        let toast_area = Rect {
            x,
            y,
            width: width as u16,
            height,
        };

        // Apply fade effect to color
        let display_color = if alpha < 0.5 { Color::DarkGray } else { color };

        let text = vec![Line::from(vec![
            Span::raw(icon),
            Span::raw(" "),
            Span::styled(&self.message, Style::default().fg(Color::White)),
        ])];

        let toast = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(display_color)),
            )
            .style(Style::default().bg(Color::Black))
            .wrap(Wrap { trim: true });

        f.render_widget(toast, toast_area);
    }
}

/// Enhanced status bar with real-time information
#[derive(Debug, Clone)]
pub struct EnhancedStatusBar {
    pub mode: String,
    pub filter: String,
    pub sort: String,
    pub items_count: usize,
    pub selected_index: Option<usize>,
    pub clock: String,
    #[allow(dead_code)]
    pub memory_usage: Option<f64>,
    pub key_hints: Vec<(String, String)>,
}

impl EnhancedStatusBar {
    pub fn new() -> Self {
        Self {
            mode: "Normal".to_string(),
            filter: "All".to_string(),
            sort: "Date".to_string(),
            items_count: 0,
            selected_index: None,
            clock: chrono::Local::now().format("%H:%M:%S").to_string(),
            memory_usage: None,
            key_hints: vec![],
        }
    }

    pub fn update_clock(&mut self) {
        self.clock = chrono::Local::now().format("%H:%M:%S").to_string();
    }

    pub fn set_key_hints(&mut self, hints: Vec<(String, String)>) {
        self.key_hints = hints;
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(20), // Mode and selection
                Constraint::Length(30), // Filter and sort
                Constraint::Min(20),    // Key hints
                Constraint::Length(10), // Clock
            ])
            .split(area);

        // Mode and selection info
        let selection_text = if let Some(idx) = self.selected_index {
            format!(" [{}/{}]", idx + 1, self.items_count)
        } else {
            format!(" [{}]", self.items_count)
        };

        let mode_text = Line::from(vec![
            Span::styled("MODE: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &self.mode,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&selection_text, Style::default().fg(Color::Cyan)),
        ]);

        let mode_widget = Paragraph::new(mode_text);
        f.render_widget(mode_widget, chunks[0]);

        // Filter and sort info
        let filter_text = Line::from(vec![
            Span::styled("FILTER: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&self.filter, Style::default().fg(Color::Green)),
            Span::styled(" | SORT: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&self.sort, Style::default().fg(Color::Blue)),
        ]);

        let filter_widget = Paragraph::new(filter_text);
        f.render_widget(filter_widget, chunks[1]);

        // Key hints
        let mut hint_spans = vec![];
        for (i, (key, action)) in self.key_hints.iter().enumerate() {
            if i > 0 {
                hint_spans.push(Span::raw(" | "));
            }
            hint_spans.push(Span::styled(
                key,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
            hint_spans.push(Span::raw(" "));
            hint_spans.push(Span::styled(action, Style::default().fg(Color::Gray)));
        }

        let hints_widget = Paragraph::new(Line::from(hint_spans));
        f.render_widget(hints_widget, chunks[2]);

        // Clock
        let clock_text = Line::from(vec![
            Span::styled("🕐 ", Style::default().fg(Color::Cyan)),
            Span::styled(&self.clock, Style::default().fg(Color::White)),
        ]);

        let clock_widget = Paragraph::new(clock_text).alignment(Alignment::Right);
        f.render_widget(clock_widget, chunks[3]);
    }
}

/// Manager for all visual effects
#[derive(Debug)]
pub struct VisualEffectsManager {
    pub loading_animations: Vec<LoadingAnimation>,
    pub progress_bars: Vec<SmoothProgressBar>,
    pub key_press_effects: Vec<KeyPressEffect>,
    pub toast_notifications: Vec<ToastNotification>,
    pub status_bar: EnhancedStatusBar,
}

impl VisualEffectsManager {
    pub fn new() -> Self {
        Self {
            loading_animations: Vec::new(),
            progress_bars: Vec::new(),
            key_press_effects: Vec::new(),
            toast_notifications: Vec::new(),
            status_bar: EnhancedStatusBar::new(),
        }
    }

    pub fn add_loading(&mut self, message: String, style: AnimationStyle) {
        self.loading_animations
            .push(LoadingAnimation::with_style(message, style));
    }

    pub fn add_key_effect(&mut self, key: String, position: Rect) {
        self.key_press_effects
            .push(KeyPressEffect::new(key, position, KeyPressStyle::Flash));
    }

    pub fn add_toast(&mut self, toast: ToastNotification) {
        self.toast_notifications.push(toast);
    }

    pub fn tick(&mut self) {
        // Update animations
        for anim in &mut self.loading_animations {
            anim.tick();
        }

        // Update progress bars
        for bar in &mut self.progress_bars {
            bar.tick();
        }

        // Remove expired effects
        self.key_press_effects.retain(|effect| effect.is_active());
        self.toast_notifications.retain(|toast| toast.is_active());

        // Update status bar clock
        self.status_bar.update_clock();
    }

    pub fn render_all(&self, f: &mut Frame) {
        // Render key press effects
        for effect in &self.key_press_effects {
            effect.render(f);
        }

        // Render toast notifications
        for toast in &self.toast_notifications {
            toast.render(f, f.area());
        }
    }
}
