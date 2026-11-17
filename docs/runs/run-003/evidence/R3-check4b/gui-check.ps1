param([string]$Lab = 'D:\WoWSyncLab\R3-check4b\gui',[switch]$ContinueRecovery)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path "$PSScriptRoot\..\..\..\..\..").Path
Set-Location $repo
. "$repo\tools\mem\MemLib.ps1"
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class Check4bCapture {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X,Y; }
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out Rect r);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref Point p);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
}
'@
[void][Check4bCapture]::SetThreadDpiAwarenessContext([IntPtr](-4))
$app = "$repo\target\release\wow-sync.exe"
$dev = "$repo\target\release\devkit.exe"
$launched = [Collections.Generic.List[object]]::new()
$results = [Collections.Generic.List[object]]::new()
$lostStandInPid = $false
function Save-Ledger {
    [IO.File]::WriteAllText("$PSScriptRoot\process-ledger.json",(ConvertTo-Json -InputObject $launched.ToArray()))
}
function Register-Own($p,$kind) {
    $r = [pscustomobject]@{Kind=$kind;Pid=$p.Id;Created=$p.StartTime.Ticks;Status='launched'}
    $launched.Add($r)
    Save-Ledger
    Write-Host "LAUNCH $kind captured PID=$($r.Pid)"
    $r
}
function Check($name,$condition,$detail) {
    $verdict = if ($condition) { 'PASS' } else { 'FAIL' }
    $results.Add([pscustomobject]@{check=$name;verdict=$verdict;detail=$detail})
    Write-Host "$verdict $name : $detail"
}
function Launch($side,[string[]]$extra=@()) {
    $args_ = @('--dev','--install',"$lab\$side\install",'--provider',"$lab\provider",'--state',"$lab\$side\state",'--name',"Lab-$side",'--interval-ms','60000','--recent-ms','100') + $extra
    $p = Start-Process $app -ArgumentList $args_ -PassThru
    $r = Register-Own $p "app-$side"
    if ($p.WaitForExit(300)) { throw "Own app exited early: $($p.ExitCode)" }
    $r
}
function Start-Fake {
    $output_ = & $dev fake-client "$lab\A\install\_forever_" 2>&1
    $fakeId = 0
    if (![int]::TryParse([string]($output_ | Select-Object -First 1),[ref]$fakeId)) {
        $script:lostStandInPid = $true
        throw 'Stand-in PID not captured. Do not discover or stop it by name or traversal.'
    }
    Register-Own (Get-Process -Id $fakeId) stand-in
}
function Stop-Own($r) {
    if (!$r) { return }
    $p = Get-Process -Id $r.Pid -ErrorAction SilentlyContinue
    if ($p -and $p.StartTime.Ticks -eq $r.Created) {
        Stop-Process -Id $r.Pid
        if ($p.WaitForExit(10000)) { $r.Status='stopped'; Write-Host "STOP captured PID=$($r.Pid)" }
        else { $r.Status='still-running' }
    } elseif ($r.Status -eq 'launched') { $r.Status='exited-or-identity-changed' }
    Save-Ledger
}
function Elements($r) {
    $w = Get-AppWindow $r.Pid
    if ($w) {
        $root = [Windows.Automation.AutomationElement]::FromHandle([IntPtr]$w[0])
        $root.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition)
    }
}
function Wait-For($r,$name) {
    for ($i=0; $i -lt 100; $i++) {
        foreach ($e in (Elements $r)) { if ($e.Current.Name -like $name) { return $e } }
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
    } else { $e.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke() }
    Start-Sleep -Milliseconds 600
    Write-Host "INVOKE $name"
}
function Capture($r,$label) {
    $w = Get-AppWindow $r.Pid
    [void][Check4bCapture]::SetForegroundWindow([IntPtr]$w[0])
    Start-Sleep -Seconds 1
    $lines = @(foreach ($e in (Elements $r)) {
        if ($e.Current.Name) { "$($e.Current.ControlType.ProgrammaticName): $($e.Current.Name)" }
    })
    [IO.File]::WriteAllLines("$PSScriptRoot\$label.uia.txt",[string[]]$lines)
    $rect = [Check4bCapture+Rect]::new()
    [void][Check4bCapture]::GetClientRect([IntPtr]$w[0],[ref]$rect)
    $origin = [Check4bCapture+Point]::new()
    [void][Check4bCapture]::ClientToScreen([IntPtr]$w[0],[ref]$origin)
    $bmp = [Drawing.Bitmap]::new($rect.Right-$rect.Left,$rect.Bottom-$rect.Top)
    $g = [Drawing.Graphics]::FromImage($bmp)
    try {
        $g.CopyFromScreen($origin.X,$origin.Y,0,0,$bmp.Size)
        $bmp.Save("$PSScriptRoot\$label.png",[Drawing.Imaging.ImageFormat]::Png)
    } finally { $g.Dispose(); $bmp.Dispose() }
    Write-Host "CAPTURE $label (body text requires visual review)"
    $lines | ForEach-Object { Write-Host $_ }
}
function Tick($r) { Use $r 'Sync now'; Start-Sleep -Seconds 3 }
function Enroll($r,$joining) {
    foreach ($n in @('Next','Use this folder','Next','Next','This is my*','These are the*','Next')) { Use $r $n }
    if ($joining) { Use $r 'Join My add-ons*' } else { Use $r 'Create a new group' }
    foreach ($n in @('Next','Set up this computer','Finish and open*')) { Use $r $n }
    Tick $r
}
function Beta-Hash($side) {
    $dir = "$lab\$side\install\_forever_\Interface\AddOns\LabBeta"
    $parts = @(foreach ($f in (Get-ChildItem $dir -File -Recurse | Sort-Object FullName)) {
        "$($f.FullName.Substring($dir.Length + 1))|$($f.Length)|$((Get-FileHash $f.FullName).Hash)"
    })
    $sha = [Security.Cryptography.SHA256]::Create()
    try { [BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($parts -join "`n"))).Replace('-','') }
    finally { $sha.Dispose() }
}
function Beta-Version($side) {
    $toc = [IO.File]::ReadAllText("$lab\$side\install\_forever_\Interface\AddOns\LabBeta\LabBeta.toc")
    [regex]::Match($toc,'## Version: ([^\r\n]+)').Groups[1].Value
}
try {
    if (!$ContinueRecovery) {
    if (Test-Path $lab) { throw 'Refusing to overwrite an existing synthetic lab; choose a fresh Lab.' }
    New-Item -ItemType Directory -Force "$lab\A\state","$lab\B\state","$lab\provider" | Out-Null
    & $dev make-install "$lab\A\install"
    & $dev add-addon "$lab\A\install" _forever_ LabBeta beta-one 1.0.0
    & $dev make-install "$lab\B\install"
    $a = Launch A
    Enroll $a $false
    $b = Launch B
    Enroll $b $true
    Tick $b
    Stop-Own $a
    & $dev add-addon "$lab\B\install" _forever_ LabBeta beta-three 3.0.0
    Start-Sleep -Seconds 1
    Tick $b
    Tick $b
    Stop-Own $b
    $a = Launch A @('--crash-after','3')
    [void](Wait-For $a 'Resume syncing')
    $journals = @(Get-ChildItem "$lab\A\install\_forever_\.wowsync\journal" -File)
    if ($journals.Count -ne 1) { throw 'Expected one journal in original RC3-1 regression.' }
    $raw = [IO.File]::ReadAllText($journals[0].FullName)
    [IO.File]::WriteAllText("$PSScriptRoot\original-crash-journal.json",$raw)
    Check 'Original crash oracle' (($raw | ConvertFrom-Json).phase -eq 'staging' -and !$raw.Contains('halted_by_game')) 'Crash happens with no stand-in; staging journal has no recorded cause.'
    Stop-Own $a
    $fake = Start-Fake
    $a = Launch A
    [void](Wait-For $a 'Resume syncing')
    Use $a 'Add-ons'
    Use $a 'LabBeta*'
    Capture $a original-running
    Send-Wheel $a.Pid -900
    Start-Sleep -Seconds 1
    Capture $a original-running-lower
    Stop-Own $a
    Stop-Own $fake
    $a = Launch A
    [void](Wait-For $a 'Resume syncing')
    Use $a 'Add-ons'
    Use $a 'LabBeta*'
    Capture $a original-stopped
    Use $a 'Overview'
    Use $a 'Resume syncing'
    Start-Sleep -Seconds 3
    Tick $a
    Check 'Original update finishes' ((Beta-Version A) -eq '3.0.0' -and (Beta-Hash A) -eq (Beta-Hash B)) 'After resume, Beta 3.0.0 bytes match the provider peer.'
    Stop-Own $a
    & $dev add-addon "$lab\B\install" _forever_ LabBeta beta-five 5.0.0
    $b = Launch B
    Tick $b
    Tick $b
    Stop-Own $b
    $a = Launch A @('--crash-after','4')
    [void](Wait-For $a 'Resume syncing')
    }
    $journals = @(Get-ChildItem "$lab\A\install\_forever_\.wowsync\journal" -File)
    if ($journals.Count -ne 1) { throw 'Expected one journal in RC4-1 recovery sequence.' }
    $journalPath = $journals[0].FullName
    $before = [IO.File]::ReadAllText($journalPath)
    $beforeJournalHash = (Get-FileHash $journalPath).Hash
    $beforeLive = Beta-Hash A
    [IO.File]::WriteAllText("$PSScriptRoot\recovery-before-journal.json",$before)
    Check 'Recovery crash oracle' (($before | ConvertFrom-Json).phase -eq 'committing' -and !$before.Contains('halted_by_game')) 'Crash with no stand-in leaves a committing journal with unknown cause.'
    Stop-Own $a
    $fake = Start-Fake
    $a = Launch A
    [void](Wait-For $a 'Resume syncing')
    Use $a 'Resume syncing'
    Start-Sleep -Seconds 4
    $after = [IO.File]::ReadAllText($journalPath)
    $afterJournalHash = (Get-FileHash $journalPath).Hash
    $waitingLive = Beta-Hash A
    [IO.File]::WriteAllText("$PSScriptRoot\recovery-after-journal.json",$after)
    Check 'Recovery retains unknown cause' (!$after.Contains('halted_by_game') -and $beforeJournalHash -eq $afterJournalHash) 'Resume while the stand-in runs leaves the journal byte-identical with no cause field.'
    Check 'Recovery does not apply while game runs' ($beforeLive -eq $waitingLive -and (Beta-Version A) -eq '3.0.0') 'Existing Beta 3.0.0 stays unchanged while the stand-in runs.'
    Use $a 'Add-ons'
    Use $a 'LabBeta*'
    Capture $a recovery-running-after-resume
    Stop-Own $fake
    Use $a 'Overview'
    Tick $a
    $remaining = @(Get-ChildItem "$lab\A\install\_forever_\.wowsync\journal" -File)
    $finalLive = Beta-Hash A
    $peerLive = Beta-Hash B
    $finalVersion = Beta-Version A
    Check 'Recovery finishes after game stops' ($remaining.Count -eq 0 -and $finalVersion -eq '5.0.0' -and $finalLive -eq $peerLive) 'After stopping the captured stand-in and clicking Sync now, journal is removed and Beta 5.0.0 matches B.'
    [IO.File]::WriteAllText("$PSScriptRoot\recovery-oracle.json",([ordered]@{
        BeforeJournalHash=$beforeJournalHash;WaitingJournalHash=$afterJournalHash
        BeforeLiveHash=$beforeLive;WaitingLiveHash=$waitingLive
        FinalLiveHash=$finalLive;PeerLiveHash=$peerLive
        FinalVersion=$finalVersion;RemainingJournals=$remaining.Count
        Retry='GUI Sync now after stand-in stopped'
    } | ConvertTo-Json))
    Use $a 'Add-ons'
    Use $a 'LabBeta*'
    Capture $a recovery-finished
}
finally {
    foreach ($r in $launched) { Stop-Own $r }
    $left = @(foreach ($r in $launched) {
        $p = Get-Process -Id $r.Pid -ErrorAction SilentlyContinue
        if ($p -and $p.StartTime.Ticks -eq $r.Created) { $r.Pid }
    })
    [IO.File]::WriteAllText("$PSScriptRoot\cleanup.json",([ordered]@{RemainingCapturedPids=$left;LostStandInPid=$lostStandInPid} | ConvertTo-Json))
    [IO.File]::WriteAllText("$PSScriptRoot\gui-results.json",(ConvertTo-Json -InputObject $results.ToArray()))
    Write-Host "REMAINING CAPTURED PROCESSES=$($left.Count); LOST STAND-IN PID=$lostStandInPid"
}
if (@($results | Where-Object verdict -eq FAIL).Count) { exit 1 }
