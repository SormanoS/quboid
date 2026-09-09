param(
    [Parameter()]
    [string]$Version = "0.1.0",

    [Parameter()]
    [string]$WixPath = "",

    [Parameter()]
    [switch]$SkipPackaging
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot

Push-Location $Root
try {
    cargo fmt --all -- --check
    cargo test --workspace -- --test-threads=1
    cargo clippy --workspace --all-targets -- -D warnings
    cargo build --locked --release --target x86_64-pc-windows-msvc

    & (Join-Path $PSScriptRoot "test-windows-e2e.ps1")
    if (-not $SkipPackaging) {
        $PackageArguments = @{
            Version = $Version
            SkipBuild = $true
        }
        if ($WixPath) {
            $PackageArguments.WixPath = $WixPath
        }
        & (Join-Path $PSScriptRoot "package.ps1") @PackageArguments
        & (Join-Path $PSScriptRoot "test-packaging.ps1") `
            -Version $Version `
            -InstallSmoke
    }
}
finally {
    Pop-Location
}

Write-Output "All Cuboid tests passed."
