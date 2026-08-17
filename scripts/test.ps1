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

$root = Split-Path -Parent $PSScriptRoot
$resDir = Join-Path $root "src-tauri\target\test-resources"
New-Item -ItemType Directory -Force -Path $resDir | Out-Null
$res = Join-Path $resDir "test-manifest.res"

& (Join-Path $mingw "windres.exe") `
  (Join-Path $root "src-tauri\resources\test-manifest.rc") `
  -O coff `
  -o $res
if ($LASTEXITCODE -ne 0) {
  throw "windres failed to build manifest resource"
}

# On windows-gnu, cargo test builds test exes without the comctl32 v6
# manifest, so TaskDialogIndirect cannot be resolved at load time
# (STATUS_ENTRYPOINT_NOT_FOUND). Inject the v6 manifest resource through
# RUSTFLAGS so test binaries link it.
$env:RUSTFLAGS = "-C link-arg=$($res -replace '\\', '/')"

Push-Location (Join-Path $root "src-tauri")
try {
  cargo test @args
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }
}
finally {
  Pop-Location
}
