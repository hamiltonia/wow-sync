# Exercises UIA patterns on the Zed-main (AccessKit) variant: Invoke, Toggle, Value read, SetFocus.
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$p = Start-Process (Join-Path $here '..\a11y-main\target\debug\spike-a11y-main.exe') -PassThru
Start-Sleep -Seconds 4; $p.Refresh()
$AE = [System.Windows.Automation.AutomationElement]
$root = $AE::FromHandle($p.MainWindowHandle)
function Find($name) {
    $c = New-Object System.Windows.Automation.PropertyCondition ($AE::NameProperty), $name
    $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $c)
}
function Try-Do($label, [scriptblock]$sb) { try { "$label -> " + (& $sb) } catch { "$label -> EXCEPTION $($_.Exception.Message)" } }
$field = Find 'Name'
Try-Do 'SetFocus Name' { $field.SetFocus(); 'ok' }
Try-Do 'Value before' { $field.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).Current.Value }
[System.Windows.Forms.Application] | Out-Null
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.SendKeys]::SendWait('hello'); Start-Sleep -Milliseconds 500
Try-Do 'Value after typing hello' { $field.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).Current.Value }
Try-Do 'ValuePattern.SetValue' { $field.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue('xyz'); Start-Sleep -Milliseconds 400; 'ok; now reads: ' + $field.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).Current.Value }
Try-Do 'Invoke Save' { (Find 'Save (0)').GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke(); Start-Sleep -Milliseconds 500; 'ok; element names now: ' + ((@('Save (0)','Save (1)') | % { if (Find $_) { $_ } }) -join ',') }
$tg = Find 'Enable sync'
Try-Do 'Toggle state before' { $tg.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Current.ToggleState }
Try-Do 'Toggle()' { $tg.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle(); Start-Sleep -Milliseconds 500; 'state after: ' + (Find 'Enable sync').GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Current.ToggleState }
Try-Do 'SetFocus List item 5' { (Find 'List item 5').SetFocus(); Start-Sleep -Milliseconds 400; 'focused: ' + $AE::FocusedElement.Current.Name }
$p.Kill()
