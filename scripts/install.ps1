#!/usr/bin/env pwsh
# cdx install script — Windows / PowerShell
# Usage: irm https://raw.githubusercontent.com/Sovengar/cdx/main/scripts/install.ps1 | iex
# Or:    ./scripts/install.ps1

$Repo = "Sovengar/cdx"
$BinDir = "$env:USERPROFILE\.local\bin"
$ConfigDir = "$env:USERPROFILE\.config\cdx"
$BinName = "cdx.exe"

# Ensure ~/.local/bin exists
if (-not (Test-Path -LiteralPath $BinDir)) {
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
}

# Check for Rust
$HasRust = Get-Command rustc -ErrorAction SilentlyContinue

if (-not $HasRust) {
    Write-Host "[cdx] Rust not found. Installing via rustup..." -ForegroundColor Yellow
    # Download and run rustup
    $RustupUrl = "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe"
    $RustupPath = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri $RustupUrl -OutFile $RustupPath
    & $RustupPath -y
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","User")
}

# Clone or update repo
$ProjectDir = "$env:TEMP\cdx-rs"
if (Test-Path -LiteralPath $ProjectDir) {
    Push-Location $ProjectDir
    git pull --rebase
    Pop-Location
} else {
    git clone "https://github.com/$Repo.git" $ProjectDir
}

# Build
Push-Location $ProjectDir
Write-Host "[cdx] Building release..." -ForegroundColor Cyan
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "[cdx] Build failed" -ForegroundColor Red
    Pop-Location
    exit 1
}

# Install binary
Copy-Item -LiteralPath "target\release\cdx-rs.exe" -Destination "$BinDir\$BinName" -Force
Pop-Location

# Create default config if not exists
if (-not (Test-Path -LiteralPath "$ConfigDir\config.toml")) {
    New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
    & "$BinDir\$BinName" --version 2>$null  # triggers config creation
    if (-not (Test-Path -LiteralPath "$ConfigDir\config.toml")) {
        # Manual fallback
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
    Write-Host "[cdx] Added $BinDir to user PATH" -ForegroundColor Yellow
    $env:Path += ";$BinDir"
}

Write-Host "[cdx] Installed! Run 'cdx' to start." -ForegroundColor Green
