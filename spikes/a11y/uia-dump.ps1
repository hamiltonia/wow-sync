# Launches a target app, dumps its UIA tree, then sends Tab keys and records UIA focus
# (plus a screenshot per step for the GPUI targets). Output: text to stdout.
# Usage: powershell -NoProfile -File uia-dump.ps1 -Target gpui|gpui-main|baseline [-OutDir <dir>]
param([ValidateSet('gpui', 'gpui-main', 'baseline')][string]$Target = 'gpui', [string]$OutDir = '.')
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Windows.Forms, System.Drawing
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
switch ($Target) {
    'gpui' { $proc = Start-Process (Join-Path $here 'target\debug\spike-a11y.exe') -PassThru }
    'gpui-main' { $proc = Start-Process (Join-Path $here '..\a11y-main\target\debug\spike-a11y-main.exe') -PassThru }
    'baseline' { $proc = Start-Process powershell -ArgumentList '-NoProfile', '-File', (Join-Path $here 'baseline-form.ps1') -PassThru }
}
Start-Sleep -Seconds 5
if ($Target -eq 'baseline') {
    $proc = Get-Process | Where-Object { $_.MainWindowTitle -eq 'Baseline' } | Select-Object -First 1
} else { $proc.Refresh() }
$hwnd = $proc.MainWindowHandle
Add-Type 'using System;using System.Runtime.InteropServices;public class FG{[DllImport("user32.dll")]public static extern bool SetForegroundWindow(IntPtr h);[DllImport("user32.dll")]public static extern void keybd_event(byte a,byte b,uint f,UIntPtr e);}'
[FG]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); [void][FG]::SetForegroundWindow($hwnd); [FG]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 500
$AE = [System.Windows.Automation.AutomationElement]
$root = $AE::FromHandle($hwnd)

function Dump($el, $depth) {
    $c = $el.Current
    $pats = ($el.GetSupportedPatterns() | ForEach-Object { $_.ProgrammaticName -replace 'Identifiers.Pattern', '' }) -join ','
    "{0}[{1}] name='{2}' class='{3}' autoId='{4}' focusable={5} enabled={6} patterns=[{7}]" -f ('  ' * $depth), ($c.ControlType.ProgrammaticName -replace 'ControlType\.', ''), $c.Name, $c.ClassName, $c.AutomationId, $c.IsKeyboardFocusable, $c.IsEnabled, $pats
    if ($depth -lt 8) {
        foreach ($k in $el.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)) { Dump $k ($depth + 1) }
    }
}
"=== Target: $Target  $(Get-Date -Format s) ==="
"=== UIA raw tree ==="
Dump $root 0
$all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
"descendant count: $($all.Count)"

"=== Focus after Tab presses (UIA FocusedElement) ==="
function Shot($n) {
    $r = $root.Current.BoundingRectangle
    $bmp = New-Object System.Drawing.Bitmap ([int]$r.Width), ([int]$r.Height)
    $g = [System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen([int]$r.X, [int]$r.Y, 0, 0, $bmp.Size)
    $bmp.Save((Join-Path $OutDir "$Target-tab$n.png")); $g.Dispose(); $bmp.Dispose()
}
for ($i = 0; $i -le 6; $i++) {
    if ($i -gt 0) { [System.Windows.Forms.SendKeys]::SendWait('{TAB}'); Start-Sleep -Milliseconds 400 }
    $fc = $AE::FocusedElement.Current
    "tab#{0}: focused type={1} name='{2}' class='{3}' pid={4}" -f $i, ($fc.ControlType.ProgrammaticName -replace 'ControlType\.', ''), $fc.Name, $fc.ClassName, $fc.ProcessId
    if ($Target -ne 'baseline' -and $OutDir -ne '.') { Shot $i }
}
$proc.Kill()
