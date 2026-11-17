# Drives the real app through UI Automation and posted key messages, to its own window only.
# Usage: powershell -File ui.ps1 -Exe X -Out DIR -AppArgs "..." -Steps "click:Next;shot:a;uia:a;key:Esc;focus:after"
# Steps: click:<name part>  key:<Tab|Esc|Enter|Space|Down|Up|Left|Right|Home|End>  shot:<file>
#        uia:<file>  focus:<label>  wait:<ms>  close (WM_CLOSE)  has:<name part> (prints found/missing)
param([string]$Exe, [string]$Out, [string]$AppArgs = "", [string]$Steps = "", [int]$Wait = 3, [switch]$KeepRunning)
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @"
using System; using System.Runtime.InteropServices;
public class W3 {
  [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Rt, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
"@
[void][W3]::SetProcessDPIAware()
$vk = @{ Tab = 0x09; Enter = 0x0D; Esc = 0x1B; Space = 0x20; Left = 0x25; Up = 0x26; Right = 0x27; Down = 0x28; Home = 0x24; End = 0x23 }
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$p = Start-Process -FilePath $Exe -ArgumentList $AppArgs -PassThru
$h = [IntPtr]::Zero
for ($i = 0; $i -lt 40 -and $h -eq [IntPtr]::Zero; $i++) { Start-Sleep -Milliseconds 250; $p.Refresh(); $h = $p.MainWindowHandle }
Start-Sleep -Seconds $Wait
"pid=$($p.Id)"
function Root { [System.Windows.Automation.AutomationElement]::FromHandle($script:h) }
function All { (Root).FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition) }
function Find([string]$part) {
  foreach ($e in (All)) { if ($e.Current.Name -eq $part -and $e.Current.IsEnabled) { return $e } }
  foreach ($e in (All)) { if ($e.Current.IsKeyboardFocusable -and $e.Current.Name -like "*$part*" -and $e.Current.IsEnabled) { return $e } }
  foreach ($e in (All)) { if ($e.Current.Name -like "*$part*" -and $e.Current.IsEnabled) { return $e } }
  return $null
}
try {
  foreach ($s in ($Steps -split ';' | Where-Object { $_ })) {
    $kind, $arg = $s.Split(':', 2)
    switch ($kind) {
      'click' {
        $e = Find $arg
        if ($null -eq $e) { "click '$arg': NOT FOUND"; break }
        $pat = $null
        if ($e.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pat)) { $pat.Invoke(); "click '$arg': invoked" }
        elseif ($e.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$pat)) { $pat.Toggle(); "click '$arg': toggled" }
        else { $e.SetFocus(); [void][W3]::PostMessage($h, 0x100, [IntPtr]0x20, [IntPtr]1); [void][W3]::PostMessage($h, 0x101, [IntPtr]0x20, [IntPtr]0xC0000001); "click '$arg': focus+space" }
        Start-Sleep -Milliseconds 700
      }
      'key' {
        $code = $vk[$arg]
        [void][W3]::PostMessage($h, 0x100, [IntPtr]$code, [IntPtr]1)
        [void][W3]::PostMessage($h, 0x101, [IntPtr]$code, [IntPtr]0xC0000001)
        Start-Sleep -Milliseconds 500
      }
      'wait' { Start-Sleep -Milliseconds ([int]$arg) }
      'close' { [void][W3]::PostMessage($h, 0x10, [IntPtr]0, [IntPtr]0); Start-Sleep -Milliseconds 700 }
      'has' { $e = Find $arg; if ($e) { "has '$arg': found ($($e.Current.Name))" } else { "has '$arg': missing" } }
      'focus' {
        $f = $null
        foreach ($x in (All)) { if ($x.Current.HasKeyboardFocus) { $f = $x; break } }
        if ($null -eq $f) { "focus[$arg]: none in app window" } else { "focus[$arg]: name='$($f.Current.Name)' type=$($f.Current.LocalizedControlType)" }
      }
      'shot' {
        $r = New-Object W3+R; [void][W3]::GetWindowRect($h, [ref]$r)
        $bmp = New-Object System.Drawing.Bitmap ($r.Rt - $r.L), ($r.B - $r.T)
        $g = [System.Drawing.Graphics]::FromImage($bmp); $dc = $g.GetHdc()
        [void][W3]::PrintWindow($h, $dc, 2); $g.ReleaseHdc($dc); $g.Dispose()
        $bmp.Save((Join-Path $Out "$arg.png"), [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
        "shot $arg"
      }
      'uia' {
        $lines = foreach ($e in (All)) {
          $c = $e.Current
          "{0} | name='{1}' | focusable={2} | focused={3} | enabled={4}" -f $c.LocalizedControlType, $c.Name, $c.IsKeyboardFocusable, $c.HasKeyboardFocus, $c.IsEnabled
        }
        [System.IO.File]::WriteAllLines((Join-Path $Out "$arg.uia.txt"), [string[]]$lines)
        "uia $arg elements=$(@($lines).Count)"
      }
    }
  }
  $p.Refresh()
  "private MiB: {0:N1}" -f ((Get-Process -Id $p.Id).PrivateMemorySize64 / 1MB)
} finally {
  if (-not $KeepRunning) { Stop-Process -Id $p.Id -ErrorAction SilentlyContinue }
}
