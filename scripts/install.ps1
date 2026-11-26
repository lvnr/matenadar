# Installs the prebuilt `matenadar.exe` CLI binary from GitHub Releases.
#
#   powershell -ExecutionPolicy Bypass -c "irm https://github.com/lvnr/matenadar/releases/latest/download/install.ps1 | iex"
#
# Environment variables:
#   MATENADAR_VERSION     Tag or version to install (default: latest, e.g. v0.1.0)
#   MATENADAR_INSTALL_DIR Destination directory (default: $HOME\.local\bin)

$ErrorActionPreference = "Stop"

$Repo = "lvnr/matenadar"
$Target = "x86_64-pc-windows-msvc"
$InstallDir = if ($env:MATENADAR_INSTALL_DIR) { $env:MATENADAR_INSTALL_DIR } else { Join-Path $HOME ".local\bin" }

if ($env:MATENADAR_VERSION) {
  $Tag = if ($env:MATENADAR_VERSION.StartsWith("v")) { $env:MATENADAR_VERSION } else { "v$($env:MATENADAR_VERSION)" }
} else {
  $Resp = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{ "User-Agent" = "matenadar-installer" }
  $Tag = $Resp.tag_name
  if (-not $Tag) { throw "Could not determine latest release tag." }
}

$Archive = "matenadar-$Tag-$Target.zip"
$BaseUrl = "https://github.com/$Repo/releases/download/$Tag"
$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("matenadar-install-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

try {
  $ArchivePath = Join-Path $TempDir $Archive
  $SumsPath = Join-Path $TempDir "SHA256SUMS.txt"

  Write-Host "Downloading matenadar $Tag ($Target)..."
  Invoke-WebRequest -Uri "$BaseUrl/$Archive" -OutFile $ArchivePath
  Invoke-WebRequest -Uri "$BaseUrl/SHA256SUMS.txt" -OutFile $SumsPath

  $ExpectedLine = Get-Content $SumsPath | Where-Object { $_ -match "\s+\*?$([regex]::Escape($Archive))$" } | Select-Object -First 1
  if (-not $ExpectedLine) { throw "Checksum for $Archive not found in SHA256SUMS.txt." }
  $Expected = ($ExpectedLine -split "\s+")[0].ToLowerInvariant()
  $Actual = (Get-FileHash -Path $ArchivePath -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($Actual -ne $Expected) {
    throw "Checksum mismatch for $Archive (expected $Expected, got $Actual)."
  }

  $ExtractDir = Join-Path $TempDir "extract"
  Expand-Archive -Path $ArchivePath -DestinationPath $ExtractDir -Force

  New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
  $DestBin = Join-Path $InstallDir "matenadar.exe"
  Copy-Item -Path (Join-Path $ExtractDir "matenadar.exe") -Destination $DestBin -Force

  $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
  $PathEntries = if ($UserPath) { $UserPath -split ";" } else { @() }
  if ($PathEntries -notcontains $InstallDir) {
    $NewPath = if ($UserPath) { "$UserPath;$InstallDir" } else { $InstallDir }
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
    Write-Host "Added $InstallDir to your User PATH. Restart your terminal to use matenadar."
  }

  $VersionOut = & $DestBin --version
  Write-Host "Installed $VersionOut to $DestBin"
} finally {
  Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
}
