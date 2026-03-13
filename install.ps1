$ErrorActionPreference = "Stop"

$repo = "jimmyalcala/ftp_downloader"
$binary = "ftp_downloader.exe"
$asset = "ftp_downloader-windows-x86_64.zip"

Write-Host "[INFO] Fetching latest release..." -ForegroundColor Green

$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest"
$tag = $release.tag_name

if (-not $tag) {
    Write-Host "[ERROR] Could not determine latest release." -ForegroundColor Red
    exit 1
}

Write-Host "[INFO] Latest version: $tag" -ForegroundColor Green

$url = "https://github.com/$repo/releases/download/$tag/$asset"
$tmpDir = Join-Path $env:TEMP "ftp_downloader_install"
$zipPath = Join-Path $tmpDir $asset

# Create temp directory
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null

# Download
Write-Host "[INFO] Downloading $asset..." -ForegroundColor Green
Invoke-WebRequest -Uri $url -OutFile $zipPath

# Extract
Write-Host "[INFO] Extracting..." -ForegroundColor Green
Expand-Archive -Path $zipPath -DestinationPath $tmpDir -Force

# Install directory
$installDir = Join-Path $env:LOCALAPPDATA "ftp_downloader"
New-Item -ItemType Directory -Force -Path $installDir | Out-Null

# Copy binary
Copy-Item -Path (Join-Path $tmpDir $binary) -Destination $installDir -Force

# Copy example config
$exampleConfig = Join-Path $tmpDir "config.toml.example"
if (Test-Path $exampleConfig) {
    Copy-Item -Path $exampleConfig -Destination $installDir -Force
}

# Add to PATH if not already there
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    Write-Host "[INFO] Adding to PATH..." -ForegroundColor Green
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
    $env:Path = "$env:Path;$installDir"
}

# Cleanup
Remove-Item -Recurse -Force $tmpDir

Write-Host ""
Write-Host "[INFO] Successfully installed ftp_downloader $tag" -ForegroundColor Green
Write-Host "[INFO] Installed to: $installDir" -ForegroundColor Green
Write-Host ""
Write-Host "Usage:" -ForegroundColor Yellow
Write-Host "  ftp_downloader                  # Run with config.toml in current directory"
Write-Host "  ftp_downloader my_config.toml   # Run with custom config"
Write-Host "  ftp_downloader --nogui          # Run without TUI"
Write-Host ""
Write-Host "NOTE: Restart your terminal for PATH changes to take effect." -ForegroundColor Yellow
