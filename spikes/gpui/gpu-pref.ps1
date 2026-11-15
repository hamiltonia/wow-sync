# Forces the Windows per-app GPU preference (power saving => integrated GPU) for the spike exe,
# runs it briefly, then removes the preference again. Prior state is restored.
param([string]$Exe = "$PSScriptRoot\target\release\spike-gpui.exe", [string]$Pref = "GpuPreference=1;")
$exe = (Resolve-Path $Exe).Path
$k = 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences'
$old = (Get-ItemProperty $k -ErrorAction SilentlyContinue).$exe
"prior preference for exe: [$old]"
$created = $false
if (-not (Test-Path $k)) { New-Item $k -Force | Out-Null; $created = $true }
New-ItemProperty $k -Name $exe -Value $Pref -PropertyType String -Force | Out-Null
try {
  $proc = Start-Process $exe -PassThru -RedirectStandardOutput "$PSScriptRoot\gpu.out"
  Start-Sleep 4
  Stop-Process -Id $proc.Id
} finally {
  if ($null -ne $old) { Set-ItemProperty $k -Name $exe -Value $old } else { Remove-ItemProperty $k -Name $exe }
  if ($created) { Remove-Item $k }
}
Get-Content "$PSScriptRoot\gpu.out" | Select-Object -First 3
