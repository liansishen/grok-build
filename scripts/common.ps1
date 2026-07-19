# Shared paths for Grok Windows build/replace/restore scripts.
$ErrorActionPreference = "Stop"

$script:GrokRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$script:CustomExe = Join-Path $GrokRoot "target\release\xai-grok-pager.exe"
$script:ProtocExe = Join-Path $GrokRoot "tools\protoc\bin\protoc.exe"
$script:InstalledDir = Join-Path $env:USERPROFILE ".grok\bin"
$script:InstalledExe = Join-Path $InstalledDir "grok.exe"
$script:BackupDir = Join-Path $InstalledDir "backups"

function Write-Step([string]$Message) {
    Write-Host ""
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Write-Ok([string]$Message) {
    Write-Host "OK: $Message" -ForegroundColor Green
}

function Write-Warn([string]$Message) {
    Write-Host "WARN: $Message" -ForegroundColor Yellow
}

function Assert-NoRunningGrok {
    $procs = Get-Process -Name "grok", "xai-grok-pager" -ErrorAction SilentlyContinue
    if ($procs) {
        $ids = ($procs | ForEach-Object { $_.Id }) -join ", "
        throw "Grok is still running (PID: $ids). Close all grok windows and try again."
    }
}

function Ensure-Protoc {
    if (Test-Path $script:ProtocExe) {
        return
    }

    Write-Step "protoc not found — downloading 29.3 win64 into tools\protoc"
    $dest = Join-Path $script:GrokRoot "tools\protoc"
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    $zip = Join-Path $dest "protoc-29.3-win64.zip"
    $url = "https://github.com/protocolbuffers/protobuf/releases/download/v29.3/protoc-29.3-win64.zip"
    Invoke-WebRequest -Uri $url -OutFile $zip
    Expand-Archive -Path $zip -DestinationPath $dest -Force
    if (-not (Test-Path $script:ProtocExe)) {
        throw "Downloaded protoc but missing: $script:ProtocExe"
    }
    Write-Ok "protoc installed at $script:ProtocExe"
}

function Get-LatestBackup {
    if (-not (Test-Path $script:BackupDir)) {
        return $null
    }
    Get-ChildItem -Path $script:BackupDir -Filter "grok.exe.bak-*" -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
}
