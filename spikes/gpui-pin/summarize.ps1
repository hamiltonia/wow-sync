# Summarises memory CSVs: per scenario/run, min/median/max of the summed private working set (MiB)
# over the whole run and the last 60 s (steady state), peak, plus GPU memory and process count.
param([string]$Dir)
function Med($a) { $s = $a | Sort-Object; $s[[int][math]::Floor(($s.Count - 1) / 2)] }
foreach ($f in Get-ChildItem $Dir -Filter *.csv | Sort-Object Name) {
  $all = Import-Csv $f.FullName
  $rows = $all | Where-Object { $_.private_ws_sum -ne 'EXITED' }
  $exited = ($all | Where-Object { $_.private_ws_sum -eq 'EXITED' }).Count
  $pw = $rows | % { [double]$_.private_ws_sum / 1MB }
  $tail = $rows | Select-Object -Last 60 | % { [double]$_.private_ws_sum / 1MB }
  $pb = ($rows | Select-Object -Last 60 | % { [double]$_.private_bytes_sum / 1MB } | Measure-Object -Average).Average
  $ws = ($rows | Select-Object -Last 60 | % { [double]$_.working_set_sum / 1MB } | Measure-Object -Average).Average
  $gd = ($rows | Select-Object -Last 1).gpu_dedicated / 1MB
  $gs = ($rows | Select-Object -Last 1).gpu_shared / 1MB
  $np = ($rows | % { [int]$_.nproc } | Measure-Object -Maximum).Maximum
  '{0,-18} n={1,3} exited={2} nproc_max={3} | whole min={4:N1} med={5:N1} max={6:N1} | last60 min={7:N1} med={8:N1} max={9:N1} | privBytes={10:N0} WS={11:N0} gpuDed={12:N1} gpuShr={13:N1}' -f $f.BaseName, $rows.Count, $exited, $np, ($pw | Measure-Object -Minimum).Minimum, (Med $pw), ($pw | Measure-Object -Maximum).Maximum, ($tail | Measure-Object -Minimum).Minimum, (Med $tail), ($tail | Measure-Object -Maximum).Maximum, $pb, $ws, $gd, $gs
}
