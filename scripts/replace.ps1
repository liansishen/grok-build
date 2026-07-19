# 一键用自建版替换官方 grok
# 用法:
#   .\scripts\replace.ps1           # 用 target\release\xai-grok-pager.exe 替换
#   .\scripts\replace.ps1 -Build    # 先编译再替换
# 安装路径: %USERPROFILE%\.grok\bin\grok.exe

param(
    [switch]$Build
)

. (Join-Path $PSScriptRoot "common.ps1")
Set-Location $GrokRoot

Write-Step "Grok one-click replace (custom -> installed)"

if ($Build) {
    Write-Host "Building first (-Build)..."
    & (Join-Path $PSScriptRoot "build.ps1")
    if ($LASTEXITCODE -ne 0) {
        throw "build.ps1 failed"
    }
}

if (-not (Test-Path $CustomExe)) {
    throw @"
Custom binary not found:
  $CustomExe

Run first:
  .\scripts\build.ps1
Or:
  .\scripts\replace.ps1 -Build
"@
}

if (-not (Test-Path $InstalledExe)) {
    throw "Installed grok not found: $InstalledExe"
}

Assert-NoRunningGrok

Write-Host "Custom:    $CustomExe"
Write-Host "Installed: $InstalledExe"

# Show versions before replace
Write-Host ""
Write-Host "Installed version (before):" -ForegroundColor DarkGray
try { & $InstalledExe --version 2>$null } catch { Write-Host "(could not run)" }
Write-Host "Custom version:" -ForegroundColor DarkGray
& $CustomExe --version

New-Item -ItemType Directory -Force -Path $BackupDir | Out-Null
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$backup = Join-Path $BackupDir "grok.exe.bak-$stamp"

Write-Step "Backup official binary"
Copy-Item -LiteralPath $InstalledExe -Destination $backup -Force
Write-Ok "Backup: $backup"

Write-Step "Replace installed grok"
Copy-Item -LiteralPath $CustomExe -Destination $InstalledExe -Force
Write-Ok "Replaced: $InstalledExe"

Write-Host ""
Write-Host "Active version:" -ForegroundColor Cyan
& grok --version
Write-Host ""
Write-Host "Restore with: .\scripts\restore.ps1" -ForegroundColor DarkGray
