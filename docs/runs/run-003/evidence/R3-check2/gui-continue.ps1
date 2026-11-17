param([int]$ComputerA, [int]$ComputerB, [string]$LabName = 'gui3')
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path "$PSScriptRoot\..\..\..\..\..").Path
. "$repo\tools\mem\MemLib.ps1"
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$lab = "$repo\target\rc2-scratch\$LabName"
$a = Get-Record $ComputerA
$b = Get-Record $ComputerB
foreach ($r in @($a,$b)) {
    if ((Get-Process -Id $r.Pid).Path -ne "$repo\target\release\wow-sync.exe") {
        throw 'The previously recorded app launch no longer has the expected executable'
    }
}
function Elements {
    $w = Get-AppWindow $a.Pid
    $root = [Windows.Automation.AutomationElement]::FromHandle([IntPtr]$w[0])
    $root.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition)
}
function Use($name) {
    $e = Elements | Where-Object { $_.Current.Name -eq $name -and $_.Current.ControlType -eq [Windows.Automation.ControlType]::Button } | Select-Object -First 1
    if (!$e) { throw "Missing $name" }
    if ($e.GetSupportedPatterns().Id -contains [Windows.Automation.TogglePattern]::Pattern.Id) {
        $e.GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Toggle()
    } else {
        $e.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
    }
    Start-Sleep -Seconds 1
}
function Dump($label) {
    Write-Host "--- $label ---"
    foreach ($e in (Elements)) { if ($e.Current.Name) { Write-Host "$($e.Current.ControlType.ProgrammaticName): $($e.Current.Name)" } }
}
try {
    & "$repo\target\release\devkit.exe" add-addon "$lab\B\install" _forever_ RC2Alpha alpha-two 2.0.0
    Start-Sleep -Seconds 10
    Use Overview
    Dump 'running client blocks pending update'
    Write-Host "LOCAL VERSION=$([IO.File]::ReadAllText("$lab\A\install\_forever_\Interface\AddOns\RC2Alpha\RC2Alpha.toc"))"
    Write-Host "PEER VERSION=$([IO.File]::ReadAllText("$lab\B\install\_forever_\Interface\AddOns\RC2Alpha\RC2Alpha.toc"))"
    Use Add-ons
    Dump 'blocked add-on'
    Use 'App settings'
    Use 'Closing the window quits WoW Sync'
    Write-Host "PERSISTED CLOSE=$([IO.File]::ReadAllText("$lab\A\state\app-settings.txt"))"
    Close-AppWindow $a.Pid
    Start-Sleep -Seconds 3
    Write-Host "CLOSING QUITS EXITED=$(-not (Test-Alive $a))"
    Write-Host 'No command was sent to the stand-in client; its PID was not delivered by the hanging native capture.'
}
finally {
    [void](Stop-Launched $a)
    [void](Stop-Launched $b)
}
