# Drives the prototype with posted key messages (to its own HWND only), then captures the window
# and optionally dumps the UI Automation tree.
# Usage: powershell -File drive.ps1 -Exe X -Out DIR -Name N -Args "--scenario S7" -Keys "Tab,Tab,Down" [-Uia]
param([string]$Exe, [string]$Out, [string]$Name, [string]$AppArgs = "", [string]$Keys = "", [switch]$Uia, [int]$Wait = 3)
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @"
using System; using System.Runtime.InteropServices;
public class W2 {
  [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Rt, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
}
"@
$vk = @{ Tab = 0x09; Enter = 0x0D; Esc = 0x1B; Space = 0x20; Left = 0x25; Up = 0x26; Right = 0x27; Down = 0x28; F6 = 0x75; Home = 0x24; End = 0x23 }
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$p = Start-Process -FilePath $Exe -ArgumentList $AppArgs -PassThru
$h = [IntPtr]::Zero
for ($i = 0; $i -lt 40 -and $h -eq [IntPtr]::Zero; $i++) { Start-Sleep -Milliseconds 250; $p.Refresh(); $h = $p.MainWindowHandle }
Start-Sleep -Seconds $Wait
try {
  foreach ($k in ($Keys -split ',' | Where-Object { $_ })) {
    if ($k.StartsWith("shift+")) {
      # Shift state is not carried by posted messages; reported as not exercised.
      "SKIP $k (modifier keys cannot be posted)"; continue
    }
    if ($vk.ContainsKey($k)) { $code = $vk[$k]; $ch = $null } else { $code = [int][char]$k.ToUpper(); $ch = [int][char]$k }
    [void][W2]::PostMessage($h, 0x100, [IntPtr]$code, [IntPtr]1)
    if ($null -ne $ch) { [void][W2]::PostMessage($h, 0x102, [IntPtr]$ch, [IntPtr]1) }
    [void][W2]::PostMessage($h, 0x101, [IntPtr]$code, [IntPtr]0xC0000001)
    Start-Sleep -Milliseconds 250
  }
  Start-Sleep -Milliseconds 600
  $r = New-Object W2+R; [void][W2]::GetWindowRect($h, [ref]$r)
  $bmp = New-Object System.Drawing.Bitmap ($r.Rt - $r.L), ($r.B - $r.T)
  $g = [System.Drawing.Graphics]::FromImage($bmp); $dc = $g.GetHdc()
  [void][W2]::PrintWindow($h, $dc, 2); $g.ReleaseHdc($dc); $g.Dispose()
  $bmp.Save((Join-Path $Out "$Name.png"), [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
  "captured $Name pid=$($p.Id)"
  if ($Uia) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
    $lines = foreach ($e in $all) {
      $c = $e.Current
      "{0} | name='{1}' | focusable={2} | focused={3} | enabled={4}" -f $c.LocalizedControlType, $c.Name, $c.IsKeyboardFocusable, $c.HasKeyboardFocus, $c.IsEnabled
    }
    [System.IO.File]::WriteAllLines((Join-Path $Out "$Name.uia.txt"), [string[]]$lines)
    "uia elements: $($all.Count)"
  }
  $m = (Get-Process -Id $p.Id).WorkingSet64 / 1MB
  "working-set MiB: {0:N1}" -f $m
} finally {
  Stop-Process -Id $p.Id
}
