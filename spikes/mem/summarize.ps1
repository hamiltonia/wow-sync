# Summarises evidence CSVs: per scenario/run, min/median/max of private_ws (MiB) over the whole run,
# the final-window steady state (last 60 s) and peak, plus GPU memory and concurrent-build count.
param([string]$Dir = "$PSScriptRoot\..\..\docs\runs\run-001\evidence\S-mem")
function Med($a) { $s = $a | Sort-Object; $s[[int][math]::Floor(($s.Count - 1) / 2)] }
foreach ($f in Get-ChildItem $Dir -Filter *.csv | Sort-Object Name) {
  $rows = Import-Csv $f | Where-Object { $_.private_ws -ne 'EXITED' }
  $exited = (Import-Csv $f | Where-Object { $_.private_ws -eq 'EXITED' }).Count
  $pw = $rows | % { [double]$_.private_ws / 1MB }
  $tail = $rows | Select-Object -Last 60 | % { [double]$_.private_ws / 1MB }
  $pb = ($rows | Select-Object -Last 60 | % { [double]$_.private_bytes / 1MB } | Measure-Object -Average).Average
  $ws = ($rows | Select-Object -Last 60 | % { [double]$_.working_set / 1MB } | Measure-Object -Average).Average
  $gd = ($rows | Select-Object -Last 1).gpu_dedicated / 1MB
  $gs = ($rows | Select-Object -Last 1).gpu_shared / 1MB
  $busy = ($rows | ? { [int]$_.other_cargo_rustc -gt 0 }).Count
  '{0,-20} n={1,3} exited={2} min={3:N1} med={4:N1} max={5:N1} | last60 min={6:N1} med={7:N1} max={8:N1} | privBytes={9:N0} WS={10:N0} gpuDed={11:N1} gpuShr={12:N1} buildSamples={13}' -f $f.BaseName, $rows.Count, $exited, ($pw | Measure-Object -Minimum).Minimum, (Med $pw), ($pw | Measure-Object -Maximum).Maximum, ($tail | Measure-Object -Minimum).Minimum, (Med $tail), ($tail | Measure-Object -Maximum).Maximum, $pb, $ws, $gd, $gs, $busy
}
