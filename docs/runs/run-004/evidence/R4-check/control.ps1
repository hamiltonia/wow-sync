param([string]$Package, [string]$Test)
$ErrorActionPreference = 'Stop'
Set-Location 'D:\GitHub\wow-sync.worktrees\R4-check'
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:TEMP = 'D:\WoWSyncLab\R4-check\temp'
$env:TMP = $env:TEMP
New-Item -ItemType Directory -Force $env:TEMP | Out-Null
$target = if ($Package -eq 'wow-sync-app') { @('--bin', 'wow-sync') } else { @('--lib') }
"cargo test -p $Package $Test $($target -join ' ') -- --exact"
$cargoArgs = @('test', '-p', $Package, $Test) + $target + @('--', '--exact')
$out = & cargo @cargoArgs 2>&1
$code = $LASTEXITCODE
foreach ($line in $out) {
    $s = "$line" -replace 'C:\\Users\\[^\\]+', '%USERPROFILE%'
    if ($s -match '^test |^test result:|^running |^error:|^failures:|^---- ') { $s }
}
"exit=$code"
exit $code
