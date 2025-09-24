. "$PSScriptRoot\_common.ps1"

$slangcPath = Get-SlangCompilerPath

$fileToCompile = "unit_tests.slang"

$compiledExeFileName = "_run_tests.exe"

& $slangcPath $fileToCompile -target exe -o $compiledExeFileName -warnings-as-errors all
