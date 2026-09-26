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
    Write-Log ("Excel Name=$($app.Name) Ver=$($app.Version) Path=$($app.Path) Pid=$($p.Id)")
    $global:OoxpExcel = $app
    $global:OoxpExcelPid = $p.Id
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
    Write-Log ("Word Name=$($app.Name) Ver=$($app.Version) Path=$($app.Path) Pid=$($p.Id)")
    $global:OoxpWord = $app
    $global:OoxpWordPid = $p.Id
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

# 跑完收尾。
#
# 不松开 Application 级的引用，Excel 会以为还有自动化客户端连着：
# 你把窗口关掉之后它不退出，变成一个两百多兆、没有窗口的进程赖在内存里。
# 那种进程还会毒害下一次运行——Get-Process 找得到它，可它没有文档子窗口，
# AccessibleObjectFromWindow 只会返回 hr=0x80004005。
#
# 默认只松引用，窗口留着给人肉眼看；加 -Quit 就连 Word / Excel 一起关掉。
function Close-OfficeApps([switch]$Quit) {
    if ($Quit) {
        # 顺序很讲究。用例里留下的工作簿、工作表、段落都是 COM 包装对象，
        # 只要还有一个活着，Excel 就当作「客户端还连着」，Quit 会被直接无视。
        # 所以先强制回收这些零散引用，再关文档，最后才 Quit。
        [GC]::Collect()
        [GC]::WaitForPendingFinalizers()
        [GC]::Collect()

        $excel = Get-Variable -Name 'OoxpExcel' -Scope Global -ValueOnly -ErrorAction SilentlyContinue
        if ($excel) {
            # 别把用户自己开着的表格一起关掉：只要发现 out 目录之外的工作簿就不动手。
            $foreign = @()
            foreach ($wb in @($excel.Workbooks)) {
                $full = [string]$wb.FullName
                if ($full -and -not $full.StartsWith($global:OoxpOutDir, [StringComparison]::OrdinalIgnoreCase)) {
                    $foreign += $full
                }
            }
            if ($foreign.Count) {
                Write-Log ("Excel 里还开着本轮之外的工作簿，不关它：" + ($foreign -join '; '))
                $Quit = $false
            }
            else {
                Close-AllWorkbooks $excel
            }
        }

        $word = Get-Variable -Name 'OoxpWord' -Scope Global -ValueOnly -ErrorAction SilentlyContinue
        if ($word) {
            foreach ($d in @($word.Documents)) {
                $full = [string]$d.FullName
                if ($full -and -not $full.StartsWith($global:OoxpOutDir, [StringComparison]::OrdinalIgnoreCase)) {
                    Write-Log "Word 里还开着本轮之外的文档，不关它：$full"
                    continue
                }
                Close-WordDoc $d
            }
        }
    }

    foreach ($name in 'OoxpExcel', 'OoxpWord') {
        $app = Get-Variable -Name $name -Scope Global -ValueOnly -ErrorAction SilentlyContinue
        if (-not $app) { continue }
        if ($Quit) {
            # DisplayAlerts 早已关掉，Quit 不会因为「要不要保存」卡住。
            try { $app.DisplayAlerts = $false } catch {}
            try { $app.Quit() } catch { Write-Log "$name Quit 失败：$($_.Exception.Message)" }
        }
        Release-Com $app
        Set-Variable -Name $name -Scope Global -Value $null
    }
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
    [GC]::Collect()

    if ($Quit) { Stop-LingeringOffice }
}

# Quit 之后的兜底。
#
# 有些实例（多半是上一轮异常留下、被我们复用的那个）根本不理 Quit：窗口关了，
# 进程照样占着两百多兆不走，下一轮还得靠它开文档。文档此时已经全部关掉，
# 也确认过没有本轮之外的工作簿，硬杀不会丢东西。只杀这一轮绑过的那个 PID。
function Stop-LingeringOffice {
    foreach ($pair in @(@('OoxpExcelPid', 'Excel'), @('OoxpWordPid', 'Word'))) {
        # 变量不能叫 $pid，那是 PowerShell 的只读自动变量。
        $procId = Get-Variable -Name $pair[0] -Scope Global -ValueOnly -ErrorAction SilentlyContinue
        if (-not $procId) { continue }
        $until = (Get-Date).AddSeconds(8)
        do {
            $p = Get-Process -Id $procId -ErrorAction SilentlyContinue
            if (-not $p) { break }
            Start-Sleep -Milliseconds 500
        } while ((Get-Date) -lt $until)

        if ($p) {
            $p.Refresh()
            if ($p.MainWindowHandle -ne [IntPtr]::Zero) {
                Write-Log "$($pair[1]) 进程 $procId 关不掉，但它还有窗口，留着不动。"
            }
            else {
                Write-Log "$($pair[1]) 进程 $procId 不理 Quit，窗口已经没了，直接结束它。"
                try { Stop-Process -Id $procId -Force -ErrorAction Stop }
                catch { Write-Log "结束 $procId 失败：$($_.Exception.Message)" }
            }
        }
        Set-Variable -Name $pair[0] -Scope Global -Value $null
    }
}

# 没有窗口的 Office 进程是上一次跑剩下的，留着只会让绑定白等 25 秒。
function Get-ZombieOfficeProcess {
    Get-Process EXCEL, WINWORD, POWERPNT -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -like '*Microsoft Office*' -and $_.MainWindowHandle -eq [IntPtr]::Zero }
}

# 段落可见文本：Word 的 Range 末尾带段落标记，比对前要去掉。
function Get-ParaText($para) {
    $r = $para.Range
    $r.MoveEnd(1, -1) | Out-Null
    $text = [string]$r.Text
    Release-Com $r
    return $text
}
