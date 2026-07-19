# 一键还原官方 grok（从备份或 grok.exe.old）
# 用法:
#   .\scripts\restore.ps1              # 用最新一次 replace 备份还原
#   .\scripts\restore.ps1 -List        # 列出可用备份
#   .\scripts\restore.ps1 -FromOld     # 用官方更新留下的 grok.exe.old
#   .\scripts\restore.ps1 -File path   # 指定备份文件

param(
    [switch]$List,
    [switch]$FromOld,
    [string]$File = ""
)

. (Join-Path $PSScriptRoot "common.ps1")

Write-Step "Grok one-click restore"

if (-not (Test-Path $InstalledDir)) {
    throw "Install dir missing: $InstalledDir"
}

$oldOfficial = Join-Path $InstalledDir "grok.exe.old"
$backups = @()
if (Test-Path $BackupDir) {
    $backups = @(
        Get-ChildItem -Path $BackupDir -Filter "grok.exe.bak-*" -File -ErrorAction SilentlyContinue |
            Sort-Object LastWriteTime -Descending
    )
}

if ($List) {
    Write-Host "Installed: $InstalledExe"
    if (Test-Path $InstalledExe) {
        try { & $InstalledExe --version 2>$null } catch { }
    }
    Write-Host ""
    Write-Host "Official updater backup (grok.exe.old):"
    if (Test-Path $oldOfficial) {
        Write-Host "  $($oldOfficial)  ($((Get-Item $oldOfficial).LastWriteTime))"
    } else {
        Write-Host "  (none)"
    }
    Write-Host ""
    Write-Host "replace.ps1 backups ($BackupDir):"
    if ($backups.Count -eq 0) {
        Write-Host "  (none)"
    } else {
        $i = 0
        foreach ($b in $backups) {
            $i++
            Write-Host ("  [{0}] {1}  {2:yyyy-MM-dd HH:mm:ss}  {3:N0} bytes" -f $i, $b.Name, $b.LastWriteTime, $b.Length)
        }
    }
    return
}

$source = $null
if ($File) {
    if (-not (Test-Path -LiteralPath $File)) {
        throw "Backup file not found: $File"
    }
    $source = (Resolve-Path -LiteralPath $File).Path
} elseif ($FromOld) {
    if (-not (Test-Path $oldOfficial)) {
        throw "No grok.exe.old at $oldOfficial"
    }
    $source = $oldOfficial
} else {
    $latest = Get-LatestBackup
    if ($latest) {
        $source = $latest.FullName
    } elseif (Test-Path $oldOfficial) {
        Write-Warn "No replace.ps1 backup found; falling back to grok.exe.old"
        $source = $oldOfficial
    } else {
        throw @"
No backup found.

Looked in:
  $BackupDir\grok.exe.bak-*
  $oldOfficial

List backups:
  .\scripts\restore.ps1 -List
"@
    }
}

Assert-NoRunningGrok

Write-Host "Restore from: $source"
Write-Host "Restore to:   $InstalledExe"

# Safety copy of whatever is currently installed (so restore is also reversible)
if (Test-Path $InstalledExe) {
    New-Item -ItemType Directory -Force -Path $BackupDir | Out-Null
    $pre = Join-Path $BackupDir ("grok.exe.before-restore-" + (Get-Date -Format "yyyyMMdd-HHmmss"))
    Copy-Item -LiteralPath $InstalledExe -Destination $pre -Force
    Write-Host "Current binary saved as: $pre" -ForegroundColor DarkGray
}

Write-Step "Copy backup over installed grok"
Copy-Item -LiteralPath $source -Destination $InstalledExe -Force
Write-Ok "Restored: $InstalledExe"

Write-Host ""
Write-Host "Active version:" -ForegroundColor Cyan
& grok --version
