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

#[test]
fn cli_ignore_option_excludes_paths_for_one_off_scans() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    fs::create_dir(temp_dir.path().join("src")).expect("create src dir");
    fs::create_dir(temp_dir.path().join("vendor")).expect("create vendor dir");
    fs::write(
        temp_dir.path().join("src").join("main.rs"),
        "fn main() {}\n",
    )
    .expect("write counted file");
    fs::write(
        temp_dir.path().join("vendor").join("skip.rs"),
        "fn skipped() {}\n",
    )
    .expect("write ignored file");

    let mut command = Command::cargo_bin("code-count").expect("binary exists");
    command.arg(temp_dir.path()).arg("--ignore").arg("vendor");

    command.assert().success().stdout(
        predicate::str::contains("Files: 1").and(predicate::str::contains("Code lines: 1")),
    );
}

#[test]
fn init_command_writes_default_project_config() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");

    let mut command = Command::cargo_bin("code-count").expect("binary exists");
    command.arg("init").arg(temp_dir.path());

    command
        .assert()
        .success()
        .stdout(predicate::str::contains("Created config"));

    let config =
        fs::read_to_string(temp_dir.path().join("code-count.toml")).expect("read generated config");
    assert!(config.contains("[scan]"));
    assert!(config.contains("ignored_paths"));
    assert!(config.contains("node_modules"));
    assert!(config.contains("[tui]"));
    assert!(config.contains("default_view = \"dashboard\""));
}

#[test]
fn init_command_refuses_to_overwrite_existing_config_without_force() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join("code-count.toml");
    fs::write(&config_path, "# keep me\n").expect("write existing config");

    let mut command = Command::cargo_bin("code-count").expect("binary exists");
    command.arg("init").arg(temp_dir.path());

    command
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    let config = fs::read_to_string(config_path).expect("read config");
    assert_eq!(config, "# keep me\n");
}

#[test]
fn init_command_force_overwrites_existing_config() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join("code-count.toml");
    fs::write(&config_path, "# replace me\n").expect("write existing config");

    let mut command = Command::cargo_bin("code-count").expect("binary exists");
    command.arg("init").arg(temp_dir.path()).arg("--force");

    command
        .assert()
        .success()
        .stdout(predicate::str::contains("Created config"));

    let config = fs::read_to_string(config_path).expect("read config");
    assert!(config.contains("[scan]"));
    assert!(!config.contains("replace me"));
}

#[test]
fn history_save_writes_snapshot_json() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let snapshot_path = temp_dir.path().join("snapshot.json");
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write rust file");

    let mut command = Command::cargo_bin("code-count").expect("binary exists");
    command
        .arg("history")
        .arg("save")
        .arg(temp_dir.path())
        .arg("--output")
        .arg(&snapshot_path);

    command
        .assert()
        .success()
        .stdout(predicate::str::contains("Saved snapshot"));

    let snapshot = fs::read_to_string(&snapshot_path).expect("read snapshot");
    assert!(snapshot.contains("\"summary\""));
    assert!(snapshot.contains("\"languages\""));
    assert!(snapshot.contains("\"Rust\""));
}

#[test]
fn diff_prints_summary_and_language_deltas_between_snapshots() {
    let before_dir = tempfile::tempdir().expect("create before dir");
    let after_dir = tempfile::tempdir().expect("create after dir");
    let snapshot_dir = tempfile::tempdir().expect("create snapshot dir");
    let before_snapshot = snapshot_dir.path().join("before.json");
    let after_snapshot = snapshot_dir.path().join("after.json");

    fs::write(before_dir.path().join("main.rs"), "fn main() {}\n").expect("write before rust");
    fs::write(
        after_dir.path().join("main.rs"),
        "fn main() {\n    println!(\"hello\");\n}\n",
    )
    .expect("write after rust");

    Command::cargo_bin("code-count")
        .expect("binary exists")
        .arg("history")
        .arg("save")
        .arg(before_dir.path())
        .arg("--output")
        .arg(&before_snapshot)
        .assert()
        .success();
    Command::cargo_bin("code-count")
        .expect("binary exists")
        .arg("history")
        .arg("save")
        .arg(after_dir.path())
        .arg("--output")
        .arg(&after_snapshot)
        .assert()
        .success();

    let mut command = Command::cargo_bin("code-count").expect("binary exists");
    command
        .arg("diff")
        .arg(&before_snapshot)
        .arg(&after_snapshot);

    command.assert().success().stdout(
        predicate::str::contains("Scan diff")
            .and(predicate::str::contains("Code lines: +2"))
            .and(predicate::str::contains("Rust"))
            .and(predicate::str::contains("+2")),
    );
}

#[test]
fn report_command_writes_markdown_report_file() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let output_path = temp_dir.path().join("reports").join("report.md");
    fs::write(
        temp_dir.path().join("main.rs"),
        "fn main() {\n    println!(\"hello\");\n}\n",
    )
    .expect("write rust file");

    let mut command = Command::cargo_bin("code-count").expect("binary exists");
    command
        .arg("report")
        .arg(temp_dir.path())
        .arg("--format")
        .arg("markdown")
        .arg("--output")
        .arg(&output_path);

    command
        .assert()
        .success()
        .stdout(predicate::str::contains("Saved report"));

    let report = fs::read_to_string(&output_path).expect("read report file");
    assert!(report.contains("# code-count report"));
    assert!(report.contains("| Rust |"));
}

#[test]
fn report_command_writes_html_report_with_file_details() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let output_path = temp_dir.path().join("report.html");
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write rust file");
    fs::write(temp_dir.path().join("README.md"), "# Notes\n").expect("write markdown file");

    let mut command = Command::cargo_bin("code-count").expect("binary exists");
    command
        .arg("report")
        .arg(temp_dir.path())
        .arg("--format")
        .arg("html")
        .arg("--output")
        .arg(&output_path)
        .arg("--files");

    command
        .assert()
        .success()
        .stdout(predicate::str::contains("Saved report"));

    let report = fs::read_to_string(&output_path).expect("read report file");
    assert!(report.contains("<!doctype html>"));
    assert!(report.contains("<td>Rust</td>"));
    assert!(report.contains("main.rs"));
    assert!(report.contains("README.md"));
}

#[test]
fn report_command_prints_csv_to_stdout() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").expect("write rust file");

    let mut command = Command::cargo_bin("code-count").expect("binary exists");
    command
        .arg("report")
        .arg(temp_dir.path())
        .arg("--format")
        .arg("csv");

    command.assert().success().stdout(
        predicate::str::contains("kind,name,files,total,code,comments,documents,blank")
            .and(predicate::str::contains("summary,"))
            .and(predicate::str::contains("language,Rust")),
    );
}
