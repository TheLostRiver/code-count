use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use code_count_core::{LanguageStat, ScanOptions, ScanReport, ScanSummary, scan_path};
use code_count_tui::{AppView, ReportFormat, TuiOptions};
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
}

#[derive(Debug, Subcommand)]
enum Command {
    Tui {
        #[arg(default_value = ".")]
        path: PathBuf,
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

    if let Some(Command::Tui { path }) = &cli.command {
        ensure_path_exists(path)?;
        let config = load_project_config(path)?;
        let options = scan_options(&cli, &config);
        let report = scan_path(path, &options);
        let tui_options = tui_options(&config)?;
        return code_count_tui::run_with_options(report, options, tui_options);
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

fn tui_options(config: &ProjectConfig) -> Result<TuiOptions> {
    let Some(tui_config) = config.tui.as_ref() else {
        return Ok(TuiOptions::default());
    };

    let initial_view = match tui_config.default_view.as_deref() {
        Some(value) => parse_app_view(value)?,
        None => TuiOptions::default().initial_view,
    };
    let report_format = match tui_config.report_format.as_deref() {
        Some(value) => parse_report_format(value)?,
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

fn parse_report_format(value: &str) -> Result<ReportFormat> {
    match value.to_ascii_lowercase().as_str() {
        "json" => Ok(ReportFormat::Json),
        "markdown" | "md" => Ok(ReportFormat::Markdown),
        "csv" => Ok(ReportFormat::Csv),
        _ => bail!("invalid tui.report_format: {value}; expected json, markdown, or csv"),
    }
}

fn ensure_path_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("path does not exist: {}", path.display());
    }

    Ok(())
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
