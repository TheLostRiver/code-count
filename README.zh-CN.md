# code-count

[English](README.md)

`code-count` 是一个跨平台的代码与文档行数统计工具。

项目先从 CLI 开始，底层使用可复用的 Rust core 库；随后在同一套扫描模型上提供 TUI。未来如果要做桌面 GUI，也不需要重写扫描引擎。

计数引擎基于 `tokei`，但本项目拥有自己的公开数据模型和用户体验。

## 计划范围

- 统计源码文件、脚本、Markdown 和纯文本文档。
- 报告总行数、代码行、注释行、文档行和空白行。
- 支持普通文本输出、JSON 输出、按语言统计、Markdown/HTML 报告、TUI 和扫描快照 diff。
- 保持 core 扫描器可被 CLI、TUI 和未来桌面 GUI 复用。

## 使用方式

```powershell
code-count .
code-count . --json
code-count . --by-language
code-count report . --format markdown --output report.md
code-count report . --format html --output report.html --files
code-count tui .
```

## Windows 便携版安装

构建便携包：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\package-windows.ps1
```

打包结果会生成到：

```text
dist\code-count-windows-x64\
```

把这个文件夹移动到你想保存工具的位置，然后在该文件夹中运行：

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

安装脚本会把便携包所在目录加入当前用户的 `PATH`，不需要管理员权限。打开一个新的终端后，就可以在任意目录直接运行：

```powershell
code-count .
code-count tui .
```

如果以后想移除全局命令入口，在便携包目录运行：

```powershell
powershell -ExecutionPolicy Bypass -File .\uninstall.ps1
```

卸载脚本只会移除用户 `PATH` 中的目录项，不会删除工具文件。

一次性扫描时可以忽略空白行或注释行：

```powershell
code-count . --ignore-blank
code-count . --ignore-comments
code-count . --ignore vendor --ignore generated
```

保存扫描快照并比较两次扫描结果：

```powershell
code-count history save . --output before.json
code-count history save . --output after.json
code-count diff before.json after.json
```

导出可分享的报告：

```powershell
code-count report . --format markdown --output report.md
code-count report . --format html --output report.html --files
code-count report . --format csv
```

临时排除目录时可以使用 `--ignore <path>`，并且可以重复指定：

```powershell
code-count . --ignore vendor --ignore build
code-count tui . --ignore generated
code-count report . --ignore third_party --format markdown
```

CLI 里的 ignore 路径会和 `code-count.toml` 中的 `ignored_paths` 合并后用于当前运行。

在 TUI 的 Explorer 视图里，Details 面板会根据当前报告中行数最多的顶层目录显示扫描范围建议。按 `i` 可以把最上方建议加入本次会话的 ignore 列表并重新扫描；按 `u` 可以撤销最后一次会话内 ignore。这些操作不会写入 `code-count.toml`。

`report` 在没有指定 `--output` 时会输出到 stdout。支持的格式包括
`json`、`markdown`、`html` 和 `csv`。需要在报告中包含每个文件的明细时，可以加上
`--files`。

## 项目配置

在项目根目录创建 `code-count.toml`，可以设置扫描和 TUI 默认值：

```toml
[scan]
include_blank_lines = true
include_comments = true
ignored_paths = ["target", ".git", "node_modules"]

[tui]
default_view = "dashboard"
report_format = "markdown"
```

支持的 TUI 视图包括 `dashboard`、`explorer` 和 `report`。支持的报告格式包括 `json`、`markdown`、`html` 和 `csv`。

`--ignore-blank`、`--ignore-comments` 和 `--ignore <path>` 会在当前运行中覆盖或扩展配置。扫描配置所在项目时，`code-count.toml` 会被自动忽略，不计入统计结果。

## 架构

```text
crates/core
  可复用扫描 API，将 tokei 输出转换为项目自有类型。

crates/cli
  命令行参数解析、文件读写和终端输出。

crates/tui
  基于共享扫描报告模型的终端 UI。
```

所有前端都应该使用同一套 core 扫描器，不要直接调用 `tokei`。

## 路线图

1. CLI 基线版本。
2. 带语言和文档分类的 `ScanReport` 模型。
3. 使用 `ratatui` 和 `crossterm` 的 TUI Dashboard。
4. Explorer 和 Report 视图。
5. `code-count.toml` 项目配置。
6. 扫描历史快照和 diff。
7. Markdown 和 HTML 报告模板。
8. 桌面 GUI 原型。

## 开发

```powershell
cargo fmt --check
cargo test
cargo clippy
cargo run -p code-count -- .
cargo run -p code-count -- . --json
```
