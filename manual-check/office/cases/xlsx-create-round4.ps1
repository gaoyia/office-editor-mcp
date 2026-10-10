# xlsx create 第四轮：docs/checklist-xlsx.md 的 C25–C29。
# 用 docs/examples/xlsx-create-round4.json 同构请求。不要 COM 打开带哈希或外部链的产物。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.IO.Compression.FileSystem

function Get-ZipEntryText([string]$zipPath, [string]$name) {
    $z = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $e = $z.GetEntry($name)
        if (-not $e) { return $null }
        $sr = New-Object IO.StreamReader($e.Open())
        try { return $sr.ReadToEnd() } finally { $sr.Dispose() }
    }
    finally { $z.Dispose() }
}

$path = Ensure-XlsxCreateRound4
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'xlsx'
    scope = @{
        sheet = 'Sheet1'
        range = 'A1:G3'
        sparklines = $true
        protectedRanges = $true
        shapes = $true
    }
} 'xlsx-create-round4-inspect'

$sh = @($tree.sheets) | Where-Object { $_.name -eq 'Sheet1' } | Select-Object -First 1
$g1 = @($sh.cells) | Where-Object { $_.id -eq 'cell:Sheet1!G1' } | Select-Object -First 1
$dv = @($sh.dataValidations) | Select-Object -First 1
$nm = @($tree.definedNames) | Where-Object { $_.name -eq 'Sales' } | Select-Object -First 1
$pr = @($sh.protectedRanges) | Select-Object -First 1
$sparkRef = $null
if ($sh.sparklines -and @($sh.sparklines).Count -ge 1) {
    $sparkRef = [string]$sh.sparklines[0].sparklines[0].ref
}

$c25ok = ($null -ne $sh) `
    -and ([int]$sh.cols[0].width -eq 14) `
    -and (@($sh.merges) -contains 'G1:G2') `
    -and ($null -ne $dv) `
    -and ([string]$sh.print.orientation -eq 'landscape') `
    -and ($null -ne $g1) `
    -and ([string]$g1.numFmt -eq '0.00')
Add-Check 'xlsx-C25' $c25ok "width=$($sh.cols[0].width) merges=$($sh.merges -join ',') dv=$($dv.type) orient=$($sh.print.orientation) numFmt=$($g1.numFmt)"

$c26ok = ([int]$sh.tableCount -eq 1) `
    -and ([string]$sh.tables[0].name -eq 'SalesTbl') `
    -and ($null -ne $nm) `
    -and ([int]$sh.sparklineCount -eq 1) `
    -and ($sparkRef -eq 'D2')
Add-Check 'xlsx-C26' $c26ok "table=$($sh.tables[0].name) name=$($nm.name) sparks=$($sh.sparklineCount) ref=$sparkRef"

$c27ok = ([bool]$sh.sheetProtection.enforced) `
    -and (-not [bool]$sh.sheetProtection.hasPassword) `
    -and ([string]$pr.name -eq 'Range1')
Add-Check 'xlsx-C27' $c27ok "enforced=$($sh.sheetProtection.enforced) hasPassword=$($sh.sheetProtection.hasPassword) range=$($pr.name)"

$drawing = Get-ZipEntryText $path 'xl/drawings/drawing1.xml'
$c28ok = ([bool]$sh.outline.summaryBelow) `
    -and ([int]$sh.view.frozenRows -eq 1) `
    -and ($drawing -match 'xdr:grpSp')
Add-Check 'xlsx-C28' $c28ok "summaryBelow=$($sh.outline.summaryBelow) frozen=$($sh.view.frozenRows) grpSp=$($drawing -match 'xdr:grpSp')"

$excel = Ensure-MsExcel $path
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $path
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$loName = ''
$loCount = 0
try {
    $los = $s1.ListObjects
    $loCount = [int]$los.Count
    if ($loCount -ge 1) {
        $lo = $los.Item(1)
        $loName = [string]$lo.Name
        Release-Com $lo
    }
    Release-Com $los
} catch {
    Write-Log "ListObjects 失败: $($_.Exception.Message)"
}
$aerCount = 0
$aerTitle = ''
try {
    $aer = $s1.Protection.AllowEditRanges
    $aerCount = [int]$aer.Count
    if ($aerCount -lt 1) {
        $s1.Protect()
        $aer = $s1.Protection.AllowEditRanges
        $aerCount = [int]$aer.Count
    }
    if ($aerCount -ge 1) {
        $item = $aer.Item(1)
        $aerTitle = [string]$item.Title
    }
} catch {
    Write-Log "AllowEditRanges 失败: $($_.Exception.Message)"
}
$rowLvl = 0
try { $rowLvl = [int]$s1.Rows.Item(2).OutlineLevel } catch {
    Write-Log "OutlineLevel 失败: $($_.Exception.Message)"
}
$groupN = 0
try {
    $shapes = $s1.Shapes
    $n = [int]$shapes.Count
    for ($i = 1; $i -le $n; $i++) {
        $shp = $null
        try { $shp = $shapes.Item($i) } catch { }
        if ($null -eq $shp) { continue }
        try {
            $ty = 0
            try { $ty = [int]$shp.Type } catch { }
            if ($ty -eq 6) { $groupN++ }
        }
        finally { Release-Com $shp }
    }
    Release-Com $shapes
} catch {
    Write-Log "Shapes 失败: $($_.Exception.Message)"
}
Release-Com $s1
Close-AllWorkbooks $excel
$c29ok = ($cap -notmatch 'Repair') `
    -and ($loCount -ge 1) `
    -and ($loName -eq 'SalesTbl') `
    -and ($aerCount -ge 1) `
    -and ($aerTitle -eq 'Range1') `
    -and ($rowLvl -ge 2) `
    -and ($groupN -ge 1)
Add-Check 'xlsx-C29' $c29ok "caption=$cap table=$loCount/$loName aer=$aerCount/$aerTitle outline=$rowLvl groups=$groupN"
