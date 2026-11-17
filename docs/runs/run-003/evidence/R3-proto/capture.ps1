# Captures only the prototype window (PrintWindow on its own HWND) for each named spec.
# Usage: powershell -File capture.ps1 -Exe <path> -Out <dir> -Specs "name|args;name|args"
param([string]$Exe, [string]$Out, [string]$Specs, [int]$Wait = 3)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public class W {
  [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Rt, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
}
"@
New-Item -ItemType Directory -Force -Path $Out | Out-Null
foreach ($spec in $Specs.Split(';')) {
  $name, $args_ = $spec.Split('|')
  $p = Start-Process -FilePath $Exe -ArgumentList $args_ -PassThru
  $h = [IntPtr]::Zero
  for ($i = 0; $i -lt 40 -and $h -eq [IntPtr]::Zero; $i++) {
    Start-Sleep -Milliseconds 250
    $p.Refresh(); $h = $p.MainWindowHandle
  }
  Start-Sleep -Seconds $Wait
  if ($h -eq [IntPtr]::Zero) { "$name : NO WINDOW"; Stop-Process -Id $p.Id; continue }
  $r = New-Object W+R; [void][W]::GetWindowRect($h, [ref]$r)
  $w = $r.Rt - $r.L; $ht = $r.B - $r.T
  $bmp = New-Object System.Drawing.Bitmap $w, $ht
  $g = [System.Drawing.Graphics]::FromImage($bmp); $dc = $g.GetHdc()
  [void][W]::PrintWindow($h, $dc, 2)
  $g.ReleaseHdc($dc); $g.Dispose()
  $bmp.Save((Join-Path $Out "$name.png"), [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  "$name : ${w}x${ht} pid=$($p.Id)"
  Stop-Process -Id $p.Id
}
