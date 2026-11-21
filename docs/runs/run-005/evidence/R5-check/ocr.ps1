param([int]$AppPid)
$ErrorActionPreference='Stop'
. 'D:\GitHub\wow-sync.worktrees\R5-check\tools\mem\MemLib.ps1'
Add-Type -AssemblyName System.Drawing,System.Runtime.WindowsRuntime,UIAutomationClient,UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class CheckForeground {
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint p);
}
'@
function Await($operation,[Type]$type) {
    $method=[System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
        $_.Name -eq 'AsTask' -and $_.IsGenericMethod -and
        $_.GetGenericArguments().Count -eq 1 -and $_.GetParameters().Count -eq 1 -and
        $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
    } | Select-Object -First 1
    $task=$method.MakeGenericMethod($type).Invoke($null,@($operation))
    $task.GetAwaiter().GetResult()
}
$w=Get-AppWindow $AppPid
if(-not $w){throw 'No window for captured app PID'}
[void][CheckForeground]::SetForegroundWindow([IntPtr]$w[0])
Start-Sleep -Milliseconds 500
$owner=[uint32]0
[void][CheckForeground]::GetWindowThreadProcessId([CheckForeground]::GetForegroundWindow(),[ref]$owner)
if($owner -ne $AppPid){throw 'Refusing capture of another foreground process'}
$root=[Windows.Automation.AutomationElement]::FromHandle([IntPtr]$w[0])
$rect=$root.Current.BoundingRectangle
$image=[Drawing.Bitmap]::new([int]$rect.Width,[int]$rect.Height)
$graphics=[Drawing.Graphics]::FromImage($image)
$memory=[IO.MemoryStream]::new()
try {
    $graphics.CopyFromScreen([int]$rect.X,[int]$rect.Y,0,0,$image.Size)
    $image.Save($memory,[Drawing.Imaging.ImageFormat]::Png)
    $stream=[Windows.Storage.Streams.InMemoryRandomAccessStream,Windows.Foundation,ContentType=WindowsRuntime]::new()
    $writer=[Windows.Storage.Streams.DataWriter,Windows.Foundation,ContentType=WindowsRuntime]::new($stream)
    $writer.WriteBytes($memory.ToArray())
    [void](Await ($writer.StoreAsync()) ([uint32]))
    $stream.Seek(0)
    $decoder=Await ([Windows.Graphics.Imaging.BitmapDecoder,Windows.Foundation,ContentType=WindowsRuntime]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder,Windows.Foundation,ContentType=WindowsRuntime])
    $bitmap=Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap,Windows.Foundation,ContentType=WindowsRuntime])
    $engine=[Windows.Media.Ocr.OcrEngine,Windows.Foundation,ContentType=WindowsRuntime]::TryCreateFromUserProfileLanguages()
    $result=Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult,Windows.Foundation,ContentType=WindowsRuntime])
    $result.Text.Replace('LABACCOUNT','[account]')
} finally {
    $graphics.Dispose()
    $image.Dispose()
    $memory.Dispose()
}
