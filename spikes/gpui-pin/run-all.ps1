# Runs memory scenarios (3 runs each). Release binary must be built.
# Usage: run-all.ps1 -OutDir <dir> -Set destroy|sw   (destroy: window closed for real, incl. scenario a;
#        sw: window kept and hidden with SW_HIDE). Two sets can run in parallel (distinct -Instance).
param([string]$OutDir, [ValidateSet('destroy', 'sw')][string]$Set = 'destroy')
$exe = "$PSScriptRoot\target\release\spike-gpui-pin.exe"
$m = "$PSScriptRoot\measure.ps1"
$hide = if ($Set -eq 'sw') { 'sw' } else { '' }
for ($r = 1; $r -le 3; $r++) {
  if ($Set -eq 'destroy') { & $m -Scenario a -Variant none -Run $r -Exe $exe -ExeArgs "hidden" -Seconds 120 -OutDir $OutDir -Instance $Set }
  & $m -Scenario b -Variant $Set -Run $r -Exe $exe -ExeArgs "show-hide 60" -Seconds 180 -OutDir $OutDir -Hide $hide -Instance $Set
  & $m -Scenario c -Variant $Set -Run $r -Exe $exe -ExeArgs "cycles 20" -Seconds 360 -OutDir $OutDir -Hide $hide -Instance $Set
}
"ALL DONE $Set"
