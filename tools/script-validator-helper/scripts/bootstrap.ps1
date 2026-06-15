$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$helperRoot = Resolve-Path (Join-Path $scriptDir "..")
$repoRoot = Resolve-Path (Join-Path $helperRoot "../..")
$cacheDir = Join-Path $repoRoot "third_party/cache"
$archive = Join-Path $cacheDir "lua-5.4.7.tar.gz"
$checksumFile = Join-Path $cacheDir "lua-5.4.7.sha256"
$depsDir = Join-Path $helperRoot "build/deps"
$extractRoot = Join-Path $depsDir "lua-5.4.7"

if (-not (Test-Path $archive)) {
  throw "Missing Lua archive: $archive"
}
if (-not (Test-Path $checksumFile)) {
  throw "Missing Lua checksum file: $checksumFile"
}

$expected = (Get-Content $checksumFile -Raw).Trim().Split(" ")[0].ToLowerInvariant()
$actual = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
if ($actual -ne $expected) {
  throw "Lua archive checksum mismatch. Expected $expected but got $actual."
}

New-Item -ItemType Directory -Force -Path $depsDir | Out-Null
if (Test-Path $extractRoot) {
  Remove-Item -Recurse -Force $extractRoot
}

tar -xzf $archive -C $depsDir
$expanded = Join-Path $depsDir "lua-5.4.7"
if (-not (Test-Path (Join-Path $expanded "src/lua.h"))) {
  throw "Lua extraction did not produce expected src/lua.h under $expanded"
}

Write-Output "Lua 5.4.7 ready at $expanded"
