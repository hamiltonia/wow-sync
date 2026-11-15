# Keyboard-only check of the GPUI spike: type, Tab order, Enter/Space activation, Shift+Tab.
# Saves a screenshot after each step into -OutDir. Run from any directory.
param([string]$OutDir = '.')
Add-Type -AssemblyName System.Windows.Forms, System.Drawing, UIAutomationClient, UIAutomationTypes
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$p = Start-Process (Join-Path $here 'target\debug\spike-a11y.exe') -PassThru
Start-Sleep -Seconds 4; $p.Refresh()
$r = [System.Windows.Automation.AutomationElement]::FromHandle($p.MainWindowHandle).Current.BoundingRectangle
function Step($name, $keys) {
    [System.Windows.Forms.SendKeys]::SendWait($keys); Start-Sleep -Milliseconds 500
    $b = New-Object System.Drawing.Bitmap ([int]$r.Width), ([int]$r.Height)
    $g = [System.Drawing.Graphics]::FromImage($b); $g.CopyFromScreen([int]$r.X, [int]$r.Y, 0, 0, $b.Size)
    $b.Save((Join-Path $OutDir "kbd-$name.png")); $b.Dispose(); $g.Dispose()
    "step $name keys=$keys"
}
Step '1-type' 'hi'
Step '2-tab-save-enter' '{TAB}{ENTER}'
Step '3-tab-cancel-space' '{TAB} '
Step '4-tab-toggle-space' '{TAB} '
Step '5-shift-tab-back-to-cancel' '+{TAB}'
$p.Kill()
