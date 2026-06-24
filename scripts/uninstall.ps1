Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$toolDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$toolDir = [System.IO.Path]::GetFullPath($toolDir)

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ([string]::IsNullOrWhiteSpace($userPath)) {
    Write-Host "User PATH is empty. Nothing to remove."
    return
}

$entries = $userPath -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
$keptEntries = $entries | Where-Object {
    [System.IO.Path]::GetFullPath($_.TrimEnd('\')) -ine $toolDir.TrimEnd('\')
}

if ($keptEntries.Count -eq $entries.Count) {
    Write-Host "code-count was not found on your user PATH:"
    Write-Host "  $toolDir"
} else {
    [Environment]::SetEnvironmentVariable("Path", ($keptEntries -join ';'), "User")
    Write-Host "Removed code-count from your user PATH:"
    Write-Host "  $toolDir"
}

$processEntries = $env:Path -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
$env:Path = ($processEntries | Where-Object {
    [System.IO.Path]::GetFullPath($_.TrimEnd('\')) -ine $toolDir.TrimEnd('\')
}) -join ';'

Write-Host ""
Write-Host "The files were not deleted. Remove this folder manually if you no longer need it."
