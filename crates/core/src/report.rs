use std::{error::Error, fmt, path::Path};

use crate::ScanReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Json,
    Markdown,
    Html,
    Csv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportRenderOptions {
    pub include_language_details: bool,
    pub include_file_details: bool,
}

impl Default for ReportRenderOptions {
    fn default() -> Self {
        Self {
            include_language_details: true,
            include_file_details: false,
        }
    }
}

#[derive(Debug)]
pub enum ReportRenderError {
    Json(serde_json::Error),
}

impl fmt::Display for ReportRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "failed to render JSON report: {error}"),
        }
    }
}

impl Error for ReportRenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
        }
    }
}

impl From<serde_json::Error> for ReportRenderError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub fn render_report(
    report: &ScanReport,
    format: ReportFormat,
    options: &ReportRenderOptions,
) -> Result<String, ReportRenderError> {
    match format {
        ReportFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        ReportFormat::Markdown => Ok(render_markdown_report(report, options)),
        ReportFormat::Html => Ok(render_html_report(report, options)),
        ReportFormat::Csv => Ok(render_csv_report(report, options)),
    }
}

pub fn report_extension(format: ReportFormat) -> &'static str {
    match format {
        ReportFormat::Json => "json",
        ReportFormat::Markdown => "md",
        ReportFormat::Html => "html",
        ReportFormat::Csv => "csv",
    }
}

fn render_markdown_report(report: &ScanReport, options: &ReportRenderOptions) -> String {
    let summary = &report.summary;
    let mut output = String::new();

    output.push_str("# code-count report\n\n");
    output.push_str(&format!("- Root: {}\n", summary.root.display()));
    output.push_str(&format!("- Files: {}\n", summary.files));
    output.push_str(&format!("- Total lines: {}\n", summary.total_lines));
    output.push_str(&format!("- Code lines: {}\n", summary.code_lines));
    output.push_str(&format!("- Comment lines: {}\n", summary.comment_lines));
    output.push_str(&format!("- Document lines: {}\n", summary.document_lines));
    output.push_str(&format!("- Blank lines: {}\n", summary.blank_lines));

    if options.include_language_details {
        output.push_str("\n## Languages\n\n");
        output.push_str("| Language | Files | Total | Code | Comments | Documents | Blank |\n");
        output.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: |\n");

        for language in &report.languages {
            output.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                markdown_escape(&language.name),
                language.files,
                language.total_lines,
                language.code_lines,
                language.comment_lines,
                language.document_lines,
                language.blank_lines
            ));

            if options.include_file_details {
                output.push_str(&format!("\n### {}\n\n", markdown_escape(&language.name)));
                output.push_str("| File | Total | Code | Comments | Documents | Blank |\n");
                output.push_str("| --- | ---: | ---: | ---: | ---: | ---: |\n");
                for file_stat in &language.file_stats {
                    output.push_str(&format!(
                        "| {} | {} | {} | {} | {} | {} |\n",
                        markdown_escape(&display_path(&file_stat.path, &report.summary.root)),
                        file_stat.total_lines,
                        file_stat.code_lines,
                        file_stat.comment_lines,
                        file_stat.document_lines,
                        file_stat.blank_lines
                    ));
                }
                output.push('\n');
            }
        }
    }

    output
}

fn render_html_report(report: &ScanReport, options: &ReportRenderOptions) -> String {
    let summary = &report.summary;
    let mut output = String::new();

    output.push_str("<!doctype html>\n");
    output.push_str("<html lang=\"en\">\n<head>\n");
    output.push_str("<meta charset=\"utf-8\">\n");
    output.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    output.push_str("<title>code-count report</title>\n");
    output.push_str("<style>\n");
    output.push_str(
        ":root{color-scheme:light dark;--bg:#f6f7f9;--panel:#ffffff;--text:#1d2430;--muted:#667085;--border:#d9dee7;--code:#2f855a;--comments:#2b6cb0;--docs:#b7791f;--blank:#718096}\
body{margin:0;font-family:Segoe UI,Arial,sans-serif;background:var(--bg);color:var(--text);line-height:1.45}\
main{max-width:1120px;margin:0 auto;padding:32px 20px 48px}\
h1{margin:0 0 6px;font-size:32px}\
h2{margin:32px 0 12px;font-size:22px}\
h3{margin:24px 0 10px;font-size:18px}\
.root{color:var(--muted);margin:0 0 24px;word-break:break-all}\
.metrics{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:12px;margin:20px 0 24px}\
.metric{background:var(--panel);border:1px solid var(--border);border-radius:8px;padding:14px}\
.metric span{display:block;color:var(--muted);font-size:13px}\
.metric strong{display:block;font-size:24px;margin-top:4px}\
.bar{display:flex;height:16px;border-radius:999px;overflow:hidden;background:#e6e9ef;margin:10px 0 12px}\
.bar div{min-width:1px}\
.code{background:var(--code)}.comments{background:var(--comments)}.docs{background:var(--docs)}.blank{background:var(--blank)}\
.legend{display:flex;flex-wrap:wrap;gap:12px;color:var(--muted);font-size:14px}.legend b{color:var(--text)}\
table{width:100%;border-collapse:collapse;background:var(--panel);border:1px solid var(--border);border-radius:8px;overflow:hidden}\
th,td{padding:10px 12px;border-bottom:1px solid var(--border);text-align:right}\
th:first-child,td:first-child{text-align:left}\
tr:last-child td{border-bottom:0}\
th{color:var(--muted);font-weight:600}\
@media(prefers-color-scheme:dark){:root{--bg:#0b0f17;--panel:#121826;--text:#e6eaf2;--muted:#99a3b5;--border:#273244}}\n",
    );
    output.push_str("</style>\n</head>\n<body>\n<main>\n");
    output.push_str("<h1>code-count report</h1>\n");
    output.push_str(&format!(
        "<p class=\"root\">Root: {}</p>\n",
        html_escape(&summary.root.display().to_string())
    ));
    output.push_str("<section class=\"metrics\" aria-label=\"Summary metrics\">\n");
    metric(&mut output, "Files", summary.files);
    metric(&mut output, "Total lines", summary.total_lines);
    metric(&mut output, "Code lines", summary.code_lines);
    metric(&mut output, "Comment lines", summary.comment_lines);
    metric(&mut output, "Document lines", summary.document_lines);
    metric(&mut output, "Blank lines", summary.blank_lines);
    output.push_str("</section>\n");
    output.push_str("<h2>Composition</h2>\n");
    output.push_str("<div class=\"bar\" aria-label=\"Line composition\">\n");
    bar_segment(
        &mut output,
        "code",
        report.categories.code,
        summary.total_lines,
    );
    bar_segment(
        &mut output,
        "comments",
        report.categories.comments,
        summary.total_lines,
    );
    bar_segment(
        &mut output,
        "docs",
        report.categories.documents,
        summary.total_lines,
    );
    bar_segment(
        &mut output,
        "blank",
        report.categories.blanks,
        summary.total_lines,
    );
    output.push_str("</div>\n");
    output.push_str("<div class=\"legend\">");
    output.push_str(&format!(
        "<span><b>Code</b> {}</span>",
        report.categories.code
    ));
    output.push_str(&format!(
        "<span><b>Comments</b> {}</span>",
        report.categories.comments
    ));
    output.push_str(&format!(
        "<span><b>Docs</b> {}</span>",
        report.categories.documents
    ));
    output.push_str(&format!(
        "<span><b>Blank</b> {}</span>",
        report.categories.blanks
    ));
    output.push_str("</div>\n");

    if options.include_language_details {
        output.push_str("<h2>Languages</h2>\n");
        output.push_str("<table>\n<thead><tr><th>Language</th><th>Files</th><th>Total</th><th>Code</th><th>Comments</th><th>Documents</th><th>Blank</th></tr></thead>\n<tbody>\n");
        for language in &report.languages {
            output.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                html_escape(&language.name),
                language.files,
                language.total_lines,
                language.code_lines,
                language.comment_lines,
                language.document_lines,
                language.blank_lines
            ));
        }
        output.push_str("</tbody>\n</table>\n");

        if options.include_file_details {
            for language in &report.languages {
                output.push_str(&format!("<h3>{}</h3>\n", html_escape(&language.name)));
                output.push_str("<table>\n<thead><tr><th>File</th><th>Total</th><th>Code</th><th>Comments</th><th>Documents</th><th>Blank</th></tr></thead>\n<tbody>\n");
                for file_stat in &language.file_stats {
                    output.push_str(&format!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                        html_escape(&display_path(&file_stat.path, &report.summary.root)),
                        file_stat.total_lines,
                        file_stat.code_lines,
                        file_stat.comment_lines,
                        file_stat.document_lines,
                        file_stat.blank_lines
                    ));
                }
                output.push_str("</tbody>\n</table>\n");
            }
        }
    }

    output.push_str("</main>\n</body>\n</html>\n");
    output
}

fn render_csv_report(report: &ScanReport, options: &ReportRenderOptions) -> String {
    let mut output = String::new();
    output.push_str("kind,name,files,total,code,comments,documents,blank\n");

    let summary = &report.summary;
    output.push_str(&format!(
        "summary,{},{},{},{},{},{},{}\n",
        csv_escape(&summary.root.display().to_string()),
        summary.files,
        summary.total_lines,
        summary.code_lines,
        summary.comment_lines,
        summary.document_lines,
        summary.blank_lines
    ));

    if options.include_language_details {
        for language in &report.languages {
            output.push_str(&format!(
                "language,{},{},{},{},{},{},{}\n",
                csv_escape(&language.name),
                language.files,
                language.total_lines,
                language.code_lines,
                language.comment_lines,
                language.document_lines,
                language.blank_lines
            ));

            if options.include_file_details {
                for file_stat in &language.file_stats {
                    output.push_str(&format!(
                        "file,{},{},{},{},{},{},{}\n",
                        csv_escape(&display_path(&file_stat.path, &report.summary.root)),
                        1,
                        file_stat.total_lines,
                        file_stat.code_lines,
                        file_stat.comment_lines,
                        file_stat.document_lines,
                        file_stat.blank_lines
                    ));
                }
            }
        }
    }

    output
}

fn metric(output: &mut String, label: &str, value: usize) {
    output.push_str(&format!(
        "<div class=\"metric\"><span>{}</span><strong>{}</strong></div>\n",
        html_escape(label),
        value
    ));
}

fn bar_segment(output: &mut String, class_name: &str, value: usize, total: usize) {
    if value == 0 {
        return;
    }

    let width = value as f64 * 100.0 / total.max(1) as f64;
    output.push_str(&format!(
        "<div class=\"{}\" style=\"width:{:.2}%\"></div>\n",
        html_escape(class_name),
        width
    ));
}

fn display_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn markdown_escape(value: &str) -> String {
    value.replace('|', "\\|")
}

fn html_escape(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(character),
        }
    }
    output
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}
