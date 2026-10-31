# 旧版 XOR 口令改写成 SHA-512 后，Excel 16.0 用同一口令解开。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'xlsx-legacy-pw-src.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-legacy-pw-create' | Out-Null

$plain = Get-OutPath 'xlsx-legacy-pw-plain.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $plain.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'sheet:Sheet1'; enforced = $true })
} 'xlsx-legacy-pw-plain' | Out-Null

$xor = Get-OutPath 'xlsx-legacy-pw-xor.xlsx'
$py = Join-Path $global:OoxpOutDir 'inject-xor.py'
@'
import sys, zipfile
from io import BytesIO
src, dest = sys.argv[1], sys.argv[2]
buf = BytesIO()
with zipfile.ZipFile(src) as zin, zipfile.ZipFile(buf, "w") as zout:
    for info in zin.infolist():
        data = zin.read(info.filename)
        if info.filename == "xl/worksheets/sheet1.xml":
            xml = data.decode("utf-8").replace("<sheetProtection ", '<sheetProtection password="DAA7" ')
            data = xml.encode("utf-8")
        zout.writestr(info, data)
open(dest, "wb").write(buf.getvalue())
'@ | Set-Content -Path $py -Encoding utf8
& python -X utf8 $py $plain $xor
if ($LASTEXITCODE -ne 0) { throw "inject failed $LASTEXITCODE" }

$out = Get-OutPath 'xlsx-legacy-pw.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $xor.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'sheet:Sheet1'; enforced = $true; password = 'secret' })
} 'xlsx-legacy-pw-rewrite' | Out-Null

$excel = Ensure-MsExcel $out
$book = Open-ExcelBook $excel $out
$cap = [string]$book.Windows.Item(1).Caption
$ws = $book.Worksheets.Item('Sheet1')
$locked = [bool]$ws.ProtectContents
$ws.Unprotect('secret')
$open = -not [bool]$ws.ProtectContents
Add-Check 'xlsx-Lp1' (($cap -notmatch 'Repair') -and $locked -and $open) "caption=$cap locked=$locked open=$open"
$book.Close($false)
Release-Com $ws
Release-Com $book
