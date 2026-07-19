# 一键编译 Grok（Windows release）
# 用法: .\scripts\build.ps1
# 详见 BUILD_WINDOWS.md

. (Join-Path $PSScriptRoot "common.ps1")
Set-Location $GrokRoot

Write-Step "Grok one-click build (release)"
Write-Host "Repo: $GrokRoot"

Ensure-Protoc
$env:PROTOC = $ProtocExe
$env:RUSTFLAGS = "-C linker=rust-lld"

Write-Host "PROTOC    = $env:PROTOC"
Write-Host "RUSTFLAGS = $env:RUSTFLAGS"
Write-Host "Target    = xai-grok-pager-bin (release + rust-lld)"

Write-Step "cargo build -p xai-grok-pager-bin --release"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
cargo build -p xai-grok-pager-bin --release
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE"
}
$sw.Stop()

if (-not (Test-Path $CustomExe)) {
    throw "Build finished but binary missing: $CustomExe"
}

Write-Ok ("Built in {0:n1}s" -f $sw.Elapsed.TotalSeconds)
Write-Host "Binary: $CustomExe"
& $CustomExe --version
Write-Host ""
Write-Host "Next: .\scripts\replace.ps1   # install over official grok" -ForegroundColor DarkGray
