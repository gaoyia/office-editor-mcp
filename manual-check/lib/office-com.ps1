# 挂上 Microsoft Office 的 COM 对象。
#
# 本机装了 WPS，`New-Object -ComObject Word.Application` 会被 WPS 抢走，
# 所以这里绕开注册表：直接启动 WINWORD.EXE / EXCEL.EXE，枚举它的窗口，
# 再用 AccessibleObjectFromWindow 从窗口句柄取出原生对象模型。
# 这段 Win32 声明原先在三个脚本里各抄了一遍，现在只留这一份。

if (-not ('OoxpOfficeOm' -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public static class OoxpOfficeOm {
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
        Guid iid = IID_IDispatch; object obj;
        int hr = AccessibleObjectFromWindow(hwnd, OBJID_NATIVEOM, ref iid, out obj);
        if (hr != 0 || obj == null) throw new Exception("hr=0x" + hr.ToString("X8"));
        return obj;
    }
}
"@
}

function Release-Com($obj) {
    if ($null -ne $obj) {
        [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($obj)
    }
}

function Get-OfficeExe([string]$exeName) {
    $roots = @($env:ProgramFiles, ${env:ProgramFiles(x86)}) | Where-Object { $_ }
    foreach ($r in $roots) {
        foreach ($ver in @('Office16', 'Office15', 'Office14')) {
            foreach ($mid in @(@('root', $ver), @($ver))) {
                $parts = @($r, 'Microsoft Office') + $mid + @($exeName)
                $p = [IO.Path]::Combine([string[]]$parts)
                if (Test-Path $p) { return $p }
            }
        }
    }
    $key = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\$exeName"
    if (Test-Path $key) {
        $p = (Get-ItemProperty $key).'(default)'
        if ($p -and (Test-Path $p)) { return $p }
    }
    throw "找不到 $exeName，本机可能没装 Microsoft Office"
}

function Wait-OfficeProcess([string]$name, [int]$seconds) {
    $until = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $until) {
        $p = Get-Process $name -ErrorAction SilentlyContinue |
            Where-Object { $_.Path -like '*Microsoft Office*' -and $_.MainWindowHandle -ne [IntPtr]::Zero } |
            Select-Object -First 1
        if ($p) { return $p }
        Start-Sleep -Milliseconds 400
    }
    throw "等 $name 启动超时"
}

function Bind-OfficeApp($process, [string]$topClass, [string]$childClass) {
    $until = (Get-Date).AddSeconds(25)
    $last = 'no hwnd'
    while ((Get-Date) -lt $until) {
        foreach ($t in [OoxpOfficeOm]::FindTop([uint32]$process.Id, $topClass)) {
            # 先试文档子窗口，再退回顶层窗口，两者都可能拿到对象模型。
            $order = New-Object System.Collections.ArrayList
            foreach ($k in [OoxpOfficeOm]::Children($t)) {
                if ($k.Value -eq $childClass) { [void]$order.Add($k.Key) }
            }
            [void]$order.Add($t)
            foreach ($h in $order) {
                try {
                    $om = [OoxpOfficeOm]::FromWindow($h)
                    $app = $om
                    try { $app = $om.Application } catch {}
                    if ($app.Path -notlike '*Microsoft Office*') {
                        $last = "wrong path $($app.Path)"
                        continue
                    }
                    return $app
                } catch { $last = $_.Exception.Message }
            }
        }
        Start-Sleep -Milliseconds 400
    }
    throw "绑 $($process.Name) 失败：$last"
}

# 一轮里只绑一次，结果缓存在全局变量里。
#
# seedBook / seedDoc 是启动时要打开的文档，不能省：AccessibleObjectFromWindow
# 要从文档子窗口（EXCEL7 / _WwG）上取对象模型，光有个空的 Excel 进程会拿到
# hr=0x80004005。所以不管进程在不在，都先让它把 seed 文件打开——
# Office 已在运行时不会新起进程，只是多开一个窗口。
function Ensure-MsExcel([string]$seedBook) {
    if ($global:OoxpExcel) { return $global:OoxpExcel }
    Start-Process -FilePath (Get-OfficeExe 'EXCEL.EXE') -ArgumentList ('"' + $seedBook + '"')
    $p = Wait-OfficeProcess 'EXCEL' 40
    $app = Bind-OfficeApp $p 'XLMAIN' 'EXCEL7'
    $app.DisplayAlerts = $false
    $app.AskToUpdateLinks = $false
    Write-Log ("Excel Name=$($app.Name) Ver=$($app.Version) Path=$($app.Path)")
    $global:OoxpExcel = $app
    return $app
}

function Ensure-MsWord([string]$seedDoc) {
    if ($global:OoxpWord) { return $global:OoxpWord }
    Start-Process -FilePath (Get-OfficeExe 'WINWORD.EXE') -ArgumentList ('"' + $seedDoc + '"')
    $p = Wait-OfficeProcess 'WINWORD' 40
    $app = Bind-OfficeApp $p 'OpusApp' '_WwG'
    $app.DisplayAlerts = 0
    try { $app.Options.ConfirmConversions = $false } catch {}
    try { $app.Options.BackgroundSave = $false } catch {}
    Write-Log ("Word Name=$($app.Name) Ver=$($app.Version) Path=$($app.Path)")
    $global:OoxpWord = $app
    return $app
}

function Open-ExcelBook($excel, [string]$path) {
    foreach ($wb in @($excel.Workbooks)) {
        if ($wb.FullName -and ($wb.FullName -ieq $path)) { return $wb }
    }
    return $excel.Workbooks.Open($path, 0, $false)
}

function Open-WordDoc($word, [string]$path) {
    foreach ($d in @($word.Documents)) {
        if ($d.FullName -and ($d.FullName -ieq $path)) { return $d }
    }
    return $word.Documents.Open($path)
}

function Close-AllWorkbooks($excel) {
    foreach ($wb in @($excel.Workbooks)) {
        try { $wb.Close($false) } catch {}
        Release-Com $wb
    }
}

function Close-WordDoc($doc) {
    if ($null -ne $doc) {
        try { $doc.Close($false) } catch {}
        Release-Com $doc
    }
}

# apply 的目标文件若正被 Word 开着，ooxp 写不进去，所以写之前先关掉。
function Close-WordDocByPath($word, [string]$path) {
    foreach ($d in @($word.Documents)) {
        if ($d.FullName -and ($d.FullName -ieq $path)) {
            Write-Log "先关掉 Word 里开着的 $path"
            Close-WordDoc $d
            return
        }
    }
}

# 段落可见文本：Word 的 Range 末尾带段落标记，比对前要去掉。
function Get-ParaText($para) {
    $r = $para.Range
    $r.MoveEnd(1, -1) | Out-Null
    $text = [string]$r.Text
    Release-Com $r
    return $text
}
