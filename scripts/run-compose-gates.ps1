#!/usr/bin/env pwsh
# Pre-merge compose gates for branch phase12-array-authority (phase 32).
# Mirrors docs/compose/BRANCH_INDEX.md. Runs all gates; exit 1 if any failed.
$ErrorActionPreference = "Continue"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$script:results = [System.Collections.Generic.List[object]]::new()

function Invoke-Gate {
    param([string]$Name, [string[]]$Cmd)
    Write-Host "`n=== $Name ===" -ForegroundColor Cyan
    Write-Host ($Cmd -join " ")
    & $Cmd[0] @($Cmd | Select-Object -Skip 1)
    $code = $LASTEXITCODE
    $ok = ($code -eq 0)
    [void]$script:results.Add([pscustomobject]@{ Name = $Name; Ok = $ok; Exit = $code })
    if (-not $ok) {
        Write-Host "FAIL $Name (exit $code)" -ForegroundColor Red
    } else {
        Write-Host "PASS $Name" -ForegroundColor Green
    }
}

Invoke-Gate "core lib (default / unity-world-primary)" @("cargo","test","-p","engine-core","--lib")
Invoke-Gate "core lib (feature off / audio)" @("cargo","test","-p","engine-core","--lib","--no-default-features","--features","audio")
Invoke-Gate "physics lib" @("cargo","test","-p","engine-physics","--lib")
Invoke-Gate "physics integration" @("cargo","test","-p","engine-physics","--test","physics_tests")
Invoke-Gate "editor tests" @("cargo","test","-p","engine-editor","--test","editor_tests")
Invoke-Gate "core examples" @("cargo","build","-p","engine-core","--examples")
Invoke-Gate "wasm engine-core" @("cargo","build","-p","engine-core","--target","wasm32-unknown-unknown","--no-default-features","--features","unity-world-primary")
Invoke-Gate "wasm engine-physics" @("cargo","build","-p","engine-physics","--target","wasm32-unknown-unknown")
Invoke-Gate "wasm engine-editor lib" @("cargo","build","-p","engine-editor","--target","wasm32-unknown-unknown","--no-default-features","--lib")
Invoke-Gate "fmt check" @("cargo","fmt","-p","engine-core","-p","engine-physics","-p","engine-editor","--check")

Write-Host "`n========== Gate summary ==========" -ForegroundColor Yellow
$failed = 0
foreach ($r in $script:results) {
    $tag = if ($r.Ok) { "PASS" } else { "FAIL" }
    $color = if ($r.Ok) { "Green" } else { "Red" }
    Write-Host ("{0,-4} {1} (exit {2})" -f $tag, $r.Name, $r.Exit) -ForegroundColor $color
    if (-not $r.Ok) { $failed++ }
}
if ($script:results.Count -eq 0) {
    Write-Host "No gates recorded — script error." -ForegroundColor Red
    exit 1
}
if ($failed -gt 0) {
    Write-Host "`n$failed gate(s) failed." -ForegroundColor Red
    exit 1
}
Write-Host "`nAll gates passed." -ForegroundColor Green
exit 0
