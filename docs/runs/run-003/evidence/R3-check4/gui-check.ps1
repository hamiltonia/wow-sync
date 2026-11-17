param([string]$Lab = 'D:\WoWSyncLab\R3-check4\gui')
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path "$PSScriptRoot\..\..\..\..\..").Path
Set-Location $repo
. "$repo\tools\mem\MemLib.ps1"
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class Check4Capture {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X,Y; }
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out Rect r);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref Point p);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
}
'@
[void][Check4Capture]::SetThreadDpiAwarenessContext([IntPtr](-4))
$app = "$repo\target\release\wow-sync.exe"
$dev = "$repo\target\release\devkit.exe"
$launched = [Collections.Generic.List[object]]::new()
$results = [Collections.Generic.List[object]]::new()
function Check($name, $condition, $detail) {
    $verdict = if ($condition) { 'PASS' } else { 'FAIL' }
    $results.Add([pscustomobject]@{check=$name;verdict=$verdict;detail=$detail})
    Write-Host "$verdict $name : $detail"
}
function Review($name,$artifact,$expected) {
    $results.Add([pscustomobject]@{check=$name;verdict='NOT RUN';detail="Automated body-text assertion unavailable in GPUI UIA. Visually inspect $artifact.png for: $expected"})
    Write-Host "VISUAL REVIEW REQUIRED $name : $artifact.png : $expected"
}
function Launch($side, [string[]]$extra = @()) {
    $args_ = @('--dev','--install',"$lab\$side\install",'--provider',"$lab\provider",'--state',"$lab\$side\state",'--name',"Lab-$side",'--interval-ms','60000','--recent-ms','100') + $extra
    $p = Start-Process $app -ArgumentList $args_ -PassThru
    $r = [pscustomobject]@{Pid=$p.Id;Created=$p.StartTime.Ticks}
    $launched.Add($r)
    Write-Host "LAUNCH $side PID=$($p.Id)"
    if ($p.WaitForExit(300)) { throw "Own app exited early: $($p.ExitCode)" }
    $r
}
function Stop-Own($r) {
    if (!$r) { return }
    $p = Get-Process -Id $r.Pid -ErrorAction SilentlyContinue
    if ($p -and $p.StartTime.Ticks -eq $r.Created) {
        Stop-Process -Id $r.Pid
        $p.WaitForExit()
        Write-Host "STOP captured PID=$($r.Pid)"
    }
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
    throw "Missing own-window UIA element: $name"
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
    [void][Check4Capture]::SetForegroundWindow([IntPtr]$w[0])
    Start-Sleep -Seconds 1
    $lines = @(foreach ($e in (Elements $r)) {
        if ($e.Current.Name) { "$($e.Current.ControlType.ProgrammaticName): $($e.Current.Name)" }
    })
    [IO.File]::WriteAllLines("$PSScriptRoot\$label.uia.txt",[string[]]$lines)
    $rect = [Check4Capture+Rect]::new()
    [void][Check4Capture]::GetClientRect([IntPtr]$w[0],[ref]$rect)
    $origin = [Check4Capture+Point]::new()
    [void][Check4Capture]::ClientToScreen([IntPtr]$w[0],[ref]$origin)
    $bmp = [Drawing.Bitmap]::new($rect.Right-$rect.Left,$rect.Bottom-$rect.Top)
    $g = [Drawing.Graphics]::FromImage($bmp)
    try {
        $g.CopyFromScreen($origin.X,$origin.Y,0,0,$bmp.Size)
        $bmp.Save("$PSScriptRoot\$label.png",[Drawing.Imaging.ImageFormat]::Png)
    } finally { $g.Dispose(); $bmp.Dispose() }
    Write-Host "CAPTURE $label"
    $lines | ForEach-Object { Write-Host $_ }
    $lines -join "`n"
}
function Enroll($r,$joining) {
    foreach ($n in @('Next','Use this folder','Next','Next','This is my*','These are the*','Next')) { Use $r $n }
    if ($joining) { Use $r 'Join My add-ons*' } else { Use $r 'Create a new group' }
    foreach ($n in @('Next','Set up this computer','Finish and open*')) { Use $r $n }
    Tick $r
}
function Tick($r) { Use $r 'Sync now'; Start-Sleep -Seconds 3 }
try {
    if (Test-Path $lab) { throw 'Refusing to overwrite an existing synthetic lab; choose a fresh Lab.' }
    New-Item -ItemType Directory -Force "$lab\A\state","$lab\B\state","$lab\provider" | Out-Null
    & $dev make-install "$lab\A\install"
    & $dev add-addon "$lab\A\install" _forever_ LabAlpha alpha-one 1.0.0
    & $dev add-addon "$lab\A\install" _forever_ LabBeta beta-one 1.0.0
    & $dev make-install "$lab\B\install"
    $a = Launch A
    Enroll $a $false
    $b = Launch B
    Enroll $b $true
    Tick $a
    & $dev add-addon "$lab\B\install" _forever_ LabAlpha alpha-other-same-label 1.0.0
    Tick $b
    Tick $a
    Use $a 'Needs attention*'
    Use $a 'LabAlpha: can*'
    [void](Capture $a unstable-decision)
    Review 'RC3-2 candidate value' unstable-decision 'Unstable candidate explicitly not available in both sentence and card.'
    Use $a 'Overview'
    Stop-Own $a
    & $dev add-addon "$lab\B\install" _forever_ LabBeta beta-three 3.0.0
    Start-Sleep -Seconds 1
    Tick $b
    Tick $b
    Stop-Own $b
    $a = Launch A @('--crash-after','3')
    [void](Wait-For $a 'Resume syncing')
    Start-Sleep -Seconds 2
    $journals = @(Get-ChildItem "$lab\A\install\_forever_\.wowsync\journal" -File)
    if ($journals.Count -ne 1) { throw "Expected one real crash journal; found $($journals.Count)." }
    $journal = [IO.File]::ReadAllText($journals[0].FullName)
    [IO.File]::WriteAllText("$PSScriptRoot\crash-journal.json",$journal)
    Check 'Crash oracle' ($journal.Contains('"staging"') -and !$journal.Contains('halted_by_game')) 'Real injected-crash journal is staging, without recorded game cause.'
    Stop-Own $a
    $fakeOut = & $dev fake-client "$lab\A\install\_forever_" 2>&1
    $fakeId = [int]($fakeOut | Select-Object -First 1)
    $fp = Get-Process -Id $fakeId
    $fake = [pscustomobject]@{Pid=$fakeId;Created=$fp.StartTime.Ticks}
    $launched.Add($fake)
    Write-Host "LAUNCH stand-in captured PID=$fakeId (after crash)"
    $a = Launch A
    [void](Wait-For $a 'Resume syncing')
    Use $a 'Add-ons'
    Use $a 'LabBeta*'
    [void](Capture $a interrupted-running)
    Review 'RC3-1 running cause and action' interrupted-running 'Neutral crash wording; Close WoW to let the update finish.'
    Send-Wheel $a.Pid -900
    Start-Sleep -Seconds 1
    [void](Capture $a interrupted-running-lower)
    Stop-Own $a
    Stop-Own $fake
    $a = Launch A
    [void](Wait-For $a 'Resume syncing')
    [void](Capture $a paused-waiting)
    Review 'RC3-3 paused hint' paused-waiting '1 add-on waiting will be tried again when you resume; no promise of zero.'
    Use $a 'Add-ons'
    Use $a 'LabBeta*'
    [void](Capture $a interrupted-stopped)
    Review 'RC3-1 stopped cause and action' interrupted-stopped 'Neutral crash wording; next-sync recovery action, no request to close WoW.'
    Send-Wheel $a.Pid -900
    Start-Sleep -Seconds 1
    [void](Capture $a interrupted-stopped-lower)

    Use $a 'Overview'
    Use $a 'Resume syncing'
    Start-Sleep -Seconds 3
    Tick $a
    Stop-Own $a
    & $dev add-addon "$lab\B\install" _forever_ LabBeta beta-five 5.0.0
    $b = Launch B
    Tick $b
    Tick $b
    Stop-Own $b
    $a = Launch A @('--crash-after','4')
    [void](Wait-For $a 'Resume syncing')
    Start-Sleep -Seconds 2
    $committing = @(Get-ChildItem "$lab\A\install\_forever_\.wowsync\journal" -File)
    if ($committing.Count -ne 1) { throw 'Expected one journal in the recovery-cause probe' }
    $before = [IO.File]::ReadAllText($committing[0].FullName)
    [IO.File]::WriteAllText("$PSScriptRoot\recovery-before-journal.json",$before)
    Check 'Recovery probe crash oracle' (($before | ConvertFrom-Json).phase -eq 'committing' -and !$before.Contains('halted_by_game')) 'Crash before any stand-in exists leaves committing journal with unknown cause.'
    Stop-Own $a
    $fakeOut = & $dev fake-client "$lab\A\install\_forever_" 2>&1
    $fakeId = [int]($fakeOut | Select-Object -First 1)
    $fp = Get-Process -Id $fakeId
    $fake = [pscustomobject]@{Pid=$fakeId;Created=$fp.StartTime.Ticks}
    $launched.Add($fake)
    Write-Host "LAUNCH stand-in captured PID=$fakeId (after committing crash)"
    $a = Launch A
    [void](Wait-For $a 'Resume syncing')
    Use $a 'Add-ons'
    Use $a 'LabBeta*'
    [void](Capture $a recovery-before-resume)
    Use $a 'Overview'
    Use $a 'Resume syncing'
    Start-Sleep -Seconds 4
    $after = [IO.File]::ReadAllText($committing[0].FullName)
    [IO.File]::WriteAllText("$PSScriptRoot\recovery-after-journal.json",$after)
    Check 'Recovery retains original unknown crash cause' (!(($after | ConvertFrom-Json).halted_by_game)) 'A game already running before recovery must not turn a crash into evidence it originally started during the update.'
    Use $a 'Add-ons'
    Use $a 'LabBeta*'
    [void](Capture $a recovery-after-resume)
    Review 'Recovery cause after resume' recovery-after-resume 'Original interruption was a crash; wording must not claim WoW started during that update.'
}
finally {
    foreach ($r in $launched) { Stop-Own $r }
    $left = @(foreach ($r in $launched) {
        $p = Get-Process -Id $r.Pid -ErrorAction SilentlyContinue
        if ($p -and $p.StartTime.Ticks -eq $r.Created) { $r.Pid }
    })
    [IO.File]::WriteAllText("$PSScriptRoot\remaining-processes.json",(ConvertTo-Json -InputObject $left))
    [IO.File]::WriteAllText("$PSScriptRoot\gui-results.json",(ConvertTo-Json -InputObject $results.ToArray() -Depth 4))
    Write-Host "REMAINING CAPTURED PROCESSES=$($left.Count)"
}
if (@($results | Where-Object verdict -eq FAIL).Count) { exit 1 }
