param(
    [Parameter()]
    [string]$Version = "0.1.0",

    [Parameter()]
    [string]$CertificateThumbprint = "",

    [Parameter()]
    [switch]$SkipBuild,

    [Parameter()]
    [string]$WixPath = ""
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Target = "x86_64-pc-windows-msvc"
$Dist = Join-Path $Root "dist"
$Stage = Join-Path $Dist "stage"
$Executable = Join-Path $Root "target\$Target\release\cuboid-app.exe"

Push-Location $Root
try {
    if (-not $SkipBuild) {
        cargo build --locked --release --target $Target
    }
    if (-not (Test-Path -LiteralPath $Executable)) {
        throw "Release executable not found: $Executable"
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
    $EmbeddedManifest = Join-Path $env:TEMP "cuboid-package-manifest-$PID.xml"
    try {
        & $Mt.FullName -nologo "-inputresource:$Executable;#1" "-out:$EmbeddedManifest"
        if ($LASTEXITCODE -ne 0) {
            throw "The release executable has no embedded Windows manifest."
        }
        $ManifestText = Get-Content -LiteralPath $EmbeddedManifest -Raw
        if ($ManifestText -notmatch "PerMonitorV2" -or $ManifestText -notmatch "asInvoker") {
            throw "The release executable has an invalid Windows manifest."
        }
    }
    finally {
        if (Test-Path -LiteralPath $EmbeddedManifest) {
            Remove-Item -LiteralPath $EmbeddedManifest -Force
        }
    }

    if (Test-Path -LiteralPath $Stage) {
        Remove-Item -LiteralPath $Stage -Recurse -Force
    }
    New-Item -ItemType Directory -Path $Stage -Force | Out-Null
    Copy-Item -LiteralPath $Executable -Destination $Stage
    Copy-Item -LiteralPath (Join-Path $Root "README.md") -Destination $Stage
    Copy-Item -LiteralPath (Join-Path $Root "LICENSE") -Destination $Stage
    New-Item -ItemType File -Path (Join-Path $Stage "portable.flag") -Force | Out-Null

    New-Item -ItemType Directory -Path $Dist -Force | Out-Null
    $Zip = Join-Path $Dist "Cuboid-$Version-windows-x64-portable.zip"
    if (Test-Path -LiteralPath $Zip) {
        Remove-Item -LiteralPath $Zip -Force
    }
    Compress-Archive -Path (Join-Path $Stage "*") -DestinationPath $Zip

    $Msi = Join-Path $Dist "Cuboid-$Version-windows-x64.msi"
    if ($WixPath) {
        $Wix = $WixPath
    }
    else {
        $WixCommand = Get-Command "wix.exe" -ErrorAction SilentlyContinue
        if ($WixCommand) {
            $Wix = $WixCommand.Source
        }
        else {
            $Wix = Join-Path $env:USERPROFILE ".dotnet\tools\wix.exe"
        }
    }
    if (-not (Test-Path -LiteralPath $Wix)) {
        throw "WiX CLI was not found. Install WiX Toolset v4 or later."
    }

    # WiX v7 refuses to build until its Open Source Maintenance Fee EULA is
    # accepted, and earlier versions reject the switch that accepts it.
    $WixArguments = @()
    $WixVersion = (& $Wix --version) -join ""
    if ($LASTEXITCODE -ne 0) {
        throw "WiX CLI at $Wix did not report a version."
    }
    if ([int]($WixVersion -replace "^(\d+).*", '$1') -ge 7) {
        $WixArguments += @("-acceptEula", "wix7")
    }

    & $Wix build "packaging\wix\Cuboid.wxs" `
        @WixArguments `
        -arch x64 `
        -d "SourceDir=$Stage" `
        -d "Version=$Version" `
        -o $Msi
    if ($LASTEXITCODE -ne 0) {
        throw "WiX packaging failed."
    }

    if ($CertificateThumbprint) {
        $SignTool = Get-Command "signtool.exe" -ErrorAction SilentlyContinue
        if (-not $SignTool) {
            throw "signtool.exe is required when CertificateThumbprint is supplied."
        }
        & $SignTool.Source sign /sha1 $CertificateThumbprint /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 $Executable $Msi
        if ($LASTEXITCODE -ne 0) {
            throw "Authenticode signing failed."
        }
    }

    Get-FileHash -Algorithm SHA256 $Zip, $Msi |
        ForEach-Object { "$($_.Hash.ToLower())  $(Split-Path -Leaf $_.Path)" } |
        Set-Content -Encoding ASCII (Join-Path $Dist "SHA256SUMS.txt")
}
finally {
    if (Test-Path -LiteralPath $Stage) {
        Remove-Item -LiteralPath $Stage -Recurse -Force
    }
    Pop-Location
}
