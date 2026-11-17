# A-29 memory measurement of the current product app (run-003, R3-mem).
#   powershell -NoProfile -File tools\mem\run.ps1 [-Scenarios a,b,c] [-Runs 3] [-OutDir <dir>] [-Scale 1.0]
# Needs: cargo build --release -p wow-sync-app   (target\release\wow-sync.exe, devkit.exe)
# -Scale shortens the idle/UI durations (smoke tests only; gated numbers use the default 1.0).
# Synthetic data only, under $env:TEMP\wowsync-mem. Never touches WoW, Battle.net or Dropbox.
param(
    [string[]]$Scenarios = @('a', 'b', 'c'),
    [int]$Runs = 3,
    [string]$OutDir = "$PSScriptRoot\..\..\docs\runs\run-003\evidence\R3-mem",
    [double]$Scale = 1.0
)
. "$PSScriptRoot\MemLib.ps1"
$Scenarios = @($Scenarios | ForEach-Object { $_ -split ',' })
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes

$Repo = (Resolve-Path "$PSScriptRoot\..\..").Path
$App = "$Repo\target\release\wow-sync.exe"
$Dev = "$Repo\target\release\devkit.exe"
$LabRoot = Join-Path $env:TEMP 'wowsync-mem'
$OutDir = (New-Item -ItemType Directory -Force $OutDir).FullName
$Findings = [Collections.Generic.List[string]]::new()
$Results = [Collections.Generic.List[object]]::new()
$SyncStart = [Collections.Generic.List[object]]::new()
function Secs([int]$n) { [int][math]::Max(1, [math]::Round($n * $Scale)) }
function Note([string]$s) { Write-Host ("[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $s) }

function New-Lab([string]$name) {
    $d = Join-Path $LabRoot $name
    if (Test-Path $d) { Remove-Item -Recurse -Force $d }
    New-Item -ItemType Directory -Force "$d\install", "$d\provider", "$d\state" | Out-Null
    & $Dev make-install "$d\install" --folder _forever_ --product wowf | Out-Null
    foreach ($n in 'SynAlpha', 'SynBeta', 'SynGamma') { & $Dev add-addon "$d\install" _forever_ $n "tag-$n" 1.0.0 | Out-Null }
    $d
}
function Remove-Lab([string]$d) {
    if ($d.StartsWith($LabRoot + '\') -and (Test-Path $d)) { Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue }
}
function App-Args($lab, [string[]]$extra = @()) {
    @('--dev', '--install', "$lab\install", '--provider', "$lab\provider", '--state', "$lab\state", '--name', 'Mem-Check', '--interval-ms', '5000', '--recent-ms', '2000') + $extra
}
function Launch([string[]]$a) {
    $p = Start-Process $App -ArgumentList $a -PassThru
    Get-Record $p.Id
}

# ---- GUI helpers (UIA on the tracked root's window only) ----
function Root-Element([int]$ProcId) {
    $w = Get-AppWindow $ProcId
    if ($w) { [Windows.Automation.AutomationElement]::FromHandle([IntPtr]$w[0]) }
}
function Find-Named([int]$ProcId, [string]$like) {
    $r = Root-Element $ProcId
    if (-not $r) { return $null }
    foreach ($e in $r.FindAll([Windows.Automation.TreeScope]::Descendants, [Windows.Automation.Condition]::TrueCondition)) {
        if ($e.Current.Name -like $like) { return $e }
    }
}
function Invoke-Named([int]$ProcId, [string]$like) {
    $e = Find-Named $ProcId $like
    if (-not $e) { return $false }
    if ($e.Current.ControlType.ProgrammaticName -like '*CheckBox') {
        $pat = $e.GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern)
        if ($pat.Current.ToggleState -ne [Windows.Automation.ToggleState]::On) { $pat.Toggle() }
    }
    else { $e.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke() }
    $true
}
function Wait-Named([int]$ProcId, [string]$like, [int]$secs = 30) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.Elapsed.TotalSeconds -lt $secs) { if (Find-Named $ProcId $like) { return $true }; Start-Sleep -Milliseconds 400 }
    $false
}

# Enrol the synthetic install once per run through the GUI (so the state is enrolled and sync runs).
function Enroll-Lab($lab) {
    $rec = Launch (App-Args $lab)
    try {
        if (-not (Wait-Named $rec.Pid 'Create a new sync group' 40)) { throw 'enroll screen did not appear' }
        Start-Sleep -Seconds 1
        # UIA invoke/toggle did not change app state on this PC; posted keys do (focus starts on the checkbox).
        Send-Key $rec.Pid 0x20; Start-Sleep -Milliseconds 500   # Space: tick "I agree"
        Send-Key $rec.Pid 0x09; Start-Sleep -Milliseconds 500   # Tab: move to Create
        Send-Key $rec.Pid 0x0D                                   # Enter: Create a new sync group
        if (-not (Wait-Named $rec.Pid 'Sync now' 40)) { throw 'enrolment did not reach the status screen' }
        Start-Sleep -Seconds 4
    }
    finally { [void](Stop-Launched $rec) }
}

# ---- sync-start time: launch -> first change in state/provider files (readiness + first sync) ----
function Snapshot($lab) {
    $m = @{}
    foreach ($d in "$lab\state", "$lab\provider") {
        Get-ChildItem $d -Recurse -File -ErrorAction SilentlyContinue | ForEach-Object { $m[$_.FullName] = "$($_.LastWriteTimeUtc.Ticks)/$($_.Length)" }
    }
    $m
}
function Measure-SyncStart($lab, $before, [Diagnostics.Stopwatch]$sw, [int]$timeoutS = 30) {
    while ($sw.Elapsed.TotalSeconds -lt $timeoutS) {
        $now = Snapshot $lab
        foreach ($k in $now.Keys) {
            if ((-not $before.ContainsKey($k)) -or $before[$k] -ne $now[$k]) {
                return [pscustomobject]@{ seconds = [math]::Round($sw.Elapsed.TotalSeconds, 2); signal = $k.Substring($lab.Length + 1) }
            }
        }
        Start-Sleep -Milliseconds 50
    }
    [pscustomobject]@{ seconds = $null; signal = 'timeout' }
}

function Launch-Measured($lab, [string[]]$extra, [string]$tag) {
    $before = Snapshot $lab
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $rec = Launch (App-Args $lab $extra)
    $ss = Measure-SyncStart $lab $before $sw
    $SyncStart.Add([pscustomobject]@{ run = $tag; seconds = $ss.seconds; signal = $ss.signal })
    Note "$tag sync start: $($ss.seconds) s ($($ss.signal))"
    $rec
}

function Finish-Run($run, $rec, [string]$tag) {
    $stopped = Stop-Launched $rec
    if (-not $stopped) { $Findings.Add("${tag}: app root did not exit after Stop-Process") }
    Start-Sleep -Milliseconds 500
    foreach ($o in $run.Seen.Values) {
        if ($o.Pid -ne $rec.Pid -and (Test-Alive $o)) { $Findings.Add("${tag}: app-owned child pid $($o.Pid) survived the root (not stopped by the harness)") }
    }
    $maxProcs = ($run.Rows | Measure-Object procs -Maximum).Maximum
    if ($maxProcs -gt 1) { $Findings.Add("${tag}: app-owned process count reached $maxProcs") }
}

function Show-Via-SecondInstance($lab, [int]$rootPid) {
    $sp = Start-Process $App -ArgumentList (App-Args $lab) -PassThru
    $s = try { Get-Record $sp.Id } catch { $null }   # it normally exits within milliseconds
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($s -and (Test-Alive $s) -and $sw.Elapsed.TotalSeconds -lt 10) { Start-Sleep -Milliseconds 100 }
    if ($s -and (Test-Alive $s)) { $Findings.Add('second instance did not exit; stopped by tracked PID'); [void](Stop-Launched $s) }
    $sw.Restart()
    while ($sw.Elapsed.TotalSeconds -lt 5) { $w = Get-AppWindow $rootPid; if ($w -and $w[2]) { return $true }; Start-Sleep -Milliseconds 100 }
    $false
}

# ---- statistics ----
function Median($v) { $s = @($v | Sort-Object); if ($s.Count -eq 0) { return $null }; $n = $s.Count; if ($n % 2) { $s[($n - 1) / 2] } else { ($s[$n / 2 - 1] + $s[$n / 2]) / 2 } }
function MiB($b) { [math]::Round($b / 1MB, 2) }
function Phase-Stats($run, [string]$phase, [int]$lastN) {
    $rows = @($run.Rows | Where-Object phase -eq $phase)
    if ($lastN -gt 0 -and $rows.Count -gt $lastN) { $rows = $rows[($rows.Count - $lastN)..($rows.Count - 1)] }
    $v = @($rows | ForEach-Object private_ws_b)
    [pscustomobject]@{
        min = ($v | Measure-Object -Minimum).Minimum; median = Median $v; max = ($v | Measure-Object -Maximum).Maximum
        pb = Median @($rows | ForEach-Object private_bytes_b); ws = Median @($rows | ForEach-Object working_set_b)
        gd = Median @($rows | ForEach-Object gpu_dedicated_b); gs = Median @($rows | ForEach-Object gpu_shared_b); n = $rows.Count
    }
}

New-Item -ItemType Directory -Force $LabRoot | Out-Null
foreach ($sc in $Scenarios) {
    for ($i = 1; $i -le $Runs; $i++) {
        $tag = "$sc-run$i"
        Note "=== $tag ==="
        $lab = New-Lab $tag
        $run = New-Run; $rec = $null
        try {
            Enroll-Lab $lab
            switch ($sc) {
                'a' {
                    $rec = Launch-Measured $lab @('--start-hidden') $tag
                    $w = Get-AppWindow $rec.Pid
                    if ($w -and $w[2]) { $Findings.Add("${tag}: window visible although started hidden") }
                    Invoke-Phase $run $rec 'idle' (Secs 120)
                }
                'b' {
                    $rec = Launch-Measured $lab @() $tag
                    $pid_ = $rec.Pid
                    Invoke-Phase $run $rec 'shown' (Secs 60) {
                        param($t)
                        Send-Wheel $pid_ $(if (($t % 10) -lt 5) { -360 } else { 360 })
                        if ($t % 5 -eq 0) { Send-Key $pid_ 0x0D }   # Enter on the focused "Sync now" button
                    }
                    Close-AppWindow $pid_
                    Start-Sleep -Milliseconds 800
                    $w = Get-AppWindow $pid_
                    if ($w -and $w[2]) { $Findings.Add("${tag}: window still visible after close") }
                    Invoke-Phase $run $rec 'hidden' (Secs 120)
                }
                'c' {
                    $rec = Launch-Measured $lab @('--start-hidden') $tag
                    $pid_ = $rec.Pid; $script:cyc = 0
                    Invoke-Phase $run $rec 'cycles' 60 {
                        param($t)
                        if ($t % 3 -eq 0) { if (Show-Via-SecondInstance $lab $pid_) { $script:cyc++ } }
                        elseif ($t % 3 -eq 1) { Close-AppWindow $pid_ }
                    }
                    Note "$tag shown-ok cycles: $($script:cyc)/20"
                    if ($script:cyc -ne 20) { $Findings.Add("${tag}: only $($script:cyc)/20 show cycles confirmed visible") }
                    Invoke-Phase $run $rec 'idle' (Secs 300)
                }
            }
            Save-Csv $run "$OutDir\$tag.csv"
            $steadyPhase = if ($sc -eq 'b') { 'hidden' } else { 'idle' }
            $steady = Phase-Stats $run $steadyPhase 60
            $peak = ($run.Rows | Measure-Object private_ws_b -Maximum).Maximum
            $Results.Add([pscustomobject]@{ scenario = $sc; run = $i; steady = $steady; whole_max = $peak })
            Note ("{0}: steady median {1} MiB, whole-run max {2} MiB" -f $tag, (MiB $steady.median), (MiB $peak))
        }
        finally {
            if ($rec) { Finish-Run $run $rec $tag }
            Remove-Lab $lab
        }
    }
}

# ---- summary ----
$sb = [Text.StringBuilder]::new()
[void]$sb.AppendLine("A-29 summary ($(Get-Date -Format 'yyyy-MM-dd HH:mm')), scale=$Scale; private working set, MiB; limit 100")
foreach ($sc in $Scenarios) {
    $rs = @($Results | Where-Object scenario -eq $sc)
    if ($rs.Count -eq 0) { continue }
    $med = @($rs | ForEach-Object { $_.steady.median })
    $wm = @($rs | ForEach-Object { $_.whole_max })
    [void]$sb.AppendLine(("scenario {0}: runs={1} steady-median per run [{2}] min/median/max = {3}/{4}/{5}; whole-run max {6}; verdict {7}" -f `
            $sc, $rs.Count, (($med | ForEach-Object { MiB $_ }) -join ', '), (MiB ($med | Measure-Object -Minimum).Minimum), (MiB (Median $med)), (MiB ($med | Measure-Object -Maximum).Maximum), (MiB ($wm | Measure-Object -Maximum).Maximum), $(if (($wm | Measure-Object -Maximum).Maximum -le 100MB) { 'PASS' } else { 'FAIL' })))
    [void]$sb.AppendLine(("   context (median of runs): private bytes {0}, working set {1}, gpu dedicated {2}, gpu shared {3}" -f (MiB (Median @($rs | ForEach-Object { $_.steady.pb }))), (MiB (Median @($rs | ForEach-Object { $_.steady.ws }))), (MiB (Median @($rs | ForEach-Object { $_.steady.gd }))), (MiB (Median @($rs | ForEach-Object { $_.steady.gs })))))
}
$ssv = @($SyncStart | Where-Object seconds -ne $null | ForEach-Object seconds)
[void]$sb.AppendLine(("sync start (launch -> first state/provider change), s: n={0} min/median/max = {1}/{2}/{3}; timeouts={4}" -f $ssv.Count, ($ssv | Measure-Object -Minimum).Minimum, (Median $ssv), ($ssv | Measure-Object -Maximum).Maximum, @($SyncStart | Where-Object seconds -eq $null).Count))
foreach ($s in $SyncStart) { [void]$sb.AppendLine(("   {0}: {1} s [{2}]" -f $s.run, $s.seconds, $s.signal)) }
[void]$sb.AppendLine("findings: $($Findings.Count)")
foreach ($f in $Findings) { [void]$sb.AppendLine("   $f") }
$text = $sb.ToString()
Write-Host $text
$name = "summary-$(($Scenarios -join ''))-$(Get-Date -Format 'HHmmss').txt"
[IO.File]::WriteAllText("$OutDir\$name", $text, [Text.UTF8Encoding]::new($false))
