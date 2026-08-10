$ErrorActionPreference = "Continue"

$targets = Get-CimInstance Win32_Process |
  Where-Object {
    $_.Name -eq "sentune.exe" -or
    ($_.Name -eq "msedgewebview2.exe" -and $_.CommandLine -match "com\.sentune\.app")
  }

foreach ($target in $targets) {
  Write-Output ("KILL " + $target.Name + " PID=" + $target.ProcessId)
  Stop-Process -Id $target.ProcessId -Force -ErrorAction SilentlyContinue
}

Start-Sleep -Seconds 2

$profileDir = Join-Path $env:LOCALAPPDATA "com.sentune.app\EBWebView"
if (Test-Path $profileDir) {
  Write-Output ("REMOVE " + $profileDir)
  Remove-Item -LiteralPath $profileDir -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Output "CLEANED"
