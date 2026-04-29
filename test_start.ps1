$errorActionPreference = "Stop"
$outputPath = "c:\Users\21306\Desktop\FlowReveal\start_log.txt"

try {
    Write-Host "Starting FlowReveal..."
    
    $process = Start-Process -FilePath "c:\Users\21306\Desktop\FlowReveal\bin\Debug\net10.0\win-x64\publish\FlowReveal.exe" -PassThru -Wait
    
    Write-Host "Process exited with code: $($process.ExitCode)"
    "Exit code: $($process.ExitCode)" | Out-File $outputPath
} catch {
    Write-Host "Error: $_"
    "Error: $_" | Out-File $outputPath
}