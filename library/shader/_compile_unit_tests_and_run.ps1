Write-Host "Compiling unit tests..."
& ".\_compile_unit_tests.ps1"

if ($LASTEXITCODE -eq 0 -and (Test-Path "_run_tests.exe")) {
    Write-Host "Compilation successful! Running tests..."
    Write-Host "----------------------------------------"
    
    # Run the tests
    & ".\_run_tests.exe"
    
    # Check test results
    if ($LASTEXITCODE -eq 0) {
        Write-Host "----------------------------------------"
        Write-Host "All tests passed!" -ForegroundColor Green
    } else {
        Write-Host "----------------------------------------"
        Write-Host "Some tests failed. Exit code: $LASTEXITCODE" -ForegroundColor Red
        exit $LASTEXITCODE
    }
} else {
    Write-Host "Compilation failed or test executable not found." -ForegroundColor Red
    Write-Host "Exit code from compilation: $LASTEXITCODE"
    exit 1
}
