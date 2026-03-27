$ErrorActionPreference = "Stop"

Write-Host "Running lib tests..." -ForegroundColor Cyan
cargo llvm-cov --no-report --lib -- --test-threads=1

Write-Host "Running integration tests (excluding cucumber)..." -ForegroundColor Cyan
$targets = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$testTargets = $targets.packages[0].targets | Where-Object {
    $_.kind -contains "test" -and $_.name -ne "cucumber"
}
foreach ($target in $testTargets) {
    Write-Host "  -> Running $($target.name)" -ForegroundColor Gray
    cargo llvm-cov --no-report --test $target.name -- --test-threads=1
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Write-Host "Running cucumber tests..." -ForegroundColor Cyan
cargo llvm-cov --no-report --test cucumber
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Generating coverage report..." -ForegroundColor Cyan
cargo llvm-cov report --html
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Coverage report available at: target/llvm-cov/html/index.html" -ForegroundColor Green