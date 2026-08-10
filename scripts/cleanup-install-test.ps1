$target = "C:\Users\Moon\AppData\Local\Temp\sentune-install-test"
$uninstaller = Join-Path $target "uninstall.exe"
if (Test-Path $uninstaller) {
  Start-Process -FilePath $uninstaller -ArgumentList "/S" -Wait
}
if (Test-Path $target) {
  Remove-Item -LiteralPath $target -Recurse -Force
}
Write-Output "CLEANED"
