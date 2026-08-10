$events = Get-WinEvent -FilterHashtable @{
  LogName = "Application"
  StartTime = (Get-Date).AddHours(-6)
} -ErrorAction SilentlyContinue |
  Where-Object {
    $_.Message -match "sentune|WebView2|msedgewebview2"
  } |
  Select-Object -First 12

foreach ($event in $events) {
  Write-Output ("TIME=" + $event.TimeCreated)
  Write-Output ("ID=" + $event.Id + " PROVIDER=" + $event.ProviderName)
  $message = $event.Message
  if ($message.Length -gt 500) {
    $message = $message.Substring(0, 500)
  }
  Write-Output $message
  Write-Output "-----"
}
