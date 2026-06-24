use std::{
    collections::BTreeMap,
    fs,
    io::{self, IsTerminal},
    path::{Component, Path},
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
const BAR_FILL: char = '█';
const BAR_EMPTY: char = '░';
const SUMMARY_WIDTH: usize = 76;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScopeSuggestion {
    path: String,
    total_lines: usize,
    files: usize,
}

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
            report_format: ReportFormat::Markdown,
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
    scope_status: Option<String>,
    ignored_paths: Vec<String>,
    session_ignored_paths: Vec<String>,
}

impl AppState {
    pub fn new(report: ScanReport) -> Self {
        Self::new_with_options(report, TuiOptions::default())
    }

    pub fn new_with_options(report: ScanReport, options: TuiOptions) -> Self {
        Self::new_with_ignored_paths(report, options, Vec::new())
    }

    pub fn new_with_scan_options(
        report: ScanReport,
        options: TuiOptions,
        scan_options: &ScanOptions,
    ) -> Self {
        Self::new_with_ignored_paths(report, options, scan_options.ignored_paths.clone())
    }

    fn new_with_ignored_paths(
        report: ScanReport,
        options: TuiOptions,
        ignored_paths: Vec<String>,
    ) -> Self {
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
            scope_status: None,
            ignored_paths,
            session_ignored_paths: Vec::new(),
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
        let effective_options = self.effective_scan_options(options);
        self.report = scan_path(root, &effective_options);
        self.clamp_selected_language();
    }

    fn ignore_top_scope_suggestion(&mut self, options: &ScanOptions) {
        let Some(suggestion) = scope_suggestions(&self.report, &self.ignored_paths)
            .into_iter()
            .next()
        else {
            return;
        };

        push_unique(&mut self.ignored_paths, suggestion.path.clone());
        push_unique(&mut self.session_ignored_paths, suggestion.path.clone());
        self.scope_status = Some(format!("Ignored {} for this session", suggestion.path));
        self.rescan(options);
    }

    fn undo_last_session_ignore(&mut self, options: &ScanOptions) {
        let Some(path) = self.session_ignored_paths.pop() else {
            self.scope_status = Some("No session ignores to undo".to_owned());
            return;
        };

        self.ignored_paths
            .retain(|ignored_path| ignored_path != &path);
        self.scope_status = Some(format!("Restored {path}"));
        self.rescan(options);
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

    fn effective_scan_options(&self, options: &ScanOptions) -> ScanOptions {
        let mut effective_options = options.clone();
        effective_options.ignored_paths = self.ignored_paths.clone();
        effective_options
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
    let state = AppState::new_with_scan_options(report, tui_options, &options);
    let result = run_app(&mut terminal, state, options);
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
        KeyCode::Char('i') if state.current_view() == AppView::Explorer => {
            state.ignore_top_scope_suggestion(options);
        }
        KeyCode::Char('u') if state.current_view() == AppView::Explorer => {
            state.undo_last_session_ignore(options);
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

    let mut title_spans = vec![
        Span::styled(command_label(state), Style::default().fg(COLOR_BRAND)),
        Span::raw("  "),
        Span::styled(status_label(state), Style::default().fg(COLOR_MUTED)),
    ];
    if let Some(scope) = scope_label(state) {
        title_spans.push(Span::raw("  "));
        title_spans.push(Span::styled(scope, Style::default().fg(COLOR_DOCS)));
    }
    title_spans.extend([
        Span::raw("  "),
        Span::styled(
            state.report.summary.root.display().to_string(),
            Style::default().fg(COLOR_MUTED),
        ),
    ]);

    let title = Paragraph::new(Line::from(title_spans))
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
    let lines = if area.width < 70 {
        compact_stats_lines(state)
    } else {
        stats_lines(state)
    };
    Paragraph::new(lines)
        .block(panel_block("Summary"))
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
            Constraint::Length(8),
            Constraint::Min(7),
            Constraint::Length(7),
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
            Constraint::Length(8),
            Constraint::Min(7),
            Constraint::Length(7),
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
        summary_total_line(summary.total_lines),
        segment_bar(
            categories.code,
            categories.comments,
            categories.documents,
            categories.blanks,
            SUMMARY_WIDTH,
        ),
        summary_pair_line(
            "code",
            summary.code_lines,
            COLOR_CODE,
            "comments",
            summary.comment_lines,
            COLOR_COMMENTS,
        ),
        summary_pair_line(
            "docs",
            summary.document_lines,
            COLOR_DOCS,
            "blank",
            summary.blank_lines,
            COLOR_MUTED,
        ),
        Line::from(vec![
            Span::styled("Files", Style::default().fg(COLOR_MUTED)),
            Span::raw(" "),
            Span::styled(
                format_count(summary.files),
                Style::default().fg(COLOR_TITLE),
            ),
        ]),
    ]
}

fn compact_stats_lines(state: &AppState) -> Vec<Line<'static>> {
    let summary = &state.report.summary;
    let categories = &state.report.categories;

    vec![
        Line::from(vec![
            Span::styled("Total", Style::default().fg(COLOR_MUTED)),
            Span::raw(" "),
            Span::styled(
                format_count(summary.total_lines),
                Style::default().fg(COLOR_CODE),
            ),
            Span::raw("   "),
            Span::styled("Files", Style::default().fg(COLOR_MUTED)),
            Span::raw(" "),
            Span::styled(
                format_count(summary.files),
                Style::default().fg(COLOR_TITLE),
            ),
        ]),
        segment_bar(
            categories.code,
            categories.comments,
            categories.documents,
            categories.blanks,
            36,
        ),
        Line::from(vec![
            Span::styled("code", Style::default().fg(COLOR_CODE)),
            Span::raw(format!(" {}", format_count(summary.code_lines))),
            Span::raw("   "),
            Span::styled("comments", Style::default().fg(COLOR_COMMENTS)),
            Span::raw(format!(" {}", format_count(summary.comment_lines))),
        ]),
        Line::from(vec![
            Span::styled("docs", Style::default().fg(COLOR_DOCS)),
            Span::raw(format!(" {}", format_count(summary.document_lines))),
            Span::raw("   "),
            Span::styled("blank", Style::default().fg(COLOR_MUTED)),
            Span::raw(format!(" {}", format_count(summary.blank_lines))),
        ]),
    ]
}

fn summary_total_line(total_lines: usize) -> Line<'static> {
    let value = format_count(total_lines);
    let spaces = SUMMARY_WIDTH.saturating_sub("Total".len() + value.len());

    Line::from(vec![
        Span::styled("Total", Style::default().fg(COLOR_MUTED)),
        Span::raw(" ".repeat(spaces)),
        Span::styled(value, Style::default().fg(COLOR_CODE)),
    ])
}

fn summary_pair_line(
    left_label: &'static str,
    left_value: usize,
    left_color: Color,
    right_label: &'static str,
    right_value: usize,
    right_color: Color,
) -> Line<'static> {
    let left = format!("{left_label} {}", format_count(left_value));
    let right = format!("{right_label} {}", format_count(right_value));
    let spaces = SUMMARY_WIDTH.saturating_sub(left.len() + right.len());

    Line::from(vec![
        Span::styled(left, Style::default().fg(left_color)),
        Span::raw(" ".repeat(spaces)),
        Span::styled(right, Style::default().fg(right_color)),
    ])
}

fn composition_lines(state: &AppState) -> Vec<Line<'static>> {
    let categories = &state.report.categories;
    let total = state.report.summary.total_lines.max(1);

    vec![
        composition_row("Code", categories.code, total, COLOR_CODE),
        composition_row("Comments", categories.comments, total, COLOR_COMMENTS),
        composition_row("Docs", categories.documents, total, COLOR_DOCS),
        composition_row("Blank", categories.blanks, total, COLOR_MUTED),
    ]
}

fn language_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for language in state.report.languages.iter().take(6) {
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
            Span::raw(format!("{:<24}", language.name)),
            Span::styled(
                format!("{:>12}", format_count(visible_lines)),
                Style::default().fg(color),
            ),
        ]));
    }

    if lines.is_empty() {
        lines.push(Line::from("No languages found"));
    }

    lines
}

fn composition_row(label: &'static str, value: usize, total: usize, color: Color) -> Line<'static> {
    let percent = percentage(value, total);
    let filled = (percent * 20 / 100).max(if value > 0 { 1 } else { 0 });
    let empty = 20usize.saturating_sub(filled);

    Line::from(vec![
        Span::styled(format!("{label:<9}"), Style::default().fg(color)),
        Span::styled(
            BAR_FILL.to_string().repeat(filled),
            Style::default().fg(color),
        ),
        Span::styled(
            BAR_EMPTY.to_string().repeat(empty),
            Style::default().fg(COLOR_MUTED),
        ),
        Span::raw(" "),
        Span::styled(format!("{percent:>3}%"), Style::default().fg(color)),
        Span::raw(format!("  {:>6} lines", format_count(value))),
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
        Span::styled(
            BAR_FILL.to_string().repeat(code_width),
            Style::default().fg(COLOR_CODE),
        ),
        Span::styled(
            BAR_FILL.to_string().repeat(comments_width),
            Style::default().fg(COLOR_COMMENTS),
        ),
        Span::styled(
            BAR_FILL.to_string().repeat(documents_width),
            Style::default().fg(COLOR_DOCS),
        ),
        Span::styled(
            BAR_FILL.to_string().repeat(blanks_width),
            Style::default().fg(COLOR_MUTED),
        ),
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

    if let Some(language) = state.selected_language()
        && !language.file_stats.is_empty()
    {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Files",
            Style::default().fg(COLOR_MUTED),
        )]));

        for (index, file_stat) in language.file_stats.iter().take(5).enumerate() {
            let marker = if index == 0 { "> " } else { "  " };
            lines.push(Line::from(vec![
                Span::styled(marker, Style::default().fg(COLOR_TITLE)),
                Span::styled(
                    display_path(&file_stat.path, &state.report.summary.root),
                    Style::default().fg(COLOR_MUTED),
                ),
            ]));
        }
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

    let share = percentage(
        language.total_lines,
        state.report.summary.total_lines.max(1),
    );

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
        metric_line("Files", format_count(language.files)),
        metric_line("Share", format!("{share}%")),
        metric_line("Total", format_count(language.total_lines)),
        metric_line("Code", format_count(language.code_lines)),
        metric_line("Comments", format_count(language.comment_lines)),
        metric_line("Documents", format_count(language.document_lines)),
        metric_line("Blank", format_count(language.blank_lines)),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Largest files",
            Style::default().fg(COLOR_MUTED),
        )]),
    ];

    lines.extend(file_detail_lines(language, &state.report.summary.root));
    lines.push(Line::from(""));
    lines.extend(scope_suggestion_lines(state));
    if let Some(status) = &state.scope_status {
        lines.push(Line::from(""));
        lines.push(Line::from(status.clone()));
    }
    lines
}

fn scope_suggestion_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![Span::styled(
        "Scope suggestions",
        Style::default().fg(COLOR_MUTED),
    )])];

    let suggestions = scope_suggestions(&state.report, &state.ignored_paths);
    if suggestions.is_empty() {
        lines.push(Line::from("No directory candidates"));
        return lines;
    }

    for (index, suggestion) in suggestions.iter().take(3).enumerate() {
        let marker = if index == 0 { "> " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(COLOR_TITLE)),
            Span::styled(
                format!("{:<16}", suggestion.path),
                Style::default().fg(COLOR_TITLE),
            ),
            Span::styled(
                format!("{:>8} lines", format_count(suggestion.total_lines)),
                Style::default().fg(COLOR_CODE),
            ),
            Span::styled(
                format!("  {:>3} files", format_count(suggestion.files)),
                Style::default().fg(COLOR_MUTED),
            ),
        ]));
    }

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
        Line::from(format!("- Total: {}", format_count(summary.total_lines))),
        Line::from(format!("- Code: {}", format_count(summary.code_lines))),
        Line::from(format!(
            "- Comments: {}",
            format_count(summary.comment_lines)
        )),
        Line::from(format!("- Docs: {}", format_count(summary.document_lines))),
    ];

    if let Some(status) = &state.report_status {
        lines.push(Line::from(""));
        lines.push(Line::from(status.clone()));
    }

    lines
}

fn report_export_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = vec![
        label_value_line(
            "Format",
            report_format_label(state.report_format),
            COLOR_CODE,
        ),
        label_value_line("Target", report_output_file_name(state), COLOR_TITLE),
    ];

    if let Some(status) = &state.report_status {
        lines.push(Line::from(status.clone()));
    }

    lines
}

fn report_output_path(state: &AppState) -> std::path::PathBuf {
    state.report.summary.root.join(format!(
        "code-count-report.{}",
        report_extension(state.report_format)
    ))
}

fn report_output_file_name(state: &AppState) -> String {
    report_output_path(state)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("code-count-report")
        .to_owned()
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
    format!("{:<36} {:>8}", path, format_count(file_stat.total_lines))
}

fn display_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn scope_suggestions(report: &ScanReport, ignored_paths: &[String]) -> Vec<ScopeSuggestion> {
    let mut totals = BTreeMap::<String, ScopeSuggestion>::new();

    for language in &report.languages {
        for file_stat in &language.file_stats {
            let Some(path) = scope_candidate_path(&file_stat.path, &report.summary.root) else {
                continue;
            };
            if ignored_paths
                .iter()
                .any(|ignored_path| ignored_path == &path)
            {
                continue;
            }

            let suggestion = totals.entry(path.clone()).or_insert(ScopeSuggestion {
                path,
                total_lines: 0,
                files: 0,
            });
            suggestion.total_lines += file_stat.total_lines;
            suggestion.files += 1;
        }
    }

    let mut suggestions = totals.into_values().collect::<Vec<_>>();
    suggestions.sort_by(|left, right| {
        right
            .total_lines
            .cmp(&left.total_lines)
            .then_with(|| right.files.cmp(&left.files))
            .then_with(|| left.path.cmp(&right.path))
    });
    suggestions
}

fn scope_candidate_path(path: &Path, root: &Path) -> Option<String> {
    let relative_path = path.strip_prefix(root).ok()?;
    let mut components = relative_path.components();
    let first = components.next()?;
    components.next()?;

    match first {
        Component::Normal(path) => Some(path.to_string_lossy().replace('\\', "/")),
        _ => None,
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn metric_line(label: &'static str, value: impl Into<String>) -> Line<'static> {
    label_value_line(label, value, COLOR_TITLE)
}

fn label_value_line(
    label: &'static str,
    value: impl Into<String>,
    value_color: Color,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<16}"), Style::default().fg(COLOR_MUTED)),
        Span::styled(value.into(), Style::default().fg(value_color)),
    ])
}

fn percentage(value: usize, total: usize) -> usize {
    value * 100 / total
}

fn format_count(value: usize) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);

    for (index, digit) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(digit);
    }

    formatted.chars().rev().collect()
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

fn scope_label(state: &AppState) -> Option<String> {
    let visible_paths = state
        .ignored_paths
        .iter()
        .filter(|path| path.as_str() != "code-count.toml")
        .collect::<Vec<_>>();
    if visible_paths.is_empty() {
        return None;
    }

    let mut label = visible_paths
        .iter()
        .take(3)
        .map(|path| path.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = visible_paths.len().saturating_sub(3);
    if remaining > 0 {
        label.push_str(&format!(" +{remaining}"));
    }

    Some(format!("ignored: {label}"))
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
            key_chip("Enter"),
            Span::raw(" detail | "),
            key_chip("/"),
            Span::raw(" filter | "),
            key_chip("i"),
            Span::raw(" ignore | "),
            key_chip("u"),
            Span::raw(" undo | "),
            key_chip("Tab"),
            Span::raw(" view | "),
            key_chip("q"),
            Span::raw(" quit"),
        ]),
        AppView::Report => Line::from(vec![
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
    println!("Files: {}", format_count(report.summary.files));
    println!("Total lines: {}", format_count(report.summary.total_lines));
    println!("Run this command in an interactive terminal for the full TUI.");
}

#[cfg(test)]
mod tests {
    use std::fs;

    use code_count_core::{ScanOptions, scan_path};
    use ratatui::text::Line;

    use crate::{AppState, AppView, ReportFormat, TuiOptions};

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

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
        assert!(output.contains("Summary"));
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
        assert!(output.contains("lines"));
        assert!(output.contains("█"));
        assert!(output.contains("Top Languages"));
        assert!(!output.contains("Language       Lines"));
        assert!(!output.contains("Share  Type"));
        assert!(output.contains("[Tab]"));
        assert!(output.contains("[r]"));
        assert!(output.contains("[q]"));
    }

    #[test]
    fn dashboard_summary_uses_long_total_bar() {
        let report = scan_path(".", &ScanOptions::default());
        let state = AppState::new(report);
        let lines: Vec<String> = crate::stats_lines(&state).iter().map(line_text).collect();

        assert!(lines[0].starts_with("Total"));
        assert!(lines[0].chars().count() >= 72);
        assert!(
            lines[0]
                .trim_end()
                .ends_with(&crate::format_count(state.report().summary.total_lines))
        );
        assert!(lines[1].chars().count() >= 72);
        assert!(lines[2].chars().count() >= 56);
        assert!(lines[3].chars().count() >= 56);
        assert!(lines.iter().any(|line| line.contains("Files")));
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
    fn tui_renders_large_counts_with_group_separators() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(
            temp_dir.path().join("main.rs"),
            "let value = 1;\n".repeat(1234),
        )
        .expect("write rust file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);

        let dashboard = crate::render_to_text(&state, 96, 28);
        assert!(dashboard.contains("1,234"));

        state.set_view(AppView::Explorer);
        let explorer = crate::render_to_text(&state, 96, 28);
        assert!(explorer.contains("Total"));
        assert!(explorer.contains("1,234"));

        state.set_view(AppView::Report);
        let report = crate::render_to_text(&state, 96, 28);
        assert!(report.contains("- Total: 1,234"));
    }

    #[test]
    fn tui_header_shows_active_ignored_paths() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write rust file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let scan_options = ScanOptions {
            ignored_paths: vec!["vendor".to_owned(), "build".to_owned()],
            ..ScanOptions::default()
        };
        let state = AppState::new_with_scan_options(report, TuiOptions::default(), &scan_options);

        let output = crate::render_to_text(&state, 96, 28);

        assert!(output.contains("ignored: vendor, build"));
    }

    #[test]
    fn scope_suggestions_rank_root_directories_by_total_lines() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let vendor_dir = temp_dir.path().join("vendor");
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&vendor_dir).expect("create vendor dir");
        fs::create_dir_all(&src_dir).expect("create src dir");
        fs::write(
            vendor_dir.join("generated.rs"),
            "pub fn generated() {}\n".repeat(24),
        )
        .expect("write generated rust file");
        fs::write(src_dir.join("main.rs"), "fn main() {}\n".repeat(4))
            .expect("write main rust file");
        fs::write(temp_dir.path().join("README.md"), "# Notes\n").expect("write readme");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());

        let suggestions = crate::scope_suggestions(&report, &[]);

        assert_eq!(suggestions[0].path, "vendor");
        assert_eq!(suggestions[0].total_lines, 24);
        assert!(
            suggestions
                .iter()
                .any(|suggestion| suggestion.path == "src")
        );
    }

    #[test]
    fn explorer_ignore_key_adds_top_scope_suggestion_and_rescans() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let vendor_dir = temp_dir.path().join("vendor");
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&vendor_dir).expect("create vendor dir");
        fs::create_dir_all(&src_dir).expect("create src dir");
        fs::write(
            vendor_dir.join("generated.rs"),
            "pub fn generated() {}\n".repeat(20),
        )
        .expect("write generated rust file");
        fs::write(src_dir.join("main.rs"), "fn main() {}\n").expect("write main rust file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);
        state.set_view(AppView::Explorer);

        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Char('i'),
            &ScanOptions::default(),
        );

        assert!(state.ignored_paths.iter().any(|path| path == "vendor"));
        assert_eq!(state.report().summary.files, 1);

        let output = crate::render_to_text(&state, 96, 28);
        assert!(output.contains("ignored: vendor"));
        assert!(output.contains("Ignored vendor for this session"));
        assert!(output.contains("Scope suggestions"));
        assert!(!output.contains("generated.rs"));
    }

    #[test]
    fn explorer_undo_key_removes_last_session_ignore_and_rescans() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let vendor_dir = temp_dir.path().join("vendor");
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&vendor_dir).expect("create vendor dir");
        fs::create_dir_all(&src_dir).expect("create src dir");
        fs::write(
            vendor_dir.join("generated.rs"),
            "pub fn generated() {}\n".repeat(20),
        )
        .expect("write generated rust file");
        fs::write(src_dir.join("main.rs"), "fn main() {}\n").expect("write main rust file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);
        state.set_view(AppView::Explorer);

        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Char('i'),
            &ScanOptions::default(),
        );
        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Char('u'),
            &ScanOptions::default(),
        );

        assert!(!state.ignored_paths.iter().any(|path| path == "vendor"));
        assert_eq!(state.report().summary.files, 2);

        let output = crate::render_to_text(&state, 96, 28);
        assert!(!output.contains("ignored: vendor"));
        assert!(output.contains("Restored vendor"));
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
        assert!(output.contains("Share"));
        assert!(!output.contains("Metric"));
        assert!(!output.contains("Value"));
        assert!(!output.contains("Path"));
        assert!(output.contains("[Enter]"));
    }

    #[test]
    fn explorer_language_panel_lists_files_for_selected_language() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write main rust file");
        fs::write(
            temp_dir.path().join("lib.rs"),
            "pub fn answer() -> usize { 42 }\n",
        )
        .expect("write lib rust file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let state = AppState::new(report);

        let lines: Vec<String> = crate::explorer_language_lines(&state)
            .iter()
            .map(line_text)
            .collect();

        assert!(lines.iter().any(|line| line.trim() == "Files"));
        assert!(lines.iter().any(|line| line.contains("main.rs")));
        assert!(lines.iter().any(|line| line.contains("lib.rs")));

        let files_start = lines
            .iter()
            .position(|line| line.trim() == "Files")
            .expect("files section");
        let marked_files = lines[files_start + 1..]
            .iter()
            .filter(|line| line.trim_start().starts_with("> "))
            .count();
        assert_eq!(marked_files, 1);
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
    fn report_state_defaults_to_markdown_with_language_details() {
        let report = scan_path(".", &ScanOptions::default());
        let state = AppState::new(report);

        assert_eq!(state.report_format(), ReportFormat::Markdown);
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
        assert_eq!(state.report_format(), ReportFormat::Html);

        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Right,
            &ScanOptions::default(),
        );
        assert_eq!(state.report_format(), ReportFormat::Csv);

        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Right,
            &ScanOptions::default(),
        );
        assert_eq!(state.report_format(), ReportFormat::Json);

        crate::handle_key(
            &mut state,
            crossterm::event::KeyCode::Left,
            &ScanOptions::default(),
        );
        assert_eq!(state.report_format(), ReportFormat::Csv);

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
        assert!(output.contains("Target"));
        assert!(output.contains("Markdown"));
        assert!(output.contains("code-count-report.md"));
        assert!(!output.contains("File details"));
        assert!(!output.contains("Options"));
        assert!(!output.contains("Export ready"));
        assert!(!output.contains("Ready"));
        assert!(output.contains("Preview"));
        assert!(output.contains("# Project Line Report"));
        assert!(output.contains("Target"));
        assert!(output.contains("- Total:"));
        assert!(output.contains("- Code:"));
        assert!(output.contains("- Comments:"));
        assert!(output.contains("- Docs:"));
        assert!(!output.contains("Total lines"));
        assert!(output.contains("[Space]"));
        assert!(output.contains("[e]"));
        assert!(!output.contains("[Left/Right]"));
    }

    #[test]
    fn report_preview_stays_summary_only_on_tall_terminals() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write rust file");
        fs::write(temp_dir.path().join("README.md"), "# Notes\n").expect("write markdown file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);
        state.set_view(AppView::Report);

        let output = crate::render_to_text(&state, 96, 36);

        assert!(output.contains("- Total:"));
        assert!(!output.contains("Languages included"));
        assert!(!output.contains("Files included"));
    }

    #[test]
    fn report_export_renders_markdown_and_csv() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write rust file");
        fs::write(temp_dir.path().join("README.md"), "# Notes\n").expect("write markdown file");
        let report = scan_path(temp_dir.path(), &ScanOptions::default());
        let mut state = AppState::new(report);
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

        let export_path = temp_dir.path().join("code-count-report.md");
        let contents = fs::read_to_string(&export_path).expect("read exported report");
        assert!(contents.contains("# code-count report"));

        let output = crate::render_to_text(&state, 96, 28);
        assert!(output.contains("Exported"));
        assert!(output.contains("code-count-report.md"));
    }
}
