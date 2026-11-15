# S-dropbox observation script. Touches ONLY the synthetic work folder under the test root.
param([string]$OutDir = "$PSScriptRoot\..\..\docs\runs\run-001\evidence\S-dropbox", [int]$BigMB = 200)
$ErrorActionPreference = 'Continue'
$root = 'D:\Dropbox\Gaming Sync\WoW Sync Test'
$work = Join-Path $root ("s-dropbox-" + (Get-Date -Format 'yyyyMMdd'))
New-Item -ItemType Directory $work -Force | Out-Null
New-Item -ItemType Directory $OutDir -Force | Out-Null
$log = Join-Path $OutDir 'observations.txt'
function L($m) { $s = "{0:HH:mm:ss.fff} {1}" -f (Get-Date), $m; Add-Content $log $s; $s }
function Info($p) {
  if (-not (Test-Path -LiteralPath $p)) { return "missing" }
  $i = Get-Item -LiteralPath $p -Force
  $tag = (fsutil reparsepoint query $p 2>&1 | Select-String 'Reparse Tag Value' | % { $_.Line.Trim() }) -join ''
  $v=[int]$i.Attributes; $n=@(@{0x20='ARCHIVE'},@{0x400='REPARSE'},@{0x1000='OFFLINE'},@{0x40000='RECALL_ON_OPEN'},@{0x80000='PINNED'},@{0x100000='UNPINNED'},@{0x400000='RECALL_ON_DATA_ACCESS'}) | % { foreach($k in $_.Keys){ if($v -band $k){$_[$k]} } }; "attr=0x{0:X} [{1}] len={2} tag='{3}'" -f $v, ($n -join '|'), $i.Length, $tag
}
Set-Content $log "S-dropbox observations $(Get-Date -Format o)"
L "root: $(Info $root)"; L "work: $(Info $work)"

for ($rep = 1; $rep -le 2; $rep++) {
  L "=== 2a rep ${rep}: attributes / online-only toggle ==="
  $f = Join-Path $work "hyd$rep.bin"
  $bytes = New-Object byte[] (4MB); (New-Object Random 1).NextBytes($bytes)
  [IO.File]::WriteAllBytes($f, $bytes)
  L "created: $(Info $f)"
  Start-Sleep 8
  L "after 8s: $(Info $f)"
  attrib +U -P $f 2>&1 | % { L "attrib +U -P: $_" }
  foreach ($w in 1..10) { Start-Sleep 1; $a = Info $f; if ($a -match 'UNPINNED|OFFLINE|RECALL_ON_DATA') { break } }
  L "after +U (waited ${w}s): $(Info $f)"
  # metadata reads must not hydrate
  $null = Get-Item -LiteralPath $f; $null = Get-ChildItem $work; $null = (Get-Item -LiteralPath $f).LastWriteTimeUtc
  L "after metadata reads: $(Info $f)"
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $b = [IO.File]::ReadAllBytes($f)
  $sw.Stop()
  L "read online-only file: $($b.Length) bytes in $($sw.ElapsedMilliseconds) ms; $(Info $f)"
  Start-Sleep 3
  L "3s after read: $(Info $f)"
  attrib +P -U $f 2>&1 | % { L "attrib +P -U: $_" }
  Start-Sleep 3
  L "after +P: $(Info $f)"
}

for ($rep = 1; $rep -le 2; $rep++) {
  L "=== 2b rep ${rep}: large file upload visibility ($BigMB MB) ==="
  $big = Join-Path $work "big$rep.bin"
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $fs = [IO.File]::Create($big); $buf = New-Object byte[] (1MB); $rnd = New-Object Random $rep
  for ($i = 0; $i -lt $BigMB; $i++) { $rnd.NextBytes($buf); $fs.Write($buf, 0, $buf.Length) }
  $fs.Dispose(); $sw.Stop()
  L "write done in $($sw.ElapsedMilliseconds) ms"
  $seen = @{}
  for ($t = 0; $t -lt 90; $t++) {
    $names = (Get-ChildItem $work -Force | % { $_.Name }) -join ','
    $a = Info $big
    $key = "$names|$a"
    if (-not $seen.ContainsKey($key)) { $seen[$key] = 1; L "t=${t}s dir=[$names] big=$a" }
    Start-Sleep 1
  }
  # Open exclusive: can a reader detect an in-progress upload via sharing?
  try { $h = [IO.File]::Open($big, 'Open', 'ReadWrite', 'None'); $h.Dispose(); L "exclusive open ok" } catch { L "exclusive open failed: $($_.Exception.Message)" }
}

for ($rep = 1; $rep -le 2; $rep++) {
  L "=== 2c rep ${rep}: replace patterns ==="
  $d = Join-Path $work "rep$rep"; New-Item -ItemType Directory $d -Force | Out-Null
  $f = Join-Path $d 'target.txt'
  Set-Content $f 'v1'
  $seen = New-Object Collections.Generic.List[string]
  $sw = [Diagnostics.Stopwatch]::StartNew()
  Set-Content "$f.tmp" 'v2'; Move-Item "$f.tmp" $f -Force
  L "temp+rename replace ok: $(Get-Content $f) ($($sw.ElapsedMilliseconds) ms) $(Info $f)"
  [IO.File]::WriteAllText($f, 'v3'); L "in-place overwrite ok: $(Get-Content $f) $(Info $f)"
  Rename-Item $f 'renamed.txt'; L "rename: $((Get-ChildItem $d -Force).Name -join ',')"
  Rename-Item $d "rep$rep-dir2"; L "dir rename: $((Get-ChildItem $work -Directory).Name -join ',')"
  Start-Sleep 5
  $d2 = Join-Path $work "rep$rep-dir2"
  Rename-Item (Join-Path $d2 'renamed.txt') 'RENAMED.TXT'; L "case-only rename: $((Get-ChildItem $d2).Name -join ',')"
  foreach ($bad in @('a:b.txt', 'a*b.txt', 'con.txt', 'trail.', 'sp ace ')) {
    try { [IO.File]::WriteAllText((Join-Path $d2 $bad), 'x'); L "create '$bad' ok -> $((Get-ChildItem $d2 -Force).Name -join ',')" } catch { L "create '$bad' failed: $($_.Exception.Message)" }
  }
  Remove-Item (Join-Path $work "rep$rep-dir2") -Recurse -Force -ErrorAction SilentlyContinue
  L "delete dir: exists=$(Test-Path (Join-Path $work "rep$rep-dir2"))"
}

for ($rep = 1; $rep -le 2; $rep++) {
  L "=== 2d rep ${rep}: edit while uploading ==="
  $f = Join-Path $work "conf$rep.bin"
  $fs = [IO.File]::Create($f); $buf = New-Object byte[] (1MB); (New-Object Random 5).NextBytes($buf)
  1..100 | % { $fs.Write($buf, 0, $buf.Length) }; $fs.Dispose()
  Start-Sleep 2
  try { $fs = [IO.File]::Open($f, 'Append'); $fs.Write($buf, 0, 1024); $fs.Dispose(); L "append mid-upload ok" } catch { L "append failed: $($_.Exception.Message)" }
  Start-Sleep 40
  L "dir: $((Get-ChildItem $work -Filter 'conf*' -Force).Name -join ',')"
}
L "done"
