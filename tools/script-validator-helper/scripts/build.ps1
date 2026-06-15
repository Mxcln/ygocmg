$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$helperRoot = Resolve-Path (Join-Path $scriptDir "..")
$repoRoot = Resolve-Path (Join-Path $helperRoot "../..")

& (Join-Path $scriptDir "bootstrap.ps1")

function ConvertTo-CMakePath($path) {
  return ([string]$path).Replace('\', '/')
}

$buildDir = ConvertTo-CMakePath (Join-Path $helperRoot "build/cmake")
$binDir = ConvertTo-CMakePath (Join-Path $helperRoot "build/bin")
$ocgcoreDir = ConvertTo-CMakePath (Join-Path $repoRoot "third_party/ocgcore")
$luaSrcDir = ConvertTo-CMakePath (Join-Path $helperRoot "build/deps/lua-5.4.7")
New-Item -ItemType Directory -Force -Path $binDir | Out-Null

$generatorArgs = @()
$vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio/Installer/vswhere.exe"
if (Test-Path $vswhere) {
  $vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
  if ($vsPath) {
    $generatorArgs = @("-G", "Visual Studio 17 2022", "-A", "x64")
  }
}

$runtimeOutputArg = "-DCMAKE_RUNTIME_OUTPUT_DIRECTORY=$binDir"
$ocgcoreArg = "-DOCGCORE_DIR=$ocgcoreDir"
$luaArg = "-DLUA_SRC_DIR=$luaSrcDir"
cmake -S $helperRoot -B $buildDir @generatorArgs $runtimeOutputArg $ocgcoreArg $luaArg
if ($LASTEXITCODE -ne 0) {
  throw "CMake configure failed with exit code $LASTEXITCODE."
}
cmake --build $buildDir --config Release
if ($LASTEXITCODE -ne 0) {
  throw "CMake build failed with exit code $LASTEXITCODE."
}

$candidate = Join-Path $binDir "Release/script-validator-helper.exe"
if (-not (Test-Path $candidate)) {
  $candidate = Join-Path $binDir "script-validator-helper.exe"
}
if (-not (Test-Path $candidate)) {
  throw "Helper executable was not produced under $binDir"
}

Write-Output "Helper built at $candidate"
