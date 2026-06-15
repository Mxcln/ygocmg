$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$helperRoot = Resolve-Path (Join-Path $scriptDir "..")
$helper = Join-Path $helperRoot "build/bin/Release/script-validator-helper.exe"
if (-not (Test-Path $helper)) {
  $helper = Join-Path $helperRoot "build/bin/script-validator-helper.exe"
}
if (-not (Test-Path $helper)) {
  throw "Helper executable not found. Run scripts/build.ps1 first."
}

function Invoke-Fixture($name, $expectedStatus, $expectedIssueCode) {
  $path = Join-Path $helperRoot "fixtures/$name"
  $raw = & $helper --input $path
  if ($LASTEXITCODE -ne 0) {
    throw "Helper exited with $LASTEXITCODE for $name"
  }
  $json = $raw | ConvertFrom-Json
  if ($json.stage -ne "ocgcore_init") {
    throw "$name expected stage ocgcore_init but got $($json.stage)"
  }
  if ($json.status -ne $expectedStatus) {
    throw "$name expected status $expectedStatus but got $($json.status). Raw: $raw"
  }
  if ($expectedIssueCode) {
    $codes = @($json.issues | ForEach-Object { $_.code })
    if ($codes -notcontains $expectedIssueCode) {
      throw "$name expected issue code $expectedIssueCode but got [$($codes -join ', ')]"
    }
  }
  Write-Output "$name => $($json.status)"
}

Invoke-Fixture "valid_getid.input.json" "pass" $null
Invoke-Fixture "missing_end.input.json" "fail" "lua_syntax_error"
Invoke-Fixture "missing_core.input.json" "fail" "missing_core_script"
