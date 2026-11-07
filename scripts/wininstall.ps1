# Yukino installer for Windows.
#
# Install from a GitHub release:
#   wininstall.ps1
# Install a specific version:
#   wininstall.ps1 -Version v0.1.0
# Install from a local bundle produced by scripts/winrelease.py:
#   wininstall.ps1 -ZipPath yukino-0.1.0-x86_64-windows.zip

param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\yukino",
    [string]$ZipPath = ""
)

$ErrorActionPreference = "Stop"
$Repo = "yukitsuri/yukino"

function Fail {
    param([string]$Message)
    Write-Host "error: $Message" -ForegroundColor Red
    exit 1
}

if (-not $ZipPath) {
    if ($Version -eq "latest") {
        $releaseUrl = "https://api.github.com/repos/$Repo/releases/latest"
    }
    else {
        $releaseUrl = "https://api.github.com/repos/$Repo/releases/tags/$Version"
    }
    $release = Invoke-RestMethod -Uri $releaseUrl
    $asset = $release.assets | Where-Object { $_.name -match "^yukino-.*-x86_64-windows\.zip$" } | Select-Object -First 1
    if (-not $asset) {
        Fail "no Windows bundle found in release: $Version"
    }
    $ZipPath = Join-Path $env:TEMP $asset.name
    Write-Host "downloading $($asset.name)..."
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $ZipPath
}

if (-not (Test-Path $ZipPath)) {
    Fail "bundle not found: $ZipPath"
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Expand-Archive -Path $ZipPath -DestinationPath $InstallDir -Force

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$parts = @($userPath -split ";" | Where-Object { $_ })
if ($parts -notcontains $InstallDir) {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$userPath", "User")
    Write-Host "added $InstallDir to the user PATH (shell restart needed)"
}

$yukinoExe = Join-Path $InstallDir "yukino.exe"
if (-not (Test-Path $yukinoExe)) {
    Fail "yukino.exe is missing in the bundle"
}
& $yukinoExe -v
if ($LASTEXITCODE -ne 0) {
    Fail "installed yukino.exe failed to run"
}

Write-Host "installed Yukino to $InstallDir"
