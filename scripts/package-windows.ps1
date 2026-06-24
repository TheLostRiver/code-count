Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$packageName = "code-count-windows-x64"
$distRoot = Join-Path $repoRoot "dist"
$packageDir = Join-Path $distRoot $packageName
$exePath = Join-Path $repoRoot "target\release\code-count.exe"

Push-Location $repoRoot
try {
    cargo build -p code-count --release

    if (Test-Path -LiteralPath $packageDir) {
        Remove-Item -LiteralPath $packageDir -Recurse -Force
    }

    New-Item -ItemType Directory -Force -Path $packageDir | Out-Null
    Copy-Item -LiteralPath $exePath -Destination (Join-Path $packageDir "code-count.exe")
    Copy-Item -LiteralPath (Join-Path $repoRoot "README.md") -Destination $packageDir
    Copy-Item -LiteralPath (Join-Path $repoRoot "README.zh-CN.md") -Destination $packageDir
    Copy-Item -LiteralPath (Join-Path $repoRoot "scripts\install.ps1") -Destination $packageDir
    Copy-Item -LiteralPath (Join-Path $repoRoot "scripts\uninstall.ps1") -Destination $packageDir

    Write-Host "Portable package created:"
    Write-Host "  $packageDir"
} finally {
    Pop-Location
}
