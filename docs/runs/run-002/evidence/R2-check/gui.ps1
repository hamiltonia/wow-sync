param([switch]$Continue, [switch]$HelpersOnly)
$ErrorActionPreference = 'Stop'
$Here = $PSScriptRoot
$Exe = "$Here\clean\target\release\wow-sync.exe"
$Dev = "$Here\clean\target\release\devkit.exe"
$Lab = "$Here\lab"
$Out = "$Here\gui"
if (-not $Continue -and -not $HelpersOnly -and (Test-Path $Lab)) { Remove-Item $Lab -Recurse -Force }
New-Item -ItemType Directory -Force $Lab, $Out | Out-Null
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Windows.Forms, System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class CheckWin {
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern void keybd_event(byte k,byte s,uint f,UIntPtr e);
 [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h,uint m,IntPtr w,IntPtr l);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
}
'@
$AE = [System.Windows.Automation.AutomationElement]
$Started = [System.Collections.Generic.List[int]]::new()
function Launch($Pc, [string[]]$Extra = @()) {
    $env:WOWSYNC_LOG = "$Lab\$Pc-app.log"
    $p = Start-Process $Exe -ArgumentList (@('--dev','--install',"$Lab\$Pc\install",'--provider',"$Lab\provider",'--state',"$Lab\$Pc\state",'--name',"Check-$Pc",'--interval-ms','1500','--recent-ms','600') + $Extra) -PassThru
    $Started.Add($p.Id)
    Start-Sleep 4
    $p.Refresh()
    if ($p.HasExited) { throw "Application $Pc exited ($($p.ExitCode))" }
    $p
}
function Stop-One($p) {
    if ($p -is [int]) { $idToStop=$p } else { $idToStop=$p.Id }
    Stop-Process -Id $idToStop -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500
}
function Nodes($p) {
    $p.Refresh()
    $root = $AE::FromHandle($p.MainWindowHandle)
    $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)
}
function Texts($p) {
    @(Nodes $p | ForEach-Object { if ($_.Current.Name) { "$($_.Current.ControlType.ProgrammaticName): $($_.Current.Name)" } })
}
function Find($p,$Like) {
    Nodes $p | Where-Object { $_.Current.Name -like $Like } | Select-Object -First 1
}
function Click($p,$Like) {
    $n = $null
    for($attempt=0; $attempt -lt 12 -and -not $n; $attempt++) {
        $n = Find $p $Like
        if (-not $n) { Start-Sleep -Milliseconds 400 }
    }
    if (-not $n) { throw "Missing control: $Like" }
    if ($n.Current.ControlType -eq [System.Windows.Automation.ControlType]::CheckBox) {
        $n.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle()
    } else {
        $n.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    }
    Start-Sleep -Milliseconds 700
}
function Wait-For($p,$Like,$Seconds=30) {
    $until=(Get-Date).AddSeconds($Seconds)
    do { if (Find $p $Like) { return }; Start-Sleep -Milliseconds 400 } while ((Get-Date) -lt $until)
    throw "Timeout waiting for: $Like"
}
function Note($Name,$p,$More='') {
    $observed=@()
    for($attempt=0; $attempt -lt 12 -and $observed.Count -eq 0; $attempt++) {
        $observed=@(Texts $p)
        if ($observed.Count -eq 0) { Start-Sleep -Milliseconds 400 }
    }
    if ($observed.Count -eq 0) { throw 'UIA text capture remained empty' }
    @($More; $observed) | ForEach-Object { $_ -replace [regex]::Escape($Lab),'<synthetic-lab>' } | Set-Content "$Out\$Name.txt"
}
function Key([byte]$vk) {
    [CheckWin]::keybd_event($vk,0,0,[UIntPtr]::Zero)
    [CheckWin]::keybd_event($vk,0,2,[UIntPtr]::Zero)
    Start-Sleep -Milliseconds 400
}
function Shot($p,$Name) {
    $p.Refresh()
    [void][CheckWin]::SetForegroundWindow($p.MainWindowHandle)
    Start-Sleep -Milliseconds 500
    $b=($AE::FromHandle($p.MainWindowHandle)).Current.BoundingRectangle
    $bmp=[Drawing.Bitmap]::new([int]$b.Width,[int]$b.Height)
    $g=[Drawing.Graphics]::FromImage($bmp)
    try { $g.CopyFromScreen([int]$b.X,[int]$b.Y,0,0,$bmp.Size); $bmp.Save("$Out\$Name.png") }
    finally { $g.Dispose(); $bmp.Dispose() }
}
function New-Pc($Pc,$Folder='_forever_',$Product='wowf') {
    & $Dev make-install "$Lab\$Pc\install" --folder $Folder --product $Product
    if ($LASTEXITCODE) { throw 'devkit failed' }
}
function Add-Addon($Pc,$Tag,$Version,$Name='Alpha',$Folder='_forever_') {
    & $Dev add-addon "$Lab\$Pc\install" $Folder $Name $Tag $Version
    if ($LASTEXITCODE) { throw 'devkit failed' }
}
function Body($Pc) { Get-Content "$Lab\$Pc\install\_forever_\Interface\AddOns\Alpha\main.lua" -Raw }
function Assert($Truth,$Message) { if (-not $Truth) { throw $Message } }

if ($HelpersOnly) { return }
try {
    if (-not $Continue) {
    New-Item -ItemType Directory -Force "$Lab\provider" | Out-Null
    New-Pc A; New-Pc B
    Add-Addon A a2 2.0; Add-Addon B b1 1.0
    $a=Launch A
    Note G1-before $a
    Click $a 'Create a new sync group'
    Assert (@(Get-ChildItem "$Lab\provider" -Recurse -File).Count -eq 0) 'Provider written before consent'
    Assert (@(Get-ChildItem "$Lab\A\state" -Recurse -File).Count -eq 0) 'Local state written before consent'
    Note G1-refusal $a 'No provider or local-state files before consent; refusal observed.'
    Click $a 'I agree*'
    Click $a 'Create a new sync group'
    Wait-For $a 'Published*'
    Note G7-before $a 'Only A exists; no peer apply report is possible.'

    $b=Launch B
    Click $b 'I agree*'; Click $b 'Join group*'
    Wait-For $b '*updated and verified*'
    Assert ((Body B) -eq (Body A)) 'Applied body differs'
    $filesA=Get-ChildItem "$Lab\A\install\_forever_\Interface\AddOns\Alpha" -File
    foreach($f in $filesA) {
        Assert ((Get-FileHash $f.FullName).Hash -eq (Get-FileHash "$Lab\B\install\_forever_\Interface\AddOns\Alpha\$($f.Name)").Hash) 'Applied file hash differs'
    }
    Note G3 $b 'All Alpha file hashes equal A after apply.'
    Wait-For $a '*applied and reported*'
    Note G7-after $a
    Stop-One $b
    } else {
        $a=Launch A
    }
    Copy-Item "$env:SystemRoot\System32\PING.EXE" "$Lab\B\install\_forever_\Wow.exe" -Force
    $fake = Start-Process "$Lab\B\install\_forever_\Wow.exe" -ArgumentList @('-t','127.0.0.1') -WindowStyle Hidden -PassThru
    $fakePid = $fake.Id
    $Started.Add($fakePid)
    Assert ($null -ne (Get-Process -Id $fakePid)) 'FakeClient not running'
    Add-Addon A a3 3.0
    Start-Sleep 5
    $b=Launch B
    Wait-For $b '*World of Warcraft is running*'
    Assert ((Body B) -eq 'a2') 'Changed while FakeClient running'
    Note G4-wait $b 'FakeClient running; B main.lua remains a2 while A is a3.'
    Stop-One $fakePid
    Wait-For $b '*updated and verified*'
    Assert ((Body B) -eq 'a3') 'Did not resume after FakeClient stopped'
    Note G4-resume $b 'Stopped FakeClient by PID; B became a3.'

    Stop-One $a; Stop-One $b
    Add-Addon A a4 4.0; Add-Addon B b4 4.0
    $a=Launch A; $b=Launch B
    Wait-For $a '*cannot tell which version*'; Wait-For $b '*cannot tell which version*'
    Assert ((Body A) -eq 'a4' -and (Body B) -eq 'b4') 'Ambiguous content was changed'
    Note G5-A $a 'Equal labels, different content; a4/b4 unchanged.'
    Note G5-B $b

    Stop-One $b
    Move-Item "$Lab\provider" "$Lab\provider.saved"
    Set-Content "$Lab\provider" 'synthetic obstruction'
    Start-Sleep 5
    Wait-For $a 'Something needs attention'
    Note G6 $a 'Provider directory replaced by a file; error observed, then restored.'
    Remove-Item "$Lab\provider"
    Move-Item "$Lab\provider.saved" "$Lab\provider"

    Add-Addon A a5 5.0
    Start-Sleep 5
    $b=Launch B @('--crash-after','2')
    Wait-For $b '*Stopped mid-way*'
    Note G8-interrupted $b 'Injected crash after 2 effects; no FakeClient running.'
    Click $b 'Sync now'
    Wait-For $b '*0 interrupted update*'
    Note G8-recovered $b
    Stop-One $a; Stop-One $b

    New-Pc C _test_ wow_test
    Add-Addon C c1 1.0 Good _test_
    Add-Addon C c2 1.0 Blocked _test_
    Rename-Item "$Lab\C\install\_test_\Interface\AddOns\Blocked\Blocked.toc" 'Blocked_Unknown.toc'
    $c=Launch C @('--folder','_test_')
    Note G2-preview $c
    Click $c 'I agree*'
    Click $c 'Create a new sync group'
    Wait-For $c '*Agreeing to the scope alone*'
    Note G2-scope-only $c
    Click $c 'I agree*'
    Click $c 'I confirm*'
    Click $c 'Create a new sync group'
    Wait-For $c '*Tick the agreement*'
    Note G2-identity-only $c
    Assert (-not (Test-Path "$Lab\C\state\enrollment.json")) 'Unfamiliar consent gate bypassed'

    [void][CheckWin]::SetForegroundWindow($c.MainWindowHandle)
    (Find $c 'I confirm*').SetFocus()
    Key 9
    $f1=$AE::FocusedElement.Current.Name
    Shot $c G9-focus
    Key 32
    Key 9
    $f2=$AE::FocusedElement.Current.Name
    Key 13
    Start-Sleep 4
    $keyboardEnrolled=Test-Path "$Lab\C\state\enrollment.json"
    Note G9-keyboard $c "Tab focus 1: $f1`nTab focus 2: $f2`nSpace then Enter enrolled: $keyboardEnrolled"
    if (-not $keyboardEnrolled) {
        $scope=Find $c 'I agree*'
        if ($scope.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Current.ToggleState -ne [System.Windows.Automation.ToggleState]::On) { Click $c 'I agree*' }
        Click $c 'Create a new sync group'
    }
    Wait-For $c 'Published*'
    Note G2-both $c 'Both identity and scope accepted; blocked-folder publication is checked separately.'
    $h=$c.MainWindowHandle
    [void][CheckWin]::PostMessageW($h,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
    Start-Sleep 2
    $hidden=-not [CheckWin]::IsWindowVisible($h)
    $reopen=Start-Process $Exe -ArgumentList @('--dev','--install',"$Lab\C\install",'--provider',"$Lab\provider",'--state',"$Lab\C\state",'--name','Check-C','--folder','_test_') -PassThru
    $Started.Add($reopen.Id)
    $reopen.WaitForExit(5000) | Out-Null
    Start-Sleep 2
    $shown=[CheckWin]::IsWindowVisible($h)
    "Close hides: $hidden; second invocation exits: $($reopen.HasExited); first reappears: $shown" | Set-Content "$Out\G9-close-reopen.txt"
    Assert ($hidden -and $shown) 'Close/reopen failed'
    'GUI probe completed.'
} finally {
    foreach($ownedPid in $Started) { Stop-Process -Id $ownedPid -ErrorAction SilentlyContinue }
    Remove-Item Env:WOWSYNC_LOG -ErrorAction SilentlyContinue
}
