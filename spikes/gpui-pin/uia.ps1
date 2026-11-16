# UIA check for spike-gpui-pin: dumps the tree, verifies the labelled controls, exercises
# Invoke/Toggle/SetFocus/Tab. Stops the app it started by PID.
# Usage: powershell -NoProfile -File uia.ps1 [-Exe <path>]
param([string]$Exe = "$PSScriptRoot\target\release\spike-gpui-pin.exe")
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Windows.Forms
Add-Type 'using System;using System.Runtime.InteropServices;public class FG{[DllImport("user32.dll")]public static extern bool SetForegroundWindow(IntPtr h);[DllImport("user32.dll")]public static extern void keybd_event(byte a,byte b,uint f,UIntPtr e);}'
$p = Start-Process $Exe -PassThru
try {
  Start-Sleep -Seconds 5; $p.Refresh()
  $hwnd = $p.MainWindowHandle
  "main window handle: $hwnd"
  [FG]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); [void][FG]::SetForegroundWindow($hwnd); [FG]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 500
  $AE = [System.Windows.Automation.AutomationElement]
  $root = $AE::FromHandle($hwnd)
  function Dump($el, $depth) {
    $c = $el.Current
    $pats = ($el.GetSupportedPatterns() | ForEach-Object { $_.ProgrammaticName -replace 'Identifiers.Pattern', '' }) -join ','
    "{0}[{1}] name='{2}' class='{3}' focusable={4} enabled={5} patterns=[{6}]" -f ('  ' * $depth), ($c.ControlType.ProgrammaticName -replace 'ControlType\.', ''), $c.Name, $c.ClassName, $c.IsKeyboardFocusable, $c.IsEnabled, $pats
    if ($depth -lt 8) { foreach ($k in $el.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)) { Dump $k ($depth + 1) } }
  }
  "=== UIA raw tree ==="
  Dump $root 0
  $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
  "descendant count: $($all.Count)"
  function Find($name) {
    $c = New-Object System.Windows.Automation.PropertyCondition ($AE::NameProperty), $name
    $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $c)
  }
  function Check($label, $el, $type) {
    if ($el) { "{0}: {1} found type={2} (expect {3})" -f $(if (($el.Current.ControlType.ProgrammaticName -replace 'ControlType\.', '') -eq $type) { 'PASS' } else { 'FAIL' }), $label, ($el.Current.ControlType.ProgrammaticName -replace 'ControlType\.', ''), $type }
    else { "FAIL: $label not found" }
  }
  "=== Labelled controls ==="
  Check 'heading' (Find 'WoW Sync Pin Heading') 'Text'
  Check 'button Sync now' (Find 'Sync now (0)') 'Button'
  Check 'button Settings' (Find 'Settings (0)') 'Button'
  Check 'toggle' (Find 'Enable sync') 'CheckBox'
  Check 'list' (Find 'Items') 'List'
  $n = 0; 1..10 | ForEach-Object { if (Find "List item $_") { $n++ } }
  "{0}: list items found {1}/10" -f $(if ($n -eq 10) { 'PASS' } else { 'FAIL' }), $n
  function Try-Do($label, [scriptblock]$sb) { try { "$label -> " + (& $sb) } catch { "$label -> EXCEPTION $($_.Exception.Message)" } }
  "=== Patterns ==="
  Try-Do 'Invoke Sync now' { (Find 'Sync now (0)').GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke(); Start-Sleep -Milliseconds 600; 'label now present: ' + ((@('Sync now (0)','Sync now (1)') | % { if (Find $_) { $_ } }) -join ',') }
  $tg = Find 'Enable sync'
  Try-Do 'Toggle state before' { $tg.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Current.ToggleState }
  Try-Do 'Toggle()' { $tg.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle(); Start-Sleep -Milliseconds 600; 'state after: ' + (Find 'Enable sync').GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Current.ToggleState }
  Try-Do 'SetFocus List item 5' { (Find 'List item 5').SetFocus(); Start-Sleep -Milliseconds 400; 'focused: ' + $AE::FocusedElement.Current.Name }
  "=== Focus after Tab presses (UIA FocusedElement) ==="
  for ($i = 1; $i -le 4; $i++) {
    [System.Windows.Forms.SendKeys]::SendWait('{TAB}'); Start-Sleep -Milliseconds 400
    $fc = $AE::FocusedElement.Current
    "tab#{0}: focused type={1} name='{2}' pid={3}" -f $i, ($fc.ControlType.ProgrammaticName -replace 'ControlType\.', ''), $fc.Name, $fc.ProcessId
  }
} finally { if (-not $p.HasExited) { Stop-Process -Id $p.Id } }
