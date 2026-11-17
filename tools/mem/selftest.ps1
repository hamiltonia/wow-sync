# Self-test of the A-29 harness safety logic. Exit code 0 = all checks passed.
#   powershell -NoProfile -File tools\mem\selftest.ps1
. "$PSScriptRoot\MemLib.ps1"
$fail = 0
function Check([string]$name, [bool]$ok) {
    if ($ok) { "PASS  $name" } else { "FAIL  $name"; $script:fail++ }
}
function Row($p, $pp, $c) { [pscustomobject]@{ Pid = $p; Ppid = $pp; Created = [long]$c } }

# --- pure checks on synthetic process tables ---
$root = [pscustomobject]@{ Pid = 100; Created = 1000 }
$t = @(
    (Row 100 1 1000), (Row 101 100 1500), (Row 102 101 1600),   # root, child, grandchild (valid)
    (Row 103 100 900),                                           # names root as parent but is OLDER (recycled parent id)
    (Row 104 101 1400),                                          # grandchild older than its parent
    (Row 200 1 1100), (Row 201 200 1200)                         # unrelated tree
)
$ids = @(Get-OwnedSet $t $root | ForEach-Object Pid) | Sort-Object
Check 'synthetic: valid child + grandchild counted' (($ids -join ',') -eq '100,101,102')
Check 'synthetic: older process naming the root as parent NOT counted' (-not ($ids -contains 103))
Check 'synthetic: grandchild older than its parent NOT counted' (-not ($ids -contains 104))
Check 'synthetic: unrelated tree NOT counted' (-not ($ids -contains 200 -or $ids -contains 201))
$reused = @((Row 100 1 5000), (Row 101 100 6000))
Check 'synthetic: root PID reused (different creation time) -> empty set' (@(Get-OwnedSet $reused $root).Count -eq 0)
Check 'synthetic: root gone -> empty set' (@(Get-OwnedSet @((Row 200 1 1100)) $root).Count -eq 0)

# --- live dummy parent/child, launched here ---
$parent = $null; $foreign = $null; $parentRec = $null; $childRec = $null; $foreignRec = $null
try {
    $parent = Start-Process "$env:SystemRoot\System32\cmd.exe" -ArgumentList '/c', 'ping -n 20 127.0.0.1 >nul' -WindowStyle Hidden -PassThru
    $foreign = Start-Process "$env:SystemRoot\System32\PING.EXE" -ArgumentList '-n', '20', '127.0.0.1' -WindowStyle Hidden -PassThru
    $parentRec = Get-Record $parent.Id
    $foreignRec = Get-Record $foreign.Id
    Start-Sleep -Seconds 2
    $tab = @(Get-ProcTable)
    $owned = @(Get-OwnedSet $tab $parentRec)
    $kid = $tab | Where-Object { $_.Ppid -eq $parent.Id } | Select-Object -First 1
    Check 'live: dummy parent has a child process' ([bool]$kid)
    if ($kid) { $childRec = [pscustomobject]@{ Pid = $kid.Pid; Created = $kid.Created } }
    Check 'live: parent + its real child counted' (($owned.Count -ge 2) -and ($owned.Pid -contains $kid.Pid) -and ($owned.Pid -contains $parent.Id))
    Check 'live: independently launched process NOT counted' (-not ($owned.Pid -contains $foreign.Id))
    $stale = [pscustomobject]@{ Pid = $parent.Id; Created = ($parentRec.Created - 10000) }
    Check 'live: record with wrong creation time (PID-reuse stand-in) owns nothing' (@(Get-OwnedSet $tab $stale).Count -eq 0)
    [void](Stop-Launched $stale 1)
    Check 'live: Stop-Launched refuses a mismatched creation time (process still alive)' (Test-Alive $parentRec)
    Check 'live: Stop-Launched stops the validated parent' (Stop-Launched $parentRec 10)
    Check 'live: the independently launched process was not touched' (Test-Alive $foreignRec)
    $kidAlive = $childRec -and (Test-Alive $childRec)
    "INFO  child survived parent stop (reported, never stopped by the harness): $kidAlive"
}
finally {
    # Cleanup of our own dummy processes only: each is re-validated by PID + creation time.
    foreach ($rec in @($parentRec, $foreignRec)) { if ($rec) { [void](Stop-Launched $rec 5) } }
}
if ($fail -eq 0) { 'SELFTEST PASS' } else { "SELFTEST FAIL ($fail)"; exit 1 }
