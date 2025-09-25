. "$PSScriptRoot\_common.ps1"

$slangcPath = Get-SlangCompilerPath

$fileToCompile = "tracer.slang"

$compiledShaderFileName = "_tracer.wgsl"
$reflectionFileName = "_reflection.json"

& $slangcPath $fileToCompile -target wgsl -o $compiledShaderFileName -reflection-json $reflectionFileName -warnings-as-errors all -matrix-layout-column-major -no-mangle

if (Test-Path $compiledShaderFileName) {
    Write-Host "Removing alignment attributes from $compiledShaderFileName..."
    
    $content = Get-Content $compiledShaderFileName -Raw
    
    # Those @align are not supported in WebGPU/WGSL
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
