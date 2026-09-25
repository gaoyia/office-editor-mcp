$ErrorActionPreference = 'Stop'
$ooxp = 'C:\Users\GaoYi-AMD\Desktop\test\target\debug\ooxp.exe'
$root = 'C:\Users\GaoYi-AMD\Desktop\test\manual-check\office'
$winwordExe = 'C:\Program Files\Microsoft Office\Root\Office16\WINWORD.EXE'
$excelExe = 'C:\Program Files\Microsoft Office\Root\Office16\EXCEL.EXE'
$logPath = Join-Path $root 'office-com-log.txt'
$utf8 = New-Object System.Text.UTF8Encoding $false
$S = Get-Content -LiteralPath (Join-Path $root 'requests\_strings.json') -Encoding UTF8 -Raw | ConvertFrom-Json

function Write-Log([string]$msg) {
    $line = "$(Get-Date -Format 'HH:mm:ss') $msg"
    Add-Content -Path $logPath -Value $line -Encoding UTF8
    Write-Host $line
}

function Invoke-Ooxp([string]$jsonPath) {
    $out = cmd /c "`"$ooxp`" --input `"$jsonPath`""
    if ($null -eq $out) { $out = '' }
    $out = [string]$out
    Write-Log ("ooxp " + [IO.Path]::GetFileName($jsonPath) + " " + $out.Trim())
    return $out.Trim()
}

function Close-AllWorkbooks($excel) {
    $books = @($excel.Workbooks)
    foreach ($wb in $books) {
        try { $wb.Close($false) } catch {}
        Release-Com $wb
    }
}

function Write-JsonFile([string]$path, $obj) {
    [System.IO.File]::WriteAllText($path, ($obj | ConvertTo-Json -Depth 12 -Compress), $utf8)
}

function Release-Com($obj) {
    if ($null -ne $obj) {
        [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($obj)
    }
}

Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public static class OfficeOmChk {
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

function Wait-OfficeProcess([string]$name, [int]$seconds) {
    $until = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $until) {
        $p = Get-Process $name -ErrorAction SilentlyContinue |
            Where-Object { $_.Path -like '*Microsoft Office*' -and $_.MainWindowHandle -ne [IntPtr]::Zero } |
            Select-Object -First 1
        if ($p) { return $p }
        Start-Sleep -Milliseconds 400
    }
    throw "wait $name timed out"
}

function Bind-OfficeApp($process, [string]$topClass, [string]$childClass) {
    $until = (Get-Date).AddSeconds(25)
    $last = 'no hwnd'
    while ((Get-Date) -lt $until) {
        $tops = [OfficeOmChk]::FindTop([uint32]$process.Id, $topClass)
        foreach ($t in $tops) {
            $order = New-Object System.Collections.ArrayList
            foreach ($k in [OfficeOmChk]::Children($t)) {
                if ($k.Value -eq $childClass) { [void]$order.Add($k.Key) }
            }
            [void]$order.Add($t)
            foreach ($h in $order) {
                try {
                    $om = [OfficeOmChk]::FromWindow($h)
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
    throw "bind $($process.Name) failed: $last"
}

function Ensure-MsExcel {
    $p = Get-Process EXCEL -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -like '*Microsoft Office*' } | Select-Object -First 1
    if (-not $p) {
        Start-Process -FilePath $excelExe -ArgumentList ('"' + (Join-Path $root 'xlsx-create.xlsx') + '"')
        $p = Wait-OfficeProcess 'EXCEL' 40
    }
    $app = Bind-OfficeApp $p 'XLMAIN' 'EXCEL7'
    $app.DisplayAlerts = $false
    $app.AskToUpdateLinks = $false
    Write-Log ("Excel Name=$($app.Name) Ver=$($app.Version) Path=$($app.Path)")
    return $app
}

function Ensure-MsWord {
    $p = Get-Process WINWORD -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -like '*Microsoft Office*' } | Select-Object -First 1
    if (-not $p) {
        Start-Process -FilePath $winwordExe -ArgumentList ('"' + (Join-Path $root 'docx-create.docx') + '"')
        $p = Wait-OfficeProcess 'WINWORD' 40
    }
    $app = Bind-OfficeApp $p 'OpusApp' '_WwG'
    $app.DisplayAlerts = 0
    try { $app.Options.ConfirmConversions = $false } catch {}
    try { $app.Options.BackgroundSave = $false } catch {}
    Write-Log ("Word Name=$($app.Name) Ver=$($app.Version) Path=$($app.Path)")
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

if (Test-Path $logPath) { Remove-Item $logPath -Force }

$results = New-Object System.Collections.ArrayList
function Add-Result([string]$id, [string]$ok, [string]$detail) {
    [void]$results.Add([pscustomobject]@{ id = $id; ok = $ok; detail = $detail })
    Write-Log ("RESULT $id $ok $detail")
}

$excel = $null
$word = $null
try {
    $excel = Ensure-MsExcel
    $word = Ensure-MsWord

    # ---- xlsx create ----
    $xlsxCreate = Join-Path $root 'xlsx-create.xlsx'
    $wb = Open-ExcelBook $excel $xlsxCreate
    $excel.CalculateFullRebuild()
    $s1 = $wb.Worksheets.Item('Sheet1')
    $a1 = $s1.Range('A1').Value2
    $a2 = [string]$s1.Range('A2').Value2
    $a3 = [string]$s1.Range('A3').Text
    $b1f = [string]$s1.Range('B1').Formula
    $b1v = $s1.Range('B1').Value2
    $title = [string]$excel.ActiveWindow.Caption
    Add-Result 'xlsx-C1' ($(if ($title -notmatch 'Repair') { 'PASS' } else { 'FAIL' })) $title
    Add-Result 'xlsx-C3' ($(if ($s1.Name -eq 'Sheet1') { 'PASS' } else { 'FAIL' })) $s1.Name
    Add-Result 'xlsx-C4' ($(if ($a1 -eq 1) { 'PASS' } else { 'FAIL' })) "A1=$a1"
    Add-Result 'xlsx-C5' ($(if ($a2 -eq 'hello') { 'PASS' } else { 'FAIL' })) "A2=$a2"
    Add-Result 'xlsx-C6' ($(if ($a3 -match 'TRUE') { 'PASS' } else { 'FAIL' })) "A3.text=$a3"
    Add-Result 'xlsx-C7' ($(if (($b1f -eq '=SUM(A1)') -and ($b1v -eq 1)) { 'PASS' } else { 'FAIL' })) "B1 formula=$b1f value=$b1v"
    $xlsxResave = Join-Path $root 'xlsx-create-excel-resaved.xlsx'
    if (Test-Path $xlsxResave) { Remove-Item $xlsxResave -Force }
    $wb.SaveCopyAs($xlsxResave)
    Add-Result 'xlsx-O1-excel-resave' 'PASS' $xlsxResave
    $wb.Close($false)
    Release-Com $s1
    Release-Com $wb

    # ---- xlsx inspect ----
    $insp = @{
        schemaVersion = 1; action = 'inspect'
        path = $xlsxCreate.Replace('\','/'); format = 'xlsx'
        scope = @{ sheet = 'Sheet1'; range = 'A1:D20' }
    }
    Write-JsonFile (Join-Path $root 'requests\xlsx-inspect.json') $insp
    $raw = Invoke-Ooxp (Join-Path $root 'requests\xlsx-inspect.json')
    $tree = $raw | ConvertFrom-Json
    $ids = @($tree.sheets | ForEach-Object { $_.id })
    $cells = @($tree.sheets[0].cells)
    $refs = @($cells | ForEach-Object { $_.ref })
    $cA1 = $cells | Where-Object { $_.ref -eq 'A1' } | Select-Object -First 1
    $cA2 = $cells | Where-Object { $_.ref -eq 'A2' } | Select-Object -First 1
    $cB1 = $cells | Where-Object { $_.ref -eq 'B1' } | Select-Object -First 1
    Add-Result 'xlsx-I1' ($(if ($ids -contains 'sheet:Sheet1') { 'PASS' } else { 'FAIL' })) ($ids -join ',')
    Add-Result 'xlsx-I2' ($(if ($refs -notcontains 'E1') { 'PASS' } else { 'FAIL' })) ($refs -join ',')
    Add-Result 'xlsx-I3' ($(if ($cA1.type -eq 'number' -and $cA1.value -eq 1) { 'PASS' } else { 'FAIL' })) "$($cA1.type)/$($cA1.value)"
    Add-Result 'xlsx-I4' ($(if ($cA2.type -eq 'text' -and $cA2.value -eq 'hello') { 'PASS' } else { 'FAIL' })) "$($cA2.type)/$($cA2.value)"
    Add-Result 'xlsx-I5' ($(if ($cB1.formula -eq '=SUM(A1)') { 'PASS' } else { 'FAIL' })) $cB1.formula
    $insp2 = @{ schemaVersion = 1; action = 'inspect'; path = $xlsxCreate.Replace('\','/'); format = 'xlsx' }
    Write-JsonFile (Join-Path $root 'requests\xlsx-inspect-sheets.json') $insp2
    $raw2 = Invoke-Ooxp (Join-Path $root 'requests\xlsx-inspect-sheets.json')
    $tree2 = $raw2 | ConvertFrom-Json
    $hasCells = $false
    foreach ($sh in @($tree2.sheets)) { if ($sh.PSObject.Properties.Name -contains 'cells') { $hasCells = $true } }
    Add-Result 'xlsx-I6' ($(if (-not $hasCells) { 'PASS' } else { 'FAIL' })) "hasCells=$hasCells"

    # ---- xlsx apply source in Excel ----
    $srcXlsx = Join-Path $root 'xlsx-excel-source.xlsx'
    $wb = $excel.Workbooks.Add()
    while ($wb.Worksheets.Count -lt 2) { [void]$wb.Worksheets.Add() }
    $s1 = $wb.Worksheets.Item(1)
    $s2 = $wb.Worksheets.Item(2)
    $s1.Name = '_tmpA'
    $s2.Name = '_tmpB'
    $s1.Name = 'Sheet1'
    $s2.Name = 'Sheet2'
    $s1.Range('A1').Value2 = 1
    $s1.Range('A2').Value2 = 2
    $s1.Range('A3').Value2 = 3
    $s1.Range('B1').Formula = '=SUM(A1)'
    $s1.Range('C1').Value2 = 123.45
    $s1.Range('C1').NumberFormat = '0.00'
    $s1.Range('D1').Formula = '=A1*2'
    $s1.Range('E1').Value2 = 'keep'
    $s1.Range('E1').Font.Bold = $true
    $s1.Range('E1').Font.Color = 255
    $s1.Range('E1').Interior.Color = 65535
    $s1.Columns.Item(5).ColumnWidth = 20
    $s2.Range('A1').Value2 = 'untouched'
    $s2.Range('B1').Value2 = 99
    $s2.Range('A1').Font.Bold = $true
    if (Test-Path $srcXlsx) { Remove-Item $srcXlsx -Force }
    $wb.SaveAs($srcXlsx, 51)
    $wb.Close($false)
    Release-Com $s1
    Release-Com $s2
    Release-Com $wb
    Write-Log "saved excel source $srcXlsx"

    Close-AllWorkbooks $excel
    $appliedXlsx = Join-Path $root 'xlsx-applied.xlsx'
    if (Test-Path $appliedXlsx) {
        try { Remove-Item $appliedXlsx -Force } catch {
            $appliedXlsx = Join-Path $root ("xlsx-applied-" + (Get-Date -Format 'HHmmss') + ".xlsx")
            Write-Log "applied path fallback $appliedXlsx"
        }
    }
    $applyX = @{
        schemaVersion = 1; action = 'apply'
        path = $srcXlsx.Replace('\','/')
        output = $appliedXlsx.Replace('\','/')
        ops = @(
            @{ op = 'setCell'; id = 'cell:Sheet1!A1'; value = 42 }
            @{ op = 'setCell'; id = 'cell:Sheet1!B1'; formula = '=SUM(A1:A10)' }
            @{ op = 'setCell'; id = 'cell:Sheet1!C1'; value = $null }
        )
    }
    Write-JsonFile (Join-Path $root 'requests\xlsx-apply.json') $applyX
    Invoke-Ooxp (Join-Path $root 'requests\xlsx-apply.json') | Out-Null

    $wb = Open-ExcelBook $excel $appliedXlsx
    $excel.CalculateFullRebuild()
    $s1 = $wb.Worksheets.Item('Sheet1')
    $s2 = $wb.Worksheets.Item('Sheet2')
    $cap = [string]$excel.ActiveWindow.Caption
    Add-Result 'xlsx-A1' ($(if ($cap -notmatch 'Repair') { 'PASS' } else { 'FAIL' })) $cap
    Add-Result 'xlsx-A2' ($(if ($s1.Range('A1').Value2 -eq 42) { 'PASS' } else { 'FAIL' })) "A1=$($s1.Range('A1').Value2)"
    $bf = [string]$s1.Range('B1').Formula
    $bv = $s1.Range('B1').Value2
    Add-Result 'xlsx-A3' ($(if ($bf -eq '=SUM(A1:A10)' -and $bv -eq 47) { 'PASS' } else { 'FAIL' })) "B1 $bf value=$bv"
    $cEmpty = ($null -eq $s1.Range('C1').Value2) -or ($s1.Range('C1').Value2 -eq '')
    $cFmt = [string]$s1.Range('C1').NumberFormat
    Add-Result 'xlsx-A4' ($(if ($cEmpty -and $cFmt -eq '0.00') { 'PASS' } else { 'FAIL' })) "empty=$cEmpty fmt=$cFmt"
    $s2ok = ($s2.Range('A1').Value2 -eq 'untouched') -and ($s2.Range('B1').Value2 -eq 99) -and ($s2.Range('A1').Font.Bold -eq $true)
    Add-Result 'xlsx-A5' ($(if ($s2ok) { 'PASS' } else { 'FAIL' })) "A1=$($s2.Range('A1').Value2) B1=$($s2.Range('B1').Value2) bold=$($s2.Range('A1').Font.Bold)"
    $e1 = $s1.Range('E1')
    $styleOk = ($e1.Value2 -eq 'keep') -and ($e1.Font.Bold -eq $true) -and ($e1.Font.Color -eq 255) -and ([math]::Abs($e1.Interior.Color - 65535) -lt 1) -and ([math]::Abs($s1.Columns.Item(5).ColumnWidth - 20) -lt 0.2)
    Add-Result 'xlsx-A6' ($(if ($styleOk) { 'PASS' } else { 'FAIL' })) "E1=$($e1.Value2) bold=$($e1.Font.Bold) color=$($e1.Font.Color) fill=$($e1.Interior.Color) width=$($s1.Columns.Item(5).ColumnWidth)"
    Add-Result 'xlsx-A7' ($(if ($s1.Range('D1').Value2 -eq 84) { 'PASS' } else { 'FAIL' })) "D1=$($s1.Range('D1').Value2)"
    Add-Result 'xlsx-O2' ($(if ($s2ok -and $styleOk) { 'PASS' } else { 'FAIL' })) 'sheet2 and E1 style kept'

    # keep applied workbook open for the user
    Release-Com $e1
    Release-Com $s1
    Release-Com $s2

    # ---- docx create ----
    $docxCreate = Join-Path $root 'docx-create.docx'
    $doc = Open-WordDoc $word $docxCreate
    $p1 = $doc.Paragraphs.Item(1)
    $p2 = $doc.Paragraphs.Item(2)
    $r1 = $p1.Range; $r1.MoveEnd(1, -1) | Out-Null
    $r2 = $p2.Range; $r2.MoveEnd(1, -1) | Out-Null
    $wtitle = [string]$word.ActiveWindow.Caption
    $compat = $null
    try { $compat = $doc.CompatibilityMode } catch {}
    Add-Result 'docx-C1' ($(if ($wtitle -notmatch 'Repair') { 'PASS' } else { 'FAIL' })) "$wtitle compat=$compat"
    $styleOk = ($p1.Style.NameLocal -eq $S.heading1) -or ($p1.Style.NameLocal -eq 'Heading 1')
    Add-Result 'docx-C3' ($(if ($styleOk -and $r1.Text -eq $S.title) { 'PASS' } else { 'FAIL' })) "style=$($p1.Style.NameLocal) text=$($r1.Text)"
    Add-Result 'docx-C4' ($(if ($r2.Text -eq 'hello') { 'PASS' } else { 'FAIL' })) $r2.Text
    $docxResave = Join-Path $root 'docx-create-word-resaved.docx'
    if (Test-Path $docxResave) { Remove-Item $docxResave -Force }
    Write-Log "word SaveAs2 format 12 -> $docxResave"
    $doc.SaveAs2($docxResave, 12)
    Write-Log "word SaveAs2 done"
    Add-Result 'docx-O1-word-resave' 'PASS' "$docxResave compat=$compat"
    $doc.Close($false)
    Release-Com $r1; Release-Com $r2; Release-Com $p1; Release-Com $p2; Release-Com $doc

    # ---- docx inspect ----
    $din = @{
        schemaVersion = 1; action = 'inspect'
        path = $docxCreate.Replace('\','/'); format = 'docx'
        scope = @{ from = 1; maxParas = 200 }
    }
    Write-JsonFile (Join-Path $root 'requests\docx-inspect.json') $din
    $draw = Invoke-Ooxp (Join-Path $root 'requests\docx-inspect.json')
    $dtree = $draw | ConvertFrom-Json
    Add-Result 'docx-I1' ($(if ($dtree.id -eq 'docx:document' -and $dtree.paraCount -eq 2) { 'PASS' } else { 'FAIL' })) "id=$($dtree.id) count=$($dtree.paraCount)"
    $ol = @($dtree.outline)
    $olOk = ($ol.Count -ge 1) -and ($ol[0].level -eq 1) -and ($ol[0].text -eq $S.title)
    Add-Result 'docx-I2' ($(if ($olOk) { 'PASS' } else { 'FAIL' })) ($ol | ConvertTo-Json -Compress)
    $paras = @($dtree.paragraphs)
    $idOk = ($paras.Count -eq 2) -and $paras[0].stable -and $paras[1].stable -and ($paras[0].id -like 'para:*') -and ($paras[0].id -notlike 'para:index:*')
    Add-Result 'docx-I3' ($(if ($idOk) { 'PASS' } else { 'FAIL' })) "$($paras[0].id),$($paras[1].id)"
    Add-Result 'docx-I4' ($(if ($paras[1].text -eq 'hello') { 'PASS' } else { 'FAIL' })) $paras[1].text
    $din2 = @{ schemaVersion = 1; action = 'inspect'; path = $docxCreate.Replace('\','/'); format = 'docx' }
    Write-JsonFile (Join-Path $root 'requests\docx-inspect-outline.json') $din2
    $draw2 = Invoke-Ooxp (Join-Path $root 'requests\docx-inspect-outline.json')
    $dtree2 = $draw2 | ConvertFrom-Json
    $noParas = -not ($dtree2.PSObject.Properties.Name -contains 'paragraphs')
    $hasOl = $dtree2.PSObject.Properties.Name -contains 'outline'
    Add-Result 'docx-I5' ($(if ($noParas -and $hasOl -and $dtree2.paraCount -eq 2) { 'PASS' } else { 'FAIL' })) "noParas=$noParas outline=$hasOl count=$($dtree2.paraCount)"

    # ---- docx apply source ----
    $docxSrcOoxp = Join-Path $root 'docx-ooxp-source.docx'
    $srcCreate = @{
        schemaVersion = 1; action = 'create'
        path = $docxSrcOoxp.Replace('\','/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{ style = 'Heading1'; text = $S.chapter1 }
                @{ text = $S.para2src }
                @{ text = $S.para3keep }
            )
        }
    }
    Write-JsonFile (Join-Path $root 'requests\docx-source-create.json') $srcCreate
    Invoke-Ooxp (Join-Path $root 'requests\docx-source-create.json') | Out-Null
    $doc = Open-WordDoc $word $docxSrcOoxp
    $doc.Paragraphs.Item(2).Range.Font.Bold = $true
    $docxSrc = Join-Path $root 'docx-word-source.docx'
    if (Test-Path $docxSrc) { Remove-Item $docxSrc -Force }
    Write-Log "word source SaveAs2 format 12 -> $docxSrc"
    $doc.SaveAs2($docxSrc, 12)
    Write-Log "word source SaveAs2 done"
    $doc.Close($false)
    Release-Com $doc
    Write-Log "saved word source $docxSrc"

    $din3 = @{
        schemaVersion = 1; action = 'inspect'
        path = $docxSrc.Replace('\','/'); format = 'docx'
        scope = @{ from = 1; maxParas = 200 }
    }
    Write-JsonFile (Join-Path $root 'requests\docx-source-inspect.json') $din3
    $dsrc = (Invoke-Ooxp (Join-Path $root 'requests\docx-source-inspect.json')) | ConvertFrom-Json
    $id1 = $dsrc.paragraphs[0].id
    $id2 = $dsrc.paragraphs[1].id
    $id3 = $dsrc.paragraphs[2].id
    $appliedDocx = Join-Path $root 'docx-applied.docx'
    $applyD = @{
        schemaVersion = 1; action = 'apply'
        path = $docxSrc.Replace('\','/')
        output = $appliedDocx.Replace('\','/')
        ops = @(
            @{ op = 'setParaText'; id = $id2; text = $S.para2new }
        )
    }
    Write-JsonFile (Join-Path $root 'requests\docx-apply.json') $applyD
    Invoke-Ooxp (Join-Path $root 'requests\docx-apply.json') | Out-Null

    $doc = Open-WordDoc $word $appliedDocx
    $q1 = $doc.Paragraphs.Item(1); $t1 = $q1.Range; $t1.MoveEnd(1,-1) | Out-Null
    $q2 = $doc.Paragraphs.Item(2); $t2 = $q2.Range; $t2.MoveEnd(1,-1) | Out-Null
    $q3 = $doc.Paragraphs.Item(3); $t3 = $q3.Range; $t3.MoveEnd(1,-1) | Out-Null
    $atitle = [string]$word.ActiveWindow.Caption
    $h1ok = ($q1.Style.NameLocal -eq $S.heading1) -or ($q1.Style.NameLocal -eq 'Heading 1')
    Add-Result 'docx-A1' ($(if ($atitle -notmatch 'Repair') { 'PASS' } else { 'FAIL' })) $atitle
    Add-Result 'docx-A2' ($(if ($t2.Text -eq $S.para2new) { 'PASS' } else { 'FAIL' })) $t2.Text
    Add-Result 'docx-A3' ($(if ($t1.Text -eq $S.chapter1 -and $h1ok) { 'PASS' } else { 'FAIL' })) "text=$($t1.Text) style=$($q1.Style.NameLocal)"
    Add-Result 'docx-A4' ($(if ($q2.Range.Font.Bold -eq -1 -or $q2.Range.Font.Bold -eq $true) { 'PASS' } else { 'FAIL' })) "bold=$($q2.Range.Font.Bold)"
    Add-Result 'docx-A5' ($(if ($t3.Text -eq $S.para3keep) { 'PASS' } else { 'FAIL' })) $t3.Text
    Write-Log "docx source ids $id1 $id2 $id3"

    $results | Export-Csv -Path (Join-Path $root 'office-results.csv') -NoTypeInformation -Encoding UTF8
    Write-Log "wrote office-results.csv"
}
catch {
    Write-Log "FAILED $($_.Exception.Message)"
    throw
}
