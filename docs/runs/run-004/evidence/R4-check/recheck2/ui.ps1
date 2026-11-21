param([int]$AppPid,[string]$Action='dump',[string]$Name='',[int]$Key=13)
$repo='D:\GitHub\wow-sync.worktrees\R4-recheck2'
$body=[IO.File]::ReadAllText("$repo\docs\runs\run-004\evidence\R4-check\recheck\ui.ps1")
$body=$body.Replace("D:\GitHub\wow-sync.worktrees\R4-recheck'","D:\GitHub\wow-sync.worktrees\R4-recheck2'")
$body=$body.Replace('$dest="$PSScriptRoot\$Name.jpg"','$dest="D:\GitHub\wow-sync.worktrees\R4-recheck2\docs\runs\run-004\evidence\R4-check\recheck2\$Name.jpg"')
& ([ScriptBlock]::Create($body)) -AppPid $AppPid -Action $Action -Name $Name -Key $Key
