$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public static class OfficeOm {
    public const uint OBJID_NATIVEOM = 0xFFFFFFF0;
    public static readonly Guid IID_IDispatch = new Guid("00020400-0000-0000-C000-000000000046");

    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetClassName(IntPtr hWnd, StringBuilder lpClassName, int nMaxCount);

    [DllImport("user32.dll")]
    public static extern IntPtr FindWindowEx(IntPtr hwndParent, IntPtr hwndChildAfter, string lpszClass, string lpszWindow);

    [DllImport("oleacc.dll")]
    public static extern int AccessibleObjectFromWindow(IntPtr hwnd, uint dwObjectID, ref Guid riid, [MarshalAs(UnmanagedType.IDispatch)] out object ppvObject);

    public static List<IntPtr> FindWindows(uint pid, string className) {
        List<IntPtr> found = new List<IntPtr>();
        EnumWindows(delegate(IntPtr hWnd, IntPtr lParam) {
            uint wpid;
            GetWindowThreadProcessId(hWnd, out wpid);
            if (wpid != pid) return true;
            StringBuilder sb = new StringBuilder(256);
            GetClassName(hWnd, sb, 256);
            if (string.Equals(sb.ToString(), className, StringComparison.Ordinal)) {
                found.Add(hWnd);
            }
            return true;
        }, IntPtr.Zero);
        return found;
    }

    public static object FromWindow(IntPtr hwnd) {
        Guid iid = IID_IDispatch;
        object obj;
        int hr = AccessibleObjectFromWindow(hwnd, OBJID_NATIVEOM, ref iid, out obj);
        if (hr != 0 || obj == null) {
            throw new Exception("AccessibleObjectFromWindow hr=0x" + hr.ToString("X8") + " hwnd=" + hwnd);
        }
        return obj;
    }
}
"@

function Wait-MainWindow([System.Diagnostics.Process]$proc, [int]$seconds) {
    $until = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $until) {
        $proc.Refresh()
        if ($proc.HasExited) { throw "process exited early pid=$($proc.Id)" }
        if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { return }
        Start-Sleep -Milliseconds 200
    }
}

function Get-MsWord {
    $exe = 'C:\Program Files\Microsoft Office\Root\Office16\WINWORD.EXE'
    $proc = Start-Process -FilePath $exe -PassThru
    Wait-MainWindow $proc 30
    $hwnds = [OfficeOm]::FindWindows([uint32]$proc.Id, 'OpusApp')
    Write-Output ("Word pid=" + $proc.Id + " opus=" + $hwnds.Count + " main=" + $proc.MainWindowHandle)
    $candidates = @()
    foreach ($h in $hwnds) { $candidates += $h }
    $candidates += $proc.MainWindowHandle
    $wwg = [OfficeOm]::FindWindowEx($proc.MainWindowHandle, [IntPtr]::Zero, '_WwG', $null)
    if ($wwg -ne [IntPtr]::Zero) { $candidates += $wwg }
    $last = $null
    foreach ($h in $candidates) {
        if ($h -eq [IntPtr]::Zero) { continue }
        try {
            $om = [OfficeOm]::FromWindow($h)
            $app = $om.Application
            Write-Output ("Word bound hwnd=" + $h + " Name=" + $app.Name + " Ver=" + $app.Version + " Path=" + $app.Path)
            return @{ App = $app; Proc = $proc }
        } catch {
            $last = $_.Exception.Message
        }
    }
    throw "Word bind failed: $last"
}

function Get-MsExcel {
    $exe = 'C:\Program Files\Microsoft Office\Root\Office16\EXCEL.EXE'
    $proc = Start-Process -FilePath $exe -ArgumentList '/x' -PassThru
    Wait-MainWindow $proc 30
    $hwnds = [OfficeOm]::FindWindows([uint32]$proc.Id, 'XLMAIN')
    Write-Output ("Excel pid=" + $proc.Id + " xlmain=" + $hwnds.Count + " main=" + $proc.MainWindowHandle)
    $last = $null
    foreach ($main in $hwnds) {
        $desk = [OfficeOm]::FindWindowEx($main, [IntPtr]::Zero, 'XLDESK', $null)
        $book = [OfficeOm]::FindWindowEx($desk, [IntPtr]::Zero, 'EXCEL7', $null)
        foreach ($h in @($book, $desk, $main)) {
            if ($h -eq [IntPtr]::Zero) { continue }
            try {
                $om = [OfficeOm]::FromWindow($h)
                $app = $om.Application
                Write-Output ("Excel bound hwnd=" + $h + " Name=" + $app.Name + " Ver=" + $app.Version + " Path=" + $app.Path)
                return @{ App = $app; Proc = $proc }
            } catch {
                $last = $_.Exception.Message
            }
        }
    }
    throw "Excel bind failed: $last"
}

$word = $null
$excel = $null
try {
    $word = Get-MsWord
    $excel = Get-MsExcel
}
finally {
    if ($word -and $word.App) {
        try { $word.App.Quit() } catch {}
    }
    if ($excel -and $excel.App) {
        try { $excel.App.Quit() } catch {}
    }
    Start-Sleep -Milliseconds 500
    if ($word -and $word.Proc -and -not $word.Proc.HasExited) { Stop-Process -Id $word.Proc.Id -Force -ErrorAction SilentlyContinue }
    if ($excel -and $excel.Proc -and -not $excel.Proc.HasExited) { Stop-Process -Id $excel.Proc.Id -Force -ErrorAction SilentlyContinue }
}
