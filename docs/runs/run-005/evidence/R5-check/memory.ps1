$ErrorActionPreference='Stop'
$repo='D:\GitHub\wow-sync.worktrees\R5-check'
$body=Get-Content "$repo\tools\mem\run.ps1" -Raw
$source='. "$PSScriptRoot\MemLib.ps1"'
$replacement=@'
. 'D:\GitHub\wow-sync.worktrees\R5-check\tools\mem\MemLib.ps1'
function Save-Csv($Run, [string]$Path) {
    Write-Host "RAW CSV BEGIN $Path"
    Write-Host 't_s,phase,procs,private_ws_b,private_bytes_b,working_set_b,gpu_dedicated_b,gpu_shared_b'
    foreach($r in $Run.Rows){
        Write-Host ('{0},{1},{2},{3},{4},{5},{6},{7}' -f $r.t_s,$r.phase,$r.procs,$r.private_ws_b,$r.private_bytes_b,$r.working_set_b,$r.gpu_dedicated_b,$r.gpu_shared_b)
    }
    Write-Host 'RAW CSV END'
}
'@
if(-not $body.Contains($source)){throw 'Harness library source changed'}
$body=$body.Replace($source,$replacement)
$body=$body.Replace('$Repo = (Resolve-Path "$PSScriptRoot\..\..").Path', '$Repo = ''D:\GitHub\wow-sync.worktrees\R5-check''')
$writer='[IO.File]::WriteAllText("$OutDir\$name", $text, [Text.UTF8Encoding]::new($false))'
if(-not $body.Contains($writer)){throw 'Harness summary writer changed'}
$body=$body.Replace($writer,'Write-Host "SUMMARY END $name"')
if($body -match '\[IO\.File\]'){throw 'Unexpected prohibited file API in harness body'}
& ([ScriptBlock]::Create($body)) -Runs 1 -LabRoot 'D:\WoWSyncLab\R5-check\mem' -OutDir 'D:\GitHub\wow-sync.worktrees\R5-check\docs\runs\run-005\evidence\R5-check\mem'
