if ($IsWindows -or $env:OS -eq "Windows_NT") {
    $slangcPath = ".\slang\slang-2025.16.1-windows-x86_64\bin\slangc.exe"
} elseif ($IsMacOS) {
    $slangcPath = "./slang/slang-2025.16.1-macos-aarch64/bin/slangc"
} elseif ($IsLinux) {
    $slangcPath = "./slang/slang-2025.16.1-linux-x86_64/bin/slangc"
} else {
    Write-Error "Unsupported operating system"
    exit 1
}

if (-not (Test-Path $slangcPath)) {
    Write-Error "Slang compiler not found at: $slangcPath"
    Write-Host "Please ensure you have the correct Slang distribution for your OS"
    exit 1
}

$outputFileName = "tracer.wgsl"

Write-Host "Using Slang compiler from: $slangcPath"
& $slangcPath tracer.slang -target wgsl -o $outputFileName -reflection-json reflection.json -warnings-as-errors all -matrix-layout-column-major -no-mangle

if (Test-Path $outputFileName) {
    Write-Host "Removing alignment attributes from $outputFileName..."
    
    $content = Get-Content $outputFileName -Raw
    
    $content = $content.Replace('@align(16) ', '')
    $content = $content.Replace('@align(8) ', '')
    $content = $content.Replace('@align(4) ', '')
    $content = $content.Replace('@align(16)', '')
    $content = $content.Replace('@align(8)', '')
    $content = $content.Replace('@align(4)', '')
    
    # Normalize line endings to LF
    $content = $content.Replace("`r`n", "`n").Replace("`r", "`n")
    
    Set-Content $outputFileName $content -NoNewline
}
