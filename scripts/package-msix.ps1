param(
    [Parameter()]
    [string]$Version = "",

    [Parameter()]
    [string]$IdentityName = "",

    [Parameter()]
    [string]$Publisher = "",

    [Parameter()]
    [switch]$SkipBuild,

    # Leaves the staged layout registered with Windows instead of producing an
    # .msix. A registered layout runs with real package identity, which is what
    # a behaviour test needs, and it requires neither a certificate nor an
    # administrator.
    [Parameter()]
    [switch]$Register
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Target = "x86_64-pc-windows-msvc"
$Source = Join-Path $Root "packaging\msix"
$Dist = Join-Path $Root "dist"
$Layout = Join-Path $Dist "msix"
$Executable = Join-Path $Root "target\$Target\release\quboid-app.exe"

Push-Location $Root
try {
    if (-not $Version) {
        $Manifest = Get-Content (Join-Path $Root "Cargo.toml") -Raw
        if ($Manifest -notmatch '(?m)^version = "(?<version>[^"]+)"') {
            throw "No version found under [workspace.package] in Cargo.toml."
        }
        $Version = $Matches.version
    }

    # MSIX versions are always four parts and the revision must be zero for
    # packages submitted to the Store.
    $Parts = $Version.Split(".")
    if ($Parts.Count -lt 3) {
        throw "Version $Version is not major.minor.patch."
    }
    $PackageVersion = "$($Parts[0]).$($Parts[1]).$($Parts[2]).0"

    if (-not $SkipBuild) {
        cargo build --locked --release --target $Target
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed."
        }
    }
    if (-not (Test-Path -LiteralPath $Executable)) {
        throw "Release executable not found: $Executable"
    }

    if (Test-Path -LiteralPath $Layout) {
        Remove-Item -LiteralPath $Layout -Recurse -Force
    }
    New-Item -ItemType Directory -Path $Layout -Force | Out-Null
    Copy-Item -LiteralPath $Executable -Destination $Layout
    Copy-Item -LiteralPath (Join-Path $Root "LICENSE") -Destination $Layout
    Copy-Item -LiteralPath (Join-Path $Source "assets") -Destination $Layout -Recurse

    [xml]$Appx = Get-Content -LiteralPath (Join-Path $Source "AppxManifest.xml")
    $Appx.Package.Identity.Version = $PackageVersion
    if ($IdentityName) {
        $Appx.Package.Identity.Name = $IdentityName
    }
    if ($Publisher) {
        $Appx.Package.Identity.Publisher = $Publisher
    }
    $Appx.Save((Join-Path $Layout "AppxManifest.xml"))

    if ($Register) {
        # Developer mode has to be on; this is a sideload of loose files.
        Add-AppxPackage -Register (Join-Path $Layout "AppxManifest.xml")
        Write-Output "Registered $($Appx.Package.Identity.Name) $PackageVersion from $Layout."
        return
    }

    $MakeAppx = Get-ChildItem -LiteralPath "${env:ProgramFiles(x86)}\Windows Kits\10\bin" `
        -Recurse `
        -File `
        -Filter "makeappx.exe" `
        -ErrorAction SilentlyContinue |
        Where-Object { $_.DirectoryName -like "*\x64" } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if (-not $MakeAppx) {
        throw "Windows SDK makeappx.exe was not found."
    }

    $Package = Join-Path $Dist "Quboid-$Version-windows-x64.msix"
    if (Test-Path -LiteralPath $Package) {
        Remove-Item -LiteralPath $Package -Force
    }
    & $MakeAppx.FullName pack /o /d $Layout /p $Package
    if ($LASTEXITCODE -ne 0) {
        throw "MSIX packaging failed."
    }

    # The Store signs submitted packages, so this one is deliberately left
    # unsigned. Signing is only needed to install it outside the Store.
    Write-Output "Wrote $Package"
}
finally {
    Pop-Location
}
