# Runs all S-mem scenarios sequentially (3 runs each). Release binaries must be built.
$exe = "$PSScriptRoot\target\release\gpui_mem.exe"
$plain = "$PSScriptRoot\target\release\plain_idle.exe"
$m = "$PSScriptRoot\measure.ps1"
for ($r = 1; $r -le 3; $r++) {
  Remove-Item Env:WOWSYNC_ANCHOR -ErrorAction SilentlyContinue
  & $m -Scenario a-noanchor -Run $r -Exe $exe -ExeArgs "hidden" -Seconds 120
  $env:WOWSYNC_ANCHOR = "1"
  & $m -Scenario a-anchor -Run $r -Exe $exe -ExeArgs "hidden" -Seconds 120
  & $m -Scenario b -Run $r -Exe $exe -ExeArgs "show-hide 60" -Seconds 180
  & $m -Scenario c -Run $r -Exe $exe -ExeArgs "cycles 20" -Seconds 360
  Remove-Item Env:WOWSYNC_ANCHOR
  & $m -Scenario d -Run $r -Exe $plain -Seconds 120
}
$env:WOWSYNC_ANCHOR = "1"
& $m -Scenario b -Run 1 -Exe $exe -ExeArgs "show-hide 60" -Seconds 180 -Trim
"ALL DONE"
