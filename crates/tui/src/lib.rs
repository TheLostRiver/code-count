use std::{
    fs,
    io::{self, IsTerminal},
    path::Path,
};

use anyhow::Result;
pub use code_count_core::ReportFormat;
use code_count_core::{
    FileStat, LanguageStat, ReportRenderOptions, ScanOptions, ScanReport,
    render_report as render_report_document, report_extension, scan_path,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    Terminal,
    buffer::Buffer,
    prelude::{Backend, Constraint, Direction, Layout, Rect, Widget},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

const COLOR_BG: Color = Color::Rgb(9, 12, 18);
const COLOR_PANEL: Color = Color::Rgb(15, 19, 29);
const COLOR_BORDER: Color = Color::Rgb(37, 42, 54);
const COLOR_TITLE: Color = Color::Rgb(192, 202, 245);
const COLOR_MUTED: Color = Color::Rgb(111, 122, 138);
const COLOR_CODE: Color = Color::Rgb(158, 206, 106);
const COLOR_COMMENTS: Color = Color::Rgb(122, 162, 247);
const COLOR_DOCS: Color = Color::Rgb(224, 175, 104);
const COLOR_BRAND: Color = Color::Rgb(125, 207, 255);
const COLOR_SELECTION: Color = Color::Rgb(31, 39, 58);
const COLOR_KEY_BG: Color = Color::Rgb(37, 44, 68);
const COLOR_KEY_TEXT: Color = Color::Rgb(198, 208, 245);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    Dashboard,
    Explorer,
    Report,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuiOptions {
    pub initial_view: AppView,
    pub report_format: ReportFormat,
}

impl Default for TuiOptions {
    fn default() -> Self {
        Self {
            initial_view: AppView::Dashboard,
            report_format: ReportFormat::Json,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    report: ScanReport,
    current_view: AppView,
    selected_language_index: usize,
    language_filter: String,
    editing_language_filter: bool,
    report_format: ReportFormat,
    include_report_languages: bool,
    include_report_files: bool,
    report_status: Option<String>,
}

impl AppState {
    pub fn new(report: ScanReport) -> Self {
        Self::new_with_options(report, TuiOptions::default())
    }

    pub fn new_with_options(report: ScanReport, options: TuiOptions) -> Self {
        Self {
            report,
            current_view: options.initial_view,
            selected_language_index: 0,
            language_filter: String::new(),
            editing_language_filter: false,
            report_format: options.report_format,
            include_report_languages: true,
            include_report_files: false,
            report_status: None,
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
        self.clamp_selected_language();
    }

    pub fn selected_language(&self) -> Option<&LanguageStat> {
        self.filtered_languages()
            .get(self.selected_language_index)
            .copied()
    }

    pub fn filtered_languages(&self) -> Vec<&LanguageStat> {
        if self.language_filter.is_empty() {
            return self.report.languages.iter().collect();
        }

        let filter = self.language_filter.to_lowercase();
        self.report
            .languages
            .iter()
            .filter(|language| language.name.to_lowercase().contains(&filter))
            .collect()
    }

    pub fn select_next_language(&mut self) {
        let language_count = self.filtered_languages().len();
        if language_count == 0 {
            self.selected_language_index = 0;
            return;
        }

        self.selected_language_index = (self.selected_language_index + 1) % language_count;
    }

    pub fn select_previous_language(&mut self) {
        let language_count = self.filtered_languages().len();
        if language_count == 0 {
            self.selected_language_index = 0;
            return;
        }

        self.selected_language_index = if self.selected_language_index == 0 {
            language_count.saturating_sub(1)
        } else {
            self.selected_language_index.saturating_sub(1)
        };
    }

    pub fn set_language_filter(&mut self, filter: impl Into<String>) {
        self.language_filter = filter.into();
        self.selected_language_index = 0;
    }

    pub fn clear_language_filter(&mut self) {
        self.language_filter.clear();
        self.selected_language_index = 0;
    }

    pub fn report_format(&self) -> ReportFormat {
        self.report_format
    }

    pub fn includes_language_details(&self) -> bool {
        self.include_report_languages
    }

    pub fn includes_file_details(&self) -> bool {
        self.include_report_files
    }

    fn next_report_format(&mut self) {
        self.report_format = next_report_format(self.report_format);
    }

    fn previous_report_format(&mut self) {
        self.report_format = previous_report_format(self.report_format);
    }

    fn toggle_report_languages(&mut self) {
        self.include_report_languages = !self.include_report_languages;
    }

    fn toggle_report_files(&mut self) {
        self.include_report_files = !self.include_report_files;
        if self.include_report_files {
            self.include_report_languages = true;
        }
    }

    fn set_report_status(&mut self, status: impl Into<String>) {
        self.report_status = Some(status.into());
    }

    fn start_language_filter(&mut self) {
        self.editing_language_filter = true;
    }

    fn stop_language_filter(&mut self) {
        self.editing_language_filter = false;
    }

    fn is_editing_language_filter(&self) -> bool {
        self.editing_language_filter
    }

    fn push_language_filter_char(&mut self, value: char) {
        self.language_filter.push(value);
        self.selected_language_index = 0;
    }

    fn pop_language_filter_char(&mut self) {
        self.language_filter.pop();
        self.selected_language_index = 0;
    }

    fn clamp_selected_language(&mut self) {
        let language_count = self.filtered_languages().len();
        if language_count == 0 {
            self.selected_language_index = 0;
        } else {
            self.selected_language_index = self
                .selected_language_index
                .min(language_count.saturating_sub(1));
        }
    }
}

pub fn run(report: ScanReport, options: ScanOptions) -> Result<()> {
    run_with_options(report, options, TuiOptions::default())
}

pub fn run_with_options(
    report: ScanReport,
    options: ScanOptions,
    tui_options: TuiOptions,
) -> Result<()> {
    if !io::stdout().is_terminal() {
        print_non_interactive_preview(&report);
        return Ok(());
    }

    let mut terminal = ratatui::init();
    let result = run_app(
        &mut terminal,
        AppState::new_with_options(report, tui_options),
        options,
    );
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

        if let Event::Key(key) = event::read()?
            && handle_key_event(&mut state, key, &options)
        {
            break;
        }
    }

    Ok(())
}

fn handle_key_event(state: &mut AppState, key: KeyEvent, options: &ScanOptions) -> bool {
    match key.kind {
        KeyEventKind::Press | KeyEventKind::Repeat => handle_key(state, key.code, options),
        KeyEventKind::Release => false,
    }
}

fn handle_key(state: &mut AppState, key_code: KeyCode, options: &ScanOptions) -> bool {
    if state.is_editing_language_filter() {
        match key_code {
            KeyCode::Enter | KeyCode::Esc => state.stop_language_filter(),
            KeyCode::Backspace => state.pop_language_filter_char(),
            KeyCode::Char(value) => state.push_language_filter_char(value),
            _ => {}
        }
        return false;
    }

    match key_code {
        KeyCode::Char('q') => return true,
        KeyCode::Char('r') => state.rescan(options),
        KeyCode::Tab => state.next_view(),
        KeyCode::Char('1') => state.set_view(AppView::Dashboard),
        KeyCode::Char('2') => state.set_view(AppView::Explorer),
        KeyCode::Char('3') => state.set_view(AppView::Report),
        KeyCode::Up if state.current_view() == AppView::Explorer => {
            state.select_previous_language()
        }
        KeyCode::Down if state.current_view() == AppView::Explorer => state.select_next_language(),
        KeyCode::Char('/') if state.current_view() == AppView::Explorer => {
            state.start_language_filter();
        }
        KeyCode::Left if state.current_view() == AppView::Report => state.previous_report_format(),
        KeyCode::Right if state.current_view() == AppView::Report => state.next_report_format(),
        KeyCode::Char('l') if state.current_view() == AppView::Report => {
            state.toggle_report_languages()
        }
        KeyCode::Char('f') if state.current_view() == AppView::Report => {
            state.toggle_report_files()
        }
        KeyCode::Char(' ') if state.current_view() == AppView::Report => {
            state.toggle_report_files()
        }
        KeyCode::Char('e') if state.current_view() == AppView::Report => {
            if let Err(error) = export_report(state) {
                state.set_report_status(format!("Export failed: {error}"));
            }
        }
        _ => {}
    }

    false
}

fn draw(area: Rect, buffer: &mut ratatui::buffer::Buffer, state: &AppState) {
    buffer.set_style(area, Style::default().bg(COLOR_BG));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::styled(command_label(state), Style::default().fg(COLOR_BRAND)),
        Span::raw("  "),
        Span::styled(
            state.report.summary.root.display().to_string(),
            Style::default().fg(COLOR_MUTED),
        ),
        Span::raw("  "),
        Span::styled(status_label(state), Style::default().fg(COLOR_MUTED)),
    ]))
    .block(panel_block(view_title(state.current_view())));
    title.render(chunks[0], buffer);

    match state.current_view {
        AppView::Dashboard => render_dashboard(chunks[1], buffer, state),
        AppView::Explorer => render_explorer(chunks[1], buffer, state),
        AppView::Report => render_report(chunks[1], buffer, state),
    }

    let footer = Paragraph::new(footer_line(state)).block(panel_block("Keys"));
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

fn panel_block(title: &'static str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, Style::default().fg(COLOR_TITLE)))
        .border_style(Style::default().fg(COLOR_BORDER).bg(COLOR_PANEL))
        .style(Style::default().bg(COLOR_PANEL))
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
        .block(panel_block("Project Dashboard"))
        .wrap(Wrap { trim: true })
        .render(area, buffer);
}

fn render_composition_panel(area: Rect, buffer: &mut Buffer, state: &AppState) {
    let lines = composition_lines(state);
    Paragraph::new(lines)
        .block(panel_block("Composition"))
        .wrap(Wrap { trim: true })
        .render(area, buffer);
}

fn render_languages_panel(area: Rect, buffer: &mut Buffer, state: &AppState) {
    let lines = language_lines(state);
    Paragraph::new(lines)
        .block(panel_block("Top Languages"))
        .wrap(Wrap { trim: true })
        .render(area, buffer);
}

fn render_explorer(area: Rect, buffer: &mut Buffer, state: &AppState) {
    if area.width < 70 {
        render_explorer_stacked(area, buffer, state);
    } else {
        render_explorer_wide(area, buffer, state);
    }
}

fn render_explorer_wide(area: Rect, buffer: &mut Buffer, state: &AppState) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);

    render_explorer_languages(columns[0], buffer, state);
    render_explorer_details(columns[1], buffer, state);
}

fn render_explorer_stacked(area: Rect, buffer: &mut Buffer, state: &AppState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    render_explorer_languages(rows[0], buffer, state);
    render_explorer_details(rows[1], buffer, state);
}

fn render_explorer_languages(area: Rect, buffer: &mut Buffer, state: &AppState) {
    Paragraph::new(explorer_language_lines(state))
        .block(panel_block("Languages"))
        .wrap(Wrap { trim: true })
        .render(area, buffer);
}

fn render_explorer_details(area: Rect, buffer: &mut Buffer, state: &AppState) {
    Paragraph::new(explorer_detail_lines(state))
        .block(panel_block("Details"))
        .wrap(Wrap { trim: true })
        .render(area, buffer);
}

fn render_report(area: Rect, buffer: &mut Buffer, state: &AppState) {
    if area.width < 70 {
        render_report_stacked(area, buffer, state);
    } else {
        render_report_wide(area, buffer, state);
    }
}

fn render_report_wide(area: Rect, buffer: &mut Buffer, state: &AppState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),
            Constraint::Min(8),
            Constraint::Length(5),
        ])
        .split(area);

    render_report_controls(rows[0], buffer, state);
    render_report_preview(rows[1], buffer, state);
    render_report_export_panel(rows[2], buffer, state);
}

fn render_report_stacked(area: Rect, buffer: &mut Buffer, state: &AppState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),
            Constraint::Min(8),
            Constraint::Length(5),
        ])
        .split(area);

    render_report_controls(rows[0], buffer, state);
    render_report_preview(rows[1], buffer, state);
    render_report_export_panel(rows[2], buffer, state);
}

fn render_report_controls(area: Rect, buffer: &mut Buffer, state: &AppState) {
    Paragraph::new(report_control_lines(state))
        .block(panel_block("Report Builder"))
        .wrap(Wrap { trim: true })
        .render(area, buffer);
}

fn render_report_preview(area: Rect, buffer: &mut Buffer, state: &AppState) {
    Paragraph::new(report_preview_lines(state))
        .block(panel_block("Preview"))
        .wrap(Wrap { trim: true })
        .render(area, buffer);
}

fn render_report_export_panel(area: Rect, buffer: &mut Buffer, state: &AppState) {
    Paragraph::new(report_export_lines(state))
        .block(panel_block("Export"))
        .wrap(Wrap { trim: true })
        .render(area, buffer);
}

fn stats_lines(state: &AppState) -> Vec<Line<'static>> {
    let summary = &state.report.summary;
    let categories = &state.report.categories;
    vec![
        Line::from(vec![
            Span::styled("Total", Style::default().fg(COLOR_MUTED)),
            Span::raw(" "),
            Span::styled(
                summary.total_lines.to_string(),
                Style::default().fg(COLOR_CODE),
            ),
            Span::raw("   "),
            Span::styled("Files", Style::default().fg(COLOR_MUTED)),
            Span::raw(" "),
            Span::styled(summary.files.to_string(), Style::default().fg(COLOR_TITLE)),
        ]),
        segment_bar(
            categories.code,
            categories.comments,
            categories.documents,
            categories.blanks,
            64,
        ),
        Line::from(vec![
            Span::styled("code", Style::default().fg(COLOR_CODE)),
            Span::raw(format!(" {}", summary.code_lines)),
            Span::raw("   "),
            Span::styled("comments", Style::default().fg(COLOR_COMMENTS)),
            Span::raw(format!(" {}", summary.comment_lines)),
        ]),
        Line::from(vec![
            Span::styled("docs", Style::default().fg(COLOR_DOCS)),
            Span::raw(format!(" {}", summary.document_lines)),
            Span::raw("   "),
            Span::styled("blank", Style::default().fg(COLOR_MUTED)),
            Span::raw(format!(" {}", summary.blank_lines)),
        ]),
    ]
}

fn composition_lines(state: &AppState) -> Vec<Line<'static>> {
    let categories = &state.report.categories;
    let total = state.report.summary.total_lines.max(1);

    vec![
        composition_row("Code", categories.code, total, COLOR_CODE, '#'),
        composition_row("Comments", categories.comments, total, COLOR_COMMENTS, '='),
        composition_row("Docs", categories.documents, total, COLOR_DOCS, '+'),
        composition_row("Blank", categories.blanks, total, COLOR_MUTED, '.'),
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

        let color = if language.is_document {
            COLOR_DOCS
        } else {
            COLOR_CODE
        };
        lines.push(Line::from(vec![
            Span::raw(format!("{:<14}", language.name)),
            Span::styled(format!("{:>8}", visible_lines), Style::default().fg(color)),
            Span::raw("  "),
            Span::styled(category, Style::default().fg(COLOR_MUTED)),
        ]));
    }

    if lines.is_empty() {
        lines.push(Line::from("No languages found"));
    }

    lines
}

fn composition_row(
    label: &'static str,
    value: usize,
    total: usize,
    color: Color,
    fill: char,
) -> Line<'static> {
    let percent = percentage(value, total);
    let filled = (percent * 20 / 100).max(if value > 0 { 1 } else { 0 });
    let empty = 20usize.saturating_sub(filled);

    Line::from(vec![
        Span::styled(format!("{label:<9}"), Style::default().fg(color)),
        Span::styled(fill.to_string().repeat(filled), Style::default().fg(color)),
        Span::styled("-".repeat(empty), Style::default().fg(COLOR_MUTED)),
        Span::raw(" "),
        Span::styled(format!("{percent:>3}%"), Style::default().fg(color)),
    ])
}

fn segment_bar(
    code: usize,
    comments: usize,
    documents: usize,
    blanks: usize,
    width: usize,
) -> Line<'static> {
    let total = (code + comments + documents + blanks).max(1);
    let code_width = segment_width(code, total, width);
    let comments_width = segment_width(comments, total, width);
    let documents_width = segment_width(documents, total, width);
    let used_width = code_width + comments_width + documents_width;
    let blanks_width = width.saturating_sub(used_width);

    Line::from(vec![
        Span::styled("#".repeat(code_width), Style::default().fg(COLOR_CODE)),
        Span::styled(
            "=".repeat(comments_width),
            Style::default().fg(COLOR_COMMENTS),
        ),
        Span::styled("+".repeat(documents_width), Style::default().fg(COLOR_DOCS)),
        Span::styled(".".repeat(blanks_width), Style::default().fg(COLOR_MUTED)),
    ])
}

fn segment_width(value: usize, total: usize, width: usize) -> usize {
    if value == 0 {
        0
    } else {
        (width * value / total).max(1)
    }
}

fn explorer_language_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let total_lines = state.report.summary.total_lines.max(1);

    if state.language_filter.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Filter", Style::default().fg(COLOR_MUTED)),
            Span::raw(":"),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("Filter", Style::default().fg(COLOR_MUTED)),
            Span::raw(": "),
            Span::styled(
                state.language_filter.clone(),
                Style::default().fg(COLOR_TITLE),
            ),
        ]));
    }

    lines.push(Line::from(""));

    for (index, language) in state.filtered_languages().iter().enumerate() {
        let selection = if index == state.selected_language_index {
            ">"
        } else {
            " "
        };
        let category = if language.is_document { "doc" } else { "code" };
        let percent = percentage(language.total_lines, total_lines);
        let color = if language.is_document {
            COLOR_DOCS
        } else {
            COLOR_CODE
        };

        let row_style = if index == state.selected_language_index {
            Style::default().bg(COLOR_SELECTION)
        } else {
            Style::default()
        };

        lines.push(Line::from(vec![
            Span::styled(selection, row_style.fg(COLOR_TITLE)),
            Span::raw(" "),
            Span::styled(format!("{:<16}", language.name), row_style.fg(color)),
            Span::styled(format!("{:>4}%", percent), row_style),
            Span::raw(" "),
            Span::styled(format!("{:>4}", category), row_style.fg(COLOR_MUTED)),
        ]));
    }

    if lines.len() == 2 {
        lines.push(Line::from("No languages found"));
    }

    lines
}

fn explorer_detail_lines(state: &AppState) -> Vec<Line<'static>> {
    let Some(language) = state.selected_language() else {
        return vec![
            Line::from("No language selected"),
            Line::from("Clear the filter to show all languages."),
        ];
    };

    let category = if language.is_document {
        "Documents"
    } else {
        "Code"
    };

    let mut lines = vec![
        Line::from(vec![Span::styled(
            language.name.clone(),
            Style::default().fg(if language.is_document {
                COLOR_DOCS
            } else {
                COLOR_CODE
            }),
        )]),
        segment_bar(
            language.code_lines,
            language.comment_lines,
            language.document_lines,
            language.blank_lines,
            36,
        ),
        Line::from(""),
        Line::from(format!("Category {}", category)),
        Line::from(format!("Files {}", language.files)),
        Line::from(format!("Total {}", language.total_lines)),
        Line::from(format!("Code {}", language.code_lines)),
        Line::from(format!("Comments {}", language.comment_lines)),
        Line::from(format!("Documents {}", language.document_lines)),
        Line::from(format!("Blank {}", language.blank_lines)),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Largest files",
            Style::default().fg(COLOR_MUTED),
        )]),
    ];

    lines.extend(file_detail_lines(language, &state.report.summary.root));
    lines
}

fn file_detail_lines(language: &LanguageStat, root: &Path) -> Vec<Line<'static>> {
    const MAX_FILES: usize = 8;
    let mut lines = Vec::new();

    for file_stat in language.file_stats.iter().take(MAX_FILES) {
        lines.push(Line::from(format_file_stat(file_stat, root)));
    }

    let remaining = language.file_stats.len().saturating_sub(MAX_FILES);
    if remaining > 0 {
        lines.push(Line::from(format!("... {} more files", remaining)));
    }

    if lines.is_empty() {
        lines.push(Line::from("No files found"));
    }

    lines
}

fn report_control_lines(state: &AppState) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "Report sections",
            Style::default().fg(Color::Rgb(187, 154, 247)),
        )]),
        Line::from("[x] Summary"),
        Line::from(format!(
            "[{}] Languages",
            checkbox(state.include_report_languages)
        )),
        Line::from("[x] Documentation ratio"),
        Line::from(format!(
            "[{}] Per-file table",
            checkbox(state.include_report_files)
        )),
        Line::from("[ ] Save history snapshot"),
    ]
}

fn checkbox(enabled: bool) -> &'static str {
    if enabled { "x" } else { " " }
}

fn report_preview_lines(state: &AppState) -> Vec<Line<'static>> {
    let summary = &state.report.summary;
    let mut lines = vec![
        Line::from(vec![Span::styled(
            "# Project Line Report",
            Style::default().fg(COLOR_TITLE),
        )]),
        Line::from(format!("- Root: {}", summary.root.display())),
        Line::from(format!("Target {}", report_output_path(state).display())),
        Line::from(""),
        Line::from(format!("- Files: {}", summary.files)),
        Line::from(format!("- Total lines: {}", summary.total_lines)),
        Line::from(format!("- Code lines: {}", summary.code_lines)),
        Line::from(format!("- Comment lines: {}", summary.comment_lines)),
        Line::from(format!("- Document lines: {}", summary.document_lines)),
        Line::from(format!("- Blank lines: {}", summary.blank_lines)),
        Line::from(""),
    ];

    if state.include_report_languages {
        lines.push(Line::from(format!(
            "Languages included {}",
            state.report.languages.len()
        )));
    } else {
        lines.push(Line::from("Languages included no"));
    }

    if state.include_report_files {
        let file_count: usize = state
            .report
            .languages
            .iter()
            .map(|language| language.file_stats.len())
            .sum();
        lines.push(Line::from(format!("Files included {}", file_count)));
    } else {
        lines.push(Line::from("Files included no"));
    }

    if let Some(status) = &state.report_status {
        lines.push(Line::from(""));
        lines.push(Line::from(status.clone()));
    }

    lines
}

fn report_export_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Format", Style::default().fg(COLOR_MUTED)),
            Span::raw(" "),
            Span::styled(
                report_format_label(state.report_format),
                Style::default().fg(COLOR_CODE),
            ),
            Span::raw("   "),
            Span::styled("Target", Style::default().fg(COLOR_MUTED)),
            Span::raw(" "),
            Span::styled(
                report_output_path(state).display().to_string(),
                Style::default().fg(COLOR_TITLE),
            ),
        ]),
        Line::from(format!(
            "Language details {}   File details {}",
            toggle_label(state.include_report_languages),
            toggle_label(state.include_report_files)
        )),
    ];

    if let Some(status) = &state.report_status {
        lines.push(Line::from(status.clone()));
    }

    lines
}

fn toggle_label(enabled: bool) -> &'static str {
    if enabled { "on" } else { "off" }
}

fn report_output_path(state: &AppState) -> std::path::PathBuf {
    state.report.summary.root.join(format!(
        "code-count-report.{}",
        report_extension(state.report_format)
    ))
}

fn export_report(state: &mut AppState) -> Result<()> {
    let output_path = report_output_path(state);
    let contents = render_report_export(state)?;

    fs::write(&output_path, contents)?;
    let file_name = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("report");
    state.set_report_status(format!("Exported {}", file_name));

    Ok(())
}

fn render_report_export(state: &AppState) -> Result<String> {
    let options = ReportRenderOptions {
        include_language_details: state.include_report_languages,
        include_file_details: state.include_report_files,
    };

    Ok(render_report_document(
        &state.report,
        state.report_format,
        &options,
    )?)
}

fn next_report_format(format: ReportFormat) -> ReportFormat {
    match format {
        ReportFormat::Json => ReportFormat::Markdown,
        ReportFormat::Markdown => ReportFormat::Html,
        ReportFormat::Html => ReportFormat::Csv,
        ReportFormat::Csv => ReportFormat::Json,
    }
}

fn previous_report_format(format: ReportFormat) -> ReportFormat {
    match format {
        ReportFormat::Json => ReportFormat::Csv,
        ReportFormat::Markdown => ReportFormat::Json,
        ReportFormat::Html => ReportFormat::Markdown,
        ReportFormat::Csv => ReportFormat::Html,
    }
}

fn report_format_label(format: ReportFormat) -> &'static str {
    match format {
        ReportFormat::Json => "JSON",
        ReportFormat::Markdown => "Markdown",
        ReportFormat::Html => "HTML",
        ReportFormat::Csv => "CSV",
    }
}

fn format_file_stat(file_stat: &FileStat, root: &Path) -> String {
    let path = display_path(&file_stat.path, root);
    format!(
        "{:<28} total {:>5} code {:>5}",
        path, file_stat.total_lines, file_stat.code_lines
    )
}

fn display_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn percentage(value: usize, total: usize) -> usize {
    value * 100 / total
}

fn view_title(view: AppView) -> &'static str {
    match view {
        AppView::Dashboard => "A. Project Dashboard",
        AppView::Explorer => "B. Explorer + Details",
        AppView::Report => "C. Report Builder",
    }
}

fn command_label(state: &AppState) -> &'static str {
    match state.current_view() {
        AppView::Dashboard => "code-count",
        AppView::Explorer => "code-count tui",
        AppView::Report => "code-count report",
    }
}

fn status_label(state: &AppState) -> &'static str {
    match state.current_view() {
        AppView::Report => "interactive",
        _ => "scan complete",
    }
}

fn key_chip(label: &'static str) -> Span<'static> {
    Span::styled(
        format!("[{label}]"),
        Style::default().fg(COLOR_KEY_TEXT).bg(COLOR_KEY_BG),
    )
}

fn footer_line(state: &AppState) -> Line<'static> {
    if state.is_editing_language_filter() {
        return Line::from(vec![
            Span::raw("type filter | "),
            key_chip("Backspace"),
            Span::raw(" delete | "),
            key_chip("Enter"),
            Span::raw(" apply | "),
            key_chip("Esc"),
            Span::raw(" done"),
        ]);
    }

    match state.current_view() {
        AppView::Explorer => Line::from(vec![
            key_chip("Up/Down"),
            Span::raw(" select | "),
            key_chip("/"),
            Span::raw(" filter | "),
            key_chip("Tab"),
            Span::raw(" view | "),
            key_chip("q"),
            Span::raw(" quit"),
        ]),
        AppView::Report => Line::from(vec![
            key_chip("Left/Right"),
            Span::raw(" format | "),
            key_chip("Space"),
            Span::raw(" toggle | "),
            key_chip("e"),
            Span::raw(" export | "),
            key_chip("q"),
            Span::raw(" quit"),
        ]),
        AppView::Dashboard => Line::from(vec![
            key_chip("Tab"),
            Span::raw(" panes | "),
            key_chip("r"),
            Span::raw(" rescan | "),
            key_chip("q"),
            Span::raw(" quit"),
        ]),
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

    use crate::{AppState, AppView, ReportFormat};

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

        assert!(output.contains("A. Project Dashboard"));
        assert!(output.contains("code-count"));
        assert!(output.contains("scan complete"));
        assert!(output.contains("Total"));
        assert!(output.contains("Files"));
        assert!(output.contains("code"));
        assert!(output.contains("comments"));
        assert!(output.contains("docs"));
        assert!(output.contains("blank"));
        assert!(output.contains("Code"));
        assert!(output.contains("Comments"));
        assert!(output.contains("Blank"));
        assert!(output.contains("Composition"));
        assert!(output.contains("Top Languages"));
        assert!(output.contains("[Tab]"));
        assert!(output.contains("[r]"));
        assert!(output.contains("[q]"));
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
        assert!(output.contains("[q]"));
    }

    #[test]
    fn explorer_renders_language_list_and_selected_details() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(
            temp_dir.path().join("main.rs"),
            "fn main() {\n    // greet\n    println!(\"hello\");\n}\n",
        )
        .expect("write rust file");
        fs::write(
            temp_dir.path().join("README.md"),
            "# Notes\n\nProject notes.\n",
        )
        .expect("write markdown file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);
        state.set_view(AppView::Explorer);

        let output = crate::render_to_text(&state, 96, 28);

        assert!(output.contains("B. Explorer + Details"));
        assert!(output.contains("Languages"));
        assert!(output.contains("Details"));
        assert!(output.contains("Largest files"));
        assert!(output.contains("Rust"));
        assert!(output.contains("Markdown"));
        assert!(output.contains("Files"));
        assert!(output.contains("Category"));
    }

    #[test]
    fn explorer_details_include_files_for_selected_language() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(
            temp_dir.path().join("main.rs"),
            "fn main() {\n    // greet\n\n    println!(\"hello\");\n}\n",
        )
        .expect("write main rust file");
        fs::write(
            temp_dir.path().join("lib.rs"),
            "pub fn answer() -> usize {\n    42\n}\n",
        )
        .expect("write lib rust file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);
        state.set_view(AppView::Explorer);

        let output = crate::render_to_text(&state, 96, 32);

        assert!(output.contains("Files"));
        assert!(output.contains("main.rs"));
        assert!(output.contains("lib.rs"));
        assert!(output.contains("Total"));
        assert!(output.contains("Code"));
    }

    #[test]
    fn explorer_selection_moves_between_languages() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write rust file");
        fs::write(temp_dir.path().join("README.md"), "# Notes\n").expect("write markdown file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);

        let first_language = state
            .selected_language()
            .expect("initial selected language")
            .name
            .clone();

        state.select_next_language();
        let second_language = state
            .selected_language()
            .expect("second selected language")
            .name
            .clone();

        assert_ne!(first_language, second_language);

        state.select_previous_language();
        assert_eq!(
            state.selected_language().expect("selected language").name,
            first_language
        );
    }

    #[test]
    fn explorer_selection_wraps_at_language_list_edges() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write rust file");
        fs::write(temp_dir.path().join("README.md"), "# Notes\n").expect("write markdown file");
        fs::write(temp_dir.path().join("script.py"), "print('hello')\n")
            .expect("write python file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);
        assert_eq!(state.filtered_languages().len(), 3);

        let first_language = state
            .selected_language()
            .expect("initial selected language")
            .name
            .clone();

        state.select_previous_language();
        let last_language = state
            .selected_language()
            .expect("wrapped previous language")
            .name
            .clone();

        assert_ne!(first_language, last_language);

        state.select_next_language();
        assert_eq!(
            state
                .selected_language()
                .expect("wrapped next language")
                .name,
            first_language
        );
    }

    #[test]
    fn explorer_ignores_key_release_when_selecting_languages() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write rust file");
        fs::write(temp_dir.path().join("README.md"), "# Notes\n").expect("write markdown file");
        fs::write(temp_dir.path().join("script.py"), "print('hello')\n")
            .expect("write python file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);
        state.set_view(AppView::Explorer);
        assert_eq!(state.filtered_languages().len(), 3);

        let first_language = state
            .selected_language()
            .expect("initial selected language")
            .name
            .clone();

        crate::handle_key_event(
            &mut state,
            crossterm::event::KeyEvent::new_with_kind(
                crossterm::event::KeyCode::Down,
                crossterm::event::KeyModifiers::NONE,
                crossterm::event::KeyEventKind::Press,
            ),
            &ScanOptions::default(),
        );
        let second_language = state
            .selected_language()
            .expect("second selected language")
            .name
            .clone();

        crate::handle_key_event(
            &mut state,
            crossterm::event::KeyEvent::new_with_kind(
                crossterm::event::KeyCode::Down,
                crossterm::event::KeyModifiers::NONE,
                crossterm::event::KeyEventKind::Release,
            ),
            &ScanOptions::default(),
        );

        assert_ne!(first_language, second_language);
        assert_eq!(
            state.selected_language().expect("selected language").name,
            second_language
        );
    }

    #[test]
    fn explorer_language_filter_limits_visible_languages() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write rust file");
        fs::write(temp_dir.path().join("README.md"), "# Notes\n").expect("write markdown file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);
        state.set_view(AppView::Explorer);
        state.set_language_filter("rust");

        let output = crate::render_to_text(&state, 96, 28);

        assert!(output.contains("Filter: rust"));
        assert!(output.contains("Rust"));
        assert!(!output.contains("Markdown"));
    }

    #[test]
    fn explorer_filter_can_be_edited_from_key_events() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write rust file");
        fs::write(temp_dir.path().join("README.md"), "# Notes\n").expect("write markdown file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);
        state.set_view(AppView::Explorer);

        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Char('/'),
            &ScanOptions::default(),
        );
        for value in ['r', 'u', 's', 't'] {
            crate::handle_key(
                &mut state,
                crossterm::event::KeyCode::Char(value),
                &ScanOptions::default(),
            );
        }
        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Enter,
            &ScanOptions::default(),
        );

        let output = crate::render_to_text(&state, 96, 28);

        assert!(output.contains("Filter: rust"));
        assert!(output.contains("Rust"));
        assert!(!output.contains("Markdown"));
    }

    #[test]
    fn report_state_defaults_to_json_with_language_details() {
        let report = scan_path(".", &ScanOptions::default());
        let state = AppState::new(report);

        assert_eq!(state.report_format(), ReportFormat::Json);
        assert!(state.includes_language_details());
        assert!(!state.includes_file_details());
    }

    #[test]
    fn app_state_can_start_from_tui_options() {
        let report = scan_path(".", &ScanOptions::default());
        let state = AppState::new_with_options(
            report,
            crate::TuiOptions {
                initial_view: AppView::Report,
                report_format: ReportFormat::Csv,
            },
        );

        assert_eq!(state.current_view(), AppView::Report);
        assert_eq!(state.report_format(), ReportFormat::Csv);
    }

    #[test]
    fn report_state_cycles_format_and_toggles_details_from_key_events() {
        let report = scan_path(".", &ScanOptions::default());
        let mut state = AppState::new(report);
        state.set_view(AppView::Report);

        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Right,
            &ScanOptions::default(),
        );
        assert_eq!(state.report_format(), ReportFormat::Markdown);

        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Right,
            &ScanOptions::default(),
        );
        assert_eq!(state.report_format(), ReportFormat::Html);

        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Right,
            &ScanOptions::default(),
        );
        assert_eq!(state.report_format(), ReportFormat::Csv);

        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Left,
            &ScanOptions::default(),
        );
        assert_eq!(state.report_format(), ReportFormat::Html);

        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Char('l'),
            &ScanOptions::default(),
        );
        assert!(!state.includes_language_details());

        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Char('f'),
            &ScanOptions::default(),
        );
        assert!(state.includes_file_details());

        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Char(' '),
            &ScanOptions::default(),
        );
        assert!(!state.includes_file_details());
    }

    #[test]
    fn report_view_renders_export_controls_and_preview() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write rust file");
        fs::write(temp_dir.path().join("README.md"), "# Notes\n").expect("write markdown file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);
        state.set_view(AppView::Report);

        let output = crate::render_to_text(&state, 96, 28);

        assert!(output.contains("C. Report Builder"));
        assert!(output.contains("Report"));
        assert!(output.contains("Report sections"));
        assert!(output.contains("Format"));
        assert!(output.contains("JSON"));
        assert!(output.contains("Language details"));
        assert!(output.contains("File details"));
        assert!(output.contains("Preview"));
        assert!(output.contains("# Project Line Report"));
        assert!(output.contains("Target"));
        assert!(output.contains("Total lines"));
        assert!(output.contains("[Space]"));
        assert!(output.contains("[e]"));
    }

    #[test]
    fn report_export_renders_markdown_and_csv() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write rust file");
        fs::write(temp_dir.path().join("README.md"), "# Notes\n").expect("write markdown file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);
        state.next_report_format();
        state.toggle_report_files();

        let markdown = crate::render_report_export(&state).expect("render markdown report");
        assert!(markdown.contains("# code-count report"));
        assert!(markdown.contains("## Languages"));
        assert!(markdown.contains("### Rust"));
        assert!(markdown.contains("main.rs"));

        state.next_report_format();
        let html = crate::render_report_export(&state).expect("render html report");
        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("<td>Rust</td>"));
        assert!(html.contains("main.rs"));

        state.next_report_format();
        let csv = crate::render_report_export(&state).expect("render csv report");
        assert!(csv.contains("kind,name,files,total,code,comments,documents,blank"));
        assert!(csv.contains("language,Rust"));
        assert!(csv.contains("file,"));
    }

    #[test]
    fn report_export_key_writes_file_and_updates_status() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write rust file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);
        state.set_view(AppView::Report);

        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Char('e'),
            &ScanOptions::default(),
        );

        let export_path = temp_dir.path().join("code-count-report.json");
        let contents = fs::read_to_string(&export_path).expect("read exported report");
        assert!(contents.contains("\"summary\""));

        let output = crate::render_to_text(&state, 96, 28);
        assert!(output.contains("Exported"));
        assert!(output.contains("code-count-report.json"));
    }
}
