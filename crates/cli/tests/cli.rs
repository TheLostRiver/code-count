use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn by_language_prints_language_breakdown() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    fs::write(
        temp_dir.path().join("main.rs"),
        "fn main() {\n    println!(\"hello\");\n}\n",
    )
    .expect("write rust file");
    fs::write(temp_dir.path().join("README.md"), "# Title\n\nNotes.\n").expect("write markdown");

    let mut command = Command::cargo_bin("code-count").expect("binary exists");
    command.arg(temp_dir.path()).arg("--by-language");

    command.assert().success().stdout(
        predicate::str::contains("Languages")
            .and(predicate::str::contains("Rust"))
            .and(predicate::str::contains("Markdown"))
            .and(predicate::str::contains("Documents")),
    );
}

#[test]
fn missing_path_prints_readable_error() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let missing_path = temp_dir.path().join("missing");

    let mut command = Command::cargo_bin("code-count").expect("binary exists");
    command.arg(&missing_path);

    command.assert().failure().stderr(
        predicate::str::contains("path does not exist")
            .and(predicate::str::contains(missing_path.display().to_string())),
    );
}

#[test]
fn config_file_can_set_scan_defaults_and_ignored_paths() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    fs::write(
        temp_dir.path().join("code-count.toml"),
        "[scan]\ninclude_blank_lines = false\nignored_paths = [\"ignored\"]\n",
    )
    .expect("write config file");
    fs::create_dir(temp_dir.path().join("src")).expect("create src dir");
    fs::create_dir(temp_dir.path().join("ignored")).expect("create ignored dir");
    fs::write(
        temp_dir.path().join("src").join("main.rs"),
        "fn main() {\n\n    println!(\"hello\");\n}\n",
    )
    .expect("write counted file");
    fs::write(
        temp_dir.path().join("ignored").join("skip.rs"),
        "fn skipped() {}\n",
    )
    .expect("write ignored file");

    let mut command = Command::cargo_bin("code-count").expect("binary exists");
    command.arg(temp_dir.path());

    command.assert().success().stdout(
        predicate::str::contains("Files: 1")
            .and(predicate::str::contains("Blank lines: 0"))
            .and(predicate::str::contains("Code lines: 3")),
    );
}
