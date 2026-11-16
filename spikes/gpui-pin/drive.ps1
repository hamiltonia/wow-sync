# External lifecycle driver for spike-gpui-pin (out-of-process observation via user32).
# Usage: powershell -NoProfile -File drive.ps1 [-Exe target\release\spike-gpui-pin.exe]
# Every process it starts is stopped by PID before it exits.
param([string]$Exe = "$PSScriptRoot\target\release\spike-gpui-pin.exe")
Add-Type @'
using System; using System.Collections.Generic; using System.Runtime.InteropServices; using System.Text;
public static class W {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32")] static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32", CharSet=CharSet.Unicode)] static extern int GetClassNameW(IntPtr h, StringBuilder s, int n);
  [DllImport("user32")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32")] public static extern bool PostMessageW(IntPtr h, uint m, IntPtr w, IntPtr l);
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
  foreach ($w in [W]::Windows($pid_)) { $f = $w.Split('|'); if ($f[1] -eq 'Zed::Window') { return [pscustomobject]@{H=[IntPtr][int64]$f[0]; Cls=$f[1]; Vis=($f[2] -eq 'True')} } }
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
function Wait-NoWindow($pid_, $ms = 3000) {
  $sw = [Diagnostics.Stopwatch]::StartNew()
  while ($sw.ElapsedMilliseconds -lt $ms) { if (-not (Main $pid_)) { return $true }; Start-Sleep -Milliseconds 50 }
  return $false
}
function Wait-Exit($p, $ms = 3000) { [void]$p.WaitForExit($ms); return $p.HasExited }
$procName = [IO.Path]::GetFileNameWithoutExtension($Exe)
$log = "$PSScriptRoot\drive.log"; Remove-Item $log -ErrorAction SilentlyContinue
$env:WOWSYNC_LOG = $log

Stamp "=== Phase 1: mode normal (close -> hide) ==="
$p = Start-Process $Exe -PassThru
Stamp "launched pid=$($p.Id)"
Start-Sleep -Seconds 4
$m = Main $p.Id; $t = Tray $p.Id
Stamp "windows: $(([W]::Windows($p.Id)) -join ' ; ')"
Stamp "L1 start: main window class=$($m.Cls) visible=$($m.Vis) (expect True); tray window found=$($t -ne [IntPtr]::Zero)"
[void][W]::PostMessageW($m.H, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
Stamp "L2 WM_CLOSE -> hidden=$(Wait-Vis $p.Id $false); process alive=$(-not $p.HasExited)"
[void][W]::PostMessageW($t, 6002, [IntPtr]1, [IntPtr]0x202)
Stamp "L3 tray click #1 -> shown=$(Wait-Vis $p.Id $true)"
[void][W]::PostMessageW($t, 6002, [IntPtr]1, [IntPtr]0x202)
Stamp "L4 tray click #2 -> hidden=$(Wait-Vis $p.Id $false)"
$p2 = Start-Process $Exe -PassThru
$ex = Wait-Exit $p2 5000
Stamp "L5 second instance exited=$ex code=$($p2.ExitCode); first -> shown=$(Wait-Vis $p.Id $true); first alive=$(-not $p.HasExited)"
$cnt = @(Get-Process $procName -ErrorAction SilentlyContinue).Count
Stamp "L6 single instance: $procName process count=$cnt (expect 1)"
# Quit through the tray menu: muda posts WM_COMMAND with the internal item id to the tray window.
foreach ($id in 1000, 1001, 1, 2) {
  if ($p.HasExited) { break }
  [void][W]::PostMessageW($t, 0x0111, [IntPtr]$id, [IntPtr]0)
  Start-Sleep -Milliseconds 600
  Stamp "  WM_COMMAND id=$id -> exited=$($p.HasExited) windowVisible=$((Main $p.Id).Vis)"
}
Stamp "L7 quit via tray menu command -> process exited=$(Wait-Exit $p 3000)"
if (-not $p.HasExited) { Stop-Process -Id $p.Id; Stamp "(stopped test process by PID)" }

Stamp "=== Phase 2: mode destroy (close really destroys; keep-alive with no window) ==="
$p = Start-Process $Exe -ArgumentList 'destroy' -PassThru
Start-Sleep -Seconds 4
$m = Main $p.Id; $t = Tray $p.Id
Stamp "K1 start: visible=$($m.Vis)"
[void][W]::PostMessageW($m.H, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
Start-Sleep -Milliseconds 800; Stamp "K2 windows after close: $(([W]::Windows($p.Id)) -join ' ; ')"; Stamp "K2 WM_CLOSE -> window gone=$(Wait-NoWindow $p.Id); process alive after 3 s=$(-not (Wait-Exit $p 3000)); tray window still present=$((Tray $p.Id) -ne [IntPtr]::Zero)"
$t = Tray $p.Id
[void][W]::PostMessageW($t, 6002, [IntPtr]1, [IntPtr]0x202)
Stamp "K3 tray click with no window -> new window shown=$(Wait-Vis $p.Id $true)"
[void][W]::PostMessageW((Main $p.Id).H, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
Stamp "K4 close again -> gone=$(Wait-NoWindow $p.Id)"
$p3 = Start-Process $Exe -PassThru
[void](Wait-Exit $p3 5000)
Stamp "K5 second instance while no window -> first shows window=$(Wait-Vis $p.Id $true); first alive=$(-not $p.HasExited)"
$ws = (Get-Process -Id $p.Id).WorkingSet64 / 1MB; Stamp ("working set MiB={0:N1} (informational only)" -f $ws)
Stop-Process -Id $p.Id
Stamp "stopped test process pid=$($p.Id)"
Start-Sleep -Seconds 2

Stamp "=== Phase 3: mode hidden (no window ever, tray only) ==="
$p = Start-Process $Exe -ArgumentList 'hidden' -PassThru
Start-Sleep -Seconds 4
Stamp "H1 windows: $(([W]::Windows($p.Id)) -join ' ; ') ; main window=$([bool](Main $p.Id)); alive=$(-not $p.HasExited)"
$t = Tray $p.Id
[void][W]::PostMessageW($t, 6002, [IntPtr]1, [IntPtr]0x202)
Stamp "H2 tray click -> window shown=$(Wait-Vis $p.Id $true)"
Stop-Process -Id $p.Id
Stamp "stopped test process pid=$($p.Id)"
Remove-Item Env:WOWSYNC_LOG
"--- app log ---"; Get-Content $log
