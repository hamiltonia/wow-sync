param([string]$Addon='SynPlain',[string]$Mode='snapshot')
$ErrorActionPreference='Stop'
$baseline=@{}
if($Mode -eq 'compare'){
    foreach($line in Get-Content "$PSScriptRoot\baseline.log"){
        $parts=$line.Split('|')
        if($parts.Length -eq 4){$baseline[$parts[0]+'|'+$parts[1]+'|'+$parts[2]]=$parts[3]}
    }
}
foreach($pc in 'A','B'){
    $variant="D:\WoWSyncLab\R5-check\recheck\$pc\install\_forever_"
    $folder="$variant\Interface\AddOns\$Addon"
    $settings="$variant\WTF\Account\LABACCOUNT\SavedVariables\$Addon.lua"
    if($Mode -eq 'absent'){
        "$pc $Addon code=$(Test-Path $folder) settings=$(Test-Path $settings)"
        if((Test-Path $folder) -or (Test-Path $settings)){throw 'Uninstall incomplete'}
        continue
    }
    foreach($entry in @(@('code',"$folder\main.lua"),@('toc',"$folder\$Addon.toc"),@('settings',$settings))){
        $key="$pc|$Addon|$($entry[0])"
        $hash=(Get-FileHash $entry[1] -Algorithm SHA256).Hash
        if($Mode -eq 'compare'){
            "$key match=$($hash -eq $baseline[$key]) hash=$hash"
            if($hash -ne $baseline[$key]){throw "Restored bytes differ: $key"}
        }else{"$key|$hash"}
    }
}
