# CI local (Windows / PowerShell). Equivalent de tools/ci.sh. Usage: powershell -File tools\ci.ps1
$ErrorActionPreference = 'Continue'
Set-Location (Split-Path -Parent $PSScriptRoot)

function Step($title, [scriptblock]$cmd) {
    Write-Host "=== CI: $title ==="
    & $cmd
    if ($LASTEXITCODE -ne 0) {
        Write-Host "CI FAILED at: $title (exit $LASTEXITCODE)" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}

Step 'cargo fmt check'                  { cargo fmt --all -- --check }
Step 'cargo clippy (warnings as errors)' { cargo clippy --workspace --all-targets -- -D warnings }
Step 'cargo test'                       { cargo test --workspace }

Write-Host '=== CI: all checks passed ===' -ForegroundColor Green
