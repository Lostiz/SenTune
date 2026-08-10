$ErrorActionPreference = "Continue"

# 清理本应用残留进程与损坏的 WebView2 用户数据目录
$targets = Get-CimInstance Win32_Process |
  Where-Object {
    $_.Name -eq "sentune.exe" -or
    ($_.Name -eq "msedgewebview2.exe" -and $_.CommandLine -match "com\.sentune\.app")
  }
foreach ($target in $targets) {
  Stop-Process -Id $target.ProcessId -Force -ErrorAction SilentlyContinue
}
Start-Sleep -Seconds 1

$profileDir = Join-Path $env:LOCALAPPDATA "com.sentune.app\EBWebView"
if (Test-Path $profileDir) {
  Remove-Item -LiteralPath $profileDir -Recurse -Force -ErrorAction SilentlyContinue
}

$exe = "E:\BLIplayer\src-tauri\target\x86_64-pc-windows-gnu\release\sentune.exe"
$process = Start-Process -FilePath $exe -PassThru

Start-Sleep -Seconds 8
$process.Refresh()
if ($process.HasExited) {
  Write-Output ("EXITED code=" + $process.ExitCode)
  exit 1
}
Write-Output ("RUNNING pid=" + $process.Id + " responding=" + $process.Responding)
