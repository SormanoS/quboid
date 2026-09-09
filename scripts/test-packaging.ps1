param(
    [Parameter()]
    [string]$Version = "0.1.0",

    [Parameter()]
    [switch]$InstallSmoke
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Dist = Join-Path $Root "dist"
$Zip = Join-Path $Dist "Cuboid-$Version-windows-x64-portable.zip"
$Msi = Join-Path $Dist "Cuboid-$Version-windows-x64.msi"
$Checksums = Join-Path $Dist "SHA256SUMS.txt"
$Workspace = Join-Path ([System.IO.Path]::GetTempPath()) "cuboid-package-test-$PID"
$InstalledExecutable = Join-Path $env:LOCALAPPDATA "Cuboid\cuboid-app.exe"
$InstalledByTest = $false

foreach ($Path in $Zip, $Msi, $Checksums) {
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Release artifact is missing: $Path"
    }
}

foreach ($Line in Get-Content -LiteralPath $Checksums) {
    $Parts = $Line -split "\s+", 2
    $Artifact = Join-Path $Dist $Parts[1]
    $Actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $Artifact).Hash.ToLower()
    if ($Actual -ne $Parts[0]) {
        throw "Checksum mismatch for $($Parts[1])."
    }
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$Archive = [System.IO.Compression.ZipFile]::OpenRead($Zip)
try {
    $Entries = $Archive.Entries.FullName
    foreach ($Required in "cuboid-app.exe", "README.md", "LICENSE", "portable.flag") {
        if ($Entries -notcontains $Required) {
            throw "Portable archive is missing $Required."
        }
    }
}
finally {
    $Archive.Dispose()
}

$Installer = New-Object -ComObject WindowsInstaller.Installer
$Database = $Installer.OpenDatabase($Msi, 0)
function Get-MsiProperty {
    param([string]$Name)

    $View = $Database.OpenView(
        "SELECT ``Value`` FROM ``Property`` WHERE ``Property``='$Name'"
    )
    $View.Execute()
    $Record = $View.Fetch()
    return $Record.StringData(1)
}

if ((Get-MsiProperty "ProductName") -ne "Cuboid") {
    throw "MSI ProductName is invalid."
}
if ((Get-MsiProperty "ProductVersion") -ne $Version) {
    throw "MSI ProductVersion is invalid."
}

New-Item -ItemType Directory -Path $Workspace -Force | Out-Null
try {
    [System.IO.Compression.ZipFile]::ExtractToDirectory($Zip, $Workspace)
    $PortableExecutable = Join-Path $Workspace "cuboid-app.exe"
    if (-not (Test-Path -LiteralPath $PortableExecutable)) {
        throw "Portable executable could not be extracted."
    }

    $Mt = Get-ChildItem -LiteralPath "${env:ProgramFiles(x86)}\Windows Kits\10\bin" `
        -Recurse `
        -File `
        -Filter "mt.exe" `
        -ErrorAction SilentlyContinue |
        Where-Object { $_.DirectoryName -like "*\x64" } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if (-not $Mt) {
        throw "Windows SDK mt.exe was not found."
    }
    $Manifest = Join-Path $Workspace "embedded.manifest"
    & $Mt.FullName -nologo "-inputresource:$PortableExecutable;#1" "-out:$Manifest"
    if ($LASTEXITCODE -ne 0) {
        throw "The executable has no readable embedded manifest."
    }
    $ManifestText = Get-Content -LiteralPath $Manifest -Raw
    if ($ManifestText -notmatch "PerMonitorV2") {
        throw "The executable manifest is not PerMonitorV2 aware."
    }
    if ($ManifestText -notmatch 'requestedExecutionLevel level="asInvoker"') {
        throw "The executable manifest does not use asInvoker."
    }

    if ($InstallSmoke) {
        if (Test-Path -LiteralPath $InstalledExecutable) {
            throw "Cuboid is already installed; refusing to replace an existing installation."
        }
        $Install = Start-Process -FilePath "msiexec.exe" `
            -ArgumentList @("/i", $Msi, "/qn", "/norestart") `
            -Wait `
            -PassThru
        if ($Install.ExitCode -ne 0) {
            throw "MSI installation failed with exit code $($Install.ExitCode)."
        }
        $InstalledByTest = $true
        if (-not (Test-Path -LiteralPath $InstalledExecutable)) {
            throw "MSI did not install the Cuboid executable."
        }
    }
}
finally {
    if ($InstalledByTest) {
        $Uninstall = Start-Process -FilePath "msiexec.exe" `
            -ArgumentList @("/x", $Msi, "/qn", "/norestart") `
            -Wait `
            -PassThru
        if ($Uninstall.ExitCode -ne 0) {
            Write-Error "MSI uninstall failed with exit code $($Uninstall.ExitCode)."
        }
    }
    if (Test-Path -LiteralPath $Workspace) {
        Remove-Item -LiteralPath $Workspace -Recurse -Force
    }
}

Write-Output "Cuboid packaging tests passed."
