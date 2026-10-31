# Hygiene check (Windows / PowerShell). Equivalent de tools/hygiene.sh.
# Usage: powershell -File tools\hygiene.ps1 [dir]   (default: crates/puce-z80/src)
param([string]$TargetDir = 'crates/puce-z80/src')
Set-Location (Split-Path -Parent $PSScriptRoot)

if (-not (Test-Path -LiteralPath $TargetDir -PathType Container)) {
    Write-Host "ERROR: directory not found: $TargetDir"
    exit 1
}
Write-Host "=== Hygiene check on $TargetDir ==="

$testCfg = '#\[cfg\(test\)\]|#\[cfg\(any\(test, feature = "std"\)\)\]'
$fail = $false

function Hits($path, $pattern) {
    @(Select-String -LiteralPath $path -Pattern $pattern -CaseSensitive -Encoding utf8)
}
function Report($path, $msg, $hits) {
    Write-Host "FAIL: ${path} $msg"
    $hits | Select-Object -First 5 | ForEach-Object { Write-Host ("{0}:{1}" -f $_.LineNumber, $_.Line) }
    $script:fail = $true
}

foreach ($file in Get-ChildItem -LiteralPath $TargetDir -Recurse -Filter '*.rs' -File) {
    $f = $file.FullName
    $hasTestCfg = (Hits $f $testCfg).Count -gt 0

    $h = Hits $f 'unsafe\s+\{|unsafe\s+fn|unsafe\s+impl|unsafe\s+extern'
    if ($h.Count) { Report $f 'contains unsafe code:' $h }

    $h = Hits $f '\bf32\b|\bf64\b'
    if ($h.Count) { Report $f 'contains floating point types:' $h }

    $h = Hits $f 'std::|extern crate std;'
    if ($h.Count -and -not $hasTestCfg) { Report $f 'uses std without test cfg:' $h }

    $h = Hits $f '\bprintln!\('
    if ($h.Count) { Report $f 'contains println!:' $h }

    $h = Hits $f '\.unwrap\(\)'
    if ($h.Count -and -not $hasTestCfg) { Report $f 'uses .unwrap() outside test context:' $h }

    $h = Hits $f '[^\x00-\x7F]'
    if ($h.Count) { Report $f 'contains non-ASCII characters:' $h }

    $h = @(Hits $f '\bTODO\b' | Where-Object { $_.Line -notmatch 'UNKNOWN - to confirm' })
    if ($h.Count) { Report $f "has TODO without 'UNKNOWN - to confirm':" $h }
}

if ($fail) {
    Write-Host '=== Hygiene check FAILED ==='
    exit 1
}
Write-Host '=== Hygiene check passed ==='
exit 0
