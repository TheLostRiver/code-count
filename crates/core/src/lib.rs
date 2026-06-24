use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tokei::{Config, LanguageType, Languages, Report};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanOptions {
    pub include_blank_lines: bool,
    pub include_comments: bool,
    pub ignored_paths: Vec<String>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            include_blank_lines: true,
            include_comments: true,
            ignored_paths: Vec::new(),
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
pub struct FileStat {
    pub path: PathBuf,
    pub total_lines: usize,
    pub blank_lines: usize,
    pub comment_lines: usize,
    pub document_lines: usize,
    pub code_lines: usize,
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
    pub file_stats: Vec<FileStat>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanReport {
    pub summary: ScanSummary,
    pub categories: LineCategories,
    pub languages: Vec<LanguageStat>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanReportDiff {
    pub before_root: PathBuf,
    pub after_root: PathBuf,
    pub summary: LineDelta,
    pub languages: Vec<LanguageDelta>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineDelta {
    pub files: isize,
    pub total_lines: isize,
    pub blank_lines: isize,
    pub comment_lines: isize,
    pub document_lines: isize,
    pub code_lines: isize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageDelta {
    pub name: String,
    pub files: isize,
    pub total_lines: isize,
    pub blank_lines: isize,
    pub comment_lines: isize,
    pub document_lines: isize,
    pub code_lines: isize,
}

impl LineDelta {
    fn between_summaries(before: &ScanSummary, after: &ScanSummary) -> Self {
        Self {
            files: delta(before.files, after.files),
            total_lines: delta(before.total_lines, after.total_lines),
            blank_lines: delta(before.blank_lines, after.blank_lines),
            comment_lines: delta(before.comment_lines, after.comment_lines),
            document_lines: delta(before.document_lines, after.document_lines),
            code_lines: delta(before.code_lines, after.code_lines),
        }
    }
}

impl LanguageDelta {
    fn from_language(language: &LanguageStat) -> Self {
        Self {
            name: language.name.clone(),
            files: 0,
            total_lines: 0,
            blank_lines: 0,
            comment_lines: 0,
            document_lines: 0,
            code_lines: 0,
        }
    }

    fn add_language(&mut self, language: &LanguageStat) {
        self.files += language.files as isize;
        self.total_lines += language.total_lines as isize;
        self.blank_lines += language.blank_lines as isize;
        self.comment_lines += language.comment_lines as isize;
        self.document_lines += language.document_lines as isize;
        self.code_lines += language.code_lines as isize;
    }

    fn subtract_language(&mut self, language: &LanguageStat) {
        self.files -= language.files as isize;
        self.total_lines -= language.total_lines as isize;
        self.blank_lines -= language.blank_lines as isize;
        self.comment_lines -= language.comment_lines as isize;
        self.document_lines -= language.document_lines as isize;
        self.code_lines -= language.code_lines as isize;
    }

    fn is_zero(&self) -> bool {
        self.files == 0
            && self.total_lines == 0
            && self.blank_lines == 0
            && self.comment_lines == 0
            && self.document_lines == 0
            && self.code_lines == 0
    }
}

fn delta(before: usize, after: usize) -> isize {
    after as isize - before as isize
}

pub fn diff_reports(before: &ScanReport, after: &ScanReport) -> ScanReportDiff {
    let mut language_deltas = BTreeMap::<String, LanguageDelta>::new();

    for language in &before.languages {
        language_deltas
            .entry(language.name.clone())
            .or_insert_with(|| LanguageDelta::from_language(language))
            .subtract_language(language);
    }
    for language in &after.languages {
        language_deltas
            .entry(language.name.clone())
            .or_insert_with(|| LanguageDelta::from_language(language))
            .add_language(language);
    }

    let mut languages = language_deltas
        .into_values()
        .filter(|language| !language.is_zero())
        .collect::<Vec<_>>();
    languages.sort_by(|left, right| {
        right
            .total_lines
            .abs()
            .cmp(&left.total_lines.abs())
            .then_with(|| left.name.cmp(&right.name))
    });

    ScanReportDiff {
        before_root: before.summary.root.clone(),
        after_root: after.summary.root.clone(),
        summary: LineDelta::between_summaries(&before.summary, &after.summary),
        languages,
    }
}

pub fn scan_path(root: impl AsRef<Path>, options: &ScanOptions) -> ScanReport {
    let root = root.as_ref().to_path_buf();
    let paths = [root.clone()];
    let ignored = options
        .ignored_paths
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
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
        let mut file_stats = language
            .reports
            .iter()
            .map(|report| file_stat_from_report(report, is_document, options))
            .collect::<Vec<_>>();
        file_stats.sort_by(|left, right| {
            right
                .total_lines
                .cmp(&left.total_lines)
                .then_with(|| left.path.cmp(&right.path))
        });

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
            file_stats,
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

fn file_stat_from_report(report: &Report, is_document: bool, options: &ScanOptions) -> FileStat {
    let document_lines = if is_document {
        report.stats.code + report.stats.comments
    } else {
        0
    };
    let code_lines = if is_document { 0 } else { report.stats.code };
    let comment_lines = if is_document {
        0
    } else {
        report.stats.comments
    };
    let blank_lines = if options.include_blank_lines {
        report.stats.blanks
    } else {
        0
    };

    FileStat {
        path: report.name.clone(),
        total_lines: report.stats.lines(),
        blank_lines,
        comment_lines: if options.include_comments {
            comment_lines
        } else {
            0
        },
        document_lines,
        code_lines,
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
        assert!(options.ignored_paths.is_empty());
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

    #[test]
    fn scan_report_includes_file_stats_per_language() {
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
        fs::write(
            temp_dir.path().join("README.md"),
            "# Title\n\nSome project notes.\n",
        )
        .expect("write markdown file");

        let report = scan_path(temp_dir.path(), &ScanOptions::default());

        let rust = report
            .languages
            .iter()
            .find(|language| language.name == "Rust")
            .expect("Rust language stat");
        assert_eq!(rust.file_stats.len(), 2);
        assert_eq!(rust.file_stats[0].path, temp_dir.path().join("main.rs"));
        assert_eq!(rust.file_stats[0].total_lines, 5);
        assert_eq!(rust.file_stats[0].code_lines, 3);
        assert_eq!(rust.file_stats[0].comment_lines, 1);
        assert_eq!(rust.file_stats[0].document_lines, 0);
        assert_eq!(rust.file_stats[0].blank_lines, 1);

        let markdown = report
            .languages
            .iter()
            .find(|language| language.name == "Markdown")
            .expect("Markdown language stat");
        assert_eq!(markdown.file_stats.len(), 1);
        assert_eq!(
            markdown.file_stats[0].path,
            temp_dir.path().join("README.md")
        );
        assert_eq!(markdown.file_stats[0].total_lines, 3);
        assert_eq!(markdown.file_stats[0].code_lines, 0);
        assert_eq!(markdown.file_stats[0].comment_lines, 0);
        assert_eq!(markdown.file_stats[0].document_lines, 2);
        assert_eq!(markdown.file_stats[0].blank_lines, 1);
    }

    #[test]
    fn scan_report_ignores_configured_paths() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        fs::create_dir(temp_dir.path().join("src")).expect("create src dir");
        fs::create_dir(temp_dir.path().join("ignored")).expect("create ignored dir");
        fs::write(
            temp_dir.path().join("src").join("main.rs"),
            "fn main() {}\n",
        )
        .expect("write counted rust file");
        fs::write(
            temp_dir.path().join("ignored").join("skip.rs"),
            "fn skipped() {}\n",
        )
        .expect("write ignored rust file");
        let options = ScanOptions {
            ignored_paths: vec!["ignored".to_owned()],
            ..ScanOptions::default()
        };

        let report = scan_path(temp_dir.path(), &options);

        assert_eq!(report.summary.files, 1);
        let rust = report
            .languages
            .iter()
            .find(|language| language.name == "Rust")
            .expect("Rust language stat");
        assert_eq!(rust.file_stats.len(), 1);
        assert_eq!(
            rust.file_stats[0].path,
            temp_dir.path().join("src").join("main.rs")
        );
    }

    #[test]
    fn scan_report_diff_tracks_summary_and_language_deltas() {
        let before = report_with_languages(&[
            language_stat("Rust", 1, 10, 8, 1, 0, 1),
            language_stat("Markdown", 1, 4, 0, 0, 3, 1),
        ]);
        let after = report_with_languages(&[
            language_stat("Rust", 2, 16, 13, 1, 0, 2),
            language_stat("Python", 1, 5, 4, 0, 0, 1),
        ]);

        let diff = diff_reports(&before, &after);

        assert_eq!(diff.summary.files, 1);
        assert_eq!(diff.summary.total_lines, 7);
        assert_eq!(diff.summary.code_lines, 9);
        assert_eq!(diff.summary.document_lines, -3);
        assert_eq!(diff.summary.blank_lines, 1);

        assert_eq!(diff.languages.len(), 3);
        let rust = diff
            .languages
            .iter()
            .find(|language| language.name == "Rust")
            .expect("Rust diff");
        assert_eq!(rust.files, 1);
        assert_eq!(rust.code_lines, 5);

        let markdown = diff
            .languages
            .iter()
            .find(|language| language.name == "Markdown")
            .expect("Markdown diff");
        assert_eq!(markdown.files, -1);
        assert_eq!(markdown.document_lines, -3);

        let python = diff
            .languages
            .iter()
            .find(|language| language.name == "Python")
            .expect("Python diff");
        assert_eq!(python.files, 1);
        assert_eq!(python.code_lines, 4);
    }

    fn report_with_languages(languages: &[LanguageStat]) -> ScanReport {
        let mut summary = ScanSummary {
            root: PathBuf::from("sample"),
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

        for language in languages {
            summary.files += language.files;
            summary.total_lines += language.total_lines;
            summary.blank_lines += language.blank_lines;
            summary.comment_lines += language.comment_lines;
            summary.document_lines += language.document_lines;
            summary.code_lines += language.code_lines;
            categories.code += language.code_lines;
            categories.comments += language.comment_lines;
            categories.documents += language.document_lines;
            categories.blanks += language.blank_lines;
        }

        ScanReport {
            summary,
            categories,
            languages: languages.to_vec(),
        }
    }

    fn language_stat(
        name: &str,
        files: usize,
        total_lines: usize,
        code_lines: usize,
        comment_lines: usize,
        document_lines: usize,
        blank_lines: usize,
    ) -> LanguageStat {
        LanguageStat {
            name: name.to_owned(),
            files,
            total_lines,
            blank_lines,
            comment_lines,
            document_lines,
            code_lines,
            is_document: document_lines > 0,
            file_stats: Vec::new(),
        }
    }
}
