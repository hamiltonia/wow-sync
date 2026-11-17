# A-29 memory harness library (run-003, R3-mem). Dot-source this file.
#
# Safety by construction (run-002 incident V1: a harness stopped unrelated processes found by
# ParentProcessId traversal after PIDs were recycled):
#   * A process is "app-owned" only if it is the launched root (same PID *and* same creation
#     time as recorded at launch) or a descendant whose ParentProcessId names an already-owned
#     process and whose creation time is LATER than that parent's. Re-validated every sample.
#   * Only the launched root is ever stopped, after re-validating PID + creation time.
#     Children are never stopped; survivors are reported.
$ErrorActionPreference = 'Stop'

if (-not ('MemWin' -as [type])) {
    Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class MemWin {
  public delegate bool EnumProc(IntPtr h, IntPtr p);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr p);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  // Top-level windows owned by the given PID: handle, title, visible flag.
  public static List<object[]> Windows(uint pid) {
    var res = new List<object[]>();
    EnumWindows((h, p) => {
      uint o; GetWindowThreadProcessId(h, out o);
      if (o == pid) {
        var sb = new StringBuilder(256); GetWindowTextW(h, sb, 256);
        res.Add(new object[] { h, sb.ToString(), IsWindowVisible(h) });
      }
      return true;
    }, IntPtr.Zero);
    return res;
  }
  public static int[] Center(IntPtr h) { RECT r; GetWindowRect(h, out r); return new int[] { (r.L + r.R) / 2, (r.T + r.B) / 2 }; }
}
'@
}

# ---- process identity -------------------------------------------------------------------------

function Get-ProcTable {
    # Snapshot of all processes: Pid, Ppid, Created (ticks).
    foreach ($p in (Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, CreationDate)) {
        [pscustomobject]@{ Pid = [int]$p.ProcessId; Ppid = [int]$p.ParentProcessId; Created = [long]$p.CreationDate.Ticks }
    }
}

function Get-OwnedSet {
    # Pure function over a process table. $Root = @{Pid; Created} recorded at launch.
    # Returns owned members as @{Pid; Created} (empty if the root is gone or the PID was reused).
    param($Table, $Root)
    $byPid = @{}
    foreach ($r in $Table) { $byPid[$r.Pid] = $r }
    $rootRow = $byPid[[int]$Root.Pid]
    if (-not $rootRow -or $rootRow.Created -ne [long]$Root.Created) { return @() }
    $owned = @{}
    $owned[[int]$Root.Pid] = [long]$Root.Created
    $grew = $true
    while ($grew) {
        $grew = $false
        foreach ($r in $Table) {
            if ($owned.Contains($r.Pid)) { continue }
            if ($owned.Contains($r.Ppid) -and $r.Created -gt $owned[$r.Ppid]) {
                $owned[$r.Pid] = $r.Created
                $grew = $true
            }
        }
    }
    foreach ($k in $owned.Keys) { [pscustomobject]@{ Pid = $k; Created = $owned[$k] } }
}

function Get-Record([int]$ProcId) {
    # Identity record of a process we just launched (PID + creation time from the OS).
    $row = Get-ProcTable | Where-Object Pid -eq $ProcId
    if (-not $row) { throw "launched process $ProcId not found" }
    [pscustomobject]@{ Pid = $ProcId; Created = $row.Created }
}

function Test-Alive($Rec) {
    $row = Get-ProcTable | Where-Object Pid -eq $Rec.Pid
    [bool]($row -and $row.Created -eq $Rec.Created)
}

function Stop-Launched($Rec, [int]$WaitSeconds = 10) {
    # Stops only the launched root, and only if PID and creation time still match.
    if (Test-Alive $Rec) {
        $id = [int]$Rec.Pid
        Stop-Process -Id $id -ErrorAction SilentlyContinue
        $sw = [Diagnostics.Stopwatch]::StartNew()
        while ((Test-Alive $Rec) -and $sw.Elapsed.TotalSeconds -lt $WaitSeconds) { Start-Sleep -Milliseconds 200 }
    }
    -not (Test-Alive $Rec)
}

# ---- counters ---------------------------------------------------------------------------------

function Get-Counters($Owned) {
    # Sum of private working set / private bytes / working set over the owned set, plus GPU memory
    # filtered by PID, reported separately.
    [long]$priv = 0; [long]$pb = 0; [long]$ws = 0; [long]$gd = 0; [long]$gs = 0
    $gpu = @(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUProcessMemory -ErrorAction SilentlyContinue)
    foreach ($o in $Owned) {
        $c = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process -Filter "IDProcess=$($o.Pid)"
        if (-not $c) { return $null }
        $priv += [long]$c.WorkingSetPrivate; $pb += [long]$c.PrivateBytes; $ws += [long]$c.WorkingSet
        foreach ($g in $gpu) {
            if ($g.Name -like "pid_$($o.Pid)_*") { $gd += [long]$g.DedicatedUsage; $gs += [long]$g.SharedUsage }
        }
    }
    [pscustomobject]@{ private_ws = $priv; private_bytes = $pb; working_set = $ws; gpu_dedicated = $gd; gpu_shared = $gs }
}

# ---- sampling ---------------------------------------------------------------------------------

function New-Run { [pscustomobject]@{ Rows = [Collections.Generic.List[object]]::new(); Seen = @{}; Clock = [Diagnostics.Stopwatch]::StartNew() } }

function Invoke-Phase {
    # Samples once per second for $Seconds. $Tick (optional) is called with the second index.
    param($Run, $Root, [string]$Phase, [int]$Seconds, [scriptblock]$Tick = $null)
    $sw = [Diagnostics.Stopwatch]::StartNew()
    for ($t = 0; $t -lt $Seconds; $t++) {
        while ($sw.ElapsedMilliseconds -lt ($t * 1000)) { Start-Sleep -Milliseconds 20 }
        if ($Tick) { & $Tick $t }
        $owned = @(Get-OwnedSet (Get-ProcTable) $Root)
        if ($owned.Count -eq 0) { throw "app root exited or was replaced during phase $Phase" }
        foreach ($o in $owned) { $Run.Seen["$($o.Pid)/$($o.Created)"] = $o }
        $c = Get-Counters $owned
        if (-not $c) { continue }
        # drop the sample if the root identity changed while counters were read
        if (-not (Test-Alive $Root)) { throw "app root exited during sample in phase $Phase" }
        $Run.Rows.Add([pscustomobject]@{
                t_s = [math]::Round($Run.Clock.Elapsed.TotalSeconds, 1); phase = $Phase; procs = $owned.Count
                private_ws_b = $c.private_ws; private_bytes_b = $c.private_bytes; working_set_b = $c.working_set
                gpu_dedicated_b = $c.gpu_dedicated; gpu_shared_b = $c.gpu_shared
            })
    }
}

function Save-Csv($Run, [string]$Path) {
    $sb = [Text.StringBuilder]::new()
    [void]$sb.AppendLine('t_s,phase,procs,private_ws_b,private_bytes_b,working_set_b,gpu_dedicated_b,gpu_shared_b')
    foreach ($r in $Run.Rows) {
        [void]$sb.AppendLine(('{0},{1},{2},{3},{4},{5},{6},{7}' -f $r.t_s, $r.phase, $r.procs, $r.private_ws_b, $r.private_bytes_b, $r.working_set_b, $r.gpu_dedicated_b, $r.gpu_shared_b))
    }
    [IO.File]::WriteAllText($Path, $sb.ToString(), [Text.UTF8Encoding]::new($false))
}

# ---- app control (windows of the tracked root only) -------------------------------------------

function Get-AppWindow([int]$ProcId) {
    foreach ($w in [MemWin]::Windows([uint32]$ProcId)) { if ($w[1] -like '*WoW Sync') { return $w } }
    $null
}

function Close-AppWindow([int]$ProcId) {
    $w = Get-AppWindow $ProcId
    if ($w) { [void][MemWin]::PostMessageW([IntPtr]$w[0], 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) }   # WM_CLOSE -> app hides
}

function Send-Key([int]$ProcId, [int]$Vk) {
    # Posted key (down+up) to the tracked root's window; GPUI activates the focused control on Enter/Space.
    $w = Get-AppWindow $ProcId
    if (-not $w) { return }
    [void][MemWin]::PostMessageW([IntPtr]$w[0], 0x0100, [IntPtr]$Vk, [IntPtr]1)
    [void][MemWin]::PostMessageW([IntPtr]$w[0], 0x0101, [IntPtr]$Vk, [IntPtr]0xC0000001)
}

function Send-Wheel([int]$ProcId, [int]$Delta) {
    $w = Get-AppWindow $ProcId
    if (-not $w) { return }
    $c = [MemWin]::Center([IntPtr]$w[0])
    $wp = [IntPtr](([int64]($Delta -band 0xFFFF)) -shl 16)
    $lp = [IntPtr](([int64]$c[1] -band 0xFFFF) -shl 16 -bor ([int64]$c[0] -band 0xFFFF))
    [void][MemWin]::PostMessageW([IntPtr]$w[0], 0x020A, $wp, $lp)   # WM_MOUSEWHEEL
}
