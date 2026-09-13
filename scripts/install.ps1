# CupraFlow - Windows Install Script
# Usage: irm https://install.cupraflow.dev | iex
# Or local: .\scripts\install.ps1
[CmdletBinding()]
param()

# Compatibilidad Windows Server 2019 / PS 5.1: Forzar TLS 1.2+ y permitir sub-scripts
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls11 -bor [Net.SecurityProtocolType]::Tls
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force -ErrorAction SilentlyContinue

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$SbAgentLabel = "cupraflow"
$libUrl = "https://raw.githubusercontent.com/securyblack/sb-agent-core/master/scripts/install-lib.ps1"
$libTmp = Join-Path ([System.IO.Path]::GetTempPath()) "sb-agent-core-install-lib.ps1"
Invoke-WebRequest -Uri $libUrl -OutFile $libTmp -UseBasicParsing
. $libTmp

# Constants
$GithubRepo  = "securyblack/cupra-flow"
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

Assert-SbAdmin
$target = Get-SbArchTarget
$version = Get-SbLatestVersion -GithubRepo $GithubRepo

$tmpDir = [System.IO.Path]::GetTempPath() + [System.IO.Path]::GetRandomFileName()
New-Item -ItemType Directory -Path $tmpDir | Out-Null

try {
    # CupraFlow empaqueta el binario en una carpeta junto a config/config.toml
    # (ver release.yml con extra-files) — no es un zip de un solo fichero como
    # los demás agentes, así que aquí no se reusa Install-SbBinaryFromZip.
    $assetName = "cupra-flow-$target.zip"
    $zipPath = Get-SbReleaseAsset -GithubRepo $GithubRepo -Version $version -AssetName $assetName -TmpDir $tmpDir

    if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
        Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
        & sc.exe delete $ServiceName | Out-Null
        Start-Sleep -Seconds 1
    }

    Write-SbInfo "Installing binary to $InstallDir..."
    Expand-Archive -Path $zipPath -DestinationPath $tmpDir -Force
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Copy-Item "$tmpDir\$BinaryName" "$InstallDir\$BinaryName" -Force

    # Install default config if not present
    New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
    if (-not (Test-Path $ConfigFile)) {
        Write-SbInfo "Writing default config to $ConfigFile..."
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
github_repo = "securyblack/cupra-flow"
"@ | Set-Content -Path $ConfigFile -Encoding UTF8
        Write-SbSuccess "Config written"
    } else {
        Write-SbInfo "Config already exists, skipping"
    }

    # ─── Windows Service ──────────────────────────────────────────────────────
    # CupraFlow registra su propio servicio vía subcomandos del binario
    # (`cupraflow.exe install` / `start`), no vía New-Service como los demás
    # agentes — es su mecanismo propio, no algo que unificar en la librería.
    Write-SbInfo "Registering Windows Service '$ServiceName'..."
    & "$InstallDir\$BinaryName" install
    if ($LASTEXITCODE -ne 0) { Invoke-SbFail "Service registration failed." }

    & sc.exe failure $ServiceName reset= 86400 actions= restart/10000/restart/30000/restart/60000 | Out-Null
    & sc.exe failureflag $ServiceName 1 | Out-Null

    & "$InstallDir\$BinaryName" start
    if ($LASTEXITCODE -ne 0) { Invoke-SbFail "Service start failed." }

    Write-SbSuccess "Service registered and started"

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
