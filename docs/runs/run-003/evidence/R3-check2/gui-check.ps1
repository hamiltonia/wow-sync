param([switch]$InspectOnly, [string]$LabName = 'gui')
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path "$PSScriptRoot\..\..\..\..\..").Path
Set-Location $repo
. "$repo\tools\mem\MemLib.ps1"
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$app = "$repo\target\release\wow-sync.exe"
$dev = "$repo\target\release\devkit.exe"
$lab = "$repo\target\rc2-scratch\$LabName"
$roots = [Collections.Generic.List[object]]::new()
$fake = $null
function Launch-App($side) {
    $args_ = @('--dev','--install',"$lab\$side\install",'--provider',"$lab\provider",'--state',"$lab\$side\state",'--name',"RC2-$side",'--interval-ms','1000','--recent-ms','100')
    $p = Start-Process $app -ArgumentList $args_ -PassThru
    if ($p.WaitForExit(200)) {
        Write-Host "LAUNCH $side PID=$($p.Id) exited=$($p.ExitCode)"
        return $null
    }
    $r = Get-Record $p.Id
    $roots.Add($r)
    Write-Host "LAUNCH $side PID=$($r.Pid)"
    $r
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
function Dump($r,$label) {
    Write-Host "--- $label ---"
    $ids = @()
    foreach ($side in @('A','B')) {
        $file = "$lab\$side\state\computer.json"
        if (Test-Path $file) { $ids += ([IO.File]::ReadAllText($file) | ConvertFrom-Json).computer_id }
    }
    $leaks = 0
    foreach ($e in (Elements $r)) {
        if ($e.Current.Name) {
            foreach ($id in $ids) { if ($e.Current.Name.Contains($id)) { $leaks++ } }
            Write-Host "$($e.Current.ControlType.ProgrammaticName): $($e.Current.Name) [focus=$($e.Current.HasKeyboardFocus)]"
        }
    }
    Write-Host "VISIBLE COMPUTER ID MATCHES=$leaks"
}
function Enroll($r,$joining) {
    foreach ($n in @('Next','Use this folder','Next','Next','This is my*','These are the*','Next')) { Use $r $n }
    Dump $r 'group choices'
    if ($joining) { Use $r 'Join My add-ons*' } else { Use $r 'Create a new group' }
    foreach ($n in @('Next','Set up this computer','Finish and open*')) { Use $r $n }
    [void](Wait-For $r 'Sync now')
    Start-Sleep -Seconds 4
}
try {
    New-Item -ItemType Directory -Force "$lab\A\state","$lab\B\state","$lab\provider" | Out-Null
    & $dev make-install "$lab\A\install"
    & $dev add-addon "$lab\A\install" _forever_ RC2Alpha alpha-one 1.0.0
    & $dev add-addon "$lab\A\install" _forever_ RC2Beta beta-one 1.0.0
    & $dev make-install "$lab\B\install"
    $a = Launch-App A
    Enroll $a $false
    Dump $a Overview
    foreach ($page in @('Add-ons','Computers','Activity','App settings')) {
        Use $a $page
        Dump $a $page
    }
    if ($InspectOnly) { return }
    Use $a 'Pause syncing'
    Write-Host "PAUSE FILE=$([IO.File]::ReadAllText("$lab\A\state\paused.txt"))"
    [void](Stop-Launched $a)
    $a = Launch-App A
    [void](Wait-For $a 'Resume syncing')
    Dump $a 'paused after restart'
    Use $a 'Resume syncing'
    Use $a 'App settings'
    $closeSetting = Wait-For $a 'Closing the window quits WoW Sync'
    Write-Host "CLOSING DEFAULT=$($closeSetting.GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Current.ToggleState)"
    foreach ($theme in @('Dark','Light','Follow system')) {
        Use $a "Theme: $theme*"
        Write-Host "THEME SELECTED=$((Wait-For $a "Theme: $theme, selected").Current.Name)"
    }
    Use $a 'Explain close-to-tray again next time'
    $opener = Wait-For $a 'Explain close-to-tray again next time'
    $opener.SetFocus()
    Start-Sleep -Milliseconds 400
    Close-AppWindow $a.Pid
    [void](Wait-For $a 'Hide window')
    Dump $a 'first close'
    Send-Key $a.Pid 0x09
    Start-Sleep -Milliseconds 300
    Send-Key $a.Pid 0x1B
    Start-Sleep -Seconds 1
    Write-Host "FOCUS RETURN=$((Wait-For $a 'Explain close-to-tray again next time').Current.HasKeyboardFocus)"
    Close-AppWindow $a.Pid
    Use $a 'Hide window'
    Write-Host "CLOSE HIDDEN=$(-not (Get-AppWindow $a.Pid)[2]); ALIVE=$(Test-Alive $a)"
    $show = Launch-App A
    Start-Sleep -Seconds 2
    Write-Host "SECOND INSTANCE REOPEN=$((Get-AppWindow $a.Pid)[2])"
    $b = Launch-App B
    Enroll $b $true
    Use $a 'Computers'
    Dump $a 'two computers'
    Use $a 'Add-ons'
    Dump $a 'addons after peer'
    Use $a 'RC2Alpha*'
    Dump $a 'addon detail'
    $start = [Diagnostics.ProcessStartInfo]::new($dev)
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.ArgumentList.Add('fake-client')
    $start.ArgumentList.Add("$lab\A\install\_forever_")
    $helper = [Diagnostics.Process]::Start($start)
    $fakeId = [int]$helper.StandardOutput.ReadLine()
    if (!$helper.WaitForExit(5000)) { throw "fake-client helper did not exit; tracked helper PID=$($helper.Id)" }
    $fake = Get-Record $fakeId
    Write-Host "FAKE CLIENT PID=$fakeId"
    & $dev add-addon "$lab\B\install" _forever_ RC2Alpha alpha-two 2.0.0
    Start-Sleep -Seconds 8
    Use $a Overview
    Dump $a 'WoW running'
    Write-Host "BLOCKED LOCAL TOC=$([IO.File]::ReadAllText("$lab\A\install\_forever_\Interface\AddOns\RC2Alpha\RC2Alpha.toc"))"
    [void](Stop-Launched $fake)
    $fake = $null
    Start-Sleep -Seconds 8
    Dump $a 'after client stops'
    Write-Host "APPLIED LOCAL TOC=$([IO.File]::ReadAllText("$lab\A\install\_forever_\Interface\AddOns\RC2Alpha\RC2Alpha.toc"))"
    Use $a 'App settings'
    Use $a 'Closing the window quits WoW Sync'
    Start-Sleep -Seconds 1
    Close-AppWindow $a.Pid
    Start-Sleep -Seconds 3
    Write-Host "CLOSING QUITS EXITED=$(-not (Test-Alive $a))"
    Write-Host "STATE FILES:"
    Get-ChildItem "$lab\A\state","$lab\B\state","$lab\provider" -Recurse -File | ForEach-Object { Write-Host $_.FullName.Substring($lab.Length) }
}
finally {
    if ($fake) { [void](Stop-Launched $fake) }
    foreach ($r in $roots) { [void](Stop-Launched $r) }
}
