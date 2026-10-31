# xlsx 断开外部链接。Excel 16.0 打开后少一条链接，剩下的链接仍然指向原来的文件。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'xlsx-break-source.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 99 }) }) }
} 'xlsx-break-source' | Out-Null

$hostBook = Get-OutPath 'xlsx-break-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostBook.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-break-host' | Out-Null

$linked = Get-OutPath 'xlsx-break-linked.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostBook.Replace('\', '/')
    output = $linked.Replace('\', '/')
    ops = @(
        @{ op = 'insertExternalLink'; id = 'xlsx:workbook'; path = 'C:\missing\gone.xlsx' }
        @{ op = 'insertExternalLink'; id = 'xlsx:workbook'; path = $src.Replace('\', '/') }
        @{ op = 'setCell'; id = 'cell:Sheet1!A1'; formula = '=[1]Sheet1!A1' }
        @{ op = 'setCell'; id = 'cell:Sheet1!C1'; formula = '=[2]Sheet1!A1' }
        @{ op = 'insertDefinedName'; id = 'xlsx:workbook'; name = 'ExtMix'; formula = 'SUM([1]Sheet1!A1,[2]Sheet1!A1)' }
    )
} 'xlsx-break-link-apply' | Out-Null

$out = Get-OutPath 'xlsx-break-out.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $linked.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(@{ op = 'deleteExternalLink'; id = 'extlink:1' })
} 'xlsx-break-delete' | Out-Null

$excel = Ensure-MsExcel $out
$book = Open-ExcelBook $excel $out
$cap = [string]$book.Windows.Item(1).Caption
$links = @($book.LinkSources(1))
$ws = $book.Worksheets.Item('Sheet1')
$a1 = $ws.Range('A1').Value2
$c1 = [string]$ws.Range('C1').Formula
$name = [string]$book.Names.Item('ExtMix').RefersTo
$one = ($links.Count -eq 1) -and ([string]$links[0] -like '*xlsx-break-source.xlsx')
Add-Check 'xlsx-Bk1' (
    ($cap -notmatch 'Repair') -and $one -and ($a1 -eq 0) -and ($c1 -like '*xlsx-break-source.xlsx*') -and ($name -match '#REF!')
) "caption=$cap links=$($links -join '|') a1=$a1 c1=$c1 name=$name"
$book.Close($false)
Release-Com $ws
Release-Com $book
