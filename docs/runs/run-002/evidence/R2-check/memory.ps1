param([int]$FirstRun=1,[int]$LastRun=3)
$ErrorActionPreference = 'Stop'
$Here=$PSScriptRoot
$Exe="$Here\clean\target\release\wow-sync.exe"
$Lab="$Here\lab"
New-Item -ItemType Directory -Force "$Here\memory" | Out-Null
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class MemoryWindow {
 public delegate bool EnumProc(IntPtr h, IntPtr p);
 [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb,IntPtr p);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
}
'@
function Owned-Ids([int]$RootId) {
    $all=Get-CimInstance Win32_Process -Property ProcessId,ParentProcessId,CreationDate
    $ids=[Collections.Generic.List[int]]::new()
    $ids.Add($RootId)
    for($i=0;$i -lt $ids.Count;$i++) {
        $parent=$all | Where-Object ProcessId -eq $ids[$i] | Select-Object -First 1
        foreach($child in $all) {
            if($child.ParentProcessId -eq $ids[$i] -and $parent -and
               $child.CreationDate -ge $parent.CreationDate -and
               -not $ids.Contains([int]$child.ProcessId)) {
                $ids.Add([int]$child.ProcessId)
            }
        }
    }
    $ids
}
$summary=@()
for($run=$FirstRun;$run -le $LastRun;$run++) {
    $owned=@()
    $env:WOWSYNC_LOG="$Here\memory\run$run.log"
    $p=Start-Process $Exe -ArgumentList @('--dev','--install',"$Lab\A\install",'--provider',"$Lab\provider",'--state',"$Lab\A\state",'--name','Check-A','--hide-after-secs','3','--interval-ms','5000','--recent-ms','600') -PassThru
    try {
        Start-Sleep 8
        $p.Refresh()
        if($p.HasExited) { throw 'Memory application exited at startup' }
        $script:visible=0
        $script:appPid=$p.Id
        $callback=[MemoryWindow+EnumProc]{
            param($h,$param)
            [uint32]$owner=0
            [void][MemoryWindow]::GetWindowThreadProcessId($h,[ref]$owner)
            if($owner -eq $script:appPid -and [MemoryWindow]::IsWindowVisible($h)) { $script:visible++ }
            return $true
        }
        [void][MemoryWindow]::EnumWindows($callback,[IntPtr]::Zero)
        if($script:visible -ne 0) { throw 'Application has a visible top-level window' }
        $rows=@()
        $watch=[Diagnostics.Stopwatch]::StartNew()
        for($t=0;$t -le 60;$t++) {
            while($watch.ElapsedMilliseconds -lt ($t*1000)) { Start-Sleep -Milliseconds 15 }
            $p.Refresh()
            if($p.HasExited) { throw 'Application exited during sampling' }
            $owned=@(Owned-Ids $p.Id)
            [long]$private=0; [long]$commit=0; [long]$working=0
            foreach($ownedPid in $owned) {
                $counter=Get-CimInstance Win32_PerfFormattedData_PerfProc_Process -Filter "IDProcess=$ownedPid"
                if(-not $counter) { throw "No private working set counter for owned PID $ownedPid" }
                $private += [long]$counter.WorkingSetPrivate
                $commit += [long]$counter.PrivateBytes
                $working += [long]$counter.WorkingSet
            }
            $rows += [pscustomobject]@{t_s=$t;elapsed_s=[math]::Round($watch.Elapsed.TotalSeconds,2);processes=$owned.Count;owned_pids=($owned -join ';');private_ws_sum=$private;private_bytes_sum=$commit;working_set_sum=$working}
        }
        $rows | Export-Csv "$Here\memory\run$run.csv" -NoTypeInformation
        $max=($rows | Measure-Object private_ws_sum -Maximum).Maximum
        $avg=($rows | Measure-Object private_ws_sum -Average).Average
        $summary += [pscustomobject]@{run=$run;samples=$rows.Count;visible_windows=$script:visible;max_processes=($rows | Measure-Object processes -Maximum).Maximum;max_private_mib=[math]::Round($max/1MB,3);average_private_mib=[math]::Round($avg/1MB,3);pass=($max -le 100MB)}
    } finally {
        # This application starts no child processes. Never terminate inferred descendants:
        # historical ParentProcessId values can name a recycled PID on a shared machine.
        if(-not $p.HasExited) { Stop-Process -Id $p.Id -ErrorAction SilentlyContinue }
        Start-Sleep 2
    }
}
$summary | ConvertTo-Json | Set-Content "$Here\memory\summary-$FirstRun-$LastRun.json"
$summary | Format-Table
Remove-Item Env:WOWSYNC_LOG -ErrorAction SilentlyContinue
