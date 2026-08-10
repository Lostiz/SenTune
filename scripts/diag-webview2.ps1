$ErrorActionPreference = "Continue"

$keys = @(
  "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
  "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
  "HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
)

foreach ($key in $keys) {
  if (Test-Path $key) {
    Write-Output ("KEY=" + $key)
    Get-ItemProperty $key | Select-Object pv, name | Format-List
  }
}

$runtimeDirs = @(
  "C:\Program Files (x86)\Microsoft\EdgeWebView\Application",
  "C:\Program Files\Microsoft\EdgeWebView\Application"
)
foreach ($dir in $runtimeDirs) {
  if (Test-Path $dir) {
    Write-Output ("DIR=" + $dir)
    Get-ChildItem $dir -Directory | Select-Object -ExpandProperty Name
  }
}

$exe = "C:\Program Files (x86)\Microsoft\EdgeWebView\Application\msedgewebview2.exe"
if (Test-Path $exe) {
  $output = & $exe --version 2>&1
  Write-Output ("VERSION=" + $output)
}
