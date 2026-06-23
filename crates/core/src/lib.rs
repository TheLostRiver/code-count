use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokei::{Config, Languages};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanOptions {
    pub include_blank_lines: bool,
    pub include_comments: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            include_blank_lines: true,
            include_comments: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanSummary {
    pub root: PathBuf,
    pub files: usize,
    pub total_lines: usize,
    pub blank_lines: usize,
    pub comment_lines: usize,
    pub code_lines: usize,
}

pub fn scan_path(root: impl AsRef<Path>, options: &ScanOptions) -> ScanSummary {
    let root = root.as_ref().to_path_buf();
    let paths = [root.clone()];
    let ignored: [&str; 0] = [];
    let config = Config::default();
    let mut languages = Languages::new();

    languages.get_statistics(&paths, &ignored, &config);

    let mut summary = ScanSummary {
        root,
        files: 0,
        total_lines: 0,
        blank_lines: 0,
        comment_lines: 0,
        code_lines: 0,
    };

    for language in languages.values() {
        summary.files += language.reports.len();
        summary.blank_lines += language.blanks;
        summary.comment_lines += language.comments;
        summary.code_lines += language.code;
    }

    summary.total_lines = summary.blank_lines + summary.comment_lines + summary.code_lines;

    ScanSummary {
        blank_lines: if options.include_blank_lines {
            summary.blank_lines
        } else {
            0
        },
        comment_lines: if options.include_comments {
            summary.comment_lines
        } else {
            0
        },
        total_lines: summary.total_lines,
        ..summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn default_options_include_blank_lines_and_comments() {
        let options = ScanOptions::default();

        assert!(options.include_blank_lines);
        assert!(options.include_comments);
    }

    #[test]
    fn scan_summary_preserves_root_path() {
        let options = ScanOptions::default();
        let summary = scan_path("sample", &options);

        assert_eq!(summary.root, PathBuf::from("sample"));
    }

    #[test]
    fn scan_summary_counts_rust_code_comments_and_blanks() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let source_path = temp_dir.path().join("main.rs");
        fs::write(
            &source_path,
            "fn main() {\n    // greet\n\n    println!(\"hello\");\n}\n",
        )
        .expect("write sample rust file");

        let summary = scan_path(temp_dir.path(), &ScanOptions::default());

        assert_eq!(summary.files, 1);
        assert_eq!(summary.total_lines, 5);
        assert_eq!(summary.code_lines, 3);
        assert_eq!(summary.comment_lines, 1);
        assert_eq!(summary.blank_lines, 1);
    }
}
