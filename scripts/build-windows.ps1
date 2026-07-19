# Build Grok Build release binary on Windows (see BUILD_WINDOWS.md)
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $Root

$Protoc = Join-Path $Root "tools\protoc\bin\protoc.exe"
if (-not (Test-Path $Protoc)) {
    Write-Error "protoc not found at $Protoc. Download protoc-29.3-win64 into tools\protoc (see BUILD_WINDOWS.md §1.3)."
}

$env:PROTOC = $Protoc
$env:RUSTFLAGS = "-C linker=rust-lld"
Write-Host "PROTOC=$env:PROTOC"
Write-Host "RUSTFLAGS=$env:RUSTFLAGS"
Write-Host "Building xai-grok-pager-bin (release)..."
cargo build -p xai-grok-pager-bin --release
$out = Join-Path $Root "target\release\xai-grok-pager.exe"
Write-Host "OK: $out"
& $out --version
