# External lifecycle driver for spike-gpui (out-of-process observation via user32).
# Usage: powershell -File drive.ps1 [-Exe target\release\spike-gpui.exe]
param([string]$Exe = "$PSScriptRoot\target\release\spike-gpui.exe")
Add-Type @'
using System; using System.Collections.Generic; using System.Runtime.InteropServices; using System.Text;
public static class W {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32")] static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32", CharSet=CharSet.Unicode)] static extern int GetClassNameW(IntPtr h, StringBuilder s, int n);
  [DllImport("user32")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32")] public static extern bool PostMessageW(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32")] public static extern IntPtr FindWindowExW(IntPtr parent, IntPtr after, string cls, string title);
  public static List<string> Windows(uint pid) {
    var r = new List<string>();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid) { var sb = new StringBuilder(256); GetClassNameW(h, sb, 256);
        r.Add(h.ToInt64() + "|" + sb + "|" + IsWindowVisible(h)); } return true; }, IntPtr.Zero);
    return r;
  }
}
'@
function Stamp($m) { "{0} {1}" -f (Get-Date -Format "HH:mm:ss.fff"), $m }
function Main($pid_) {
  foreach ($w in [W]::Windows($pid_)) { $f = $w.Split('|'); if ($f[1] -like 'Zed*') { return [pscustomobject]@{H=[IntPtr][int64]$f[0]; Cls=$f[1]; Vis=($f[2] -eq 'True')} } }
}
function Tray($pid_) {
  foreach ($w in [W]::Windows($pid_)) { $f = $w.Split('|'); if ($f[1] -eq 'tray_icon_app') { return [IntPtr][int64]$f[0] } }
  [IntPtr]::Zero
}
function Wait-Vis($pid_, $want, $ms = 3000) {
  $sw = [Diagnostics.Stopwatch]::StartNew()
  while ($sw.ElapsedMilliseconds -lt $ms) { $m = Main $pid_; if ($m -and $m.Vis -eq $want) { return $true }; Start-Sleep -Milliseconds 50 }
  return $false
}
$p = Start-Process $Exe -PassThru -RedirectStandardOutput "$PSScriptRoot\drive.out"
Stamp "launched pid=$($p.Id)"
Start-Sleep -Seconds 4
$m = Main $p.Id; $t = Tray $p.Id
Stamp "windows: $(([W]::Windows($p.Id)) -join ' ; ')"
Stamp "main window class=$($m.Cls) visible=$($m.Vis) (expect True); tray hidden window found=$($t -ne [IntPtr]::Zero)"
# close button path
[void][W]::PostMessageW($m.H, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
Stamp "WM_CLOSE -> hidden=$(Wait-Vis $p.Id $false); process alive=$(-not $p.HasExited)"
# tray left click (WM_USER+... 6002 with WM_LBUTTONUP)
[void][W]::PostMessageW($t, 6002, [IntPtr]1, [IntPtr]0x202)
Stamp "tray click #1 -> shown=$(Wait-Vis $p.Id $true)"
[void][W]::PostMessageW($t, 6002, [IntPtr]1, [IntPtr]0x202)
Stamp "tray click #2 -> hidden=$(Wait-Vis $p.Id $false)"
# second instance while hidden
$p2 = Start-Process $Exe -PassThru -RedirectStandardOutput "$PSScriptRoot\drive2.out"
[void]$p2.WaitForExit(5000)
Stamp "second instance exited=$($p2.HasExited) code=$($p2.ExitCode); first -> shown=$(Wait-Vis $p.Id $true); first alive=$(-not $p.HasExited)"
$procs = @(Get-Process spike-gpui -ErrorAction SilentlyContinue).Count
Stamp "spike-gpui process count=$procs (expect 1)"
$ws = (Get-Process -Id $p.Id).WorkingSet64 / 1MB; Stamp ("working set MiB={0:N1} (informational only)" -f $ws)
Stop-Process -Id $p.Id
Stamp "stopped test process"
