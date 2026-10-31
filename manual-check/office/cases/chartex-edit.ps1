# chartex 改数据后，三家 Office 16.0 打开不报修复，源格是新值。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$xlsx = Get-OutPath 'chartex-edit.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'chartex-xlsx-create' | Out-Null
$xlsx2 = Get-OutPath 'chartex-edit-2.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $xlsx2.Replace('\', '/')
    ops = @(
        @{
            op = 'insertChart'; id = 'sheet:Sheet1'; type = 'chartex'; layoutId = 'waterfall'
            from = 'E2'; dataFrom = 'A1'; title = '水量'
            series = @(@{ name = '值'; categories = @('甲','乙'); values = @(10, 4) })
        }
    )
} 'chartex-xlsx-ins' | Out-Null
$xlsx3 = Get-OutPath 'chartex-edit-3.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx2.Replace('\', '/')
    output = $xlsx3.Replace('\', '/')
    ops = @(@{
        op = 'setChartSeries'; id = 'sheet:Sheet1/chart:1/series:0'; values = @(42, 7)
    })
} 'chartex-xlsx-set' | Out-Null
$xlsx4 = Get-OutPath 'chartex-edit-4.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx3.Replace('\', '/')
    output = $xlsx4.Replace('\', '/')
    ops = @(@{
        op = 'addChartSeries'; id = 'sheet:Sheet1/chart:1'; name = '第二'; values = @(1, 2)
    })
} 'chartex-xlsx-add' | Out-Null

$excel = Ensure-MsExcel $xlsx4
$book = Open-ExcelBook $excel $xlsx4
$cap = [string]$book.Windows.Item(1).Caption
$ws = $book.Worksheets.Item('Sheet1')
$b2 = $ws.Range('B2').Value2
$b3 = $ws.Range('B3').Value2
$c2 = $ws.Range('C2').Value2
Add-Check 'xlsx-Cx1' (($cap -notmatch 'Repair') -and ($b2 -eq 42) -and ($b3 -eq 7) -and ($c2 -eq 1)) "caption=$cap b2=$b2 b3=$b3 c2=$c2"
$book.Close($false)
Release-Com $ws
Release-Com $book

$docx = Get-OutPath 'chartex-edit.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $docx.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = '甲' }) }
} 'chartex-docx-create' | Out-Null
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $docx.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 3 }
} 'chartex-docx-inspect'
$para = [string]$tree.paragraphs[0].id
$docx2 = Get-OutPath 'chartex-edit-2.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $docx.Replace('\', '/')
    output = $docx2.Replace('\', '/')
    ops = @(
        @{
            op = 'insertChart'; id = $para; type = 'chartex'; layoutId = 'funnel'; title = '漏斗'
            series = @(@{ name = '值'; categories = @('甲','乙'); values = @(3, 1) })
        }
        @{
            op = 'setChartSeries'; id = "$para/chart:1/series:0"; values = @(9, 8)
        }
    )
} 'chartex-docx-set' | Out-Null

$word = Ensure-MsWord $docx2
$doc = Open-WordDoc $word $docx2
$capW = [string]$doc.ActiveWindow.Caption
Add-Check 'docx-Cx1' ($capW -notmatch 'Repair') "caption=$capW"
Close-WordDoc $doc

$pptx = Get-OutPath 'chartex-edit.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $pptx.Replace('\', '/'); format = 'pptx'
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'chartex-pptx-create' | Out-Null
$pptx2 = Get-OutPath 'chartex-edit-2.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $pptx.Replace('\', '/')
    output = $pptx2.Replace('\', '/')
    ops = @(
        @{
            op = 'insertChart'; id = 'slide:1'; type = 'chartex'; layoutId = 'treemap'
            series = @(@{ name = '值'; categories = @('甲','乙'); values = @(3, 1) })
        }
    )
} 'chartex-pptx-ins' | Out-Null
$treeP = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $pptx2.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'chartex-pptx-inspect'
$cid = [string]$treeP.slides[0].charts[0].id
$pptx3 = Get-OutPath 'chartex-edit-3.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $pptx2.Replace('\', '/')
    output = $pptx3.Replace('\', '/')
    ops = @(@{ op = 'setChartSeries'; id = "$cid/series:0"; values = @(9, 8) })
} 'chartex-pptx-set' | Out-Null

$ppt = Ensure-MsPpt $pptx3
$pres = Open-PptPres $ppt $pptx3
$capP = Get-PptCaption $pres
Add-Check 'pptx-Cx1' ($capP -notmatch 'Repair') "caption=$capP"
Close-PptPres $pres
