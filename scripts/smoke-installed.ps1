$ErrorActionPreference = "Stop"

$exe = "C:\Users\Moon\AppData\Local\Temp\sentune-install-test\sentune.exe"
$process = Start-Process -FilePath $exe -PassThru

for ($i = 0; $i -lt 20; $i++) {
  Start-Sleep -Seconds 1
  $process.Refresh()
  if ($process.HasExited) {
    Write-Output ("EXITED_AFTER_SECONDS=" + ($i + 1))
    Write-Output ("EXIT_CODE=" + $process.ExitCode)
    exit 1
  }
  if ($i % 5 -eq 0) {
    Write-Output ("t=" + ($i + 1) + "s responding=" + $process.Responding)
  }
}

Write-Output "STILL_RUNNING_AND_RESPONSIVE"
Stop-Process -Id $process.Id -Force
