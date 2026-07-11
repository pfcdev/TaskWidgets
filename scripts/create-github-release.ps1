param(
    [string]$Tag = "v0.1.0",
    [string]$Repo = "pfcdev/TaskWidgets",
    [switch]$Draft,
    [switch]$Prerelease
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$BuildScript = Join-Path $RepoRoot "scripts\build-product.ps1"
$ArtifactDir = Join-Path $RepoRoot "artifacts\TaskbarStats"
$Exe = Join-Path $ArtifactDir "TaskbarStats.exe"
$Sha = Join-Path $ArtifactDir "TaskbarStats.exe.sha256"

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    throw "GitHub CLI (gh) was not found in PATH. Install gh or run this script from a shell where gh is available."
}

powershell -ExecutionPolicy Bypass -File $BuildScript -Configuration Release

if (-not (Test-Path $Exe)) {
    throw "Expected artifact not found: $Exe"
}

$Hash = (Get-FileHash $Exe -Algorithm SHA256).Hash.ToLowerInvariant()
Set-Content -Path $Sha -Value "$Hash  TaskbarStats.exe" -Encoding ASCII

$ReleaseExists = $false
try {
    gh release view $Tag --repo $Repo *> $null
    $ReleaseExists = $true
} catch {
    $ReleaseExists = $false
}

if ($ReleaseExists) {
    gh release upload $Tag $Exe $Sha --repo $Repo --clobber
} else {
    $Args = @(
        "release", "create", $Tag,
        $Exe, $Sha,
        "--repo", $Repo,
        "--title", "TaskbarStats $Tag",
        "--notes", "TaskbarStats product build."
    )
    if ($Draft) {
        $Args += "--draft"
    }
    if ($Prerelease) {
        $Args += "--prerelease"
    }

    gh @Args
}

Write-Host "Release asset uploaded: $Repo $Tag"
