#!/usr/bin/env pwsh
# cdx install script — Windows / PowerShell
# Usage: irm https://raw.githubusercontent.com/Sovengar/cdx/master/scripts/install.ps1 | iex
# Or:    ./scripts/install.ps1

$ErrorActionPreference = "Stop"
$Repo = "Sovengar/cdx"
$BinDir = "$env:USERPROFILE\.local\bin"
$ConfigDir = "$env:USERPROFILE\.config\cdx"
$BinName = "cdx.exe"

Write-Host "[cdx] Installing..." -ForegroundColor Cyan

# Ensure directories exist
if (-not (Test-Path -LiteralPath $BinDir)) {
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
}
if (-not (Test-Path -LiteralPath $ConfigDir)) {
    New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
}

# Check for Rust
$HasRust = Get-Command rustc -ErrorAction SilentlyContinue

if (-not $HasRust) {
    Write-Host "[cdx] Rust not found. Installing via rustup..." -ForegroundColor Yellow
    $RustupUrl = "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe"
    $RustupPath = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri $RustupUrl -OutFile $RustupPath
    & $RustupPath -y
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "User")
}

# Clone repo to temp
$ProjectDir = "$env:TEMP\cdx-install"
if (Test-Path -LiteralPath $ProjectDir) {
    Remove-Item -LiteralPath $ProjectDir -Recurse -Force
}

Write-Host "[cdx] Cloning repository..." -ForegroundColor Cyan
git clone --depth 1 "https://github.com/$Repo.git" $ProjectDir

# Build
Push-Location $ProjectDir
Write-Host "[cdx] Building release binary..." -ForegroundColor Cyan
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "[cdx] Build failed" -ForegroundColor Red
    Pop-Location
    exit 1
}

# Install binary
Copy-Item -LiteralPath "target\release\$BinName" -Destination "$BinDir\$BinName" -Force
Pop-Location

# Cleanup temp dir
Remove-Item -LiteralPath $ProjectDir -Recurse -Force -ErrorAction SilentlyContinue

# Create default config if not exists
if (-not (Test-Path -LiteralPath "$ConfigDir\config.toml")) {
    & "$BinDir\$BinName" --version 2>$null  # triggers config creation
    if (-not (Test-Path -LiteralPath "$ConfigDir\config.toml")) {
        @'
# cdx config — see https://github.com/Sovengar/cdx
show_dotfiles = false
show_winhidden = false
'@ | Out-File -FilePath "$ConfigDir\config.toml" -Encoding utf8
    }
}

# Add to PATH if not present
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$BinDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$BinDir", "User")
    $env:Path += ";$BinDir"
    Write-Host "[cdx] Added $BinDir to user PATH" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "[cdx] Installed! Run 'cdx' to start." -ForegroundColor Green
