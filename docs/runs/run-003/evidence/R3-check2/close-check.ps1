$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path "$PSScriptRoot\..\..\..\..\..").Path
. "$repo\tools\mem\MemLib.ps1"
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$lab = "$repo\target\rc2-scratch\gui4"
$p = Start-Process "$repo\target\release\wow-sync.exe" -ArgumentList @('--dev','--install',"$lab\A\install",'--provider',"$lab\provider",'--state',"$lab\A\state",'--name','RC2-A','--interval-ms','60000') -PassThru
$rec = Get-Record $p.Id
Write-Host "CLOSE CHECK PID=$($rec.Pid)"
function Find($name) {
    $w = Get-AppWindow $rec.Pid
    $root = [Windows.Automation.AutomationElement]::FromHandle([IntPtr]$w[0])
    $root.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition) |
        Where-Object { $_.Current.Name -eq $name } | Select-Object -First 1
}
try {
    Start-Sleep -Seconds 3
    (Find 'App settings').GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
    Start-Sleep -Seconds 1
    $switch = Find 'Closing the window quits WoW Sync'
    Write-Host "INITIAL=$($switch.GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Current.ToggleState)"
    $switch.SetFocus()
    Start-Sleep -Milliseconds 500
    if ($switch.GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Current.ToggleState -eq [Windows.Automation.ToggleState]::On) {
        Send-Key $rec.Pid 0x0D
        Start-Sleep -Seconds 1
    }
    Write-Host "BEFORE UIA TOGGLE=$((Find 'Closing the window quits WoW Sync').GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Current.ToggleState)"
    Write-Host "OFFSCREEN=$($switch.Current.IsOffscreen); FOCUSED=$($switch.Current.HasKeyboardFocus); RECT=$($switch.Current.BoundingRectangle)"
    $switch.GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Toggle()
    Start-Sleep -Seconds 1
    Write-Host "AFTER UIA TOGGLE=$((Find 'Closing the window quits WoW Sync').GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Current.ToggleState)"
    if ((Find 'Closing the window quits WoW Sync').GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Current.ToggleState -eq [Windows.Automation.ToggleState]::Off) {
        Send-Key $rec.Pid 0x0D
    }
    Start-Sleep -Seconds 2
    Write-Host "KEYBOARD ENTER=$((Find 'Closing the window quits WoW Sync').GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Current.ToggleState)"
    Write-Host "PERSISTED=$([IO.File]::ReadAllText("$lab\A\state\app-settings.txt"))"
    Close-AppWindow $rec.Pid
    Start-Sleep -Seconds 3
    Write-Host "KEYBOARD ENABLED CLOSE QUITS=$(-not (Test-Alive $rec))"
    if (Test-Alive $rec) {
        $safe = Find 'Quit when safe'
        if ($safe) {
            $safe.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
            Start-Sleep -Seconds 2
            Write-Host "QUIT WHEN SAFE EXIT=$(-not (Test-Alive $rec))"
        }
    }
}
finally { [void](Stop-Launched $rec) }
