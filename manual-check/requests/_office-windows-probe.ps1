$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;

public static class OfficeOm2 {
    public const uint OBJID_NATIVEOM = 0xFFFFFFF0;
    public static readonly Guid IID_IDispatch = new Guid("00020400-0000-0000-C000-000000000046");
    public static readonly Guid IID_IUnknown = new Guid("00000000-0000-0000-C000-000000000046");

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

    [DllImport("ole32.dll")]
    public static extern int GetRunningObjectTable(int reserved, out IRunningObjectTable pprot);

    [DllImport("ole32.dll")]
    public static extern int CreateBindCtx(int reserved, out IBindCtx ppbc);

    public static List<IntPtr> AllWindows(uint pid) {
        List<IntPtr> found = new List<IntPtr>();
        EnumWindows(delegate(IntPtr hWnd, IntPtr lParam) {
            uint wpid;
            GetWindowThreadProcessId(hWnd, out wpid);
            if (wpid == pid) found.Add(hWnd);
            return true;
        }, IntPtr.Zero);
        return found;
    }

    public static List<IntPtr> AllChildren(IntPtr parent) {
        List<IntPtr> found = new List<IntPtr>();
        EnumChildWindows(parent, delegate(IntPtr hWnd, IntPtr lParam) {
            found.Add(hWnd);
            return true;
        }, IntPtr.Zero);
        return found;
    }

    public static string ClassName(IntPtr hwnd) {
        StringBuilder sb = new StringBuilder(256);
        GetClassName(hwnd, sb, 256);
        return sb.ToString();
    }

    public static string TryOm(IntPtr hwnd, bool unk) {
        Guid iid = unk ? IID_IUnknown : IID_IDispatch;
        object obj;
        int hr = AccessibleObjectFromWindow(hwnd, OBJID_NATIVEOM, ref iid, out obj);
        if (hr != 0) return "hr=0x" + hr.ToString("X8");
        if (obj == null) return "null";
        return obj.GetType().FullName;
    }

    public static List<string> RotNames() {
        List<string> names = new List<string>();
        IRunningObjectTable rot;
        if (GetRunningObjectTable(0, out rot) != 0 || rot == null) return names;
        IEnumMoniker enumerator;
        rot.EnumRunning(out enumerator);
        if (enumerator == null) return names;
        enumerator.Reset();
        IMoniker[] monikers = new IMoniker[1];
        IntPtr fetched = IntPtr.Zero;
        IBindCtx ctx;
        CreateBindCtx(0, out ctx);
        while (enumerator.Next(1, monikers, fetched) == 0) {
            string name;
            monikers[0].GetDisplayName(ctx, null, out name);
            names.Add(name);
            Marshal.ReleaseComObject(monikers[0]);
        }
        Marshal.ReleaseComObject(enumerator);
        Marshal.ReleaseComObject(rot);
        if (ctx != null) Marshal.ReleaseComObject(ctx);
        return names;
    }
}
"@

$exe = 'C:\Program Files\Microsoft Office\Root\Office16\WINWORD.EXE'
$proc = Start-Process -FilePath $exe -PassThru
$until = (Get-Date).AddSeconds(20)
while ((Get-Date) -lt $until) {
    $proc.Refresh()
    if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 250
}
$proc.Refresh()
Write-Output ("pid=" + $proc.Id + " main=" + $proc.MainWindowHandle)
$tops = [OfficeOm2]::AllWindows([uint32]$proc.Id)
Write-Output ("top windows=" + $tops.Count)
foreach ($h in $tops) {
    $cls = [OfficeOm2]::ClassName($h)
    Write-Output ("TOP " + $h + " " + $cls)
    $kids = [OfficeOm2]::AllChildren($h)
    foreach ($c in $kids) {
        $ccls = [OfficeOm2]::ClassName($c)
        if ($ccls -match 'Ww|Opus|NetUI|EXCEL|XLMAIN') {
            $d = [OfficeOm2]::TryOm($c, $false)
            $u = [OfficeOm2]::TryOm($c, $true)
            Write-Output ("  CHILD " + $c + " " + $ccls + " disp=" + $d + " unk=" + $u)
        }
    }
    $d0 = [OfficeOm2]::TryOm($h, $false)
    Write-Output ("  TOP disp=" + $d0)
}
Write-Output "==== ROT ===="
[OfficeOm2]::RotNames() | ForEach-Object { Write-Output $_ }

try { Stop-Process -Id $proc.Id -Force } catch {}
