param([int]$AppPid,[string]$Action='dump',[string]$Name='',[int]$Key=13)
$ErrorActionPreference='Stop'
$repo='D:\GitHub\wow-sync.worktrees\R4-recheck'
. "$repo\tools\mem\MemLib.ps1"
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes
function Root {
    $w=Get-AppWindow $AppPid
    if(-not $w){throw "No app window for captured PID $AppPid"}
    [Windows.Automation.AutomationElement]::FromHandle([IntPtr]$w[0])
}
function Elements {(Root).FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition)}
function Named($pattern) {
    foreach($e in (Elements)){
        if($e.Current.ControlType.ProgrammaticName -match 'Button|CheckBox|RadioButton|ListItem' -and $e.Current.Name -like $pattern){return $e}
    }
    throw "No actionable control '$pattern'"
}
function Activate($pattern) {
    $e=Named $pattern
    if($e.Current.ControlType.ProgrammaticName -match 'CheckBox'){
        $e.GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Toggle()
    } else {
        $e.SetFocus()
        Send-Key $AppPid 13
    }
    Start-Sleep -Milliseconds 900
}
switch($Action){
    'dump' {
        foreach($e in (Elements)){
            if($e.Current.Name){"{0} | enabled={1} focus={2} | {3}" -f $e.Current.ControlType.ProgrammaticName,$e.Current.IsEnabled,$e.Current.HasKeyboardFocus,$e.Current.Name}
        }
    }
    'invoke' {Activate $Name}
    'focus' {(Named $Name).SetFocus()}
    'key' {Send-Key $AppPid $Key; Start-Sleep -Milliseconds 500}
    'scroll' {Send-Wheel $AppPid $Key; Start-Sleep -Milliseconds 500}
    'enroll' {
        foreach($n in @('Next','Use this folder','Next','Next','This is my*','These are the*','Next','Create a new group','Next','Set up this computer','Finish and open*')){Activate $n}
    }
    'shot' {
        Add-Type -AssemblyName System.Drawing
        Add-Type @'
using System;
using System.Runtime.InteropServices;
public class RecheckCapture {
 [StructLayout(LayoutKind.Sequential)] public struct Rect {public int L,T,R,B;}
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
}
'@
        $w=Get-AppWindow $AppPid
        [void][RecheckCapture]::SetForegroundWindow([IntPtr]$w[0])
        Start-Sleep -Milliseconds 500
        $p=[uint32]0
        [void][RecheckCapture]::GetWindowThreadProcessId([RecheckCapture]::GetForegroundWindow(),[ref]$p)
        if($p -ne $AppPid){throw 'Refusing screenshot of a different foreground PID'}
        $r=New-Object RecheckCapture+Rect
        [void][RecheckCapture]::GetWindowRect([IntPtr]$w[0],[ref]$r)
        $full=[Drawing.Bitmap]::new($r.R-$r.L-24,$r.B-$r.T-24)
        $g=[Drawing.Graphics]::FromImage($full)
        $small=$null
        try {
            $g.CopyFromScreen($r.L+12,$r.T+12,0,0,$full.Size)
            foreach($e in (Elements)){
                if($e.Current.Name -like 'Account *' -and -not $e.Current.IsOffscreen){
                    $box=$e.Current.BoundingRectangle
                    $g.FillRectangle([Drawing.Brushes]::Black,[int]($box.X-$r.L-12),[int]($box.Y-$r.T-12),[int]$box.Width,[int]$box.Height)
                }
            }
            $small=[Drawing.Bitmap]::new($full,[Drawing.Size]::new([int]($full.Width*0.6),[int]($full.Height*0.6)))
            $panel=$small.Clone([Drawing.Rectangle]::new(200,90,$small.Width-200,$small.Height-90),$small.PixelFormat)
            $small.Dispose()
            $small=$panel
            $codec=[Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object MimeType -eq 'image/jpeg'
            $params=[Drawing.Imaging.EncoderParameters]::new(1)
            $params.Param[0]=[Drawing.Imaging.EncoderParameter]::new([Drawing.Imaging.Encoder]::Quality,[long]25)
            $dest="$PSScriptRoot\$Name.jpg"
            $small.Save($dest,$codec,$params)
            $params.Dispose()
            $dest
        } finally {$g.Dispose(); $full.Dispose(); if($small){$small.Dispose()}}
    }
}
