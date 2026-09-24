param(
    # The draft submission as `msstore submission get` prints it.
    [Parameter(Mandatory)]
    [string]$SubmissionPath,

    # A directory holding store-notes.en.txt and store-notes.it.txt, as
    # scripts\release-notes.ps1 -Format Store writes them.
    [Parameter(Mandatory)]
    [string]$NotesDirectory,

    [Parameter(Mandatory)]
    [string]$OutFile
)

# Puts the release notes into every listing of a Store submission.
#
# Only releaseNotes is touched. The document is edited as a JSON tree rather
# than converted to PowerShell objects, so every other field goes back to
# Partner Center exactly as it came, dates and nulls included. Needs
# PowerShell 7, which ships System.Text.Json.

$ErrorActionPreference = "Stop"

function Get-JsonProperty {
    param($Object, [string]$Name)

    # The CLI's casing is its own business; match names regardless of it.
    foreach ($Property in $Object.AsObject()) {
        if ($Property.Key -ieq $Name) {
            # A JSON object is enumerable, so it has to be kept from unrolling.
            return , $Property.Value
        }
    }
    return $null
}

$Output = [System.IO.File]::ReadAllText(
    $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($SubmissionPath)
)
# The CLI may print progress around the document; the JSON is what lies
# between the first opening and the last closing brace.
$Start = $Output.IndexOf("{")
$End = $Output.LastIndexOf("}")
if ($Start -lt 0 -or $End -le $Start) {
    throw "The submission is not a JSON document."
}
$Submission = [System.Text.Json.Nodes.JsonNode]::Parse($Output.Substring($Start, $End - $Start + 1))

$Notes = @{}
foreach ($Language in "en", "it") {
    $Path = Join-Path $NotesDirectory "store-notes.$Language.txt"
    $Notes[$Language] = [System.IO.File]::ReadAllText(
        $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Path)
    ).Trim()
}

$Listings = Get-JsonProperty $Submission "listings"
if ($null -eq $Listings) {
    throw "The submission has no listings to put release notes into."
}
$Updated = 0
foreach ($Listing in $Listings.AsObject()) {
    $Base = Get-JsonProperty $Listing.Value "baseListing"
    if ($null -eq $Base) {
        Write-Warning "The $($Listing.Key) listing has no base listing; its release notes are left alone."
        continue
    }
    # Every listing that is not Italian gets the English notes, the language
    # Quboid falls back to wherever it is not translated.
    $Language = if ($Listing.Key -like "it*") { "it" } else { "en" }
    $Key = "releaseNotes"
    foreach ($Property in $Base.AsObject()) {
        if ($Property.Key -ieq $Key) {
            $Key = $Property.Key
        }
    }
    $Base[$Key] = [System.Text.Json.Nodes.JsonValue]::Create([string]$Notes[$Language])
    Write-Output "Release notes set for the $($Listing.Key) listing ($Language)."
    $Updated++
}
if ($Updated -eq 0) {
    throw "No listing of the submission could take release notes."
}

$Options = New-Object System.Text.Json.JsonSerializerOptions
$Options.WriteIndented = $true
$Encoding = New-Object System.Text.UTF8Encoding $false
[System.IO.File]::WriteAllText(
    $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutFile),
    $Submission.ToJsonString($Options),
    $Encoding
)
