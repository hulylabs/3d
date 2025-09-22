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

$compiledShaderFileName = "_tracer.wgsl"
$reflectionFileName = "_reflection.json"

Write-Host "Using Slang compiler from: $slangcPath"
& $slangcPath tracer.slang -target wgsl -o $compiledShaderFileName -reflection-json $reflectionFileName -warnings-as-errors all -matrix-layout-column-major -no-mangle

if (Test-Path $compiledShaderFileName) {
    Write-Host "Removing alignment attributes from $compiledShaderFileName..."
    
    $content = Get-Content $compiledShaderFileName -Raw
    
    $content = $content.Replace('@align(16) ', '')
    $content = $content.Replace('@align(8) ', '')
    $content = $content.Replace('@align(4) ', '')
    $content = $content.Replace('@align(16)', '')
    $content = $content.Replace('@align(8)', '')
    $content = $content.Replace('@align(4)', '')
    
    # Normalize line endings to LF
    $content = $content.Replace("`r`n", "`n").Replace("`r", "`n")
    
    Set-Content $compiledShaderFileName $content -NoNewline
}
