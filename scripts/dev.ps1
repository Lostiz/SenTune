$ErrorActionPreference = "Stop"

$mingwCandidates = @(
  "C:\Users\Moon\.local\msys64\mingw64\bin",
  "C:\msys64\mingw64\bin"
)
$mingw = $mingwCandidates | Where-Object { Test-Path (Join-Path $_ "gcc.exe") } | Select-Object -First 1
if (-not $mingw) {
  throw "未找到 MinGW-w64（gcc.exe），请安装 MSYS2 并安装 mingw-w64-x86_64-gcc"
}
$env:Path = "$mingw;$env:Path"
$env:CC_x86_64_pc_windows_gnu = "gcc"
$env:AR_x86_64_pc_windows_gnu = "ar"
$env:RANLIB_x86_64_pc_windows_gnu = "ranlib"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
  npm.cmd run tauri dev
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }
}
finally {
  Pop-Location
}
