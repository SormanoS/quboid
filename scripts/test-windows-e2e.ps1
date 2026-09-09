param(
    [Parameter()]
    [string]$ExecutablePath = "",

    [Parameter()]
    [switch]$SkipDrag
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not $ExecutablePath) {
    $ExecutablePath = Join-Path $Root "target\x86_64-pc-windows-msvc\release\quboid-app.exe"
}
if (-not (Test-Path -LiteralPath $ExecutablePath)) {
    throw "Quboid executable not found: $ExecutablePath"
}
$ExecutablePath = (Resolve-Path -LiteralPath $ExecutablePath).Path

$ExecutableDirectory = Split-Path -Parent $ExecutablePath
$PortableFlag = Join-Path $ExecutableDirectory "portable.flag"
$PortableConfig = Join-Path $ExecutableDirectory "config.json"
$LocalLog = Join-Path $ExecutableDirectory "logs"
$FlagExisted = Test-Path -LiteralPath $PortableFlag
$FlagBackup = if ($FlagExisted) {
    [System.IO.File]::ReadAllBytes($PortableFlag)
}
else {
    $null
}
$ConfigExisted = Test-Path -LiteralPath $PortableConfig
$ConfigBackup = if ($ConfigExisted) {
    [System.IO.File]::ReadAllBytes($PortableConfig)
}
else {
    $null
}
$LogExisted = Test-Path -LiteralPath $LocalLog
$InstalledConfig = Join-Path $env:APPDATA "Quboid\config.json"
$InstalledConfigHash = if (Test-Path -LiteralPath $InstalledConfig) {
    (Get-FileHash -Algorithm SHA256 -LiteralPath $InstalledConfig).Hash
}
else {
    ""
}
$Quboid = $null
$Form = $null

function Get-VisibleRect {
    param([IntPtr]$Window)

    $Rect = New-Object QuboidE2E+RECT
    $Result = [QuboidE2E]::DwmGetWindowAttribute(
        $Window,
        9,
        [ref]$Rect,
        [Runtime.InteropServices.Marshal]::SizeOf([type][QuboidE2E+RECT])
    )
    if ($Result -lt 0) {
        if (-not [QuboidE2E]::GetWindowRect($Window, [ref]$Rect)) {
            throw "GetWindowRect failed."
        }
    }
    return $Rect
}

function Get-WorkArea {
    param([IntPtr]$Window)

    $Monitor = [QuboidE2E]::MonitorFromWindow($Window, 2)
    $Info = New-Object QuboidE2E+MONITORINFO
    $Info.cbSize = [Runtime.InteropServices.Marshal]::SizeOf([type][QuboidE2E+MONITORINFO])
    if (-not [QuboidE2E]::GetMonitorInfo($Monitor, [ref]$Info)) {
        throw "GetMonitorInfo failed."
    }
    return $Info.rcWork
}

function Send-QuboidHotkey {
    param([byte]$Key)

    [QuboidE2E]::keybd_event(0x11, 0, 0, [UIntPtr]::Zero)
    [QuboidE2E]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)
    [QuboidE2E]::keybd_event($Key, 0, 0, [UIntPtr]::Zero)
    [QuboidE2E]::keybd_event($Key, 0, 2, [UIntPtr]::Zero)
    [QuboidE2E]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
    [QuboidE2E]::keybd_event(0x11, 0, 2, [UIntPtr]::Zero)
}

function Set-TestWindowForeground {
    $Deadline = [DateTime]::UtcNow.AddSeconds(3)
    do {
        [QuboidE2E]::ForceForeground($Form.MainWindowHandle) | Out-Null
        Start-Sleep -Milliseconds 100
    } while (
        [QuboidE2E]::GetForegroundWindow() -ne $Form.MainWindowHandle -and
        [DateTime]::UtcNow -lt $Deadline
    )
    if ([QuboidE2E]::GetForegroundWindow() -ne $Form.MainWindowHandle) {
        throw "Synthetic test window could not receive foreground focus."
    }
}

function Assert-Near {
    param(
        [int]$Actual,
        [int]$Expected,
        [string]$Edge,
        [int]$Tolerance = 12
    )

    if ([Math]::Abs($Actual - $Expected) -gt $Tolerance) {
        throw "$Edge differs: actual=$Actual expected=$Expected tolerance=$Tolerance"
    }
}

try {
    New-Item -ItemType File -Path $PortableFlag -Force | Out-Null
    $ConfigJson = @'
{
  "schema_version": 4,
  "language": "english",
  "launch_at_login": false,
  "drag_snap_enabled": true,
  "hotkeys": [
    { "action": "left_half", "modifiers": 3, "virtual_key": 37 },
    { "action": "right_half", "modifiers": 3, "virtual_key": 39 },
    { "action": "maximize", "modifiers": 3, "virtual_key": 38 },
    { "action": "restore", "modifiers": 3, "virtual_key": 40 },
    { "action": "next_monitor", "modifiers": 3, "virtual_key": 78 },
    { "action": "previous_monitor", "modifiers": 3, "virtual_key": 80 }
  ],
  "gap": 0,
  "snap_threshold": 500
}
'@
    $Utf8WithoutBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText(
        $PortableConfig,
        $ConfigJson,
        $Utf8WithoutBom
    )

    Add-Type -AssemblyName Microsoft.VisualBasic
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class QuboidE2E {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    public struct MONITORINFO {
        public int cbSize;
        public RECT rcMonitor;
        public RECT rcWork;
        public uint dwFlags;
    }

    [DllImport("dwmapi.dll")]
    public static extern int DwmGetWindowAttribute(
        IntPtr hwnd, int attribute, out RECT value, int size);
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr hwnd, IntPtr processId);
    [DllImport("kernel32.dll")]
    private static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")]
    private static extern bool AttachThreadInput(uint from, uint to, bool attach);
    [DllImport("user32.dll")]
    private static extern bool BringWindowToTop(IntPtr hwnd);
    [DllImport("user32.dll")]
    private static extern IntPtr SetActiveWindow(IntPtr hwnd);
    [DllImport("user32.dll")]
    private static extern IntPtr SetFocus(IntPtr hwnd);
    [DllImport("user32.dll")]
    private static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")]
    public static extern IntPtr MonitorFromWindow(IntPtr hwnd, uint flags);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern bool GetMonitorInfo(IntPtr monitor, ref MONITORINFO info);
    [DllImport("user32.dll")]
    public static extern void keybd_event(
        byte virtualKey, byte scanCode, uint flags, UIntPtr extra);
    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")]
    public static extern void mouse_event(
        uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")]
    public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);

    public static bool ForceForeground(IntPtr hwnd) {
        var currentThread = GetCurrentThreadId();
        var foregroundThread = GetWindowThreadProcessId(GetForegroundWindow(), IntPtr.Zero);
        var targetThread = GetWindowThreadProcessId(hwnd, IntPtr.Zero);
        var attachedForeground = foregroundThread != 0 && foregroundThread != currentThread;
        var attachedTarget =
            targetThread != 0 && targetThread != currentThread && targetThread != foregroundThread;
        if (attachedForeground) AttachThreadInput(currentThread, foregroundThread, true);
        if (attachedTarget) AttachThreadInput(currentThread, targetThread, true);
        try {
            BringWindowToTop(hwnd);
            SetActiveWindow(hwnd);
            SetFocus(hwnd);
            SetForegroundWindow(hwnd);
            return GetForegroundWindow() == hwnd;
        }
        finally {
            if (attachedTarget) AttachThreadInput(currentThread, targetThread, false);
            if (attachedForeground) AttachThreadInput(currentThread, foregroundThread, false);
        }
    }
}
'@
    $PreviousDpiContext = [QuboidE2E]::SetThreadDpiAwarenessContext([IntPtr](-4))

    $FormCommand = @'
Add-Type -AssemblyName System.Windows.Forms
$form = New-Object System.Windows.Forms.Form
$form.Text = "Quboid E2E Test Window"
$form.Width = 800
$form.Height = 500
$form.StartPosition = "CenterScreen"
[void]$form.ShowDialog()
'@
    $Encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($FormCommand))
    $Form = Start-Process -FilePath "powershell.exe" `
        -ArgumentList @("-NoProfile", "-WindowStyle", "Hidden", "-EncodedCommand", $Encoded) `
        -PassThru
    $Quboid = Start-Process -FilePath $ExecutablePath -PassThru
    Start-Sleep -Seconds 1
    $Quboid.Refresh()
    if ($Quboid.HasExited) {
        throw "Quboid exited before the end-to-end test (exit code $($Quboid.ExitCode))."
    }

    $Deadline = [DateTime]::UtcNow.AddSeconds(10)
    do {
        Start-Sleep -Milliseconds 100
        $Form.Refresh()
    } while ($Form.MainWindowHandle -eq 0 -and [DateTime]::UtcNow -lt $Deadline)
    if ($Form.MainWindowHandle -eq 0) {
        throw "Synthetic test window did not start."
    }
    Start-Sleep -Seconds 2

    Set-TestWindowForeground
    $Original = Get-VisibleRect $Form.MainWindowHandle
    $Work = Get-WorkArea $Form.MainWindowHandle
    $ExpectedMiddle = $Work.Left + [int](($Work.Right - $Work.Left) / 2)

    Send-QuboidHotkey 0x25
    Start-Sleep -Milliseconds 500
    $Snapped = Get-VisibleRect $Form.MainWindowHandle
    Assert-Near $Snapped.Left $Work.Left "hotkey left"
    Assert-Near $Snapped.Top $Work.Top "hotkey top"
    Assert-Near $Snapped.Right $ExpectedMiddle "hotkey right"
    Assert-Near $Snapped.Bottom $Work.Bottom "hotkey bottom"

    Set-TestWindowForeground
    Send-QuboidHotkey 0x25
    Start-Sleep -Milliseconds 500
    $Repeated = Get-VisibleRect $Form.MainWindowHandle
    Assert-Near $Repeated.Left $Work.Left "repeat left"
    Assert-Near $Repeated.Top $Work.Top "repeat top"
    Assert-Near $Repeated.Right $ExpectedMiddle "repeat right"
    Assert-Near $Repeated.Bottom $Work.Bottom "repeat bottom"

    Set-TestWindowForeground
    Send-QuboidHotkey 0x28
    Start-Sleep -Milliseconds 500
    $Restored = Get-VisibleRect $Form.MainWindowHandle
    Assert-Near $Restored.Left $Original.Left "restore left"
    Assert-Near $Restored.Top $Original.Top "restore top"
    Assert-Near $Restored.Right $Original.Right "restore right"
    Assert-Near $Restored.Bottom $Original.Bottom "restore bottom"

    Add-Type -AssemblyName System.Windows.Forms
    if ([System.Windows.Forms.Screen]::AllScreens.Count -gt 1) {
        Set-TestWindowForeground
        Send-QuboidHotkey 0x4E
        Start-Sleep -Milliseconds 700
        $NextMonitorWork = Get-WorkArea $Form.MainWindowHandle
        if (
            $NextMonitorWork.Left -eq $Work.Left -and
            $NextMonitorWork.Top -eq $Work.Top -and
            $NextMonitorWork.Right -eq $Work.Right -and
            $NextMonitorWork.Bottom -eq $Work.Bottom
        ) {
            throw "Next-monitor hotkey did not move the test window to another monitor."
        }
        Set-TestWindowForeground
        Send-QuboidHotkey 0x50
        Start-Sleep -Milliseconds 700
        $PreviousMonitorWork = Get-WorkArea $Form.MainWindowHandle
        if (
            $PreviousMonitorWork.Left -ne $Work.Left -or
            $PreviousMonitorWork.Top -ne $Work.Top -or
            $PreviousMonitorWork.Right -ne $Work.Right -or
            $PreviousMonitorWork.Bottom -ne $Work.Bottom
        ) {
            throw "Previous-monitor hotkey did not return the test window."
        }
        Set-TestWindowForeground
        Send-QuboidHotkey 0x28
        Start-Sleep -Milliseconds 500
        $Restored = Get-VisibleRect $Form.MainWindowHandle
    }

    if (-not $SkipDrag) {
        Set-TestWindowForeground
        $StartX = [int](($Restored.Left + $Restored.Right) / 2)
        $StartY = $Restored.Top + 15
        [QuboidE2E]::SetCursorPos($StartX, $StartY) | Out-Null
        [QuboidE2E]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
        for ($Step = 1; $Step -le 20; $Step++) {
            $X = [int]($StartX + (($Work.Left + 1) - $StartX) * $Step / 20)
            [QuboidE2E]::SetCursorPos($X, $StartY + 100) | Out-Null
            Start-Sleep -Milliseconds 30
        }
        [QuboidE2E]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 700
        $Dragged = Get-VisibleRect $Form.MainWindowHandle
        Assert-Near $Dragged.Left $Work.Left "drag left"
        Assert-Near $Dragged.Top $Work.Top "drag top"
        Assert-Near $Dragged.Right $ExpectedMiddle "drag right"
        Assert-Near $Dragged.Bottom $Work.Bottom "drag bottom"
    }

    if (-not (Test-Path -LiteralPath $LocalLog)) {
        throw "Portable mode did not create its local log directory."
    }
}
finally {
    if ($PreviousDpiContext -and $PreviousDpiContext -ne [IntPtr]::Zero) {
        [QuboidE2E]::SetThreadDpiAwarenessContext($PreviousDpiContext) | Out-Null
    }
    if ($Quboid -and -not $Quboid.HasExited) {
        Stop-Process -Id $Quboid.Id
        $Quboid.WaitForExit()
    }
    if ($Form -and -not $Form.HasExited) {
        Stop-Process -Id $Form.Id
        $Form.WaitForExit()
    }
    if ($ConfigExisted) {
        [System.IO.File]::WriteAllBytes($PortableConfig, $ConfigBackup)
    }
    elseif (Test-Path -LiteralPath $PortableConfig) {
        Remove-Item -LiteralPath $PortableConfig -Force
    }
    if ($FlagExisted) {
        [System.IO.File]::WriteAllBytes($PortableFlag, $FlagBackup)
    }
    elseif (Test-Path -LiteralPath $PortableFlag) {
        Remove-Item -LiteralPath $PortableFlag -Force
    }
    if (-not $LogExisted -and (Test-Path -LiteralPath $LocalLog)) {
        Remove-Item -LiteralPath $LocalLog -Recurse -Force
    }
}

$CurrentInstalledHash = if (Test-Path -LiteralPath $InstalledConfig) {
    (Get-FileHash -Algorithm SHA256 -LiteralPath $InstalledConfig).Hash
}
else {
    ""
}
if ($CurrentInstalledHash -ne $InstalledConfigHash) {
    throw "Portable mode changed the installed AppData configuration."
}

Write-Output "Quboid Windows end-to-end tests passed."
