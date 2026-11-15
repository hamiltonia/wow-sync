# S-mem harness: launches a target, samples PID once per second, writes CSV.
# Usage: measure.ps1 -Scenario a -Run 1 -Exe <path> -ExeArgs "hidden" -Seconds 120 [-OutDir <dir>] [-Trim]
# Columns: t_s, private_ws, private_bytes, working_set (bytes), gpu_dedicated, gpu_shared, other_procs
# "Private resident" = Win32_PerfFormattedData_PerfProc_Process.WorkingSetPrivate:
# physical pages in the process working set that are not shareable (not mapped
# DLL/image/section pages shared with other processes). PrivateBytes = commit.
param(
  [string]$Scenario, [int]$Run, [string]$Exe, [string]$ExeArgs = "",
  [int]$Seconds = 120, [string]$OutDir = "$PSScriptRoot\..\..\docs\runs\run-001\evidence\S-mem",
  [switch]$Trim
)
$ErrorActionPreference = 'Stop'
if ($Trim) {
  Add-Type -Namespace W -Name P -MemberDefinition '[DllImport("psapi.dll")] public static extern bool EmptyWorkingSet(IntPtr h);'
}
$name = [IO.Path]::GetFileNameWithoutExtension($Exe)
$suffix = if ($Trim) { "-trim" } else { "" }
$csv = Join-Path $OutDir "$Scenario$suffix-run$Run.csv"
$log = Join-Path $OutDir "$Scenario$suffix-run$Run.log"
$ps = if ($ExeArgs) { Start-Process -FilePath $Exe -ArgumentList $ExeArgs -PassThru -RedirectStandardOutput $log } else { Start-Process -FilePath $Exe -PassThru -RedirectStandardOutput $log }
$id = $ps.Id
$rows = @("t_s,private_ws,private_bytes,working_set,gpu_dedicated,gpu_shared,other_cargo_rustc")
$sw = [Diagnostics.Stopwatch]::StartNew()
for ($t = 0; $t -le $Seconds; $t++) {
  $target = $t * 1000
  while ($sw.ElapsedMilliseconds -lt $target) { Start-Sleep -Milliseconds 20 }
  if ($ps.HasExited) { $rows += "$t,EXITED,,,,,"; break }
  if ($Trim -and $t -ge 1) { [void][W.P]::EmptyWorkingSet($ps.Handle) }
  $p = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process -Filter "IDProcess=$id"
  $g = Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUProcessMemory -Filter "Name LIKE 'pid_${id}_%'" -ErrorAction SilentlyContinue
  $ded = ($g | Measure-Object DedicatedUsage -Sum).Sum
  $shr = ($g | Measure-Object SharedUsage -Sum).Sum
  $busy = @(Get-Process cargo, rustc -ErrorAction SilentlyContinue).Count
  $rows += "$t,$($p.WorkingSetPrivate),$($p.PrivateBytes),$($p.WorkingSet),$([int64]$ded),$([int64]$shr),$busy"
}
if (-not $ps.HasExited) { Stop-Process -Id $id }
$rows | Set-Content $csv
"$csv done"
