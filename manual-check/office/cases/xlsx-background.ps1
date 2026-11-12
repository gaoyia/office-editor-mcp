# xlsx 工作表背景图：docs/checklist-xlsx.md 的 Xbg1–Xbg4。
# create 写入背景，Excel 16.0 打开无 Repair，删掉后再设一次。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$png = Ensure-OoxpPngB
$created = Get-OutPath 'xlsx-bg-created.xlsx'
if (Test-Path $created) { Remove-Item $created -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action = 'create'
    path = $created.Replace('\', '/')
    workbook = @{
        sheets = @(
            @{ name = 'Sheet1'; background = $png.Replace('\', '/') }
        )
    }
} 'xlsx-bg-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1
    action = 'inspect'
    path = $created.Replace('\', '/')
    format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-bg-inspect'
$sheet = $tree.sheets[0]
$bgOk = ($sheet.background.id -eq 'sheet:Sheet1/background') -and ($sheet.background.contentType -eq 'image/png')
$notPic = ($sheet.pictureCount -eq 0) -and ($tree.pictureCount -eq 0) -and (-not $sheet.pictures)
Add-Check 'xlsx-Xbg1' ($bgOk -and $notPic) "id=$($sheet.background.id) ct=$($sheet.background.contentType) pics=$($sheet.pictureCount)"

$exported = Get-OutPath 'xlsx-bg-export.png'
if (Test-Path $exported) { Remove-Item $exported -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action = 'exportMedia'
    path = $created.Replace('\', '/')
    id = 'sheet:Sheet1/background'
    output = $exported.Replace('\', '/')
} 'xlsx-bg-export' | Out-Null
$same = ((Get-FileHash $exported).Hash -eq (Get-FileHash $png).Hash)
Add-Check 'xlsx-Xbg2' $same 'export bytes'

$excel = Ensure-MsExcel $created
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $created
$cap = [string]$wb.Windows.Item(1).Caption
$saved = Get-OutPath 'xlsx-bg-saved.xlsx'
if (Test-Path $saved) { Remove-Item $saved -Force }
$wb.SaveAs($saved)
Close-AllWorkbooks $excel
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1
    action = 'inspect'
    path = $saved.Replace('\', '/')
    format = 'xlsx'
} 'xlsx-bg-saved'
$still = $treeS.sheets[0].background.id -eq 'sheet:Sheet1/background'
Add-Check 'xlsx-Xbg3' (($cap -notmatch 'Repair') -and $still) "caption=$cap id=$($treeS.sheets[0].background.id)"

$deleted = Get-OutPath 'xlsx-bg-deleted.xlsx'
if (Test-Path $deleted) { Remove-Item $deleted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action = 'apply'
    path = $created.Replace('\', '/')
    output = $deleted.Replace('\', '/')
    ops = @(@{ op = 'deleteSheetBackground'; id = 'sheet:Sheet1/background' })
} 'xlsx-bg-delete' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1
    action = 'inspect'
    path = $deleted.Replace('\', '/')
    format = 'xlsx'
} 'xlsx-bg-deleted'
$gone = $null -eq $treeD.sheets[0].background
Add-Check 'xlsx-Xbg4' $gone "background=$($treeD.sheets[0].background)"
