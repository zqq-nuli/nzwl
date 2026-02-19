param(
    [string]$Timestamp = $(Get-Date -Format "yyyyMMdd-HHmmss")
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

Write-Host "[1/5] Building release binaries..."
cargo build --release --bins

$releaseRoot = Join-Path $root ("dist/release/nzwl-" + $Timestamp)
$binDir = Join-Path $releaseRoot "bin"
$modelsDir = Join-Path $releaseRoot "models"
$strategyDir = Join-Path $releaseRoot "strategies"

Write-Host "[2/5] Preparing release folder: $releaseRoot"
if (Test-Path $releaseRoot) {
    Remove-Item -Recurse -Force $releaseRoot
}
New-Item -ItemType Directory -Force -Path $binDir, $modelsDir, $strategyDir | Out-Null

$binaries = @(
    "nz-rust.exe",
    "ocr-test.exe",
    "map-editor.exe",
    "logitech-test.exe",
    "mouse_test.exe"
)

Write-Host "[3/5] Copying binaries..."
foreach ($exe in $binaries) {
    $src = Join-Path $root ("target/release/" + $exe)
    if (-not (Test-Path $src)) {
        throw "Missing binary: $src"
    }
    Copy-Item -Force $src $binDir
}

# Optional runtime DLL for Logitech backend
$dllCandidates = @(
    (Join-Path $root "IbInputSimulator.dll"),
    (Join-Path $root "tools/IbInputSimulator_Release/IbInputSimulator.AHK2/IbInputSimulator.dll")
)
$dllCopied = $false
foreach ($dll in $dllCandidates) {
    if (Test-Path $dll) {
        Copy-Item -Force $dll (Join-Path $releaseRoot "IbInputSimulator.dll")
        $dllCopied = $true
        break
    }
}
if (-not $dllCopied) {
    Write-Warning "IbInputSimulator.dll not found. Logitech backend may be unavailable in release package."
}

# Required OCR models
$modelFiles = @(
    "ch_PP-OCRv4_det_infer.mnn",
    "ch_PP-OCRv4_rec_infer.mnn",
    "ppocr_keys_v4.txt"
)

foreach ($model in $modelFiles) {
    $src = Join-Path $root ("models/" + $model)
    if (Test-Path $src) {
        Copy-Item -Force $src $modelsDir
    } else {
        Write-Warning "Model missing: $src"
    }
}

# Optional strategy presets
if (Test-Path (Join-Path $root "strategies")) {
    Copy-Item -Recurse -Force (Join-Path $root "strategies/*") $strategyDir
}

# Release quick guide
@"
nzwl release package
====================

Binaries:
- bin/nz-rust.exe (main app)
- bin/ocr-test.exe
- bin/map-editor.exe
- bin/logitech-test.exe
- bin/mouse_test.exe

Resources:
- models/*
- strategies/*
- IbInputSimulator.dll (if found)

Quick start:
1) Run bin/nz-rust.exe
2) Use F1 to start, F2 to stop
3) Use --strategy <json> when launching if needed
"@ | Set-Content -Encoding UTF8 (Join-Path $releaseRoot "README-release.txt")

$zipPath = Join-Path $root ("dist/nzwl-release-" + $Timestamp + ".zip")
if (Test-Path $zipPath) {
    Remove-Item -Force $zipPath
}

Write-Host "[4/5] Creating zip: $zipPath"
Compress-Archive -Path $releaseRoot -DestinationPath $zipPath -CompressionLevel Optimal

Write-Host "[5/5] Done"
Write-Host "Release directory: $releaseRoot"
Write-Host "Release zip: $zipPath"
