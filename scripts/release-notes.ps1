param(
    # The version whose notes are wanted. Defaults to the one in Cargo.toml.
    [Parameter()]
    [string]$Version,

    [Parameter()]
    [ValidateSet("en", "it")]
    [string]$Language = "en",

    # Markdown for the GitHub release, or Store for the plain text the
    # Microsoft Store listing accepts as "What's new in this version".
    [Parameter()]
    [ValidateSet("Markdown", "Store")]
    [string]$Format = "Markdown",

    # Writes the notes to this file, as UTF-8, instead of the pipeline.
    [Parameter()]
    [string]$OutFile,

    # Only checks that both changelogs describe the version and that each
    # fits in the Store's limit, so a release cannot reach the tag without them.
    [Parameter()]
    [switch]$Check
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Changelogs = @{
    en = Join-Path $Root "CHANGELOG.md"
    it = Join-Path $Root "CHANGELOG.it.md"
}
# Partner Center rejects release notes longer than this.
$StoreLimit = 1500

if (-not $Version) {
    $Manifest = Get-Content -LiteralPath (Join-Path $Root "Cargo.toml") -Raw
    if ($Manifest -notmatch '(?m)^version = "(?<version>[^"]+)"') {
        throw "No version found under [workspace.package] in Cargo.toml."
    }
    $Version = $Matches.version
}

function Get-Section {
    param([string]$Path, [string]$Version)

    $Lines = [System.IO.File]::ReadAllLines($Path, [System.Text.Encoding]::UTF8)
    $Heading = "^## \[$([regex]::Escape($Version))\]"
    $Section = $null
    foreach ($Line in $Lines) {
        if ($null -eq $Section) {
            if ($Line -match $Heading) {
                $Section = New-Object System.Collections.Generic.List[string]
            }
            continue
        }
        if ($Line -match "^## ") {
            break
        }
        $Section.Add($Line)
    }
    if ($null -eq $Section) {
        throw "$(Split-Path -Leaf $Path) has no section for $Version."
    }
    $Text = ($Section -join "`n").Trim()
    if (-not $Text) {
        throw "The $Version section of $(Split-Path -Leaf $Path) is empty."
    }
    return $Text
}

function ConvertTo-StoreText {
    param([string]$Markdown)

    # The Store shows plain text, so headings become lines of their own,
    # bullets become bullet characters and wrapped lines are joined again.
    $Blocks = New-Object System.Collections.Generic.List[string]
    foreach ($Line in $Markdown -split "`n") {
        $Plain = $Line.Trim() `
            -replace '\[([^\]]+)\]\([^)]+\)', '$1' `
            -replace '\*\*|`', ''
        if (-not $Plain) {
            continue
        }
        if ($Plain -match "^#+\s+(?<title>.+)$") {
            $Blocks.Add("`n$($Matches.title)")
        }
        elseif ($Plain -match "^[-*]\s+(?<item>.+)$") {
            $Blocks.Add("$([char]0x2022) $($Matches.item)")
        }
        elseif ($Blocks.Count -gt 0 -and $Line -match "^\s") {
            $Blocks[$Blocks.Count - 1] += " $Plain"
        }
        else {
            $Blocks.Add($Plain)
        }
    }
    return ($Blocks -join "`n").Trim()
}

function Get-Notes {
    param([string]$Language, [string]$Format)

    $Markdown = Get-Section -Path $Changelogs[$Language] -Version $Version
    if ($Format -eq "Store") {
        return ConvertTo-StoreText $Markdown
    }
    return $Markdown
}

if ($Check) {
    foreach ($Name in $Changelogs.Keys) {
        $Store = Get-Notes -Language $Name -Format Store
        if ($Store.Length -gt $StoreLimit) {
            throw "The $Version notes in $(Split-Path -Leaf $Changelogs[$Name]) are $($Store.Length) characters as Store text; the Store accepts $StoreLimit."
        }
    }
    Write-Output "Both changelogs describe $Version."
    return
}

$Notes = Get-Notes -Language $Language -Format $Format
if ($OutFile) {
    $Target = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutFile)
    $Encoding = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllText($Target, $Notes + "`n", $Encoding)
}
else {
    Write-Output $Notes
}
