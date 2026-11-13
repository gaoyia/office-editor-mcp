# 窗体控件。编号 Xfc。Excel 先放入按钮、复选框、下拉，库再查看和修改。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Set-Face($obj, [string]$text) {
    foreach ($name in @('Text', 'Caption')) {
        try {
            $obj.$name = $text
            return
        } catch { }
    }
    throw "写不上标题：$text"
}

function Get-Face($obj) {
    foreach ($name in @('Text', 'Caption')) {
        try { return [string]$obj.$name } catch { }
    }
    return ''
}

$seed = Get-OutPath 'xlsx-form-seed.xlsx'
$edited = Get-OutPath 'xlsx-form-edit.xlsx'
$saved = Get-OutPath 'xlsx-form-saved.xlsx'
foreach ($p in @($seed, $edited, $saved)) { if (Test-Path $p) { Remove-Item -Force $p } }

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $seed.Replace('\', '/')
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = '甲' }
                @{ ref = 'A2'; value = '乙' }
            )
        })
    }
} 'xlsx-xfc-seed' | Out-Null

$excel = Ensure-MsExcel $seed
$excel.DisplayAlerts = $false
$wb = Open-ExcelBook $excel $seed
$ws = $wb.Worksheets.Item(1)
$btn = $ws.Buttons().Add(20, 20, 80, 24)
Set-Face $btn '按钮 1'
$box = $ws.CheckBoxes().Add(20, 50, 90, 20)
Set-Face $box '复选框 2'
$box.LinkedCell = '$B$1'
$drop = $ws.DropDowns().Add(20, 80, 90, 20)
$drop.ListFillRange = '$A$1:$A$2'
$wb.Save()
Close-AllWorkbooks $excel
Release-Com $drop
Release-Com $box
Release-Com $btn
Release-Com $ws

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'; format = 'xlsx'
    path = $seed.Replace('\', '/')
    scope = @{ formControls = $true }
} 'xlsx-xfc-inspect'
$items = @($tree.sheets[0].formControls)
$button = $items | Where-Object { $_.objectType -eq 'Button' } | Select-Object -First 1
$check = $items | Where-Object { $_.objectType -eq 'CheckBox' } | Select-Object -First 1
$menu = $items | Where-Object { $_.objectType -eq 'Drop' } | Select-Object -First 1
$seen = ($null -ne $button) -and ($null -ne $check) -and ($null -ne $menu) `
    -and ($button.caption -eq '按钮 1') -and ($check.caption -eq '复选框 2') `
    -and ($null -eq $menu.caption) -and ($check.fmlaLink -eq '$B$1') `
    -and ($menu.fmlaRange -eq '$A$1:$A$2') -and ($check.checked -eq $false)
Add-Check 'xlsx-Xfc1' $seen ("count=$($items.Count) button=$($button.caption) link=$($check.fmlaLink) range=$($menu.fmlaRange)")

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'; format = 'xlsx'
    path = $seed.Replace('\', '/')
    output = $edited.Replace('\', '/')
    ops = @(
        @{ op = 'setFormControl'; id = $button.id; caption = '按钮改' }
        @{ op = 'setFormControl'; id = $check.id; caption = '复选改'; fmlaLink = '$C$1'; checked = $true }
        @{ op = 'setFormControl'; id = $menu.id; sel = 2 }
    )
} 'xlsx-xfc-apply' | Out-Null

$excel = Ensure-MsExcel
$excel.DisplayAlerts = $false
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $edited
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)
$btn = $ws.Buttons().Item(1)
$box = $ws.CheckBoxes().Item(1)
$drop = $ws.DropDowns().Item(1)
$btnText = Get-Face $btn
$boxText = Get-Face $box
$link = [string]$box.LinkedCell
$boxOn = [int]$box.Value
$list = [string]$drop.ListFillRange
$index = [int]$drop.ListIndex
$cell = [string]$ws.Range('C1').Text
$noRepair = $cap -notmatch 'Repair'
$editedOk = $noRepair -and ($btnText -eq '按钮改') -and ($boxText -eq '复选改') `
    -and ($link -eq '$C$1') -and ($boxOn -eq 1) -and ($list -eq '$A$1:$A$2') `
    -and ($index -eq 2) -and ($cell -eq 'TRUE')
Add-Check 'xlsx-Xfc2' $editedOk ("caption=$cap btn=$btnText box=$boxText link=$link on=$boxOn list=$list index=$index cell=$cell")

$wb.SaveAs($saved)
Close-AllWorkbooks $excel
Release-Com $drop
Release-Com $box
Release-Com $btn
Release-Com $ws

$again = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'; format = 'xlsx'
    path = $saved.Replace('\', '/')
    scope = @{ formControls = $true }
} 'xlsx-xfc-saved'
$savedBtn = @($again.sheets[0].formControls) | Where-Object { $_.objectType -eq 'Button' } | Select-Object -First 1
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead($saved)
$drawing = ''
$vml = ''
foreach ($entry in $zip.Entries) {
    $stream = $entry.Open()
    $reader = New-Object IO.StreamReader($stream)
    $text = $reader.ReadToEnd()
    $reader.Dispose()
    $stream.Dispose()
    if ($entry.FullName -like '*drawing*.xml') { $drawing += $text }
    if ($entry.FullName -like '*.vml') { $vml += $text }
}
$zip.Dispose()
$both = ($savedBtn.caption -eq '按钮改') -and ($drawing.Contains('按钮改')) -and ($vml.Contains('按钮改')) `
    -and (-not $drawing.Contains('按钮 1')) -and (-not $vml.Contains('按钮 1'))
Add-Check 'xlsx-Xfc3' ($noRepair -and $both) ("caption=$($savedBtn.caption)")

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
