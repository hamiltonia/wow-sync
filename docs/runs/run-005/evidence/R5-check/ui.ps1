param([int]$AppPid, [string]$Action='dump', [string]$Name='', [int]$Key=13, [int]$MatchIndex=0)
$ErrorActionPreference='Stop'
. 'D:\GitHub\wow-sync.worktrees\R5-check\tools\mem\MemLib.ps1'
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes
function Elements {
    $w=Get-AppWindow $AppPid
    if(-not $w){throw "No app window for captured PID $AppPid"}
    $r=[Windows.Automation.AutomationElement]::FromHandle([IntPtr]$w[0])
    $r.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition)
}
function Activate([string]$pattern) {
    $found=$null
    $seen=0
    foreach($e in (Elements)){
        if($e.Current.ControlType.ProgrammaticName -match 'Button|CheckBox|RadioButton|ListItem' -and $e.Current.Name -like $pattern){
            if($seen -eq $MatchIndex){$found=$e;break}
            $seen++
        }
    }
    if(-not $found){throw "No actionable control '$pattern'"}
    if(-not $found.Current.IsEnabled){throw "Disabled control '$pattern'"}
    if($found.Current.IsOffscreen){throw "Offscreen control '$pattern'"}
    "ACTIVATE $pattern -> $($found.Current.Name.Replace('LABACCOUNT','[account]'))"
    if($found.Current.ControlType.ProgrammaticName -match 'CheckBox'){
        $found.GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Toggle()
    } else {
        $found.SetFocus()
        Send-Key $AppPid 13
    }
    Start-Sleep -Milliseconds 1000
}
switch($Action){
    'dump' {
        foreach($e in (Elements)){
            if($e.Current.Name){"{0} | enabled={1} offscreen={2} focus={3} | {4}" -f $e.Current.ControlType.ProgrammaticName,$e.Current.IsEnabled,$e.Current.IsOffscreen,$e.Current.HasKeyboardFocus,$e.Current.Name.Replace('LABACCOUNT','[account]')}
        }
    }
    'invoke' {Activate $Name}
    'key' {Send-Key $AppPid $Key; Start-Sleep -Milliseconds 500}
    'scroll' {Send-Wheel $AppPid $Key; Start-Sleep -Milliseconds 500}
    'enroll' {
        foreach($n in @('Next','Use this folder','Next','Next','This is my*','These are the*','Next','Create a new group','Next','Set up this computer','Finish and open*')){Activate $n}
    }
}
