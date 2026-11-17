param(
    [ValidateSet('keyboard', 'pages', 'supplemental', 'launcher', 'focusrestore')][string]$Mode = 'keyboard',
    [string]$Exe = 'D:\WoWSyncDemo\bin\wow-sync-proto.exe'
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class CheckWindow {
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
}
'@
function Start-Prototype([string[]]$Arguments) {
    $script:proc = Start-Process $Exe -ArgumentList $Arguments -PassThru
    $script:born = $script:proc.StartTime
    "LAUNCH pid=$($script:proc.Id) args=$($Arguments -join ' ')"
    Start-Sleep -Seconds 2
    $script:proc.Refresh()
    $script:root = [Windows.Automation.AutomationElement]::FromHandle($script:proc.MainWindowHandle)
    [void][Windows.Automation.AutomationElement]::FocusedElement
    Start-Sleep -Milliseconds 500
}
function Stop-Prototype {
    if ($script:proc) {
        $script:proc.Refresh()
        if (!$script:proc.HasExited -and $script:proc.StartTime -eq $script:born) {
            Stop-Process -Id $script:proc.Id
            $script:proc.WaitForExit(5000) | Out-Null
            "STOP tracked pid=$($script:proc.Id)"
        }
        $script:proc = $null
    }
}
function Elements {
    @($script:root.FindAll([Windows.Automation.TreeScope]::Descendants, [Windows.Automation.Condition]::TrueCondition))
}
function Focus([string]$Label) {
    $f = [Windows.Automation.AutomationElement]::FocusedElement
    if ($f.Current.ProcessId -ne $script:proc.Id) { throw 'Focus left the launched prototype; no keys sent' }
    "$Label FOCUS=$($f.Current.ControlType.ProgrammaticName)|$($f.Current.Name) offscreen=$($f.Current.IsOffscreen)"
}
function Key([string]$Keys) {
    [void][CheckWindow]::SetForegroundWindow($script:proc.MainWindowHandle)
    Start-Sleep -Milliseconds 100
    [uint32]$owner = 0
    [void][CheckWindow]::GetWindowThreadProcessId([CheckWindow]::GetForegroundWindow(), [ref]$owner)
    if ($owner -ne $script:proc.Id) {
        [void](New-Object -ComObject WScript.Shell).AppActivate($script:proc.Id)
        Start-Sleep -Milliseconds 200
        [void][CheckWindow]::GetWindowThreadProcessId([CheckWindow]::GetForegroundWindow(), [ref]$owner)
    }
    if ($owner -ne $script:proc.Id) { throw 'Cannot foreground the launched prototype; no keys sent' }
    [Windows.Forms.SendKeys]::SendWait($Keys)
    Start-Sleep -Milliseconds 250
}
function Select-Named([string]$Pattern) {
    $e = Elements | Where-Object { $_.Current.Name -like $Pattern -and $_.Current.IsKeyboardFocusable } | Select-Object -First 1
    if (!$e) { throw "No focusable UIA match: $Pattern" }
    $e.SetFocus()
    Start-Sleep -Milliseconds 200
    Focus "UIA SetFocus '$Pattern'"
}
try {
    if ($Mode -eq 'keyboard') {
        Start-Prototype @('--scenario', 'S14', '--page', 'Add-ons')
        Focus 'initial'
        Key '{TAB}'; Focus 'Tab'
        Key '+{TAB}'; Focus 'Shift+Tab'
        Select-Named '*add-ons*selected'
        Key '{TAB}'; Focus 'S14 first row'
        Key '{DOWN}'; Focus 'S14 Down'
        Key '{END}'; Focus 'S14 End'
        Key '{ENTER}'; Focus 'S14 open detail'
        Key '{ESC}'; Focus 'Esc detail'
        Stop-Prototype

        Start-Prototype @('--scenario', 'S3', '--sheet', 'quit')
        Focus 'sheet initial'
        1..6 | ForEach-Object { Key '{TAB}'; Focus "sheet Tab $_" }
        1..4 | ForEach-Object { Key '+{TAB}'; Focus "sheet Shift+Tab $_" }
        Key '{ESC}'; Focus 'Esc quit sheet'
        "DIALOGS_AFTER_ESC=$(@(Elements | Where-Object { $_.Current.ControlType -eq [Windows.Automation.ControlType]::Window -or $_.Current.LocalizedControlType -eq 'dialog' }).Count)"
        Stop-Prototype

        Start-Prototype @('--scenario', 'S3', '--sheet', 'first-close')
        Key '{ESC}'
        $script:proc.Refresh()
        "ESC_FIRST_CLOSE process_alive=$(!$script:proc.HasExited) main_handle=$($script:proc.MainWindowHandle)"
        Stop-Prototype
    } elseif ($Mode -eq 'focusrestore') {
        Start-Prototype @('--scenario', 'S4', '--page', 'Add-ons')
        Focus 'initial'
        Key '+{TAB}'; Focus 'Shift+Tab 1'
        Key '+{TAB}'; Focus 'Shift+Tab 2'
        $before = [Windows.Automation.AutomationElement]::FocusedElement
        $expected = $before.Current.Name
        Key '^q'; Focus 'open quit sheet'
        Key '{ESC}'; Focus 'Esc restoration'
        $after = [Windows.Automation.AutomationElement]::FocusedElement
        if ($after.Current.Name -ne $expected) {
            "FAIL_FOCUS_RESTORE expected='$expected' actual='$($after.Current.Name)'"
            exit 1
        }
        'PASS_FOCUS_RESTORE'
    } elseif ($Mode -eq 'launcher') {
        $log = Join-Path $PSScriptRoot 'launcher.log'
        if (Test-Path $log) { throw 'Launcher log already exists; preserve the earlier evidence' }
        $env:WOWSYNC_PROTO_LOG = $log
        $started = Get-Date
        $wrapper = Start-Process $env:ComSpec -ArgumentList '/c', 'D:\WoWSyncDemo\Start-Prototype.cmd', '--scenario', 'S3', '--sheet', 'first-close' -PassThru
        "LAUNCHER_WRAPPER_PID=$($wrapper.Id)"
        Start-Sleep -Seconds 3
        $ids = @(Get-Content $log | ForEach-Object { if ($_ -match '^pid=(\d+) ') { [int]$Matches[1] } } | Sort-Object -Unique)
        if ($ids.Count -ne 1) { throw 'Cannot uniquely identify the app from this launch-specific log' }
        $launched = Get-Process -Id $ids[0]
        if ($launched.StartTime -lt $started.AddSeconds(-1) -or $launched.MainModule.FileName -ne $Exe) {
            throw 'Launcher app identity does not match the launched target and launch time'
        }
        "LAUNCHER_APP_SELF_REPORTED_PID=$($launched.Id)"
        $script:proc = $launched
        $script:born = $launched.StartTime
        $script:root = [Windows.Automation.AutomationElement]::FromHandle($launched.MainWindowHandle)
        [void][Windows.Automation.AutomationElement]::FocusedElement
        Start-Sleep -Milliseconds 400
        Select-Named 'Quit WoW Sync'
        Key '{ENTER}'
        $launched.WaitForExit(5000) | Out-Null
        "LAUNCHER_NORMAL_QUIT=$($launched.HasExited)"
        $script:proc = $null
        if (!$launched.HasExited) { throw 'Normal GUI Quit did not exit the launcher-created prototype' }
        Remove-Item Env:\WOWSYNC_PROTO_LOG
    } elseif ($Mode -eq 'supplemental') {
        Start-Prototype @('--scenario', 'S1', '--setup-step', '6')
        Key '{ENTER}'
        Start-Sleep -Seconds 3
        foreach ($e in (Elements)) {
            if ($e.Current.Name) { "SETUP_FINISH=$($e.Current.ControlType.ProgrammaticName)|$($e.Current.Name)" }
        }
        Stop-Prototype

        Start-Prototype @('--scenario', 'S3', '--sheet', 'first-close')
        1..4 | ForEach-Object { Key '{TAB}'; Focus "first-close Tab $_" }
        1..2 | ForEach-Object { Key '+{TAB}'; Focus "first-close Shift+Tab $_" }
        Select-Named 'Sync now'
        Focus 'attempt UIA focus outside sheet'
        Stop-Prototype

        Start-Prototype @('--scenario', 'S4', '--page', 'Add-ons')
        Select-Named 'Pause syncing'
        Key '^q'
        Focus 'Ctrl+Q from Pause syncing'
        Key '{ESC}'
        Focus 'Esc restoration after Ctrl+Q'
        Stop-Prototype
    } else {
        $pages = @(
            @('P1/P2 Overview', '--scenario S3 --page Overview'),
            @('P3 Computers', '--scenario S9 --page Computers'),
            @('P3 Computer detail', '--scenario S9 --detail computer:Laptop'),
            @('P4 Add-ons', '--scenario S14 --page Add-ons'),
            @('P5 Add-on detail', '--scenario S7 --detail addon:WeakAuras'),
            @('P6 Reconfirm', '--scenario S12 --detail reconfirm'),
            @('P7 Needs attention', '--scenario S7 --page Needs-attention'),
            @('P7 Decision', '--scenario S7 --detail decision:WeakAuras'),
            @('P7 Needs-help detail', '--scenario S16 --detail needshelp'),
            @('P8 Activity', '--scenario S5 --page Activity'),
            @('P9 Settings sync', '--scenario S3 --page Settings-sync'),
            @('P10 App settings', '--scenario S3 --page App-settings'),
            @('P11 About', '--scenario S13 --page About'),
            @('P13 First-close sheet', '--scenario S3 --sheet first-close'),
            @('P13 Quit sheet', '--scenario S4 --sheet quit'),
            @('P14 Scenario switcher', '--scenario S3 --page Prototype-scenarios')
        )
        1..6 | ForEach-Object { $pages += ,@("P6 Setup step $_", "--scenario S1 --setup-step $_") }
        foreach ($page in $pages) {
            try {
                Start-Prototype ($page[1] -split ' ')
                $es = @(Elements)
                $controls = @($es | Where-Object { $_.Current.IsKeyboardFocusable })
                $unnamed = @($controls | Where-Object { [string]::IsNullOrWhiteSpace($_.Current.Name) })
                "PAGE=$($page[0]) nodes=$($es.Count) focusable=$($controls.Count) unnamed=$($unnamed.Count)"
                foreach ($e in ($controls | Select-Object -First 20)) {
                    "  $($e.Current.ControlType.ProgrammaticName)|$($e.Current.Name)"
                }
                $headers = @($es | Where-Object { $_.Current.ControlType -eq [Windows.Automation.ControlType]::Text -or $_.Current.ControlType -eq [Windows.Automation.ControlType]::Group })
                "  named_nonfocus=$(@($headers | Where-Object { $_.Current.Name }).Count)"
            } finally { Stop-Prototype }
        }
    }
} finally {
    if ($Mode -ne 'launcher') { Stop-Prototype }
}
