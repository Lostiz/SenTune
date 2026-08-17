$ErrorActionPreference = "Stop"

$exe = "E:\BLIplayer\src-tauri\target\release\sentune.exe"
$process = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds 8
if ($process.HasExited) {
  Write-Output ("EXITED code=" + $process.ExitCode)
  exit 1
}
Write-Output "RUNNING"
Stop-Process -Id $process.Id -Force
Write-Output "STOPPED"
