use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use code_count_core::{ScanOptions, scan_path};

#[derive(Debug, Parser)]
#[command(name = "code-count")]
#[command(about = "Count source and document lines in a project.")]
struct Cli {
    #[arg(default_value = ".")]
    path: PathBuf,

    #[arg(long)]
    json: bool,

    #[arg(long)]
    ignore_blank: bool,

    #[arg(long)]
    ignore_comments: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let options = ScanOptions {
        include_blank_lines: !cli.ignore_blank,
        include_comments: !cli.ignore_comments,
    };
    let report = scan_path(&cli.path, &options);

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        let summary = &report.summary;
        println!("Root: {}", summary.root.display());
        println!("Files: {}", summary.files);
        println!("Total lines: {}", summary.total_lines);
        println!("Code lines: {}", summary.code_lines);
        println!("Comment lines: {}", summary.comment_lines);
        println!("Document lines: {}", summary.document_lines);
        println!("Blank lines: {}", summary.blank_lines);
    }

    Ok(())
}
