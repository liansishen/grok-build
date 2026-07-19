# Compatibility wrapper — prefer .\scripts\build.ps1
& (Join-Path $PSScriptRoot "build.ps1") @args
exit $LASTEXITCODE
