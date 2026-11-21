param([ValidateSet('snapshot','absent','compare')][string]$Mode='snapshot')
$ErrorActionPreference='Stop'
$variant='D:\WoWSyncLab\R5-check\final\A\install\_forever_'
$folder="$variant\Interface\AddOns\SynFinal"
$settings=@(Get-ChildItem "$variant\WTF" -Recurse -File -Filter SynFinal.lua)
if($Mode -eq 'absent'){
    "code=$(Test-Path $folder) settings=$($settings.Count -gt 0)"
    if((Test-Path $folder) -or $settings.Count){throw 'Partial or unexpected installation'}
    return
}
if($settings.Count -ne 1){throw 'Expected one synthetic settings unit'}
$baseline=@{}
if($Mode -eq 'compare'){
    foreach($line in Get-Content "$PSScriptRoot\baseline.log"){
        $parts=$line.Split('|')
        if($parts.Length -eq 2){$baseline[$parts[0]]=$parts[1]}
    }
}
foreach($entry in @(@('code',"$folder\main.lua"),@('toc',"$folder\SynFinal.toc"),@('settings',$settings[0].FullName))){
    $hash=(Get-FileHash $entry[1] -Algorithm SHA256).Hash
    if($Mode -eq 'compare'){
        "$($entry[0]) match=$($hash -eq $baseline[$entry[0]]) hash=$hash"
        if($hash -ne $baseline[$entry[0]]){throw 'Restored fingerprint mismatch'}
    }else{"$($entry[0])|$hash"}
}
