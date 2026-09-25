$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public static class OfficeOm3 {
    public const uint OBJID_NATIVEOM = 0xFFFFFFF0;
    public static readonly Guid IID_IDispatch = new Guid("00020400-0000-0000-C000-000000000046");

    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern bool EnumChildWindows(IntPtr hWndParent, EnumWindowsProc lpEnumFunc, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetClassName(IntPtr hWnd, StringBuilder lpClassName, int nMaxCount);

    [DllImport("oleacc.dll")]
    public static extern int AccessibleObjectFromWindow(IntPtr hwnd, uint dwObjectID, ref Guid riid, [MarshalAs(UnmanagedType.IDispatch)] out object ppvObject);

    public static List<IntPtr> FindTop(uint processId, string className) {
        List<IntPtr> found = new List<IntPtr>();
        EnumWindows(delegate(IntPtr hWnd, IntPtr lParam) {
            uint wpid;
            GetWindowThreadProcessId(hWnd, out wpid);
            if (wpid != processId) return true;
            StringBuilder sb = new StringBuilder(256);
            GetClassName(hWnd, sb, 256);
            if (string.Equals(sb.ToString(), className, StringComparison.Ordinal)) found.Add(hWnd);
            return true;
        }, IntPtr.Zero);
        return found;
    }

    public static List<KeyValuePair<IntPtr,string>> Children(IntPtr parent) {
        List<KeyValuePair<IntPtr,string>> found = new List<KeyValuePair<IntPtr,string>>();
        EnumChildWindows(parent, delegate(IntPtr hWnd, IntPtr lParam) {
            StringBuilder sb = new StringBuilder(256);
            GetClassName(hWnd, sb, 256);
            found.Add(new KeyValuePair<IntPtr,string>(hWnd, sb.ToString()));
            return true;
        }, IntPtr.Zero);
        return found;
    }

    public static object FromWindow(IntPtr hwnd) {
        Guid iid = IID_IDispatch;
        object obj;
        int hr = AccessibleObjectFromWindow(hwnd, OBJID_NATIVEOM, ref iid, out obj);
        if (hr != 0 || obj == null) throw new Exception("hr=0x" + hr.ToString("X8") + " hwnd=" + hwnd);
        return obj;
    }
}
"@

function Try-Bind($processId, $topClass, $preferChild) {
    Write-Host ("---- pid=$processId class=$topClass ----")
    $tops = [OfficeOm3]::FindTop([uint32]$processId, $topClass)
    Write-Host ("tops=" + $tops.Count)
    foreach ($t in $tops) {
        Write-Host ("TOP $t")
        $kids = [OfficeOm3]::Children($t)
        $order = New-Object System.Collections.ArrayList
        foreach ($k in $kids) {
            if ($preferChild -and $k.Value -eq $preferChild) { [void]$order.Add($k.Key) }
        }
        foreach ($k in $kids) {
            if ($k.Value -match 'WwG|EXCEL7') { [void]$order.Add($k.Key) }
        }
        [void]$order.Add($t)
        foreach ($h in $order) {
            try {
                $om = [OfficeOm3]::FromWindow($h)
                $app = $om
                try { $app = $om.Application } catch {}
                Write-Host ("BOUND hwnd=$h Name=$($app.Name) Ver=$($app.Version) Path=$($app.Path)")
                return $app
            } catch {
                Write-Host ("  fail hwnd=$h $($_.Exception.Message)")
            }
        }
    }
    return $null
}

$word = Get-Process WINWORD -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '*Microsoft Office*' } | Select-Object -First 1
$excel = Get-Process EXCEL -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '*Microsoft Office*' } | Select-Object -First 1
Write-Host "word pid=$($word.Id) title=$($word.MainWindowTitle) hwnd=$($word.MainWindowHandle)"
Write-Host "excel pid=$($excel.Id) title=$($excel.MainWindowTitle) hwnd=$($excel.MainWindowHandle)"

$wapp = $null
$eapp = $null
if ($word) { $wapp = Try-Bind $word.Id 'OpusApp' '_WwG' }
if ($excel) { $eapp = Try-Bind $excel.Id 'XLMAIN' 'EXCEL7' }

if ($wapp) { Write-Host "WORD OK Path=$($wapp.Path) Ver=$($wapp.Version)" } else { Write-Host "WORD FAIL" }
if ($eapp) { Write-Host "EXCEL OK Path=$($eapp.Path) Ver=$($eapp.Version)" } else { Write-Host "EXCEL FAIL" }
