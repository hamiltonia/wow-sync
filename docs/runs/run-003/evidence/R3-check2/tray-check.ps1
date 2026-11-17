$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path "$PSScriptRoot\..\..\..\..\..").Path
. "$repo\tools\mem\MemLib.ps1"
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class OwnTray {
  [StructLayout(LayoutKind.Sequential)] public struct Ident { public uint Size; public IntPtr Window; public uint Id; public Guid Guid; }
  [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; }
  [DllImport("shell32.dll")] static extern int Shell_NotifyIconGetRect(ref Ident id, out Rect rect);
  [DllImport("user32.dll")] public static extern IntPtr FindWindowW([MarshalAs(UnmanagedType.LPWStr)] string cls, IntPtr title);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags,uint dx,uint dy,uint data,UIntPtr extra);
  public static int[] Bounds(IntPtr window,uint id) {
    var n=new Ident { Size=(uint)Marshal.SizeOf<Ident>(), Window=window, Id=id };
    Rect r;
    return Shell_NotifyIconGetRect(ref n,out r)==0 && r.Right>r.Left && r.Bottom>r.Top
      ? new int[] { r.Left,r.Top,r.Right,r.Bottom } : null;
  }
}
'@
[void][OwnTray]::SetThreadDpiAwarenessContext([IntPtr](-4))
$lab = "$repo\target\rc2-scratch\gui4"
$p = Start-Process "$repo\target\release\wow-sync.exe" -ArgumentList @('--dev','--install',"$lab\A\install",'--provider',"$lab\provider",'--state',"$lab\A\state",'--name','RC2-A','--start-hidden') -PassThru
$rec = Get-Record $p.Id
Write-Host "TRAY ROOT PID=$($rec.Pid)"
function Expand-Overflow {
    $taskbar = [OwnTray]::FindWindowW('Shell_TrayWnd',[IntPtr]::Zero)
    if (!$taskbar) { return }
    $root = [Windows.Automation.AutomationElement]::FromHandle($taskbar)
    foreach ($e in $root.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition)) {
        if ($e.Current.Name -eq 'Show hidden icons') {
            $e.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern).Invoke()
            Start-Sleep -Seconds 1
            return
        }
    }
}
function Icon {
    foreach ($w in [MemWin]::Windows([uint32]$rec.Pid)) {
        foreach ($id in 0..32) {
            $rect = [OwnTray]::Bounds([IntPtr]$w[0],[uint32]$id)
            if ($rect) { return [pscustomobject]@{ Handle=[IntPtr]$w[0]; Id=[uint32]$id; Rect=$rect } }
        }
    }
}
function Click-OwnIcon([bool]$right) {
    Expand-Overflow
    $icon = Icon
    if (!$icon) { throw 'BLOCKED: no shell rectangle for a tray icon owned by the launched PID' }
    $x = [int](($icon.Rect[0]+$icon.Rect[2])/2)
    $y = [int](($icon.Rect[1]+$icon.Rect[3])/2)
    $point = [Windows.Point]::new($x,$y)
    $element = [Windows.Automation.AutomationElement]::FromPoint($point)
    Write-Host "OWN ICON hwnd=$($icon.Handle) id=$($icon.Id) point=$x,$y name=$($element.Current.Name)"
    if ($element.Current.Name -notlike 'WoW Sync:*') {
        foreach ($child in $element.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition)) {
            Write-Host "POINT CHILD: $($child.Current.Name)"
        }
        throw 'BLOCKED: resolved icon is not exposed at that screen point; no click sent'
    }
    [void][OwnTray]::SetCursorPos($x,$y)
    if ($right) {
        [OwnTray]::mouse_event(0x0008,0,0,0,[UIntPtr]::Zero)
        [OwnTray]::mouse_event(0x0010,0,0,0,[UIntPtr]::Zero)
    } else {
        [OwnTray]::mouse_event(0x0002,0,0,0,[UIntPtr]::Zero)
        [OwnTray]::mouse_event(0x0004,0,0,0,[UIntPtr]::Zero)
    }
    Start-Sleep -Seconds 2
}
try {
    Start-Sleep -Seconds 4
    Write-Host "INITIAL VISIBLE=$((Get-AppWindow $rec.Pid)[2])"
    Click-OwnIcon $false
    Write-Host "LEFT CLICK FROM HIDDEN VISIBLE=$((Get-AppWindow $rec.Pid)[2])"
    Click-OwnIcon $true
    foreach ($w in [MemWin]::Windows([uint32]$rec.Pid)) {
        if ($w[2]) {
            $root = [Windows.Automation.AutomationElement]::FromHandle([IntPtr]$w[0])
            foreach ($e in $root.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition)) {
                if ($e.Current.Name) { Write-Host "RIGHT CLICK OWN WINDOW: $($e.Current.Name)" }
            }
        }
        [void][MemWin]::PostMessageW([IntPtr]$w[0],0x001F,[IntPtr]::Zero,[IntPtr]::Zero)
    }
    Click-OwnIcon $false
    Write-Host "LEFT CLICK FROM VISIBLE VISIBLE=$((Get-AppWindow $rec.Pid)[2])"
}
finally { [void](Stop-Launched $rec) }
