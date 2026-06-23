use std::io::{self, IsTerminal};

use anyhow::Result;
use code_count_core::{ScanOptions, ScanReport, scan_path};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    Terminal,
    buffer::Buffer,
    prelude::{Backend, Constraint, Direction, Layout, Rect, Widget},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    Dashboard,
    Explorer,
    Report,
}

#[derive(Debug, Clone)]
pub struct AppState {
    report: ScanReport,
    current_view: AppView,
}

impl AppState {
    pub fn new(report: ScanReport) -> Self {
        Self {
            report,
            current_view: AppView::Dashboard,
        }
    }

    pub fn report(&self) -> &ScanReport {
        &self.report
    }

    pub fn current_view(&self) -> AppView {
        self.current_view
    }

    pub fn next_view(&mut self) {
        self.current_view = match self.current_view {
            AppView::Dashboard => AppView::Explorer,
            AppView::Explorer => AppView::Report,
            AppView::Report => AppView::Dashboard,
        };
    }

    pub fn set_view(&mut self, view: AppView) {
        self.current_view = view;
    }

    pub fn rescan(&mut self, options: &ScanOptions) {
        let root = self.report.summary.root.clone();
        self.report = scan_path(root, options);
    }
}

pub fn run(report: ScanReport, options: ScanOptions) -> Result<()> {
    if !io::stdout().is_terminal() {
        print_non_interactive_preview(&report);
        return Ok(());
    }

    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, AppState::new(report), options);
    ratatui::restore();

    result
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut state: AppState,
    options: ScanOptions,
) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    loop {
        terminal.draw(|frame| draw(frame.area(), frame.buffer_mut(), &state))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('r') => state.rescan(&options),
                KeyCode::Tab => state.next_view(),
                KeyCode::Char('1') => state.set_view(AppView::Dashboard),
                KeyCode::Char('2') => state.set_view(AppView::Explorer),
                KeyCode::Char('3') => state.set_view(AppView::Report),
                _ => {}
            }
        }
    }

    Ok(())
}

fn draw(area: Rect, buffer: &mut ratatui::buffer::Buffer, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::styled("code-count", Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::raw(state.report.summary.root.display().to_string()),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Project"));
    title.render(chunks[0], buffer);

    match state.current_view {
        AppView::Dashboard => render_dashboard(chunks[1], buffer, state),
        AppView::Explorer => render_placeholder(
            chunks[1],
            buffer,
            "Explorer",
            placeholder_lines("Explorer View", "Language and file browsing"),
        ),
        AppView::Report => render_placeholder(
            chunks[1],
            buffer,
            "Report",
            placeholder_lines("Report View", "Interactive report export"),
        ),
    }

    let footer =
        Paragraph::new("q quit | r rescan | Tab next view | 1 dashboard | 2 explorer | 3 report")
            .block(Block::default().borders(Borders::ALL).title("Keys"));
    footer.render(chunks[2], buffer);
}

pub fn render_to_text(state: &AppState, width: u16, height: u16) -> String {
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    draw(area, &mut buffer, state);
    buffer_to_string(&buffer, width, height)
}

fn buffer_to_string(buffer: &Buffer, width: u16, height: u16) -> String {
    let mut output = String::new();

    for y in 0..height {
        for x in 0..width {
            output.push_str(buffer[(x, y)].symbol());
        }
        output.push('\n');
    }

    output
}

fn render_dashboard(area: Rect, buffer: &mut Buffer, state: &AppState) {
    if area.width < 70 {
        render_dashboard_stacked(area, buffer, state);
    } else {
        render_dashboard_wide(area, buffer, state);
    }
}

fn render_dashboard_wide(area: Rect, buffer: &mut Buffer, state: &AppState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(8)])
        .split(area);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    render_stats_panel(rows[0], buffer, state);
    render_composition_panel(columns[0], buffer, state);
    render_languages_panel(columns[1], buffer, state);
}

fn render_dashboard_stacked(area: Rect, buffer: &mut Buffer, state: &AppState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Length(8),
            Constraint::Min(8),
        ])
        .split(area);

    render_stats_panel(rows[0], buffer, state);
    render_composition_panel(rows[1], buffer, state);
    render_languages_panel(rows[2], buffer, state);
}

fn render_stats_panel(area: Rect, buffer: &mut Buffer, state: &AppState) {
    let lines = stats_lines(state);
    Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Dashboard"))
        .wrap(Wrap { trim: true })
        .render(area, buffer);
}

fn render_composition_panel(area: Rect, buffer: &mut Buffer, state: &AppState) {
    let lines = composition_lines(state);
    Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Composition"))
        .wrap(Wrap { trim: true })
        .render(area, buffer);
}

fn render_languages_panel(area: Rect, buffer: &mut Buffer, state: &AppState) {
    let lines = language_lines(state);
    Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Top Languages"),
        )
        .wrap(Wrap { trim: true })
        .render(area, buffer);
}

fn render_placeholder(
    area: Rect,
    buffer: &mut Buffer,
    title: &'static str,
    lines: Vec<Line<'static>>,
) {
    Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: true })
        .render(area, buffer);
}

fn stats_lines(state: &AppState) -> Vec<Line<'static>> {
    let summary = &state.report.summary;
    vec![
        Line::from(format!(
            "Files {}  Total {}",
            summary.files, summary.total_lines
        )),
        Line::from(format!("Code {}", summary.code_lines)),
        Line::from(format!("Comments {}", summary.comment_lines)),
        Line::from(format!("Documents {}", summary.document_lines)),
        Line::from(format!("Blank {}", summary.blank_lines)),
    ]
}

fn composition_lines(state: &AppState) -> Vec<Line<'static>> {
    let categories = &state.report.categories;
    let total = state.report.summary.total_lines.max(1);

    vec![
        Line::from(composition_bar(
            categories.code,
            categories.documents,
            categories.comments,
            categories.blanks,
        )),
        Line::from(format!("Code {:>3}%", percentage(categories.code, total))),
        Line::from(format!(
            "Documents {:>3}%",
            percentage(categories.documents, total)
        )),
        Line::from(format!(
            "Comments {:>3}%",
            percentage(categories.comments, total)
        )),
        Line::from(format!(
            "Blank {:>3}%",
            percentage(categories.blanks, total)
        )),
    ]
}

fn language_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for language in state.report.languages.iter().take(6) {
        let category = if language.is_document {
            "Documents"
        } else {
            "Code"
        };
        let visible_lines = if language.is_document {
            language.document_lines
        } else {
            language.code_lines
        };

        lines.push(Line::from(format!(
            "{:<14} {:>6} {:>10}",
            language.name, visible_lines, category
        )));
    }

    if lines.is_empty() {
        lines.push(Line::from("No languages found"));
    }

    lines
}

fn composition_bar(code: usize, documents: usize, comments: usize, blanks: usize) -> String {
    const WIDTH: usize = 30;
    let total = (code + documents + comments + blanks).max(1);
    let code_width = WIDTH * code / total;
    let document_width = WIDTH * documents / total;
    let comment_width = WIDTH * comments / total;
    let mut blank_width = WIDTH.saturating_sub(code_width + document_width + comment_width);

    if code + documents + comments + blanks > 0 && blank_width == 0 && blanks > 0 {
        blank_width = 1;
    }

    format!(
        "{}{}{}{}",
        "#".repeat(code_width),
        "=".repeat(document_width),
        "+".repeat(comment_width),
        ".".repeat(blank_width)
    )
}

fn percentage(value: usize, total: usize) -> usize {
    value * 100 / total
}

fn placeholder_lines(title: &'static str, description: &'static str) -> Vec<Line<'static>> {
    vec![Line::from(title), Line::from(description)]
}

fn print_non_interactive_preview(report: &ScanReport) {
    println!("TUI preview");
    println!("Root: {}", report.summary.root.display());
    println!("Files: {}", report.summary.files);
    println!("Total lines: {}", report.summary.total_lines);
    println!("Run this command in an interactive terminal for the full TUI.");
}

#[cfg(test)]
mod tests {
    use std::fs;

    use code_count_core::{ScanOptions, scan_path};

    use crate::{AppState, AppView};

    #[test]
    fn app_state_starts_on_dashboard_and_cycles_views() {
        let report = scan_path(".", &ScanOptions::default());
        let mut state = AppState::new(report);

        assert_eq!(state.current_view(), AppView::Dashboard);

        state.next_view();
        assert_eq!(state.current_view(), AppView::Explorer);

        state.next_view();
        assert_eq!(state.current_view(), AppView::Report);

        state.next_view();
        assert_eq!(state.current_view(), AppView::Dashboard);
    }

    #[test]
    fn app_state_can_jump_to_numbered_views() {
        let report = scan_path(".", &ScanOptions::default());
        let mut state = AppState::new(report);

        state.set_view(AppView::Report);
        assert_eq!(state.current_view(), AppView::Report);

        state.set_view(AppView::Explorer);
        assert_eq!(state.current_view(), AppView::Explorer);
    }

    #[test]
    fn app_state_can_rescan_root_path() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);

        fs::write(
            temp_dir.path().join("lib.rs"),
            "pub fn answer() -> usize { 42 }\n",
        )
        .expect("write second file");

        state.rescan(&ScanOptions::default());

        assert_eq!(state.report().summary.files, 2);
    }

    #[test]
    fn dashboard_renders_summary_composition_languages_and_keys() {
        let report = scan_path(".", &ScanOptions::default());
        let state = AppState::new(report);

        let output = crate::render_to_text(&state, 96, 28);

        assert!(output.contains("code-count"));
        assert!(output.contains("Files"));
        assert!(output.contains("Total"));
        assert!(output.contains("Code"));
        assert!(output.contains("Comments"));
        assert!(output.contains("Documents"));
        assert!(output.contains("Blank"));
        assert!(output.contains("Composition"));
        assert!(output.contains("Top Languages"));
        assert!(output.contains("q quit"));
        assert!(output.contains("Tab next view"));
    }

    #[test]
    fn dashboard_renders_key_information_on_narrow_terminal() {
        let report = scan_path(".", &ScanOptions::default());
        let state = AppState::new(report);

        let output = crate::render_to_text(&state, 44, 32);

        assert!(output.contains("Dashboard"));
        assert!(output.contains("Files"));
        assert!(output.contains("Composition"));
        assert!(output.contains("Top Languages"));
        assert!(output.contains("q quit"));
    }
}
