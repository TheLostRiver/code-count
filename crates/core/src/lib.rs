use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokei::{Config, LanguageType, Languages};

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
    pub document_lines: usize,
    pub code_lines: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineCategories {
    pub code: usize,
    pub comments: usize,
    pub documents: usize,
    pub blanks: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageStat {
    pub name: String,
    pub files: usize,
    pub total_lines: usize,
    pub blank_lines: usize,
    pub comment_lines: usize,
    pub document_lines: usize,
    pub code_lines: usize,
    pub is_document: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanReport {
    pub summary: ScanSummary,
    pub categories: LineCategories,
    pub languages: Vec<LanguageStat>,
}

pub fn scan_path(root: impl AsRef<Path>, options: &ScanOptions) -> ScanReport {
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
        document_lines: 0,
        code_lines: 0,
    };
    let mut categories = LineCategories {
        code: 0,
        comments: 0,
        documents: 0,
        blanks: 0,
    };
    let mut language_stats = Vec::new();

    for (language_type, language) in &languages {
        let is_document = is_document_language(*language_type);
        let document_lines = if is_document {
            language.code + language.comments
        } else {
            0
        };
        let code_lines = if is_document { 0 } else { language.code };
        let comment_lines = if is_document { 0 } else { language.comments };
        let language_stat = LanguageStat {
            name: language_type.name().to_owned(),
            files: language.reports.len(),
            total_lines: language.blanks + comment_lines + document_lines + code_lines,
            blank_lines: if options.include_blank_lines {
                language.blanks
            } else {
                0
            },
            comment_lines: if options.include_comments {
                comment_lines
            } else {
                0
            },
            document_lines,
            code_lines,
            is_document,
        };

        summary.files += language.reports.len();
        summary.blank_lines += language.blanks;
        summary.comment_lines += comment_lines;
        summary.document_lines += document_lines;
        summary.code_lines += code_lines;

        if is_document {
            categories.documents += document_lines;
        } else {
            categories.code += code_lines;
        }
        categories.comments += comment_lines;
        categories.blanks += language.blanks;

        language_stats.push(language_stat);
    }

    summary.total_lines =
        summary.blank_lines + summary.comment_lines + summary.document_lines + summary.code_lines;

    summary = ScanSummary {
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
    };
    if !options.include_blank_lines {
        categories.blanks = 0;
    }
    if !options.include_comments {
        categories.comments = 0;
    }

    language_stats.sort_by(|left, right| {
        right
            .total_lines
            .cmp(&left.total_lines)
            .then_with(|| left.name.cmp(&right.name))
    });

    ScanReport {
        summary,
        categories,
        languages: language_stats,
    }
}

fn is_document_language(language_type: LanguageType) -> bool {
    language_type.is_literate()
        || matches!(language_type, LanguageType::Markdown | LanguageType::Text)
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
    fn scan_report_preserves_root_path() {
        let options = ScanOptions::default();
        let report = scan_path("sample", &options);

        assert_eq!(report.summary.root, PathBuf::from("sample"));
    }

    #[test]
    fn scan_report_counts_rust_code_comments_and_blanks() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let source_path = temp_dir.path().join("main.rs");
        fs::write(
            &source_path,
            "fn main() {\n    // greet\n\n    println!(\"hello\");\n}\n",
        )
        .expect("write sample rust file");

        let report = scan_path(temp_dir.path(), &ScanOptions::default());

        assert_eq!(report.summary.files, 1);
        assert_eq!(report.summary.total_lines, 5);
        assert_eq!(report.summary.code_lines, 3);
        assert_eq!(report.summary.comment_lines, 1);
        assert_eq!(report.summary.document_lines, 0);
        assert_eq!(report.summary.blank_lines, 1);
    }

    #[test]
    fn scan_report_includes_language_stats_and_document_category() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::write(
            temp_dir.path().join("main.rs"),
            "fn main() {\n    // greet\n\n    println!(\"hello\");\n}\n",
        )
        .expect("write sample rust file");
        fs::write(
            temp_dir.path().join("README.md"),
            "# Title\n\nSome project notes.\n",
        )
        .expect("write sample markdown file");

        let report = scan_path(temp_dir.path(), &ScanOptions::default());

        assert_eq!(report.summary.files, 2);
        assert_eq!(report.summary.total_lines, 8);
        assert_eq!(report.categories.code, 3);
        assert_eq!(report.categories.comments, 1);
        assert_eq!(report.categories.documents, 2);
        assert_eq!(report.categories.blanks, 2);

        let rust = report
            .languages
            .iter()
            .find(|language| language.name == "Rust")
            .expect("Rust language stat");
        assert_eq!(rust.files, 1);
        assert_eq!(rust.code_lines, 3);
        assert!(!rust.is_document);

        let markdown = report
            .languages
            .iter()
            .find(|language| language.name == "Markdown")
            .expect("Markdown language stat");
        assert_eq!(markdown.files, 1);
        assert_eq!(markdown.code_lines, 0);
        assert_eq!(markdown.comment_lines, 0);
        assert_eq!(markdown.document_lines, 2);
        assert!(markdown.is_document);
    }
}
