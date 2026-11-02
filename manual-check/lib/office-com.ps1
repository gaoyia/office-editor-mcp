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
    if ($null -eq $obj) { return }
    if (-not [System.Runtime.InteropServices.Marshal]::IsComObject($obj)) { return }
    [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($obj)
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
                    # 窗口对象只是引路用的。摸到 Application 之后就得松手：
                    # 留着它，PowerPoint 会一直当客户端还连着，收尾时 Quit 被无视。
                    $app = $null
                    try { $app = $om.Application } catch {}
                    if ($app) { Release-Com $om } else { $app = $om }
                    if ($app.Path -notlike '*Microsoft Office*') {
                        $last = "wrong path $($app.Path)"
                        Release-Com $app
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
# hr=0x80004005。Excel 用 /x 强制新实例，并绑 Start-Process 返回的 pid：
# 残留进程若没有 EXCEL7，DDE 复用会让 FromWindow 一直失败。
function Ensure-MsExcel([string]$seedBook) {
    if ($global:OoxpExcel) { return $global:OoxpExcel }
    $started = Start-Process -FilePath (Get-OfficeExe 'EXCEL.EXE') -ArgumentList @('/x', $seedBook) -PassThru
    $until = (Get-Date).AddSeconds(40)
    $p = $null
    do {
        $p = Get-Process -Id $started.Id -ErrorAction SilentlyContinue
        if ($p -and $p.MainWindowHandle -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 400
    } while ((Get-Date) -lt $until)
    if (-not $p) { throw '等 EXCEL 启动超时' }
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
    try { $app.AskToUpdateLinks = $false } catch {}
    try { $app.Options.ConfirmConversions = $false } catch {}
    try { $app.Options.UpdateLinksAtOpen = $false } catch {}
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
    try { $word.AskToUpdateLinks = $false } catch {}
    try { $word.Options.UpdateLinksAtOpen = $false } catch {}
    # ConfirmConversions=$false，打开时不要再问转换或更新域。
    return $word.Documents.Open($path, $false)
}

function Ensure-MsPpt([string]$seedDeck) {
    if ($global:OoxpPpt) { return $global:OoxpPpt }
    Start-Process -FilePath (Get-OfficeExe 'POWERPNT.EXE') -ArgumentList ('"' + $seedDeck + '"')
    $p = Wait-OfficeProcess 'POWERPNT' 40
    # Office 16 的 Native OM 在 mdiClass 上，paneClassDC 这个类本机没有。
    $app = Bind-OfficeApp $p 'PPTFrameClass' 'mdiClass'
    $app.DisplayAlerts = 1
    Write-Log ("PowerPoint Name=$($app.Name) Ver=$($app.Version) Path=$($app.Path) Pid=$($p.Id)")
    $global:OoxpPpt = $app
    $global:OoxpPptPid = $p.Id
    return $app
}

# 下面几个都遵一条规矩：`Presentations`、`Slides`、`Shapes` 这些集合本身也是
# COM 包装对象，`$pres.Slides.Item(1)` 这种链式写法会把集合漏在原地。漏够了
# PowerPoint 就认为客户端还连着，收尾时 Quit 被无视，留下一个空窗口进程。
function Open-PptPres($ppt, [string]$path) {
    $collection = $ppt.Presentations
    try {
        foreach ($pres in @($collection)) {
            if ($pres.FullName -and ($pres.FullName -ieq $path)) { return $pres }
            Release-Com $pres
        }
        return $collection.Open($path, $false, $false, $true)
    }
    finally { Release-Com $collection }
}

function Get-PptSlide($pres, [int]$index) {
    $slides = $pres.Slides
    try { return $slides.Item($index) }
    finally { Release-Com $slides }
}

# $key 传索引或形状名都行，Item() 两种都收。
function Get-PptShape($slide, $key) {
    $shapes = $slide.Shapes
    try { return $shapes.Item($key) }
    finally { Release-Com $shapes }
}

function Close-PptPres($pres) {
    if ($null -ne $pres) {
        try { $pres.Close() } catch {}
        Release-Com $pres
    }
}

function Close-PptPresByPath($ppt, [string]$path) {
    $collection = $ppt.Presentations
    try {
        foreach ($pres in @($collection)) {
            if ($pres.FullName -and ($pres.FullName -ieq $path)) {
                Write-Log "先关掉 PowerPoint 里开着的 $path"
                Close-PptPres $pres
                return
            }
            Release-Com $pres
        }
    }
    finally { Release-Com $collection }
}

function Get-PptCaption($pres) {
    $windows = $null
    $win = $null
    try {
        $windows = $pres.Windows
        $win = $windows.Item(1)
        return [string]$win.Caption
    }
    catch {
        return ''
    }
    finally {
        Release-Com $win
        Release-Com $windows
    }
}

# 一行链式点出去，中间的 TextFrame 与 TextRange 都是 COM 包装对象，
# 没人松手，PowerPoint 就当客户端还连着，收尾时 Quit 会被无视。
function Get-ShapeText($shape) {
    if ([int]$shape.HasTextFrame -eq 0) { return $null }
    $frame = $null
    $range = $null
    try {
        $frame = $shape.TextFrame
        $range = $frame.TextRange
        return [string]$range.Text
    }
    finally {
        Release-Com $range
        Release-Com $frame
    }
}

function Close-AllWorkbooks($excel) {
    $collection = $excel.Workbooks
    try {
        foreach ($wb in @($collection)) {
            try { $wb.Close($false) } catch {}
            Release-Com $wb
        }
    }
    finally { Release-Com $collection }
}

function Close-WordDoc($doc) {
    if ($null -ne $doc) {
        try { $doc.Close($false) } catch {}
        Release-Com $doc
    }
}

# 打开时 Word 很宽容，有些文件要另存才会让它崩溃（例如没包 AlternateContent 的 chartex）。
# 判据：另存后进程还在、文件写出来、Application 日志没有新的 WINWORD.EXE 崩溃（事件 1000）。
# 崩了就清掉共用实例，后面的脚本会重新起一个 Word。
function Save-WordDocChecked($doc, [string]$savedPath) {
    $since = Get-Date
    $procId = $global:OoxpWordPid
    if (Test-Path $savedPath) { Remove-Item -Force $savedPath }
    $err = ''
    try { $doc.SaveAs2($savedPath) } catch { $err = $_.Exception.Message }
    Start-Sleep -Seconds 2
    $alive = [bool]($procId -and (Get-Process -Id $procId -ErrorAction SilentlyContinue))
    $crashes = @()
    try {
        $crashes = @(Get-WinEvent -FilterHashtable @{ LogName = 'Application'; Id = 1000; StartTime = $since } -ErrorAction Stop |
            Where-Object { $_.Message -match 'WINWORD\.EXE' })
    } catch {}
    if (-not $alive) {
        $global:OoxpWord = $null
        $global:OoxpWordPid = $null
    }
    $saved = Test-Path $savedPath
    [pscustomobject]@{
        Ok = ($alive -and -not $err -and $saved -and $crashes.Count -eq 0)
        Detail = "alive=$alive saved=$saved crash=$($crashes.Count) err=$err"
    }
}

# apply 的目标文件若正被 Word 开着，ooxp 写不进去，所以写之前先关掉。
function Close-WordDocByPath($word, [string]$path) {
    $collection = $word.Documents
    try {
        foreach ($d in @($collection)) {
            if ($d.FullName -and ($d.FullName -ieq $path)) {
                Write-Log "先关掉 Word 里开着的 $path"
                Close-WordDoc $d
                return
            }
            Release-Com $d
        }
    }
    finally { Release-Com $collection }
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
    # 三家各自决定能不能 Quit。一家里有用户自己的文档，只影响它自己，
    # 不牵连另外两家——否则会为了保住 Word 的文档而留下两个僵尸进程。
    $quitted = @()

    if ($Quit) {
        # 顺序很讲究。用例里留下的工作簿、工作表、段落都是 COM 包装对象，
        # 只要还有一个活着，Excel 就当作「客户端还连着」，Quit 会被直接无视。
        # 所以先强制回收这些零散引用，再关文档，最后才 Quit。
        [GC]::Collect()
        [GC]::WaitForPendingFinalizers()
        [GC]::Collect()
    }

    foreach ($pair in @(@('OoxpExcel', 'Excel'), @('OoxpWord', 'Word'), @('OoxpPpt', 'PowerPoint'))) {
        $app = Get-Variable -Name $pair[0] -Scope Global -ValueOnly -ErrorAction SilentlyContinue
        if (-not $app) { continue }

        $mayQuit = $false
        if ($Quit) { $mayQuit = Close-OwnDocs $app $pair[1] }

        if ($mayQuit) {
            # 文档此时已经全关，Quit 不会再问「要不要保存」。
            try { $app.DisplayAlerts = $false } catch {}
            try {
                $app.Quit()
                $quitted += $pair[0]
            }
            catch { Write-Log "$($pair[1]) Quit 失败：$($_.Exception.Message)" }
        }
        Release-Com $app
        Set-Variable -Name $pair[0] -Scope Global -Value $null
    }
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
    [GC]::Collect()

    if ($quitted.Count) { Stop-LingeringOffice $quitted }
}

# 关掉本轮自己开的文档，并回答「这一家能不能 Quit」。
#
# 三家一个规矩：只要发现 out 目录之外的文档，就整家放弃 Quit。原先只有 Excel 这么做，
# Word 与 PowerPoint 是跳过那一份、照旧 Quit，而 DisplayAlerts 早已关掉，
# 用户自己开着的文档会被连带关掉，还不提示。
#
# 另外，枚举出来的每份文档和文档集合本身都是 COM 包装对象。漏掉一个，
# PowerPoint 就认为客户端还连着，Quit 被无视，留下一个没有文档的空窗口进程。
function Close-OwnDocs($app, [string]$kind) {
    $collection = $null
    try {
        $collection = switch ($kind) {
            'Excel' { $app.Workbooks }
            'Word' { $app.Documents }
            'PowerPoint' { $app.Presentations }
        }
    }
    catch {
        Write-Log "$kind 的文档集合取不到：$($_.Exception.Message)"
        return $false
    }
    if (-not $collection) { return $true }

    $docs = @($collection)
    $foreign = @()
    foreach ($d in $docs) {
        $full = [string]$d.FullName
        if ($full -and -not $full.StartsWith($global:OoxpOutDir, [StringComparison]::OrdinalIgnoreCase)) {
            $foreign += $full
        }
    }

    if ($foreign.Count) {
        Write-Log ("$kind 里还开着本轮之外的文档，这一家整个不关：" + ($foreign -join '; '))
    }
    else {
        foreach ($d in $docs) {
            # PowerPoint 的 Presentation.Close 不收参数，另外两家用 Close($false) 丢弃改动。
            try {
                if ($kind -eq 'PowerPoint') { $d.Close() } else { $d.Close($false) }
            }
            catch {}
        }
    }

    foreach ($d in $docs) { Release-Com $d }
    Release-Com $collection
    return ($foreign.Count -eq 0)
}

# Quit 之后的兜底。
#
# 有些实例根本不理 Quit：文档关了，进程照样占着两百多兆不走，下一轮还得靠它开文档。
# PowerPoint 尤其如此，它会留下一个没有文档的空窗口赖着。
#
# 只处理 `$quitted` 里那几家，也就是「确认过没有本轮之外的文档、并且真的调过 Quit」的。
# 有这两个前提，剩下的窗口只可能是空的应用窗口，硬杀不会丢东西。
# 早先这里遇到「还有窗口」就放手，于是 `-CloseOffice` 明明写着不留进程却留下了一个。
function Stop-LingeringOffice([string[]]$quitted) {
    $pidNames = @{ 'OoxpExcel' = 'Excel'; 'OoxpWord' = 'Word'; 'OoxpPpt' = 'PowerPoint' }
    foreach ($appVar in $quitted) {
        # 变量不能叫 $pid，那是 PowerShell 的只读自动变量。
        $procId = Get-Variable -Name "${appVar}Pid" -Scope Global -ValueOnly -ErrorAction SilentlyContinue
        if (-not $procId) { continue }
        $kind = $pidNames[$appVar]

        # Quit 是异步的，先给它 15 秒自己走。
        $until = (Get-Date).AddSeconds(15)
        do {
            $p = Get-Process -Id $procId -ErrorAction SilentlyContinue
            if (-not $p) { break }
            Start-Sleep -Milliseconds 500
        } while ((Get-Date) -lt $until)

        if ($p) {
            Write-Log "$kind 进程 $procId 不理 Quit，直接结束它。"
            try { Stop-Process -Id $procId -Force -ErrorAction Stop }
            catch { Write-Log "结束 $procId 失败：$($_.Exception.Message)" }
        }
        Set-Variable -Name "${appVar}Pid" -Scope Global -Value $null
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

# pwsh 取不了 Office DocumentProperties.Item（空引用）。
# Explorer 属性页走同一套 OPC core/app，按列名取标题和公司。
function Get-ShellFileProp([string]$path, [string[]]$names) {
    $sh = New-Object -ComObject Shell.Application
    try {
        $dir = [IO.Path]::GetDirectoryName($path)
        $leaf = [IO.Path]::GetFileName($path)
        $folder = $sh.NameSpace($dir)
        try {
            $item = $folder.ParseName($leaf)
            if ($null -eq $item) { return $null }
            try {
                for ($i = 0; $i -lt 80; $i++) {
                    $n = [string]$folder.GetDetailsOf($null, $i)
                    if ($names -contains $n) {
                        return [string]$folder.GetDetailsOf($item, $i)
                    }
                }
                return $null
            }
            finally { Release-Com $item }
        }
        finally { Release-Com $folder }
    }
    finally { Release-Com $sh }
}

function Get-ZipPartText([string]$path, [string]$part) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($path)
    try {
        $e = $zip.GetEntry($part.TrimStart('/').Replace('\', '/'))
        if ($null -eq $e) { return $null }
        $st = $e.Open()
        try {
            $sr = New-Object IO.StreamReader($st, [Text.Encoding]::UTF8)
            try { return $sr.ReadToEnd() }
            finally { $sr.Dispose() }
        }
        finally { $st.Dispose() }
    }
    finally { $zip.Dispose() }
}

function Get-CustomPropFromZip([string]$path, [string]$name) {
    $xml = Get-ZipPartText $path 'docProps/custom.xml'
    if ([string]::IsNullOrEmpty($xml)) { return $null }
    $pat = 'name="' + [regex]::Escape($name) + '"[^>]*>\s*<[^>]+>([^<]*)'
    $m = [regex]::Match($xml, $pat)
    if ($m.Success) { return $m.Groups[1].Value }
    return $null
}
