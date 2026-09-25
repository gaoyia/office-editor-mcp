$ErrorActionPreference = 'Stop'
$ooxp = 'C:\Users\GaoYi-AMD\Desktop\test\target\debug\ooxp.exe'
$root = 'C:\Users\GaoYi-AMD\Desktop\test\manual-check\office'
$tmp = Join-Path $env:TEMP 'ooxp-office'
$winwordExe = 'C:\Program Files\Microsoft Office\Root\Office16\WINWORD.EXE'
$logPath = Join-Path $root 'office-com-log.txt'
$utf8 = New-Object System.Text.UTF8Encoding $false
$S = Get-Content -LiteralPath (Join-Path $root 'requests\_strings.json') -Encoding UTF8 -Raw | ConvertFrom-Json

New-Item -ItemType Directory -Force -Path $tmp | Out-Null

function Write-Log([string]$msg) {
    $line = "$(Get-Date -Format 'HH:mm:ss') $msg"
    Add-Content -Path $logPath -Value $line -Encoding UTF8
    Write-Host $line
}
function Invoke-Ooxp([string]$jsonPath) {
    $out = cmd /c "`"$ooxp`" --input `"$jsonPath`""
    if ($null -eq $out) { $out = '' }
    Write-Log ("ooxp " + [IO.Path]::GetFileName($jsonPath) + " " + ([string]$out).Trim())
    return ([string]$out).Trim()
}
function Write-JsonFile([string]$path, $obj) {
    [System.IO.File]::WriteAllText($path, ($obj | ConvertTo-Json -Depth 12 -Compress), $utf8)
}
function Release-Com($obj) {
    if ($null -ne $obj) { [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($obj) }
}

Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class OmDocx {
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

function Bind-Word($process) {
    $until = (Get-Date).AddSeconds(25)
    $last = 'no hwnd'
    while ((Get-Date) -lt $until) {
        foreach ($t in [OmDocx]::FindTop([uint32]$process.Id, 'OpusApp')) {
            foreach ($h in [OmDocx]::FindChild($t, '_WwG')) {
                try {
                    $om = [OmDocx]::FromWindow($h)
                    $app = $om.Application
                    if ($app.Path -like '*Microsoft Office*') { return $app }
                    $last = $app.Path
                } catch { $last = $_.Exception.Message }
            }
        }
        Start-Sleep -Milliseconds 400
    }
    throw "bind Word failed: $last"
}

$results = New-Object System.Collections.ArrayList
function Add-Result([string]$id, [string]$ok, [string]$detail) {
    [void]$results.Add([pscustomobject]@{ id = $id; ok = $ok; detail = $detail })
    Write-Log ("RESULT $id $ok $detail")
}

$docxCreate = Join-Path $root 'docx-create.docx'
$din = @{
    schemaVersion = 1; action = 'inspect'
    path = $docxCreate.Replace('\','/'); format = 'docx'
    scope = @{ from = 1; maxParas = 200 }
}
Write-JsonFile (Join-Path $root 'requests\docx-inspect.json') $din
$dtree = (Invoke-Ooxp (Join-Path $root 'requests\docx-inspect.json')) | ConvertFrom-Json
Add-Result 'docx-I1' ($(if ($dtree.id -eq 'docx:document' -and $dtree.paraCount -eq 2) { 'PASS' } else { 'FAIL' })) "id=$($dtree.id) count=$($dtree.paraCount)"
$ol = @($dtree.outline)
Add-Result 'docx-I2' ($(if ($ol.Count -ge 1 -and $ol[0].level -eq 1 -and $ol[0].text -eq $S.title) { 'PASS' } else { 'FAIL' })) ($ol | ConvertTo-Json -Compress)
$paras = @($dtree.paragraphs)
$idOk = ($paras.Count -eq 2) -and $paras[0].stable -and $paras[1].stable -and ($paras[0].id -like 'para:*') -and ($paras[0].id -notlike 'para:index:*')
Add-Result 'docx-I3' ($(if ($idOk) { 'PASS' } else { 'FAIL' })) "$($paras[0].id),$($paras[1].id)"
Add-Result 'docx-I4' ($(if ($paras[1].text -eq 'hello') { 'PASS' } else { 'FAIL' })) $paras[1].text
$din2 = @{ schemaVersion = 1; action = 'inspect'; path = $docxCreate.Replace('\','/'); format = 'docx' }
Write-JsonFile (Join-Path $root 'requests\docx-inspect-outline.json') $din2
$dtree2 = (Invoke-Ooxp (Join-Path $root 'requests\docx-inspect-outline.json')) | ConvertFrom-Json
$noParas = -not ($dtree2.PSObject.Properties.Name -contains 'paragraphs')
Add-Result 'docx-I5' ($(if ($noParas -and ($dtree2.PSObject.Properties.Name -contains 'outline') -and $dtree2.paraCount -eq 2) { 'PASS' } else { 'FAIL' })) "noParas=$noParas count=$($dtree2.paraCount)"

$srcPath = Join-Path $tmp 'docx-word-source.docx'
$srcCreate = @{
    schemaVersion = 1; action = 'create'
    path = $srcPath.Replace('\','/'); format = 'docx'
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

Start-Process -FilePath $winwordExe -ArgumentList ('"' + $srcPath + '"')
$until = (Get-Date).AddSeconds(40)
$proc = $null
while ((Get-Date) -lt $until) {
    $proc = Get-Process WINWORD -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -like '*Microsoft Office*' -and $_.MainWindowHandle -ne [IntPtr]::Zero } |
        Select-Object -First 1
    if ($proc) { break }
    Start-Sleep -Milliseconds 400
}
if (-not $proc) { throw 'Word did not start' }
$word = Bind-Word $proc
$word.DisplayAlerts = 0
Write-Log ("Word Name=$($word.Name) Ver=$($word.Version) Path=$($word.Path)")
$doc = $null
foreach ($d in @($word.Documents)) {
    if ($d.FullName -and ($d.FullName -ieq $srcPath)) { $doc = $d; break }
}
if (-not $doc) { $doc = $word.Documents.Open($srcPath) }
$doc.Paragraphs.Item(2).Range.Font.Bold = $true
Write-Log 'word Save() overwrite TEMP source'
$doc.Save()
Write-Log 'word Save() done'
$doc.Close($false)
Release-Com $doc

$copySrc = Join-Path $root 'docx-word-source.docx'
Copy-Item -Force $srcPath $copySrc

$din3 = @{
    schemaVersion = 1; action = 'inspect'
    path = $copySrc.Replace('\','/'); format = 'docx'
    scope = @{ from = 1; maxParas = 200 }
}
Write-JsonFile (Join-Path $root 'requests\docx-source-inspect.json') $din3
$dsrc = (Invoke-Ooxp (Join-Path $root 'requests\docx-source-inspect.json')) | ConvertFrom-Json
$id2 = $dsrc.paragraphs[1].id
$appliedDocx = Join-Path $root 'docx-applied.docx'
$applyD = @{
    schemaVersion = 1; action = 'apply'
    path = $copySrc.Replace('\','/')
    output = $appliedDocx.Replace('\','/')
    ops = @( @{ op = 'setParaText'; id = $id2; text = $S.para2new } )
}
Write-JsonFile (Join-Path $root 'requests\docx-apply.json') $applyD
Invoke-Ooxp (Join-Path $root 'requests\docx-apply.json') | Out-Null

$doc = $word.Documents.Open($appliedDocx)
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

$results | Export-Csv -Path (Join-Path $root 'office-docx-results.csv') -NoTypeInformation -Encoding UTF8
Write-Log 'wrote office-docx-results.csv'
# leave applied doc open
Start-Process -FilePath $winwordExe -ArgumentList ('"' + $docxCreate + '"')
$excelExe = 'C:\Program Files\Microsoft Office\Root\Office16\EXCEL.EXE'
Start-Process -FilePath $excelExe -ArgumentList ('"' + (Join-Path $root 'xlsx-create.xlsx') + '"')
Start-Process -FilePath $excelExe -ArgumentList ('"' + (Join-Path $root 'xlsx-applied.xlsx') + '"')
Write-Log 'opened create/applied files in Office16'
