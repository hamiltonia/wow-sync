# Helpers for the G1-G9 checks. Synthetic data under $Lab only. Dot-source this file.
# Every process started is tracked in $Pids and stopped by PID with Stop-All.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Windows.Forms
Add-Type 'using System;using System.Runtime.InteropServices;public class FG{[DllImport("user32.dll")]public static extern bool SetForegroundWindow(IntPtr h);[DllImport("user32.dll")]public static extern void keybd_event(byte a,byte b,uint f,UIntPtr e);[DllImport("user32.dll")]public static extern bool PostMessageW(IntPtr h,uint m,IntPtr w,IntPtr l);[DllImport("user32.dll")]public static extern bool IsWindowVisible(IntPtr h);}'
$AE = [System.Windows.Automation.AutomationElement]
$Repo = (Resolve-Path "$PSScriptRoot\..\..\..\..\..").Path
$App = "$Repo\target\release\wow-sync.exe"
$Dev = "$Repo\target\release\devkit.exe"
$Lab = Join-Path $env:TEMP 'wowsync-glab'
$Pids = New-Object System.Collections.ArrayList

function Stop-All { foreach ($i in $Pids) { try { Stop-Process -Id $i -ErrorAction Stop } catch {} }; $Pids.Clear() }
function Start-App([string]$Install, [string]$Provider, [string]$State, [string]$Name, [string[]]$Extra = @()) {
  $a = @('--dev', '--install', $Install, '--provider', $Provider, '--state', $State, '--name', $Name, '--interval-ms', '1500', '--recent-ms', '800') + $Extra
  $p = Start-Process $App -ArgumentList $a -PassThru
  [void]$Pids.Add($p.Id); Start-Sleep -Seconds 4; $p
}
function Root($p) { $p.Refresh(); $AE::FromHandle($p.MainWindowHandle) }
function Names($p) {
  $r = Root $p
  foreach ($e in $r.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)) {
    if ($e.Current.Name) { '[{0}] {1}' -f ($e.Current.ControlType.ProgrammaticName -replace 'ControlType\.', ''), $e.Current.Name }
  }
}
function Find($p, [string]$like) {
  $r = Root $p
  foreach ($e in $r.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)) {
    if ($e.Current.Name -like $like) { return $e }
  }
}
function Click($p, [string]$like) {
  $e = Find $p $like; if (-not $e) { throw "not found: $like" }
  if ($e.Current.ControlType.ProgrammaticName -like "*CheckBox") { $e.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle() } else { $e.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke() }; Start-Sleep -Milliseconds 600
}
function Wait-Text($p, [string]$like, [int]$secs = 15) {
  $sw = [Diagnostics.Stopwatch]::StartNew()
  while ($sw.Elapsed.TotalSeconds -lt $secs) { if (Find $p $like) { return $true }; Start-Sleep -Milliseconds 400 }
  $false
}
function Shot($p, [string]$file) {
  Add-Type -AssemblyName System.Drawing
  $r = $p.MainWindowHandle
  $b = (Root $p).Current.BoundingRectangle
  $bmp = New-Object System.Drawing.Bitmap ([int]$b.Width), ([int]$b.Height)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen([int]$b.X, [int]$b.Y, 0, 0, $bmp.Size)
  $bmp.Save($file, [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
}
function New-Pc([string]$name, [string]$folder = '_forever_', [string]$product = 'wowf') {
  $d = "$Lab\$name"; New-Item -ItemType Directory -Force "$d\install" | Out-Null
  & $Dev make-install "$d\install" --folder $folder --product $product
  $d
}
function Reset-Lab { Stop-All; if (Test-Path $Lab) { Remove-Item -Recurse -Force $Lab }; New-Item -ItemType Directory -Force $Lab | Out-Null }
function Stop-One([int]$i) { Stop-Process -Id $i }
function Start-Fake([string]$variant) { Copy-Item "$env:SystemRoot\System32\PING.EXE" "$variant\Wow.exe"; $p = Start-Process "$variant\Wow.exe" -ArgumentList @("-t","127.0.0.1") -PassThru -WindowStyle Hidden; [void]$Pids.Add($p.Id); $p.Id }
