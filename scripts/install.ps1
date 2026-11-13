# CupraFlow — Windows Install Script
# Usage: irm https://install.cupraflow.dev | iex -Token <TOKEN>
[CmdletBinding()]
param(
    [string]$Token = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$SbAgentLabel = "cupraflow"
$libUrl = "https://raw.githubusercontent.com/securyblack/sb-agent-core/master/scripts/install-lib.ps1"
$libTmp = Join-Path ([System.IO.Path]::GetTempPath()) "sb-agent-core-install-lib.ps1"
Invoke-WebRequest -Uri $libUrl -OutFile $libTmp -UseBasicParsing
. $libTmp

# ─── Constants ────────────────────────────────────────────────────────────────
$GithubRepo  = "securyblack/cupra-flow"
$BinaryName  = "cupraflow.exe"
$InstallDir  = "$env:ProgramFiles\CupraFlow"
$ConfigDir   = "$env:ProgramData\cupraflow"
$ConfigFile  = "$ConfigDir\config.toml"
$ServiceName = "CupraFlow"

Write-Host ""
Write-Host "  CupraFlow — Traffic Agent" -ForegroundColor Cyan -NoNewline
Write-Host " (Windows Installer)" -ForegroundColor Gray
Write-Host ""

Assert-SbAdmin
$target = Get-SbArchTarget
$version = Get-SbLatestVersion -GithubRepo $GithubRepo

$tmpDir = [System.IO.Path]::GetTempPath() + [System.IO.Path]::GetRandomFileName()
New-Item -ItemType Directory -Path $tmpDir | Out-Null

try {
    $assetName = "cupraflow-$target.zip"
    $zipPath = Get-SbReleaseAsset -GithubRepo $GithubRepo -Version $version -AssetName $assetName -TmpDir $tmpDir
    Install-SbBinaryFromZip -ZipPath $zipPath -BinaryName $BinaryName -InstallDir $InstallDir -ServiceName $ServiceName

    New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null

    if (-not $Token) {
        $secToken = Read-Host "  Auth token" -AsSecureString
        $Token = [Runtime.InteropServices.Marshal]::PtrToStringAuto(
                    [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secToken))
    }
    if (-not $Token) { Invoke-SbFail "Token cannot be empty" }

    Write-SbInfo "Writing config to $ConfigFile..."
    @"
# CupraFlow configuration
# Do not share this file — it contains your auth token.
version = "$version"
token = "$Token"
"@ | Set-Content -Path $ConfigFile -Encoding UTF8

    Protect-SbConfigFile -Path $ConfigFile
    Write-SbSuccess "Config written"

    Register-SbWindowsService -ServiceName $ServiceName -DisplayName "CupraFlow Traffic Agent" `
        -BinaryPath "$InstallDir\$BinaryName" `
        -Description "SecuryBlack CupraFlow traffic agent. See https://github.com/$GithubRepo"

} finally {
    Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "  CupraFlow $version installed successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "  Status:  " -NoNewline; Write-Host "Get-Service CupraFlow" -ForegroundColor White
Write-Host "  Config:  " -NoNewline; Write-Host $ConfigFile -ForegroundColor White
Write-Host ""
