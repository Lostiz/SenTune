$ErrorActionPreference = "Stop"

$mingw = "C:\Users\Moon\.local\msys64\mingw64\bin"
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
