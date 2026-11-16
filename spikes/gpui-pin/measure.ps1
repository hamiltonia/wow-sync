# Memory harness (PLAN 6.6): launches the target, samples once per second the SUM of
# "Working Set - Private" over the target PID and ALL descendant processes (no trimming),
# plus commit/working set and per-PID GPU memory (reported separately). Writes a CSV.
# Usage: measure.ps1 -Scenario a -Variant destroy -Run 1 -Exe <path> -ExeArgs "hidden" -Seconds 120 -OutDir <dir> [-Hide sw]
param(
  [string]$Scenario, [string]$Variant = "", [int]$Run, [string]$Exe, [string]$ExeArgs = "",
  [int]$Seconds = 120, [string]$OutDir, [string]$Hide = "", [string]$Instance = ""
)
$ErrorActionPreference = 'Stop'
$tag = if ($Variant) { "$Scenario-$Variant" } else { $Scenario }
$csv = Join-Path $OutDir "$tag-run$Run.csv"
$log = Join-Path $OutDir "$tag-run$Run.log"
Remove-Item $log -ErrorAction SilentlyContinue
$env:WOWSYNC_LOG = $log
$env:WOWSYNC_INSTANCE = $Instance
if ($Hide) { $env:WOWSYNC_HIDE = $Hide } else { Remove-Item Env:WOWSYNC_HIDE -ErrorAction SilentlyContinue }
$ps = if ($ExeArgs) { Start-Process -FilePath $Exe -ArgumentList $ExeArgs -PassThru } else { Start-Process -FilePath $Exe -PassThru }
$root = $ps.Id
function Tree($rootId) {
  $all = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId
  $ids = [System.Collections.Generic.List[int]]::new(); $ids.Add($rootId)
  for ($i = 0; $i -lt $ids.Count; $i++) { foreach ($c in $all) { if ($c.ParentProcessId -eq $ids[$i] -and -not $ids.Contains([int]$c.ProcessId)) { $ids.Add([int]$c.ProcessId) } } }
  $ids
}
$rows = @("t_s,nproc,private_ws_sum,private_bytes_sum,working_set_sum,gpu_dedicated,gpu_shared")
$sw = [Diagnostics.Stopwatch]::StartNew()
for ($t = 0; $t -le $Seconds; $t++) {
  $target = $t * 1000
  while ($sw.ElapsedMilliseconds -lt $target) { Start-Sleep -Milliseconds 20 }
  if ($ps.HasExited) { $rows += "$t,0,EXITED,,,,"; break }
  $ids = Tree $root
  $pw = 0; $pb = 0; $ws = 0; $ded = 0; $shr = 0
  foreach ($id in $ids) {
    $p = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process -Filter "IDProcess=$id"
    if ($p) { $pw += [int64]$p.WorkingSetPrivate; $pb += [int64]$p.PrivateBytes; $ws += [int64]$p.WorkingSet }
    $g = Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUProcessMemory -Filter "Name LIKE 'pid_${id}_%'" -ErrorAction SilentlyContinue
    $ded += [int64](($g | Measure-Object DedicatedUsage -Sum).Sum); $shr += [int64](($g | Measure-Object SharedUsage -Sum).Sum)
  }
  $rows += "$t,$($ids.Count),$pw,$pb,$ws,$ded,$shr"
}
if (-not $ps.HasExited) { Stop-Process -Id $root }
[void]$ps.WaitForExit(10000)
Start-Sleep -Seconds 3   # let the single-instance mutex disappear before the next launch
$rows | Set-Content $csv
Remove-Item Env:WOWSYNC_LOG, Env:WOWSYNC_INSTANCE
"$csv done"
