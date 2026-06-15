$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$helperRoot = Resolve-Path (Join-Path $scriptDir "..")

& (Join-Path $scriptDir "bootstrap.ps1")

$buildDir = Join-Path $helperRoot "build/cmake"
$binDir = Join-Path $helperRoot "build/bin"
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
cmake -S $helperRoot -B $buildDir @generatorArgs $runtimeOutputArg
cmake --build $buildDir --config Release

$candidate = Join-Path $binDir "Release/script-validator-helper.exe"
if (-not (Test-Path $candidate)) {
  $candidate = Join-Path $binDir "script-validator-helper.exe"
}
if (-not (Test-Path $candidate)) {
  throw "Helper executable was not produced under $binDir"
}

Write-Output "Helper built at $candidate"
