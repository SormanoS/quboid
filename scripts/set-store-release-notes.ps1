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

# Puts the release notes and listing copy into every listing of a Store
# submission.
#
# Only the text fields of each listing's baseListing (releaseNotes,
# shortDescription, description, features and keywords) are touched. The
# document is edited as a JSON tree rather than converted to PowerShell objects,
# so every other field (images, trailers, hardware requirements, pricing) goes
# back to Partner Center exactly as it came, dates and nulls included. Needs
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

function Set-JsonStringProperty {
    param($Object, [string]$Name, [string]$Value)

    $Key = $Name
    foreach ($Property in $Object.AsObject()) {
        if ($Property.Key -ieq $Name) {
            $Key = $Property.Key
            break
        }
    }
    $Object[$Key] = [System.Text.Json.Nodes.JsonValue]::Create($Value)
}

function Set-JsonStringArrayProperty {
    param($Object, [string]$Name, [string[]]$Values)

    $Key = $Name
    foreach ($Property in $Object.AsObject()) {
        if ($Property.Key -ieq $Name) {
            $Key = $Property.Key
            break
        }
    }
    $Json = ConvertTo-Json -InputObject @($Values) -Compress
    $Object[$Key] = [System.Text.Json.Nodes.JsonNode]::Parse($Json)
}

$ListingCopy = @{
    en = @{
        shortDescription = "Snap windows to halves, thirds and quarters with Rectangle's keyboard shortcuts — across mixed-DPI monitors, with automatic restore after docking and no network code."
        description = @"
Quboid is a fast, native window manager for Windows 10 and 11 with Rectangle's default keyboard shortcuts.

Snap the focused window to halves, quarters, thirds, two-thirds or a maximized layout with a single shortcut — no mouse dragging and no animations to wait for. On first launch Quboid shows its shortcut sheet right away, then lives quietly in the notification area (system tray) and reopens whenever you press Ctrl+Alt+S.

Built for multi-monitor and laptop-docking setups:
- Rectangle's default shortcuts on Windows, plus multi-step undo (Ctrl+Alt+Z) and restore (Ctrl+Alt+Backspace)
- Automatic layout restore after docking, undocking or monitor wake: placed windows return to the same fraction of their display
- Full multi-monitor and mixed-DPI support, including negative monitor coordinates and DPI-scaled window gaps
- Optional edge and corner drag snapping with a live preview overlay
- Conflict indicator when another application has already registered a shortcut
- Native Rust executable (~4 MB download), with English and Italian UI
- Verifiable privacy: no network code, no telemetry, no accounts and no background services
- Free and open source software under the GPL-3.0
"@ -replace "`r`n", "`n"
        features = @(
            "Snap the focused window to halves, quarters, thirds or two-thirds with Rectangle's default keyboard shortcuts",
            "Automatically restores placed windows to their display and layout fraction after docking or undocking",
            "Multi-step undo (Ctrl+Alt+Z), restore original size (Ctrl+Alt+Backspace) and instant shortcut cheat-sheet (Ctrl+Alt+S)",
            "28 layout actions, including center, almost maximize, maximize height, grow, shrink and directional step moves",
            "Full multi-monitor and mixed-DPI support, with a configurable gap that scales to each monitor's DPI",
            "Move windows across monitors without reaching for the mouse, or snap by dragging to screen edges and corners",
            "Every shortcut is customizable and Quboid highlights bindings already taken by another app",
            "Available in English and Italian, selected automatically from your Windows language or chosen in settings",
            "No network code, no telemetry, no accounts and no data collection — verifiable on the binary",
            "Free and open source software under the GPL-3.0, written in native Rust (~4 MB download)"
        )
        keywords = @(
            "window manager",
            "snap windows",
            "split screen",
            "multi monitor",
            "keyboard shortcuts",
            "rectangle",
            "window snapping"
        )
    }
    it = @{
        shortDescription = "Affianca le finestre a metà, terzi e quarti con le scorciatoie di Rectangle — supporto multi-monitor a DPI misti, ripristino dopo dock/undock e zero codice di rete."
        description = @"
Quboid è un window manager nativo e veloce per Windows 10 e 11 con le scorciatoie predefinite di Rectangle.

Affianca la finestra attiva a metà, a un quarto, a un terzo, a due terzi o a schermo intero con una sola scorciatoia da tastiera: niente trascinamenti lenti e nessuna animazione da attendere. Al primo avvio Quboid mostra subito il riepilogo delle scorciatoie, poi resta nell'area di notifica e si riapre in qualsiasi momento con Ctrl+Alt+S.

Pensato per chi lavora con più monitor e docking station:
- Scorciatoie predefinite di Rectangle su Windows, con annullamento multi-livello (Ctrl+Alt+Z) e ripristino (Ctrl+Alt+Backspace)
- Ripristino automatico dopo dock/undock: quando ricolleghi i monitor, le finestre posizionate da Quboid tornano nella stessa frazione del proprio schermo
- Supporto completo multi-monitor e DPI misti, incluse coordinate negative e margine (gap) scalato in base al DPI di ogni schermo
- Affiancamento opzionale trascinando le finestre verso i bordi o gli angoli dello schermo
- Indicatore automatico delle scorciatoie già occupate da altri programmi
- Eseguibile nativo in Rust (~4 MB di download), con interfaccia in italiano e inglese
- Privacy verificabile: nessun codice di rete, nessuna telemetria, nessun account e nessun servizio in background
- Software libero e open source con licenza GPL-3.0
"@ -replace "`r`n", "`n"
        features = @(
            "Dispone la finestra attiva a metà, a un quarto, a un terzo o a due terzi con le scorciatoie di Rectangle",
            "Ripristina automaticamente la posizione delle finestre quando colleghi o scolleghi monitor e docking station",
            "Annullamento multi-livello (Ctrl+Alt+Z), ripristino dimensione (Ctrl+Alt+Backspace) e riepilogo rapido (Ctrl+Alt+S)",
            "28 azioni di disposizione, tra cui centra, quasi a schermo intero, altezza massima, ingrandisci e riduci",
            "Supporto completo multi-monitor e DPI misti, con margine (gap) scalato automaticamente per ogni schermo",
            "Sposta le finestre da un monitor all'altro da tastiera, oppure trascinandole verso bordi e angoli",
            "Ogni scorciatoia è personalizzabile e Quboid segnala quelle già occupate da altre applicazioni",
            "Disponibile in italiano e inglese, scelti automaticamente in base alla lingua di Windows",
            "Nessun codice di rete, nessuna telemetria, nessun account e nessuna raccolta dati",
            "Software libero e open source con licenza GPL-3.0, scritto in Rust nativo (~4 MB di download)"
        )
        keywords = @(
            "window manager",
            "affianca finestre",
            "dividere schermo",
            "multi monitor",
            "scorciatoie tastiera",
            "rectangle",
            "snap finestre"
        )
    }
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
    $Copy = $ListingCopy[$Language]
    Set-JsonStringProperty $Base "releaseNotes" $Notes[$Language]
    Set-JsonStringProperty $Base "shortDescription" $Copy.shortDescription
    Set-JsonStringProperty $Base "description" $Copy.description
    Set-JsonStringArrayProperty $Base "features" $Copy.features
    Set-JsonStringArrayProperty $Base "keywords" $Copy.keywords
    Write-Output "Store listing and release notes set for the $($Listing.Key) listing ($Language)."
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
