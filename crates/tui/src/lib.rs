use std::io::{self, IsTerminal};

use anyhow::Result;
use code_count_core::{ScanOptions, ScanReport, scan_path};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    Terminal,
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

    let body = match state.current_view {
        AppView::Dashboard => dashboard_lines(state),
        AppView::Explorer => placeholder_lines("Explorer View", "Language and file browsing"),
        AppView::Report => placeholder_lines("Report View", "Interactive report export"),
    };
    let content = Paragraph::new(body)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(view_title(state.current_view)),
        )
        .wrap(Wrap { trim: true });
    content.render(chunks[1], buffer);

    let footer =
        Paragraph::new("q quit | r rescan | Tab next view | 1 dashboard | 2 explorer | 3 report")
            .block(Block::default().borders(Borders::ALL).title("Keys"));
    footer.render(chunks[2], buffer);
}

fn dashboard_lines(state: &AppState) -> Vec<Line<'static>> {
    let summary = &state.report.summary;
    vec![
        Line::from(format!("Files: {}", summary.files)),
        Line::from(format!("Total lines: {}", summary.total_lines)),
        Line::from(format!("Code lines: {}", summary.code_lines)),
        Line::from(format!("Comment lines: {}", summary.comment_lines)),
        Line::from(format!("Document lines: {}", summary.document_lines)),
        Line::from(format!("Blank lines: {}", summary.blank_lines)),
    ]
}

fn placeholder_lines(title: &'static str, description: &'static str) -> Vec<Line<'static>> {
    vec![Line::from(title), Line::from(description)]
}

fn view_title(view: AppView) -> &'static str {
    match view {
        AppView::Dashboard => "Dashboard",
        AppView::Explorer => "Explorer",
        AppView::Report => "Report",
    }
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
}
