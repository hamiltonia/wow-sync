$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path "$PSScriptRoot\..\..\..\..\..").Path
Set-Location $repo
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$old = [IO.File]::ReadAllText("$PSScriptRoot\..\R3-check3\gui-interrupted.txt")
$match = [regex]::Match($old,'(?s)JOURNAL=(\{.*?\r?\n\})')
if (!$match.Success) { throw 'Retained pre-fix2 journal was not found' }
[IO.File]::WriteAllText("$PSScriptRoot\legacy-journal.json",$match.Groups[1].Value)
$rlibs = @(Get-ChildItem "$repo\target\release" -Filter 'libwow_sync_core-*.rlib' -Recurse)
if ($rlibs.Count -ne 1) { throw 'Expected one freshly built release core rlib' }
$dirs = @(Get-ChildItem "$repo\target\release" -Recurse -File |
    Where-Object Extension -in '.rlib','.rmeta','.dll' |
    ForEach-Object DirectoryName | Sort-Object -Unique)
$response = @('--edition=2021',"$PSScriptRoot\legacy-probe.rs", '--extern',
    "wow_sync_core=$($rlibs[0].FullName)", '-o', "$repo\target\check4-legacy-probe.exe")
foreach ($dir in $dirs) { $response += '-L'; $response += "dependency=$dir" }
[IO.File]::WriteAllLines("$repo\target\check4-rustc.rsp",$response)
& rustc "@$repo\target\check4-rustc.rsp"
if ($LASTEXITCODE) { exit $LASTEXITCODE }
& "$repo\target\check4-legacy-probe.exe" "$PSScriptRoot\legacy-journal.json"
exit $LASTEXITCODE
