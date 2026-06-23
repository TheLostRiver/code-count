use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use code_count_core::{LanguageStat, ScanOptions, ScanReport, ScanSummary, scan_path};

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

fn main() -> Result<()> {
    let cli = Cli::parse();
    let options = ScanOptions {
        include_blank_lines: !cli.ignore_blank,
        include_comments: !cli.ignore_comments,
    };

    if let Some(Command::Tui { path }) = cli.command {
        ensure_path_exists(&path)?;
        let report = scan_path(path, &options);
        return code_count_tui::run(report, options);
    }

    ensure_path_exists(&cli.path)?;
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
