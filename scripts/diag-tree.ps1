$ErrorActionPreference = "Continue"

$exe = "E:\BLIplayer\src-tauri\target\x86_64-pc-windows-gnu\release\sentune.exe"
$app = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds 8

$all = Get-CimInstance Win32_Process
$appProcess = Get-Process -Id $app.Id -ErrorAction SilentlyContinue
if ($appProcess) {
  Write-Output ("APP responding=" + $appProcess.Responding)
}

$ours = $all | Where-Object {
    $_.Name -eq "sentune.exe" -or
    ($_.Name -eq "msedgewebview2.exe" -and $_.CommandLine -match "com\.moon\.sentune")
}

foreach ($p in $ours) {
  $proc = Get-Process -Id $p.ProcessId -ErrorAction SilentlyContinue
  $status = "?"
  if ($proc) {
    $status = if ($proc.Responding) { "Running" } else { "NotResponding" }
  }
  $type = "browser"
  if ($p.CommandLine -match "--type=renderer") { $type = "renderer" }
  elseif ($p.CommandLine -match "--type=gpu-process") { $type = "gpu" }
  elseif ($p.CommandLine -match "--type=utility") { $type = "utility" }
  elseif ($p.CommandLine -match "--type=crashpad") { $type = "crashpad" }
  Write-Output ("PID=" + $p.ProcessId + " PARENT=" + $p.ParentProcessId + " TYPE=" + $type + " STATUS=" + $status)
}

foreach ($p in $ours) {
  Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue
}
