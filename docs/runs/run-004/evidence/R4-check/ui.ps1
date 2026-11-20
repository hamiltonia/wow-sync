param([int]$AppPid, [string]$Action = 'dump', [string]$Name = '', [int]$Key = 9)
$ErrorActionPreference = 'Stop'
$repo = 'D:\GitHub\wow-sync.worktrees\R4-check'
. "$repo\tools\mem\MemLib.ps1"
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
function Root {
    $w = Get-AppWindow $AppPid
    if (-not $w) { throw "No WoW Sync window for captured PID $AppPid" }
    [Windows.Automation.AutomationElement]::FromHandle([IntPtr]$w[0])
}
function Elements {
    (Root).FindAll([Windows.Automation.TreeScope]::Descendants, [Windows.Automation.Condition]::TrueCondition)
}
function Named($pattern) {
    foreach ($e in (Elements)) {
        if ($Action -in @('invoke', 'focus') -and
            $e.Current.ControlType.ProgrammaticName -notmatch 'Button|CheckBox|RadioButton|ListItem') { continue }
        if ($e.Current.Name -like $pattern) { return $e }
    }
    throw "No control '$pattern' for captured PID $AppPid"
}
switch ($Action) {
    'dump' {
        foreach ($e in (Elements)) {
            if ($e.Current.Name) {
                "{0} | enabled={1} focus={2} | {3}" -f $e.Current.ControlType.ProgrammaticName, $e.Current.IsEnabled, $e.Current.HasKeyboardFocus, $e.Current.Name
            }
        }
    }
    'invoke' {
        $e = Named $Name
        if ($e.Current.ControlType.ProgrammaticName -like '*CheckBox') {
            $e.GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Toggle()
        } else {
            $e.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
        }
        Start-Sleep -Milliseconds 1300
    }
    'focus' { (Named $Name).SetFocus() }
    'key' { Send-Key $AppPid $Key; Start-Sleep -Milliseconds 600 }
    'close' { Close-AppWindow $AppPid; Start-Sleep -Seconds 1 }
    'shot' {
        Add-Type -AssemblyName System.Drawing
        if (-not ('CheckCapture' -as [type])) {
            Add-Type @'
using System;
using System.Runtime.InteropServices;
public class CheckCapture {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int L,T,R,B; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out Rect r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
}
'@
        }
        $w = Get-AppWindow $AppPid
        [void][CheckCapture]::SetForegroundWindow([IntPtr]$w[0])
        Start-Sleep -Milliseconds 500
        $foregroundPid = [uint32]0
        [void][CheckCapture]::GetWindowThreadProcessId([CheckCapture]::GetForegroundWindow(), [ref]$foregroundPid)
        if ($foregroundPid -ne $AppPid) { throw 'Refusing screenshot: target is not the foreground window.' }
        $r = New-Object CheckCapture+Rect
        [void][CheckCapture]::GetWindowRect([IntPtr]$w[0], [ref]$r)
        $bmp = [Drawing.Bitmap]::new($r.R-$r.L-24, $r.B-$r.T-24)
        $graphics = [Drawing.Graphics]::FromImage($bmp)
        try {
            $graphics.CopyFromScreen($r.L+12, $r.T+12, 0, 0, $bmp.Size)
            $dest = "$repo\docs\runs\run-004\evidence\R4-check\$Name.png"
            $bmp.Save($dest, [Drawing.Imaging.ImageFormat]::Png)
            $dest
        } finally { $graphics.Dispose(); $bmp.Dispose() }
    }
    'enroll' {
        foreach ($n in @('Next', 'Use this folder', 'Next', 'Next', 'This is my*', 'These are the*', 'Next', 'Create a new group', 'Next', 'Set up this computer', 'Finish and open*')) {
            $e = Named $n
            if ($e.Current.ControlType.ProgrammaticName -like '*CheckBox') {
                $e.GetCurrentPattern([Windows.Automation.TogglePattern]::Pattern).Toggle()
            } else {
                $e.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
            }
            Start-Sleep -Milliseconds 1300
        }
    }
}
