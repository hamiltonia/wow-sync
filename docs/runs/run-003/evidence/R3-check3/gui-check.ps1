param([string]$Lab = 'D:\WoWSyncLab\R3-check3\gui', [switch]$Resume, [switch]$InterruptOnly, [switch]$FinalOnly)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path "$PSScriptRoot\..\..\..\..\..").Path
Set-Location $repo
. "$repo\tools\mem\MemLib.ps1"
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class CaptureOwnWindow {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X,Y; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out Rect r);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out Rect r);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref Point p);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
}
'@
[void][CaptureOwnWindow]::SetThreadDpiAwarenessContext([IntPtr](-4))
$app = "$repo\target\release\wow-sync.exe"
$dev = "$repo\target\release\devkit.exe"
$roots = [Collections.Generic.List[object]]::new()
$fake = $null
function Launch($side, [string[]]$extra = @()) {
    $args_ = @('--dev','--install',"$lab\$side\install",'--provider',"$lab\provider",'--state',"$lab\$side\state",'--name',"Lab-$side",'--interval-ms','60000','--recent-ms','100') + $extra
    $p = Start-Process $app -ArgumentList $args_ -PassThru
    Write-Host "LAUNCH $side PID=$($p.Id)"
    if ($p.WaitForExit(300)) { Write-Host "SECONDARY EXIT=$($p.ExitCode)"; return $null }
    $rec = Get-Record $p.Id
    $roots.Add($rec)
    $rec
}
function Elements($r) {
    $w = Get-AppWindow $r.Pid
    if ($w) {
        $root = [Windows.Automation.AutomationElement]::FromHandle([IntPtr]$w[0])
        $root.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition)
    }
}
function Find($r,$name) {
    foreach ($e in (Elements $r)) { if ($e.Current.Name -like $name) { return $e } }
}
function Wait-For($r,$name) {
    for ($i=0; $i -lt 100; $i++) {
        $e = Find $r $name
        if ($e) { return $e }
        Start-Sleep -Milliseconds 200
    }
    throw "Missing UIA element: $name"
}
function Use($r,$name) {
    $e = Wait-For $r $name
    if ($e.GetSupportedPatterns().Id -contains [Windows.Automation.TogglePattern]::Pattern.Id) {
        $e.GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Toggle()
    } elseif ($e.GetSupportedPatterns().Id -contains [Windows.Automation.SelectionItemPattern]::Pattern.Id) {
        $e.GetCurrentPattern([Windows.Automation.SelectionItemPattern]::Pattern).Select()
    } else {
        $e.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
    }
    Start-Sleep -Milliseconds 600
    Write-Host "INVOKE $name"
}
function Capture($r,$label) {
    $w = Get-AppWindow $r.Pid
    [void][CaptureOwnWindow]::SetForegroundWindow([IntPtr]$w[0])
    Start-Sleep -Seconds 1
    $lines = foreach ($e in (Elements $r)) {
        if ($e.Current.Name) { "$($e.Current.ControlType.ProgrammaticName): $($e.Current.Name)" }
    }
    [IO.File]::WriteAllLines("$PSScriptRoot\$label.uia.txt", [string[]]$lines)
    $rect = [CaptureOwnWindow+Rect]::new()
    [void][CaptureOwnWindow]::GetClientRect([IntPtr]$w[0],[ref]$rect)
    $origin = [CaptureOwnWindow+Point]::new()
    [void][CaptureOwnWindow]::ClientToScreen([IntPtr]$w[0],[ref]$origin)
    $bmp = [Drawing.Bitmap]::new($rect.Right-$rect.Left,$rect.Bottom-$rect.Top)
    $g = [Drawing.Graphics]::FromImage($bmp)
    try {
        $g.CopyFromScreen($origin.X,$origin.Y,0,0,$bmp.Size)
        $bmp.Save("$PSScriptRoot\$label.png",[Drawing.Imaging.ImageFormat]::Png)
    } finally { $g.Dispose(); $bmp.Dispose() }
    Write-Host "CAPTURE $label"
    $lines | Where-Object { $_ -match 'waiting|decision|interrupted|pause|next check|up to date|version' } | ForEach-Object { Write-Host $_ }
}
function Enroll($r,$joining) {
    foreach ($n in @('Next','Use this folder','Next','Next','This is my*','These are the*','Next')) { Use $r $n }
    if ($joining) { Use $r 'Join My add-ons*' } else { Use $r 'Create a new group' }
    foreach ($n in @('Next','Set up this computer','Finish and open*')) { Use $r $n }
    [void](Wait-For $r 'Sync now')
    Use $r 'Sync now'
    Start-Sleep -Seconds 3
}
function Tick($r) {
    Use $r 'Sync now'
    Start-Sleep -Seconds 3
}
try {
    if ($FinalOnly) {
        $a = Launch A
        [void](Wait-For $a 'Resume syncing')
        Capture $a damaged-pause
        Use $a 'Add-ons'
        Use $a 'LabBeta*'
        Capture $a interrupted-no-game
        Send-Wheel $a.Pid -900
        Start-Sleep -Seconds 1
        Capture $a interrupted-no-game-lower
        Use $a 'Overview'
        Use $a 'Resume syncing'
        Start-Sleep -Seconds 2
        Write-Host "DAMAGED PAUSE CLEARED=$(-not (Test-Path "$lab\A\state\paused.txt"))"
        Tick $a
        Use $a 'Needs attention*'
        Capture $a pending-attention-stable
        Use $a 'LabAlpha: can*'
        Capture $a pending-decision-stable
        return
    }
    if (!$InterruptOnly) {
    if (!$Resume) {
    New-Item -ItemType Directory -Force "$lab\A\state","$lab\B\state","$lab\provider" | Out-Null
    & $dev make-install "$lab\A\install"
    & $dev add-addon "$lab\A\install" _forever_ LabAlpha alpha-one 1.0.0
    & $dev add-addon "$lab\A\install" _forever_ LabBeta beta-one 1.0.0
    & $dev make-install "$lab\B\install"
    $a = Launch A
    Enroll $a $false
    Capture $a dashboard

    Use $a 'App settings'
    $switch = Wait-For $a 'Closing the window quits WoW Sync'
    Write-Host "TOGGLE BEFORE=$($switch.GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Current.ToggleState)"
    $switch.GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Toggle()
    Start-Sleep -Seconds 1
    Write-Host "TOGGLE AFTER=$((Find $a 'Closing the window quits WoW Sync').GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Current.ToggleState)"
    Write-Host "SETTINGS DISK=$([IO.File]::ReadAllText("$lab\A\state\app-settings.txt"))"
    [void](Stop-Launched $a)
    $a = Launch A
    Use $a 'App settings'
    Write-Host "TOGGLE RESTART=$((Find $a 'Closing the window quits WoW Sync').GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Current.ToggleState)"
    Capture $a close-option
    Use $a 'Closing the window quits WoW Sync'
    Close-AppWindow $a.Pid
    Use $a 'Hide window'
    Write-Host "HIDDEN=$(-not (Get-AppWindow $a.Pid)[2])"
    for ($i=0; $i -lt 3; $i++) {
        [void](Launch A)
        Start-Sleep -Seconds 1
        Write-Host "SECOND INSTANCE VISIBLE ($i)=$((Get-AppWindow $a.Pid)[2])"
    }
    Close-AppWindow $a.Pid
    Start-Sleep -Seconds 1
    Write-Host "HIDDEN AGAIN=$(-not (Get-AppWindow $a.Pid)[2])"
    [void](Launch A)
    Start-Sleep -Seconds 1
    Write-Host "SECOND INSTANCE REOPEN=$((Get-AppWindow $a.Pid)[2])"

    $b = Launch B
    Enroll $b $true
    Tick $a
    & $dev add-addon "$lab\B\install" _forever_ LabAlpha alpha-other-same-label 1.0.0
    Tick $b
    Tick $a
    } else {
        $a = Launch A
        $b = Launch B
        Start-Sleep -Seconds 3
    }
    Use $a 'Needs attention*'
    Capture $a pending-attention
    Use $a 'LabAlpha: can*'
    Capture $a pending-decision
    Use $a 'Overview'
    [void](Stop-Launched $a)
    } else {
        $b = Launch B
        Start-Sleep -Seconds 3
    }

    & $dev add-addon "$lab\B\install" _forever_ LabBeta beta-three 3.0.0
    Start-Sleep -Seconds 1
    Tick $b
    Tick $b
    [void](Stop-Launched $b)
    $a = Launch A @('--crash-after','3')
    Start-Sleep -Seconds 4
    $journals = @(Get-ChildItem "$lab\A\install\_forever_\.wowsync\journal" -File)
    Write-Host "CRASH JOURNAL COUNT=$($journals.Count)"
    foreach ($j in $journals) { Write-Host "JOURNAL=$([IO.File]::ReadAllText($j.FullName))" }
    [void](Stop-Launched $a)
    if ($journals.Count -eq 0) { throw 'No interrupted journal was created by crash-after 3' }
    $clock = [Diagnostics.Stopwatch]::StartNew()
    $out = & $dev fake-client "$lab\A\install\_forever_" 2>&1
    $clock.Stop()
    $fakeId = [int]($out | Select-Object -First 1)
    $fake = Get-Record $fakeId
    Write-Host "PIPED CAPTURE MS=$($clock.ElapsedMilliseconds); STAND-IN PID=$fakeId"
    $a = Launch A
    Start-Sleep -Seconds 4
    Use $a 'Add-ons'
    Capture $a interrupted-list
    Use $a 'LabBeta*'
    Capture $a interrupted-addon
    [void](Stop-Launched $a)
    [void](Stop-Launched $fake)
    $fake = $null
    [IO.File]::WriteAllText("$lab\A\state\paused.txt",'damaged-synthetic-pause')
    $a = Launch A
    [void](Wait-For $a 'Resume syncing')
    Capture $a damaged-pause
    Use $a 'Resume syncing'
    Start-Sleep -Seconds 2
    Write-Host "DAMAGED PAUSE CLEARED=$(-not (Test-Path "$lab\A\state\paused.txt"))"
}
finally {
    if ($fake) { Write-Host "STOP STAND-IN $($fake.Pid)=$([bool](Stop-Launched $fake))" }
    foreach ($r in $roots) { Write-Host "STOP APP $($r.Pid)=$([bool](Stop-Launched $r))" }
}
