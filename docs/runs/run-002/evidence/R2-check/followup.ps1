$ErrorActionPreference='Stop'
. "$PSScriptRoot\gui.ps1" -Continue -HelpersOnly
try {
    $c=Launch C @('--folder','_test_')
    Note G2-blocked-status $c 'Already enrolled unfamiliar client; Blocked was excluded on consent screen.'
    Add-Addon C extra 1.0 Extra _test_
    Wait-For $c '*StaleConfirmation*'
    Note unfamiliar-addition $c 'Added a new plain-toc addon Extra after enrollment; sync stops without a GUI reconfirm action.'
    Stop-One $c

    New-Pc D _test_ wow_test
    Add-Addon D d1 1.0 Good _test_
    Add-Addon D d2 1.0 Blocked _test_
    Rename-Item "$Lab\D\install\_test_\Interface\AddOns\Blocked\Blocked.toc" 'Blocked_Unknown.toc'
    $d=Launch D @('--folder','_test_')
    Wait-For $d 'I confirm*'
    Note G2-preview $d 'Fresh second unfamiliar synthetic install; same product and layout as C before enrollment.'
    [void][CheckWin]::SetForegroundWindow($d.MainWindowHandle)
    (Find $d 'I confirm*').SetFocus()
    Key 9
    $next=$AE::FocusedElement.Current.Name
    [CheckWin]::keybd_event(16,0,0,[UIntPtr]::Zero)
    Key 9
    [CheckWin]::keybd_event(16,0,2,[UIntPtr]::Zero)
    $back=$AE::FocusedElement.Current.Name
    Note G9-shift-tab $d "Tab: $next`nShift-Tab: $back"
    Stop-One $d

    $b=Launch B
    Wait-For $b 'Sync now'
    [void][CheckWin]::SetForegroundWindow($b.MainWindowHandle)
    (Find $b 'Sync now').SetFocus()
    Key 13
    Start-Sleep 2
    Note G9-sync-enter $b 'Focused Sync now by UIA, activated by physical Enter key; no stuck busy state.'
    $latest=Get-ChildItem "$Lab\provider\WoWSync\groups" -Recurse -File |
        Where-Object { $_.Directory.Name -eq 'state' } |
        Sort-Object LastWriteTime -Descending
    $notes=@()
    foreach($f in $latest) {
        $st=Get-Content $f.FullName -Raw | ConvertFrom-Json
        $blocked=$st.folders | Where-Object key -eq 'blocked'
        if($blocked) {
            $notes += "Published blocked-folder inventory: status=$($blocked.status), toc=$($blocked.toc.kind), files=$($blocked.files), bytes=$($blocked.bytes), tree=$($blocked.tree)"
        }
    }
    $notes | Set-Content "$Out\blocked-publication.txt"
} finally {
    foreach($ownedPid in $Started) { Stop-Process -Id $ownedPid -ErrorAction SilentlyContinue }
    Remove-Item Env:WOWSYNC_LOG -ErrorAction SilentlyContinue
}
