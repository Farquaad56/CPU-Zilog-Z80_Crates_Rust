# Validation d'une tache (role "validateur"). Deterministe: le bot ne fait que lancer ce script.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File tools\validate.ps1 E03-01 [-NoScope]
# Etapes: task.py check -> cargo fmt + tools\ci.ps1 (fmt, clippy -D warnings, tests) -> task.py done -> git add/commit (local). Ne pousse JAMAIS, n'approuve JAMAIS une gate.
param([Parameter(Mandatory = $true)][string]$Id, [switch]$NoScope, [switch]$SkipCi)
Set-Location (Split-Path -Parent $PSScriptRoot)
$env:PATH = "C:\Program Files\Git\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"

function Fail($msg, $out) {
    Write-Host "VALIDATION FAILED $Id : $msg"
    if ($out) { $out | Select-Object -Last 25 }
    exit 1
}

$file = "docs\tasks\$Id.md"
if (-not (Test-Path $file)) { Fail 'unknown task id' $null }
$first = (Get-Content $file -TotalCount 1)
$title = ($first -replace "^#\s*$([regex]::Escape($Id))\s*-\s*", '').Trim()

$args1 = @('docs\tools\task.py', 'check', $Id)
if ($NoScope) { $args1 += '--no-scope' }
$out = & python @args1 2>&1
if ($LASTEXITCODE -ne 0) { Fail 'task.py check failed' $out }
$out | Select-Object -Last 3

# CI complete (format, clippy, tests): evite d'accumuler des erreurs de lint d'une tache a l'autre.
if (-not $SkipCi) {
    cargo fmt --all 2>&1 | Out-Null
    $out = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'ci.ps1') 2>&1
    if ($LASTEXITCODE -ne 0) { Fail 'CI failed (fmt/clippy/tests): fix the code, then rerun' $out }
    Write-Host 'CI OK'
}

$out = & python docs\tools\task.py done $Id 2>&1
if ($LASTEXITCODE -ne 0) { Fail 'task.py done failed' $out }

git add -A 2>&1 | Out-Null
git diff --cached --quiet
if ($LASTEXITCODE -eq 0) { Write-Host "VALIDATED $Id (nothing to commit)"; exit 0 }
git commit -q -m "${Id}: $title" 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Fail 'git commit failed' $null }
$hash = (git rev-parse --short HEAD)
Write-Host "VALIDATED $Id COMMIT $hash"
exit 0
