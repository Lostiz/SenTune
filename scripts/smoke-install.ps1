$ErrorActionPreference = "Stop"

$installer = "E:\BLIplayer\src-tauri\target\x86_64-pc-windows-gnu\release\bundle\nsis\SenTune_1.1.0_x64-setup.exe"
$target = "C:\Users\Moon\AppData\Local\Temp\sentune-install-test"

if (Test-Path $target) {
  Remove-Item -LiteralPath $target -Recurse -Force
}

$process = Start-Process -FilePath $installer -ArgumentList "/S", "/D=$target" -Wait -PassThru
Write-Output ("INSTALLER_EXIT=" + $process.ExitCode)

if (Test-Path $target) {
  Get-ChildItem -Recurse -File $target | Select-Object FullName, Length
}
