# Build script with signing key for BeeftextAI
# Usage: .\scripts\build-tauri.ps1

param(
    [switch]$SkipSign  # Add -SkipSign for unsigned builds
)

$ErrorActionPreference = "Stop"

$KeyFile = "apps/desktop/src-tauri/signing_key.key"
$KeyPassword = "BeeftextAI2026!"

if (-not $SkipSign -and (Test-Path $KeyFile)) {
    # Read key and remove newlines (CRLF/LF)
    $SigningKey = (Get-Content $KeyFile -Raw).Replace("`r`n","").Replace("`n","")
    $env:TAURI_SIGNING_PRIVATE_KEY = $SigningKey
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $KeyPassword
    Write-Host "Signing enabled for Tauri build" -ForegroundColor Green
} elseif ($SkipSign) {
    Write-Host "Skipping signature (unsigned build)" -ForegroundColor Yellow
    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
} else {
    Write-Host "Warning: signing_key.key not found. Build will be unsigned." -ForegroundColor Yellow
}

Write-Host "Building BeeftextAI..."
npm run tauri build

if ($LASTEXITCODE -eq 0) {
    Write-Host "Build completed successfully!" -ForegroundColor Green
} else {
    Write-Host "Build failed with exit code $LASTEXITCODE" -ForegroundColor Red
}
