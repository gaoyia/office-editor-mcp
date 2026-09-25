$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public static class HwndDump {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lp);
    [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h, EnumWindowsProc cb, IntPtr lp);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);

    public static void Dump(uint processId) {
        EnumWindows(delegate(IntPtr hWnd, IntPtr lParam) {
            uint wpid;
            GetWindowThreadProcessId(hWnd, out wpid);
            if (wpid != processId) return true;
            StringBuilder c = new StringBuilder(256);
            StringBuilder t = new StringBuilder(256);
            GetClassName(hWnd, c, 256);
            GetWindowText(hWnd, t, 256);
            Console.WriteLine("TOP " + hWnd + " [" + c + "] '" + t + "'");
            EnumChildWindows(hWnd, delegate(IntPtr ch, IntPtr lp2) {
                StringBuilder cc = new StringBuilder(256);
                StringBuilder tt = new StringBuilder(256);
                GetClassName(ch, cc, 256);
                GetWindowText(ch, tt, 256);
                Console.WriteLine("  " + ch + " [" + cc + "] '" + tt + "'");
                return true;
            }, IntPtr.Zero);
            return true;
        }, IntPtr.Zero);
    }
}
"@

foreach ($p in (Get-Process WINWORD,EXCEL -ErrorAction SilentlyContinue)) {
    Write-Host "======== $($p.Name) $($p.Id) $($p.Path) title=$($p.MainWindowTitle) ========"
    [HwndDump]::Dump([uint32]$p.Id)
}
