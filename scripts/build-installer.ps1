param(
    [ValidateSet("Release", "Debug")]
    [string]$Configuration = "Release",
    [string]$Version = "0.1.0",
    [switch]$SkipProductBuild
)

$ErrorActionPreference = "Stop"

if ($Version -notmatch '^\d+\.\d+\.\d+(\.\d+)?$') {
    throw "Version must be numeric SemVer-like text such as 0.1.0 or 0.1.0.1."
}

$AssemblyVersion = if ($Version.Split('.').Count -eq 3) { "$Version.0" } else { $Version }

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$ProductBuildScript = Join-Path $RepoRoot "scripts\build-product.ps1"
$ProductDir = Join-Path $RepoRoot "artifacts\TaskbarStats"
$InstallerProject = Join-Path $RepoRoot "product\TaskbarStatsInstaller\TaskbarStatsInstaller.csproj"
$InstallerResourceDir = Join-Path $RepoRoot "product\TaskbarStatsInstaller\Resources"
$PackageZip = Join-Path $InstallerResourceDir "TaskbarStatsPackage.zip"
$InstallerArtifactDir = Join-Path $RepoRoot "artifacts"
$InstallerOutput = Join-Path $InstallerArtifactDir "TaskbarStatsSetup.exe"
$InstallerSha = Join-Path $InstallerArtifactDir "TaskbarStatsSetup.exe.sha256"
$StagingRoot = Join-Path $RepoRoot "artifacts\installer"
$PackageRoot = Join-Path $StagingRoot "package"
$InstallerPublishDir = Join-Path $StagingRoot "publish"

if (-not $SkipProductBuild) {
    powershell -ExecutionPolicy Bypass -File $ProductBuildScript -Configuration $Configuration -Version $Version
}

foreach ($ProcessName in @("TaskbarStats", "TaskbarStatsMediaHelper", "TaskbarStatsSettings")) {
    Get-Process $ProcessName -ErrorAction SilentlyContinue |
        Stop-Process -Force -ErrorAction SilentlyContinue
}

if (-not (Test-Path (Join-Path $ProductDir "TaskbarStats.exe"))) {
    throw "Product artifact was not found. Run scripts\build-product.ps1 first."
}

Remove-Item -Recurse -Force $StagingRoot -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $PackageRoot | Out-Null
New-Item -ItemType Directory -Force $InstallerResourceDir | Out-Null

$RequiredFiles = @(
    "TaskbarStats.exe",
    "TaskbarStats.exe.sha256",
    "TaskbarStatsSettings.exe",
    "TaskbarStatsMediaHelper.exe"
)

foreach ($FileName in $RequiredFiles) {
    $Source = Join-Path $ProductDir $FileName
    if (-not (Test-Path $Source)) {
        throw "Required product file is missing: $Source"
    }

    Copy-Item -Force $Source (Join-Path $PackageRoot $FileName)
}

$AssetsSource = Join-Path $RepoRoot "assets"
if (Test-Path $AssetsSource) {
    Copy-Item -Path $AssetsSource -Destination (Join-Path $PackageRoot "Assets") -Recurse -Force
}

$WidgetLibraries = Join-Path $PackageRoot "WidgetLibraries"
New-Item -ItemType Directory -Force $WidgetLibraries | Out-Null
Set-Content `
    -Path (Join-Path $WidgetLibraries "README.txt") `
    -Value "TaskbarStats widget design packs." `
    -Encoding ASCII

Remove-Item -Force $PackageZip -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $PackageRoot "*") -DestinationPath $PackageZip -CompressionLevel Optimal

dotnet publish $InstallerProject `
    -c $Configuration `
    -r win-x64 `
    --self-contained true `
    -p:Version=$Version `
    -p:AssemblyVersion=$AssemblyVersion `
    -p:FileVersion=$AssemblyVersion `
    -p:PublishSingleFile=true `
    -p:EnableCompressionInSingleFile=true `
    -o $InstallerPublishDir
if ($LASTEXITCODE -ne 0) {
    throw "Installer publish failed with exit code $LASTEXITCODE."
}

$PublishedInstaller = Join-Path $InstallerPublishDir "TaskbarStatsSetup.exe"
if (-not (Test-Path $PublishedInstaller)) {
    throw "Installer output was not found: $PublishedInstaller"
}

Copy-Item -Force $PublishedInstaller $InstallerOutput
$Hash = (Get-FileHash $InstallerOutput -Algorithm SHA256).Hash.ToLowerInvariant()
Set-Content -Path $InstallerSha -Value "$Hash  TaskbarStatsSetup.exe" -Encoding ASCII

Write-Host "Installer output:"
Get-Item $InstallerOutput, $InstallerSha | Select-Object FullName, Length, LastWriteTime
