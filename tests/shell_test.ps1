$binary = $args[0]
if (-not $binary) {
    Write-Error "Usage: .\shell_test.ps1 <path-to-binary>"
    exit 1
}

if (-not (Test-Path $binary)) {
    Write-Error "Error: Binary not found at $binary"
    exit 1
}

Write-Host "Running shell tests on $binary..." -ForegroundColor Cyan

function Fail-Test($msg) {
    Write-Host "FAIL: $msg" -ForegroundColor Red
    exit 1
}

function Pass-Test($msg) {
    Write-Host "PASS: $msg" -ForegroundColor Green
}

# Test: Help output
$help = & $binary --help
if ($help -match "Usage: asleep") {
    Pass-Test "Help output"
} else {
    Fail-Test "Help output should contain usage"
}

# Test: Invalid duration
try {
    $output = & $binary invalid 2>&1
} catch {}
if ($output -match "Error parsing duration") {
    Pass-Test "Invalid duration handled"
} else {
    Fail-Test "Invalid duration did not show correct error"
}

# Test: Sleep duration
$start = Get-Date
& $binary 2s --no-progress
$end = Get-Date
$elapsed = ($end - $start).TotalSeconds

if ($elapsed -ge 1.9 -and $elapsed -le 4.1) {
    Pass-Test "Sleep duration (elapsed: ${elapsed}s)"
} else {
    Fail-Test "Sleep duration was out of expected range (elapsed: ${elapsed}s)"
}

# Test: Until past
try {
    $output = & $binary --until "@0" 2>&1
} catch {}
if ($output -match "in the past") {
    Pass-Test "Past --until handled"
} else {
    Fail-Test "Past --until did not show correct error"
}

# Test: --monotonic flag
& $binary 1s --monotonic
if ($LASTEXITCODE -eq 0) {
    Pass-Test "--monotonic flag works"
} else {
    Fail-Test "--monotonic flag failed"
}

# Test: -m flag
& $binary 1s -m
if ($LASTEXITCODE -eq 0) {
    Pass-Test "-m flag works"
} else {
    Fail-Test "-m flag failed"
}

# Test: Multiple durations summing
$start = Get-Date
& $binary 1s 2s 1s --no-progress
$end = Get-Date
$elapsed = ($end - $start).TotalSeconds
if ($elapsed -ge 3.9 -and $elapsed -le 6.1) {
    Pass-Test "Multiple durations summed correctly (elapsed: ${elapsed}s)"
} else {
    Fail-Test "Multiple durations summing out of range (elapsed: ${elapsed}s)"
}

Write-Host "All shell tests passed!" -ForegroundColor Green
