use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use code_count_core::{
    LanguageDelta, LanguageStat, LineDelta, ReportFormat, ReportRenderOptions, ScanOptions,
    ScanReport, ScanSummary, diff_reports, render_report, scan_path,
};
use code_count_tui::{AppView, ReportFormat as TuiReportFormat, TuiOptions};
use serde::Deserialize;

#[derive(Debug, Parser)]
#[command(name = "code-count")]
#[command(about = "Count source and document lines in a project.")]
struct Cli {
    #[arg(default_value = ".")]
    path: PathBuf,

    #[command(subcommand)]
    command: Option<Command>,

    #[arg(long)]
    json: bool,

    #[arg(long)]
    by_language: bool,

    #[arg(long)]
    ignore_blank: bool,

    #[arg(long)]
    ignore_comments: bool,

    #[arg(long = "ignore", value_name = "PATH", global = true)]
    ignore_paths: Vec<String>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Tui {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    Report {
        #[arg(default_value = ".")]
        path: PathBuf,

        #[arg(long, default_value = "markdown")]
        format: String,

        #[arg(short, long)]
        output: Option<PathBuf>,

        #[arg(long)]
        files: bool,
    },
    History {
        #[command(subcommand)]
        command: HistoryCommand,
    },
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,

        #[arg(long)]
        force: bool,
    },
    Diff {
        before: PathBuf,
        after: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum HistoryCommand {
    Save {
        #[arg(default_value = ".")]
        path: PathBuf,

        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Debug, Default, Deserialize)]
struct ProjectConfig {
    scan: Option<ScanConfig>,
    tui: Option<TuiConfig>,
    #[serde(skip)]
    config_path: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct ScanConfig {
    include_blank_lines: Option<bool>,
    include_comments: Option<bool>,
    ignored_paths: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
struct TuiConfig {
    default_view: Option<String>,
    report_format: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Command::Tui { path }) => {
            ensure_path_exists(path)?;
            let config = load_project_config(path)?;
            let options = scan_options(&cli, &config);
            let report = scan_path(path, &options);
            let tui_options = tui_options(&config)?;
            return code_count_tui::run_with_options(report, options, tui_options);
        }
        Some(Command::Report {
            path,
            format,
            output,
            files,
        }) => {
            ensure_path_exists(path)?;
            let config = load_project_config(path)?;
            let options = scan_options(&cli, &config);
            let report = scan_path(path, &options);
            let report_format = parse_cli_report_format(format)?;
            let render_options = ReportRenderOptions {
                include_language_details: true,
                include_file_details: *files,
            };
            let contents = render_report(&report, report_format, &render_options)?;
            write_report_output(&contents, output.as_deref())?;
            return Ok(());
        }
        Some(Command::History {
            command: HistoryCommand::Save { path, output },
        }) => {
            ensure_path_exists(path)?;
            let config = load_project_config(path)?;
            let options = scan_options(&cli, &config);
            let report = scan_path(path, &options);
            save_snapshot(&report, output)?;
            return Ok(());
        }
        Some(Command::Diff { before, after }) => {
            let before_report = read_snapshot(before)?;
            let after_report = read_snapshot(after)?;
            let diff = diff_reports(&before_report, &after_report);
            print_diff(&diff);
            return Ok(());
        }
        Some(Command::Init { path, force }) => {
            ensure_path_exists(path)?;
            write_default_config(path, *force)?;
            return Ok(());
        }
        None => {}
    }

    ensure_path_exists(&cli.path)?;
    let config = load_project_config(&cli.path)?;
    let options = scan_options(&cli, &config);
    let report = scan_path(&cli.path, &options);

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if cli.by_language {
        print_summary(&report.summary);
        println!();
        print_languages(&report);
    } else {
        print_summary(&report.summary);
    }

    Ok(())
}

fn load_project_config(path: &Path) -> Result<ProjectConfig> {
    let config_path = project_config_path(path);
    if !config_path.exists() {
        return Ok(ProjectConfig::default());
    }

    let contents = fs::read_to_string(&config_path)?;
    let mut config = toml::from_str::<ProjectConfig>(&contents)?;
    config.config_path = Some(config_path);
    Ok(config)
}

fn project_config_path(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.join("code-count.toml")
    } else {
        path.parent()
            .unwrap_or_else(|| Path::new("."))
            .join("code-count.toml")
    }
}

fn scan_options(cli: &Cli, config: &ProjectConfig) -> ScanOptions {
    let scan_config = config.scan.as_ref();
    let include_blank_lines = if cli.ignore_blank {
        false
    } else {
        scan_config
            .and_then(|scan| scan.include_blank_lines)
            .unwrap_or(true)
    };
    let include_comments = if cli.ignore_comments {
        false
    } else {
        scan_config
            .and_then(|scan| scan.include_comments)
            .unwrap_or(true)
    };
    let mut ignored_paths = scan_config
        .and_then(|scan| scan.ignored_paths.clone())
        .unwrap_or_default();
    for ignored_path in &cli.ignore_paths {
        push_unique(&mut ignored_paths, ignored_path.clone());
    }
    if let Some(config_file_name) = config
        .config_path
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        && !ignored_paths
            .iter()
            .any(|ignored_path| ignored_path == config_file_name)
    {
        ignored_paths.push(config_file_name.to_owned());
    }

    ScanOptions {
        include_blank_lines,
        include_comments,
        ignored_paths,
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn tui_options(config: &ProjectConfig) -> Result<TuiOptions> {
    let Some(tui_config) = config.tui.as_ref() else {
        return Ok(TuiOptions::default());
    };

    let initial_view = match tui_config.default_view.as_deref() {
        Some(value) => parse_app_view(value)?,
        None => TuiOptions::default().initial_view,
    };
    let report_format = match tui_config.report_format.as_deref() {
        Some(value) => parse_tui_report_format(value)?,
        None => TuiOptions::default().report_format,
    };

    Ok(TuiOptions {
        initial_view,
        report_format,
    })
}

fn parse_app_view(value: &str) -> Result<AppView> {
    match value.to_ascii_lowercase().as_str() {
        "dashboard" => Ok(AppView::Dashboard),
        "explorer" => Ok(AppView::Explorer),
        "report" => Ok(AppView::Report),
        _ => bail!("invalid tui.default_view: {value}; expected dashboard, explorer, or report"),
    }
}

fn parse_tui_report_format(value: &str) -> Result<TuiReportFormat> {
    match value.to_ascii_lowercase().as_str() {
        "json" => Ok(TuiReportFormat::Json),
        "markdown" | "md" => Ok(TuiReportFormat::Markdown),
        "html" => Ok(TuiReportFormat::Html),
        "csv" => Ok(TuiReportFormat::Csv),
        _ => bail!("invalid tui.report_format: {value}; expected json, markdown, html, or csv"),
    }
}

fn parse_cli_report_format(value: &str) -> Result<ReportFormat> {
    match value.to_ascii_lowercase().as_str() {
        "json" => Ok(ReportFormat::Json),
        "markdown" | "md" => Ok(ReportFormat::Markdown),
        "html" => Ok(ReportFormat::Html),
        "csv" => Ok(ReportFormat::Csv),
        _ => bail!("invalid report format: {value}; expected json, markdown, html, or csv"),
    }
}

fn ensure_path_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("path does not exist: {}", path.display());
    }

    Ok(())
}

fn save_snapshot(report: &ScanReport, output: &Path) -> Result<()> {
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    fs::write(output, serde_json::to_string_pretty(report)?)?;
    println!("Saved snapshot: {}", output.display());
    Ok(())
}

fn write_report_output(contents: &str, output: Option<&Path>) -> Result<()> {
    let Some(output) = output else {
        println!("{contents}");
        return Ok(());
    };

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    fs::write(output, contents)?;
    println!("Saved report: {}", output.display());
    Ok(())
}

fn write_default_config(path: &Path, force: bool) -> Result<()> {
    let config_path = project_config_path(path);
    if config_path.exists() && !force {
        bail!(
            "config already exists: {} (use --force to overwrite)",
            config_path.display()
        );
    }

    fs::write(&config_path, default_config_contents())?;
    println!("Created config: {}", config_path.display());
    Ok(())
}

fn default_config_contents() -> &'static str {
    r#"[scan]
include_blank_lines = true
include_comments = true
ignored_paths = ["target", ".git", "node_modules", "dist", "build", "vendor"]

[tui]
default_view = "dashboard"
report_format = "markdown"
"#
}

fn read_snapshot(path: &Path) -> Result<ScanReport> {
    ensure_path_exists(path)?;
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}

fn print_summary(summary: &ScanSummary) {
    println!("Root: {}", summary.root.display());
    println!("Files: {}", summary.files);
    println!("Total lines: {}", summary.total_lines);
    println!("Code lines: {}", summary.code_lines);
    println!("Comment lines: {}", summary.comment_lines);
    println!("Document lines: {}", summary.document_lines);
    println!("Blank lines: {}", summary.blank_lines);
}

fn print_languages(report: &ScanReport) {
    println!("Languages:");
    println!(
        "{:<16} {:>8} {:>8} {:>8} {:>10} {:>8} {:>10}",
        "Name", "Files", "Total", "Code", "Comments", "Blank", "Category"
    );

    for language in &report.languages {
        println!(
            "{:<16} {:>8} {:>8} {:>8} {:>10} {:>8} {:>10}",
            language.name,
            language.files,
            language.total_lines,
            language_visible_code(language),
            language.comment_lines,
            language.blank_lines,
            if language.is_document {
                "Documents"
            } else {
                "Code"
            }
        );
    }
}

fn print_diff(diff: &code_count_core::ScanReportDiff) {
    println!("Scan diff");
    println!("Before: {}", diff.before_root.display());
    println!("After: {}", diff.after_root.display());
    println!();
    print_delta_summary(&diff.summary);

    if !diff.languages.is_empty() {
        println!();
        println!("Languages:");
        println!(
            "{:<16} {:>8} {:>8} {:>8} {:>10} {:>10} {:>8}",
            "Name", "Files", "Total", "Code", "Comments", "Documents", "Blank"
        );

        for language in &diff.languages {
            print_language_delta(language);
        }
    }
}

fn print_delta_summary(delta: &LineDelta) {
    println!("Files: {}", signed(delta.files));
    println!("Total lines: {}", signed(delta.total_lines));
    println!("Code lines: {}", signed(delta.code_lines));
    println!("Comment lines: {}", signed(delta.comment_lines));
    println!("Document lines: {}", signed(delta.document_lines));
    println!("Blank lines: {}", signed(delta.blank_lines));
}

fn print_language_delta(language: &LanguageDelta) {
    println!(
        "{:<16} {:>8} {:>8} {:>8} {:>10} {:>10} {:>8}",
        language.name,
        signed(language.files),
        signed(language.total_lines),
        signed(language.code_lines),
        signed(language.comment_lines),
        signed(language.document_lines),
        signed(language.blank_lines)
    );
}

fn signed(value: isize) -> String {
    if value > 0 {
        format!("+{value}")
    } else {
        value.to_string()
    }
}

fn language_visible_code(language: &LanguageStat) -> usize {
    if language.is_document {
        language.document_lines
    } else {
        language.code_lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use code_count_tui::{AppView, ReportFormat};

    fn cli() -> Cli {
        Cli {
            path: PathBuf::from("."),
            command: None,
            json: false,
            by_language: false,
            ignore_blank: false,
            ignore_comments: false,
            ignore_paths: Vec::new(),
        }
    }

    fn cli_with_ignores(ignored_paths: &[&str]) -> Cli {
        Cli {
            ignore_paths: ignored_paths
                .iter()
                .map(|path| (*path).to_owned())
                .collect(),
            ..cli()
        }
    }

    #[test]
    fn scan_options_apply_config_defaults_and_ignore_config_file() {
        let config = ProjectConfig {
            scan: Some(ScanConfig {
                include_blank_lines: Some(false),
                include_comments: Some(false),
                ignored_paths: Some(vec!["ignored".to_owned()]),
            }),
            tui: None,
            config_path: Some(PathBuf::from("code-count.toml")),
        };

        let options = scan_options(&cli(), &config);

        assert!(!options.include_blank_lines);
        assert!(!options.include_comments);
        assert_eq!(
            options.ignored_paths,
            vec!["ignored".to_owned(), "code-count.toml".to_owned()]
        );
    }

    #[test]
    fn scan_options_merge_cli_and_config_ignored_paths() {
        let config = ProjectConfig {
            scan: Some(ScanConfig {
                include_blank_lines: None,
                include_comments: None,
                ignored_paths: Some(vec!["vendor".to_owned(), "build".to_owned()]),
            }),
            tui: None,
            config_path: Some(PathBuf::from("code-count.toml")),
        };

        let options = scan_options(&cli_with_ignores(&["build", "generated"]), &config);

        assert_eq!(
            options.ignored_paths,
            vec![
                "vendor".to_owned(),
                "build".to_owned(),
                "generated".to_owned(),
                "code-count.toml".to_owned()
            ]
        );
    }

    #[test]
    fn tui_command_accepts_global_ignore_paths() {
        let cli = Cli::try_parse_from(["code-count", "tui", ".", "--ignore", "vendor"])
            .expect("parse tui ignore");

        assert_eq!(cli.ignore_paths, vec!["vendor".to_owned()]);
    }

    #[test]
    fn tui_options_apply_config_defaults() {
        let config = ProjectConfig {
            scan: None,
            tui: Some(TuiConfig {
                default_view: Some("report".to_owned()),
                report_format: Some("csv".to_owned()),
            }),
            config_path: Some(PathBuf::from("code-count.toml")),
        };

        let options = tui_options(&config).expect("parse tui options");

        assert_eq!(options.initial_view, AppView::Report);
        assert_eq!(options.report_format, ReportFormat::Csv);
    }

    #[test]
    fn tui_options_reject_invalid_default_view() {
        let config = ProjectConfig {
            scan: None,
            tui: Some(TuiConfig {
                default_view: Some("grid".to_owned()),
                report_format: None,
            }),
            config_path: Some(PathBuf::from("code-count.toml")),
        };

        let error = tui_options(&config).expect_err("invalid view should fail");

        assert!(error.to_string().contains("invalid tui.default_view"));
    }
}
