param([switch]$HelpersOnly)
$ErrorActionPreference='Stop'
$Here=$PSScriptRoot
$Exe="$Here\clean\target\release\wow-sync.exe"
$Dev="$Here\clean\target\release\devkit.exe"
$Lab="$Here\lab"
$Out="$Here\gui"
New-Item -ItemType Directory -Force $Lab,$Out,"$Lab\provider" | Out-Null
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class RecheckWindow {
 [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h,uint m,IntPtr w,IntPtr l);
}
'@
$AE=[System.Windows.Automation.AutomationElement]
$Launched=[Collections.Generic.List[Diagnostics.Process]]::new()
function Launch($Pc,[string[]]$Extra=@()) {
    $env:WOWSYNC_LOG="$Lab\$Pc-app.log"
    $p=Start-Process $Exe -ArgumentList (@('--dev','--install',"$Lab\$Pc\install",'--provider',"$Lab\provider",'--state',"$Lab\$Pc\state",'--name',"Recheck-$Pc",'--interval-ms','1500','--recent-ms','600')+$Extra) -PassThru
    $Launched.Add($p)
    Start-Sleep 4
    if($p.HasExited) { throw "App $Pc exited" }
    $p
}
function Stop-Owned([Diagnostics.Process]$p) {
    if(-not $Launched.Contains($p)) { throw 'Refusing to stop a process not directly launched here' }
    if(-not $p.HasExited) {
        Stop-Process -Id $p.Id -ErrorAction Stop
        $p.WaitForExit(5000) | Out-Null
    }
}
function Nodes($p) {
    $p.Refresh()
    ($AE::FromHandle($p.MainWindowHandle)).FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)
}
function Find($p,$Like) {
    Nodes $p | Where-Object { $_.Current.Name -like $Like } | Select-Object -First 1
}
function Wait-For($p,$Like,$Seconds=30) {
    $end=(Get-Date).AddSeconds($Seconds)
    do { $n=Find $p $Like; if($n) { return $n }; Start-Sleep -Milliseconds 400 } while((Get-Date) -lt $end)
    throw "Timeout: $Like"
}
function Click($p,$Like) {
    $n=Wait-For $p $Like
    $b=($AE::FromHandle($p.MainWindowHandle)).Current.BoundingRectangle
    $control=$n.Current.BoundingRectangle
    if($control.Top -lt $b.Top -or $control.Bottom -gt $b.Bottom) {
        $position=([int]($b.Y+100) -shl 16) -bor [int]($b.X+100)
        for($scroll=0;$scroll -lt 4;$scroll++) {
            [void][RecheckWindow]::PostMessageW($p.MainWindowHandle,0x20A,[IntPtr](-1200 -shl 16),[IntPtr]$position)
            Start-Sleep -Milliseconds 200
        }
        $n=Wait-For $p $Like
        $control=$n.Current.BoundingRectangle
        if($control.Top -lt $b.Top -or $control.Bottom -gt $b.Bottom) { throw "Control remains outside window after scrolling: $Like" }
    }
    if($n.Current.ControlType -eq [System.Windows.Automation.ControlType]::CheckBox) {
        $n.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle()
    } else {
        $n.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    }
    Start-Sleep -Milliseconds 600
}
function Note($Name,$p,$More='') {
    $text=@()
    for($i=0;$i -lt 12 -and $text.Count -eq 0;$i++) {
        $text=@(Nodes $p | ForEach-Object { if($_.Current.Name) { "$($_.Current.ControlType.ProgrammaticName): $($_.Current.Name)" } })
        if($text.Count -eq 0) { Start-Sleep -Milliseconds 400 }
    }
    if($text.Count -eq 0) { throw 'Empty UIA capture' }
    @($More;$text) | ForEach-Object { ($_ -replace [regex]::Escape($Lab),'<synthetic-lab>').TrimEnd() } | Set-Content "$Out\$Name.txt"
}
function Assert($Truth,$Message) { if(-not $Truth) { throw $Message } }
function New-Pc($Pc,$Folder='_forever_',$Product='wowf') {
    & $Dev make-install "$Lab\$Pc\install" --folder $Folder --product $Product
    if($LASTEXITCODE) { throw 'make-install failed' }
}
function Add-Addon($Pc,$Tag,$Version,$Name='Alpha',$Folder='_forever_') {
    & $Dev add-addon "$Lab\$Pc\install" $Folder $Name $Tag $Version
    if($LASTEXITCODE) { throw 'add-addon failed' }
}
function Body($Pc) { Get-Content "$Lab\$Pc\install\_forever_\Interface\AddOns\Alpha\main.lua" -Raw }
if($HelpersOnly) { return }
try {
    New-Pc A; New-Pc B
    Add-Addon A a2 2.0; Add-Addon B b1 1.0
    $a=Launch A
    Note G1-M4-m1 $a 'Fresh recognized enrollment: independently captured disclosures before consent.'
    Click $a 'Create a new sync group'
    $null=Wait-For $a '*Tick the agreement*'
    Assert (@(Get-ChildItem "$Lab\provider" -Recurse -File).Count -eq 0) 'Provider written before consent'
    Assert (@(Get-ChildItem "$Lab\A\state" -Recurse -File).Count -eq 0) 'Local state written before consent'
    Note G1-no-consent $a 'No provider or local-state file written before consent.'
    Click $a 'I agree*'; Click $a 'Create a new sync group'
    $null=Wait-For $a 'Published*'
    $b=Launch B
    Click $b 'I agree*'; Click $b 'Join group*'
    $null=Wait-For $b '*updated*from computer*'
    Assert ((Body B) -eq 'a2') 'Initial apply failed'
    foreach($file in Get-ChildItem "$Lab\A\install\_forever_\Interface\AddOns\Alpha" -File) {
        Assert ((Get-FileHash $file.FullName).Hash -eq (Get-FileHash "$Lab\B\install\_forever_\Interface\AddOns\Alpha\$($file.Name)").Hash) 'Applied file hash differs'
    }
    Note G3-m3 $b 'Every Alpha file hash equals A; recovery count visible.'

    Stop-Owned $b
    Copy-Item "$env:SystemRoot\System32\PING.EXE" "$Lab\B\install\_forever_\Wow.exe"
    $fake=Start-Process "$Lab\B\install\_forever_\Wow.exe" -ArgumentList @('-t','127.0.0.1') -WindowStyle Hidden -PassThru
    $Launched.Add($fake)
    Assert (-not $fake.HasExited) 'Directly launched FakeClient exited'
    Add-Addon A a3 3.0
    Start-Sleep 4
    $b=Launch B
    $null=Wait-For $b '*WoW is running*'
    Assert ((Body B) -eq 'a2') 'Changed while FakeClient running'
    Note G4-m2-wait $b 'B remains a2 while directly launched FakeClient runs; A is a3.'
    Stop-Owned $fake
    $null=Wait-For $b '*updated*from computer*'
    Assert ((Body B) -eq 'a3') 'Did not resume'
    Note G4-m2-resume $b 'Directly launched FakeClient stopped by its launch PID; B became a3.'

    Stop-Owned $a; Stop-Owned $b
    Add-Addon A a4 4.0; Add-Addon B b4 4.0
    $a=Launch A; $b=Launch B
    $null=Wait-For $a '*cannot tell which version*'; $null=Wait-For $b '*cannot tell which version*'
    Assert ((Body A) -eq 'a4' -and (Body B) -eq 'b4') 'Ambiguous content changed'
    Note G5-M5-A $a 'Equal labels/different bytes; a4 and b4 unchanged.'
    Note G5-M5-B $b

    Stop-Owned $b
    Move-Item "$Lab\provider" "$Lab\provider.saved"
    Set-Content "$Lab\provider" 'synthetic obstruction'
    $null=Wait-For $a 'Something needs attention'
    Note G6-m4 $a 'Provider root replaced by a file; error duplication and identity text checked.'
    Remove-Item "$Lab\provider"
    Move-Item "$Lab\provider.saved" "$Lab\provider"
    Add-Addon A a5 5.0
    Start-Sleep 5
    $b=Launch B @('--crash-after','2')
    $null=Wait-For $b '*Stopped mid-way*'
    Note G8-m5-interrupted $b 'No FakeClient exists; injected failure after two mutating effects.'
    Click $b 'Sync now'
    $null=Wait-For $b '*0 interrupted update*'
    Assert ((Body B) -eq 'a5') 'Interrupted apply did not recover and update'
    Note G8-m5-recovered $b
    Stop-Owned $a; Stop-Owned $b

    New-Pc C _test_ wow_test
    Add-Addon C c1 1.0 Good _test_
    Add-Addon C blocked-code-unique 1.0 Blocked _test_
    Rename-Item "$Lab\C\install\_test_\Interface\AddOns\Blocked\Blocked.toc" 'Blocked_Unknown.toc'
    $c=Launch C @('--folder','_test_')
    Note G2-M4 $c 'Fresh unfamiliar client: actual paths, mapping and disclosures before consent.'
    Click $c 'I agree*'; Click $c 'Create a new sync group'
    $null=Wait-For $c '*Agreeing to the scope alone*'
    Note G2-scope-only $c
    Click $c 'I agree*'; Click $c 'I confirm*'; Click $c 'Create a new sync group'
    $null=Wait-For $c '*Tick the agreement*'
    Note G2-identity-only $c
    Click $c 'I agree*'
    Click $c 'Join group*wowf*'
    $null=Wait-For $c '*that group syncs wowf*'
    Assert (-not (Test-Path "$Lab\C\state\enrollment.json")) 'Cross-product join accepted'
    Note M3-cross-product $c 'Core-owned join refusal is visible; C not enrolled.'
    Click $c 'Create a new sync group'
    $null=Wait-For $c 'Published*'
    Note G2-M2-status $c

    $group=(Get-Content "$Lab\C\state\enrollment.json" -Raw | ConvertFrom-Json).group_id
    $groupRoot="$Lab\provider\WoWSync\groups\$group"
    $badPath="$Lab\C\install\_test_\Interface\AddOns\Blocked\main.lua"
    $badHash=(Get-FileHash $badPath).Hash.ToLowerInvariant()
    $badBlob=@(Get-ChildItem $groupRoot -Recurse -File | Where-Object Name -eq $badHash)
    Assert ($badBlob.Count -eq 0) 'Blocked main.lua blob was published'
    "Blocked main.lua SHA-256 object absent from its group: True" | Set-Content "$Out\M2-publication.txt"
    Add-Addon C extra-code 1.0 Extra _test_
    $null=Wait-For $c '*extra:*'
    Start-Sleep 3
    Assert (-not (Find $c '*StaleConfirmation*')) 'Addition staled enrollment'
    Note M1-addition $c 'Added plain-toc Extra after enrollment; sync remains usable.'
    $states=Get-ChildItem "$groupRoot\members" -Recurse -File | Where-Object { $_.Directory.Name -eq 'state' }
    $publishedExtra=@($states | Where-Object { (Get-Content $_.FullName -Raw | ConvertFrom-Json).folders.key -contains 'extra' }).Count -gt 0
    Assert $publishedExtra 'Extra was not published'
    'Extra appears in published state after addition: True' | Set-Content "$Out\M1-publication.txt"
    Stop-Owned $c
    $c=Launch C @('--folder','_test_')
    $null=Wait-For $c '*extra:*'
    Click $c 'Sync now'
    Note M1-restart $c 'Restart and GUI retry remain usable after addition.'
} finally {
    foreach($p in $Launched) { if(-not $p.HasExited) { Stop-Owned $p } }
    Remove-Item Env:WOWSYNC_LOG -ErrorAction SilentlyContinue
}
