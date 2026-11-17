$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path "$PSScriptRoot\..\..\..\..\..").Path
Set-Location $repo
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:TEMP = 'D:\WoWSyncLab\R3-check4b\scratch'
$env:TMP = $env:TEMP
$runs = [Collections.Generic.List[object]]::new()
function Run-Cargo([string]$Label,[string[]]$Command) {
    & cargo @Command 2>&1 | ForEach-Object {
        "$_".Replace($env:USERPROFILE,'<profile>')
    } | Tee-Object -FilePath "$PSScriptRoot\$Label.txt" | Out-Host
    $code = $LASTEXITCODE
    $runs.Add([pscustomobject]@{label=$Label;command="cargo $($Command -join ' ')";exit=$code})
    [IO.File]::WriteAllText("$PSScriptRoot\final-exits.json",(ConvertTo-Json -InputObject $runs.ToArray()))
    Write-Host "$Label EXIT=$code"
    if ($code) { throw "$Label did not pass; see retained output." }
}
Run-Cargo positive-core @('test','--locked','-p','wow-sync-core','--lib')
Run-Cargo offline @('xtask','verify','--tier','offline')
Copy-Item "$repo\target\verify\report.json" "$PSScriptRoot\offline-report.json"
