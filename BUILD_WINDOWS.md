# Windows build notes (local fork)

## Prerequisites

1. Rust toolchain (repo pins via `rust-toolchain.toml`)
2. `protoc` 29.x — this tree’s `bin/protoc` is a DotSlash wrapper **without a Windows platform entry**. Use a real binary:

   ```powershell
   # Already vendored after first setup:
   $env:PROTOC = "D:\Projects\grok-build\tools\protoc\bin\protoc.exe"
   ```

3. Visual Studio Build Tools (MSVC) for compiling native deps

## Build

MSVC `link.exe` hits **LNK1318 (PDB size limit)** on this binary. Use the bundled LLVM linker:

```powershell
$env:PROTOC = "D:\Projects\grok-build\tools\protoc\bin\protoc.exe"
$env:RUSTFLAGS = "-C linker=rust-lld"
cargo build -p xai-grok-pager-bin --release
```

Output:

- `target\release\xai-grok-pager.exe` (crate name)
- Optionally copy to `grok.exe` and put on `PATH` ahead of the official install

## Transparent background

```toml
# ~/.grok/config.toml
[ui]
transparent_bg = true
```

Or for one launch:

```powershell
$env:GROK_TRANSPARENT_BG = "1"
.\target\release\xai-grok-pager.exe
```
