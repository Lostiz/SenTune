$ErrorActionPreference = "Continue"

$exe = "E:\BLIplayer\src-tauri\target\x86_64-pc-windows-gnu\release\sentune.exe"
$process = Start-Process -FilePath $exe -PassThru

for ($i = 0; $i -lt 20; $i++) {
  Start-Sleep -Seconds 1
  $process.Refresh()
  if ($process.HasExited) {
    Write-Output ("EXITED_AFTER_SECONDS=" + ($i + 1))
    Write-Output ("EXIT_CODE=" + $process.ExitCode)
    exit 0
  }
  if ($i % 5 -eq 0) {
    Write-Output ("t=" + ($i + 1) + "s responding=" + $process.Responding)
  }
}

Write-Output "STILL_RUNNING_AFTER_20S"
Stop-Process -Id $process.Id -Force
