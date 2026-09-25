$ErrorActionPreference = 'Stop'
$root = 'C:\Users\GaoYi-AMD\Desktop\test\manual-check\office'
$logPath = Join-Path $root 'office-com-log.txt'
$S = Get-Content -LiteralPath (Join-Path $root 'requests\_strings.json') -Encoding UTF8 -Raw | ConvertFrom-Json
function Write-Log([string]$msg) {
    $line = "$(Get-Date -Format 'HH:mm:ss') $msg"
    Add-Content -Path $logPath -Value $line -Encoding UTF8
    Write-Host $line
}
Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class OmDump {
    public const uint OBJID_NATIVEOM = 0xFFFFFFF0;
    public static readonly Guid IID_IDispatch = new Guid("00020400-0000-0000-C000-000000000046");
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lp);
    [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h, EnumWindowsProc cb, IntPtr lp);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    [DllImport("oleacc.dll")] public static extern int AccessibleObjectFromWindow(IntPtr hwnd, uint id, ref Guid iid, [MarshalAs(UnmanagedType.IDispatch)] out object obj);
    public static List<IntPtr> FindTop(uint processId, string className) {
        List<IntPtr> found = new List<IntPtr>();
        EnumWindows(delegate(IntPtr hWnd, IntPtr lParam) {
            uint wpid; GetWindowThreadProcessId(hWnd, out wpid);
            if (wpid != processId) return true;
            StringBuilder sb = new StringBuilder(256);
            GetClassName(hWnd, sb, 256);
            if (string.Equals(sb.ToString(), className, StringComparison.Ordinal)) found.Add(hWnd);
            return true;
        }, IntPtr.Zero);
        return found;
    }
    public static List<IntPtr> FindChild(IntPtr parent, string className) {
        List<IntPtr> found = new List<IntPtr>();
        EnumChildWindows(parent, delegate(IntPtr hWnd, IntPtr lParam) {
            StringBuilder sb = new StringBuilder(256);
            GetClassName(hWnd, sb, 256);
            if (string.Equals(sb.ToString(), className, StringComparison.Ordinal)) found.Add(hWnd);
            return true;
        }, IntPtr.Zero);
        return found;
    }
    public static object FromWindow(IntPtr hwnd) {
        Guid iid = IID_IDispatch; object obj;
        int hr = AccessibleObjectFromWindow(hwnd, OBJID_NATIVEOM, ref iid, out obj);
        if (hr != 0 || obj == null) throw new Exception("hr=0x" + hr.ToString("X8"));
        return obj;
    }
}
"@
$proc = Get-Process WINWORD | Where-Object { $_.Path -like '*Microsoft Office*' } | Select-Object -First 1
$word = $null
foreach ($t in [OmDump]::FindTop([uint32]$proc.Id, 'OpusApp')) {
    foreach ($h in [OmDump]::FindChild($t, '_WwG')) {
        try {
            $om = [OmDump]::FromWindow($h)
            if ($om.Application.Path -like '*Microsoft Office*') { $word = $om.Application; break }
        } catch {}
    }
    if ($word) { break }
}
if (-not $word) { throw 'no Word' }
$word.DisplayAlerts = 0
$path = Join-Path $root 'docx-applied.docx'
$doc = $null
foreach ($d in @($word.Documents)) {
    if ($d.FullName -and ($d.FullName -ieq $path)) { $doc = $d; break }
}
if (-not $doc) { $doc = $word.Documents.Open($path) }
$q1 = $doc.Paragraphs.Item(1); $t1 = $q1.Range; $t1.MoveEnd(1,-1) | Out-Null
$q2 = $doc.Paragraphs.Item(2); $t2 = $q2.Range; $t2.MoveEnd(1,-1) | Out-Null
$q3 = $doc.Paragraphs.Item(3); $t3 = $q3.Range; $t3.MoveEnd(1,-1) | Out-Null
Write-Log ("A1 caption=" + $word.ActiveWindow.Caption)
Write-Log ("A2 text=" + $t2.Text + " expect=" + $S.para2new + " match=" + ($t2.Text -eq $S.para2new))
Write-Log ("A3 text=" + $t1.Text + " style=" + $q1.Style.NameLocal + " expect=" + $S.chapter1)
Write-Log ("A4 bold=" + $q2.Range.Font.Bold)
Write-Log ("A5 text=" + $t3.Text + " expect=" + $S.para3keep + " match=" + ($t3.Text -eq $S.para3keep))
