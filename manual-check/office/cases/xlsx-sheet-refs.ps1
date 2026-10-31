# xlsx 改名、删表改写引用。Excel 16.0 打开后公式是新表名或 #REF!，没有 Repair。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'xlsx-sheet-refs-src.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(
            @{ name = 'Sheet1'; cells = @(
                @{ ref = 'A1'; formula = '=Sheet2!B1' }
                @{ ref = 'A2'; formula = '="Sheet2!B1"' }
            ) }
            @{ name = 'Sheet2'; cells = @(@{ ref = 'B1'; value = 7 }) }
        )
    }
} 'xlsx-sheet-refs-create' | Out-Null

$named = Get-OutPath 'xlsx-sheet-refs-named.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $named.Replace('\', '/')
    ops = @(@{
        op = 'insertDefinedName'; id = 'xlsx:workbook'
        name = 'SrcCells'; formula = 'Sheet2!$B$1'
    })
} 'xlsx-sheet-refs-name' | Out-Null

$renamed = Get-OutPath 'xlsx-sheet-refs-renamed.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $named.Replace('\', '/')
    output = $renamed.Replace('\', '/')
    ops = @(@{ op = 'renameSheet'; id = 'sheet:Sheet2'; name = 'New Name' })
} 'xlsx-sheet-refs-rename' | Out-Null

$deleted = Get-OutPath 'xlsx-sheet-refs-deleted.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $named.Replace('\', '/')
    output = $deleted.Replace('\', '/')
    ops = @(@{ op = 'deleteSheet'; id = 'sheet:Sheet2' })
} 'xlsx-sheet-refs-delete' | Out-Null

$excel = Ensure-MsExcel $renamed
$book = Open-ExcelBook $excel $renamed
$cap = [string]$book.Windows.Item(1).Caption
$ws = $book.Worksheets.Item('Sheet1')
$f1 = [string]$ws.Range('A1').Formula
$f2 = [string]$ws.Range('A2').Formula
$name = [string]$book.Names.Item('SrcCells').RefersTo
Add-Check 'xlsx-Sr1' (
    ($cap -notmatch 'Repair') -and ($f1 -eq "='New Name'!B1") -and ($f2 -eq '="Sheet2!B1"') -and ($name -eq "='New Name'!`$B`$1")
) "caption=$cap f1=$f1 f2=$f2 name=$name"
$book.Close($false)
Release-Com $ws
Release-Com $book

$book = Open-ExcelBook $excel $deleted
$capD = [string]$book.Windows.Item(1).Caption
$ws = $book.Worksheets.Item('Sheet1')
$d1 = [string]$ws.Range('A1').Formula
$d2 = [string]$ws.Range('A2').Formula
$nameD = [string]$book.Names.Item('SrcCells').RefersTo
Add-Check 'xlsx-Sr2' (
    ($capD -notmatch 'Repair') -and ($d1 -eq '=#REF!') -and ($d2 -eq '="Sheet2!B1"') -and ($nameD -match '#REF!')
) "caption=$capD f1=$d1 f2=$d2 name=$nameD"
$book.Close($false)
Release-Com $ws
Release-Com $book
