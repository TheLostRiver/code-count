Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$toolDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$toolDir = [System.IO.Path]::GetFullPath($toolDir)
$exePath = Join-Path $toolDir "code-count.exe"

if (-not (Test-Path -LiteralPath $exePath)) {
    throw "code-count.exe was not found next to install.ps1: $exePath"
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$entries = @()
if (-not [string]::IsNullOrWhiteSpace($userPath)) {
    $entries = $userPath -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
}

$alreadyInstalled = $entries | Where-Object {
    [System.IO.Path]::GetFullPath($_.TrimEnd('\')) -ieq $toolDir.TrimEnd('\')
}

if ($alreadyInstalled) {
    Write-Host "code-count is already on your user PATH:"
    Write-Host "  $toolDir"
} else {
    $newEntries = @($entries + $toolDir)
    $newPath = ($newEntries -join ';')
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "Added code-count to your user PATH:"
    Write-Host "  $toolDir"
}

$processEntries = $env:Path -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
$processHasToolDir = $processEntries | Where-Object {
    [System.IO.Path]::GetFullPath($_.TrimEnd('\')) -ieq $toolDir.TrimEnd('\')
}
if (-not $processHasToolDir) {
    $env:Path = ($processEntries + $toolDir) -join ';'
}

Write-Host ""
Write-Host "Open a new terminal, then run:"
Write-Host "  code-count ."
Write-Host "  code-count tui ."
