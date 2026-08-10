$processes = Get-Process -Name sentune -ErrorAction SilentlyContinue
foreach ($process in $processes) {
  $process.Refresh()
  Write-Output ("PID=" + $process.Id)
  Write-Output ("PATH=" + $process.Path)
  Write-Output ("RESPONDING=" + $process.Responding)
  Write-Output ("CPU=" + $process.CPU)
}
