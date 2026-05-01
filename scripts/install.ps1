# CupraFlow - Windows Install Script
# Usage: irm https://install.cupraflow.dev | iex
# Or local: .\scripts\install.ps1
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Helpers
function Write-Info    { param($msg) Write-Host "[cupraflow] $msg" -ForegroundColor Cyan }
function Write-Success { param($msg) Write-Host "[cupraflow] $msg" -ForegroundColor Green }
function Write-Warn    { param($msg) Write-Host "[cupraflow] $msg" -ForegroundColor Yellow }
function Fail          { param($msg) Write-Host "[cupraflow] ERROR: $msg" -ForegroundColor Red; exit 1 }

# Constants
$GithubRepo  = "sb-mcampoe/cupraflow"
$BinaryName  = "cupraflow.exe"
$InstallDir  = "$env:ProgramFiles\CupraFlow"
$ConfigDir   = "$env:ProgramData\CupraFlow"
$ConfigFile  = "$ConfigDir\config.toml"
$ServiceName = "CupraFlow"

# Banner
Write-Host ""
Write-Host "  CupraFlow - Load Balancer Agent" -ForegroundColor Cyan -NoNewline
Write-Host " (Windows Installer)" -ForegroundColor Gray
Write-Host ""

# Admin check
$currentPrincipal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Fail "This script must be run as Administrator. Right-click PowerShell and select 'Run as Administrator'."
}

# Architecture detection
$procArch = $env:PROCESSOR_ARCHITECTURE
$target = switch ($procArch) {
    "AMD64" { "x86_64-pc-windows-msvc" }
    "ARM64" { "aarch64-pc-windows-msvc" }
    default { Fail "Unsupported architecture: $procArch" }
}

Write-Info "Detected architecture: $procArch ($target)"

# Resolve latest release version
Write-Info "Fetching latest release from GitHub..."
$releaseApi  = "https://api.github.com/repos/$GithubRepo/releases/latest"
try {
    $releaseInfo = Invoke-RestMethod -Uri $releaseApi -Headers @{ "User-Agent" = "cupraflow-installer" }
    $version     = $releaseInfo.tag_name
} catch {
    Fail "Could not reach GitHub API. Check your internet connection."
}

if (-not $version) { Fail "Could not determine latest version." }

Write-Info "Latest version: $version"

# Download binary
$assetName   = "cupraflow-$target.zip"
$downloadUrl = "https://github.com/$GithubRepo/releases/download/$version/$assetName"
$checksumUrl = "$downloadUrl.sha256"
$tmpDir      = [System.IO.Path]::GetTempPath() + [System.IO.Path]::GetRandomFileName()
New-Item -ItemType Directory -Path $tmpDir | Out-Null

try {
    Write-Info "Downloading $assetName..."
    $zipPath = "$tmpDir\$assetName"
    Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -UseBasicParsing

    # Verify checksum if available
    try {
        $checksumFile = "$tmpDir\$assetName.sha256"
        Invoke-WebRequest -Uri $checksumUrl -OutFile $checksumFile -UseBasicParsing
        $expected = (Get-Content $checksumFile).Split(" ")[0].Trim().ToLower()
        $actual   = (Get-FileHash -Algorithm SHA256 $zipPath).Hash.ToLower()
        if ($expected -ne $actual) { Fail "Checksum mismatch. Download may be corrupted." }
        Write-Success "Checksum OK"
    } catch {
        Write-Warn "No checksum file found, skipping verification"
    }

    # Install binary
    Write-Info "Installing binary to $InstallDir..."
    Expand-Archive -Path $zipPath -DestinationPath $tmpDir -Force
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Copy-Item "$tmpDir\$BinaryName" "$InstallDir\$BinaryName" -Force

    # Install default config if not present
    New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
    if (-not (Test-Path $ConfigFile)) {
        Write-Info "Writing default config to $ConfigFile..."
        @"
[server]
port = 8080
bind_address = "0.0.0.0"

[logging]
level = "info"
format = "pretty"

[service]
name = "CupraFlow"
description = "Agente de gestion de red y balanceo de carga"
startup = "auto"

[loadbalancer]
enabled = false
algorithm = "round_robin"
health_check_interval = 30
backends = []

[update]
channel = "stable"
check_on_startup = true
check_interval = 24
github_repo = "sb-mcampoe/cupraflow"
"@ | Set-Content -Path $ConfigFile -Encoding UTF8
        Write-Success "Config written"
    } else {
        Write-Info "Config already exists, skipping"
    }

    # Windows Service
    Write-Info "Registering Windows Service '$ServiceName'..."

    # Remove existing service if present
    if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
        Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
        & sc.exe delete $ServiceName | Out-Null
        Start-Sleep -Seconds 1
    }

    # Use the binary's built-in install command
    & "$InstallDir\$BinaryName" install
    if ($LASTEXITCODE -ne 0) { Fail "Service registration failed." }

    # Configure restart on failure
    & sc.exe failure $ServiceName reset= 86400 actions= restart/10000/restart/30000/restart/60000 | Out-Null
    & sc.exe failureflag $ServiceName 1 | Out-Null

    # Start service
    & "$InstallDir\$BinaryName" start
    if ($LASTEXITCODE -ne 0) { Fail "Service start failed." }

    Write-Success "Service registered and started"

} finally {
    Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
}

# Done
Write-Host ""
Write-Host "  CupraFlow $version installed successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "  Status:  " -NoNewline; Write-Host "Get-Service CupraFlow" -ForegroundColor White
Write-Host "  Logs:    " -NoNewline; Write-Host "$ConfigDir\cupraflow.log.*" -ForegroundColor White
Write-Host "  Config:  " -NoNewline; Write-Host $ConfigFile -ForegroundColor White
Write-Host "  Binary:  " -NoNewline; Write-Host "$InstallDir\$BinaryName" -ForegroundColor White
Write-Host ""
