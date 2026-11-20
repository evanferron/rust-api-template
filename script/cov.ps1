$ErrorActionPreference = "Stop"

# Lance tous les tests (unitaires, intégration, Cucumber) avec couverture.
# Prérequis : cargo install cargo-llvm-cov + base de test (DATABASE_TEST_URL)
cargo llvm-cov --html
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Coverage report available at: target/llvm-cov/html/index.html" -ForegroundColor Green
