function Get-SlangCompilerPath {
    <#
    .SYNOPSIS
    Gets the appropriate Slang compiler path based on the current operating system.
    
    .DESCRIPTION
    This function detects the current operating system and returns the relative path 
    to the appropriate Slang compiler binary. It also validates that the compiler 
    exists at the expected location.
    
    .OUTPUTS
    String - The path to the Slang compiler executable
    
    .EXAMPLE
    $slangcPath = Get-SlangCompilerPath
    #>
    
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

    Write-Host "Using Slang compiler from: $slangcPath"
    return $slangcPath
}
