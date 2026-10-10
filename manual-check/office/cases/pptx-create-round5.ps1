# pptx create 第五轮：docs/checklist-pptx.md 的 Ck1–Ck5。
# 备注页公式与 3D、图 / 表 / 图表 / SmartArt 链、表单元格链。
# 不算放映引擎。不写批注内图。不要和 Cq、C、Phr、Pcip 撞。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Invoke-OoxpExpectFail($obj, [string]$name) {
    $path = Join-Path $global:OoxpReqDir "$name.json"
    Write-JsonFile $path $obj
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $global:OoxpExe
    $psi.Arguments = "--input `"$path`""
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [System.Text.Encoding]::UTF8
    $proc = [System.Diagnostics.Process]::Start($psi)
    $stdout = $proc.StandardOutput.ReadToEnd().Trim()
    $stderr = $proc.StandardError.ReadToEnd().Trim()
    $proc.WaitForExit()
    $code = $proc.ExitCode
    $proc.Dispose()
    Write-Log "ooxp $name 退出码 $code stderr=$stderr"
    return @{ ExitCode = $code; Stderr = $stderr; Stdout = $stdout }
}

function Get-PptErrorCode($raw) {
    try { return ($raw.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

function Get-PptActionAddress($shape, [int]$which) {
    $settings = $null
    $item = $null
    $hl = $null
    try {
        $settings = $shape.ActionSettings
        $item = $settings.Item($which)
        $hl = $item.Hyperlink
        return [string]$hl.Address
    }
    catch {
        return ''
    }
    finally {
        Release-Com $hl
        Release-Com $item
        Release-Com $settings
    }
}

function Find-PptShapeByType($slide, [int]$type) {
    $shapes = $slide.Shapes
    try {
        $n = [int]$shapes.Count
        for ($i = 1; $i -le $n; $i++) {
            $cand = $shapes.Item($i)
            $hit = $false
            try { $hit = ([int]$cand.Type -eq $type) } catch { $hit = $false }
            if ($hit) { return $cand }
            Release-Com $cand
        }
    }
    finally { Release-Com $shapes }
    return $null
}

function Get-PptTableCellRunAddress($tableShape, [int]$row, [int]$col) {
    $tbl = $null
    $cell = $null
    $cellShape = $null
    $tf = $null
    $tr = $null
    $settings = $null
    $item = $null
    $hl = $null
    try {
        $tbl = $tableShape.Table
        $cell = $tbl.Cell($row, $col)
        $cellShape = $cell.Shape
        $tf = $cellShape.TextFrame
        $tr = $tf.TextRange
        $settings = $tr.ActionSettings
        $item = $settings.Item(1)
        $hl = $item.Hyperlink
        return [string]$hl.Address
    }
    catch {
        return ''
    }
    finally {
        Release-Com $hl
        Release-Com $item
        Release-Com $settings
        Release-Com $tr
        Release-Com $tf
        Release-Com $cellShape
        Release-Com $cell
        Release-Com $tbl
    }
}

$png = Ensure-OoxpPngA
$glb = Ensure-OoxpGlb
$pngFile = $png.Replace('\', '/')
$glbFile = $glb.Replace('\', '/')

$path = Get-OutPath 'pptx-create-round5.pptx'
if (Test-Path $path) { Remove-Item -Force $path }

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $path.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(
            @{
                shapes = @(@{ text = '第一页' })
                pictures = @(
                    @{
                        file = $pngFile
                        hyperlink = @{ url = 'https://example.com/pic' }
                    }
                )
                tables = @(
                    @{
                        rows = 2
                        cols = 2
                        hyperlink = @{ url = 'https://example.com/table' }
                        cells = (ConvertFrom-Json @'
[
  ["甲", {"text":"乙","url":"https://example.com/cell"}],
  ["丙", "丁"]
]
'@)
                    }
                )
                charts = @(
                    @{
                        type = 'col'
                        series = @(@{ values = @(1, 2, 3) })
                        hyperlink = @{ url = 'https://example.com/chart' }
                    }
                )
                smartArts = @(
                    @{
                        layout = 'list'
                        hyperlink = @{ url = 'https://example.com/sa' }
                    }
                )
                notes = @{
                    text = '备注'
                    maths = @(@{ text = 'E=mc^2' })
                    model3ds = @(@{ file = $glbFile })
                }
            }
            @{
                shapes = @(@{ text = '第二页' })
            }
        )
    }
} 'pptx-ck-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'pptx'
    scope = @{
        slide = 1
        notes = $true
        maths = $true
        model3ds = $true
        hyperlinks = $true
        pictures = $true
        tables = $true
        charts = $true
        smartArt = $true
    }
} 'pptx-ck-inspect'

$ppt = Ensure-MsPpt $path
Close-PptPresByPath $ppt $path
$pres = Open-PptPres $ppt $path
$cap = Get-PptCaption $pres
$slide1 = Get-PptSlide $pres 1
$picSh = Find-PptShapeByType $slide1 13
$tblSh = Find-PptShapeByType $slide1 19
$chartSh = Find-PptShapeByType $slide1 3
$saSh = Find-PptShapeByType $slide1 24
$addrPic = Get-PptActionAddress $picSh 1
$addrTbl = Get-PptActionAddress $tblSh 1
$addrCell = Get-PptTableCellRunAddress $tblSh 1 2
$addrChart = Get-PptActionAddress $chartSh 1
$addrSa = Get-PptActionAddress $saSh 1
Release-Com $saSh
Release-Com $chartSh
Release-Com $tblSh
Release-Com $picSh
Release-Com $slide1
Close-PptPres $pres

$notes = $tree.slides[0].notes
$notesXml = Get-ZipPartText $path 'ppt/notesSlides/notesSlide1.xml'
$ck1ok = ($cap -notmatch 'Repair') `
    -and ([int]$tree.mathCount -eq 0) `
    -and ([int]$tree.model3dCount -eq 0) `
    -and ([int]$notes.mathCount -eq 1) `
    -and ([string]$notes.maths[0].text -eq 'E=mc^2') `
    -and ([int]$notes.model3dCount -eq 1) `
    -and ($notesXml -like '*a14:m*')
Add-Check 'pptx-Ck1' $ck1ok "rootMath=$($tree.mathCount) notesMath=$($notes.mathCount) notes3d=$($notes.model3dCount) caption=$cap"

$links = @($tree.slides[0].hyperlinks | Where-Object { $_ })
function Find-LinkOn($on) {
    return @($links | Where-Object { $_.on -eq $on } | Select-Object -First 1)
}
$picLink = Find-LinkOn 'pic'
$ck2ok = ($cap -notmatch 'Repair') `
    -and ([string]$picLink.on -eq 'pic') `
    -and ([string]$picLink.url -eq 'https://example.com/pic') `
    -and ($addrPic -match 'example.com/pic')
Add-Check 'pptx-Ck2' $ck2ok "on=$($picLink.on) addr=$addrPic"

$tableLink = Find-LinkOn 'table'
$cellLink = Find-LinkOn 'cell'
$ck3ok = ($cap -notmatch 'Repair') `
    -and ([string]$tableLink.on -eq 'table') `
    -and ([string]$cellLink.on -eq 'cell') `
    -and ($addrTbl -match 'example.com/table') `
    -and ($addrCell -match 'example.com/cell')
Add-Check 'pptx-Ck3' $ck3ok "table=$($tableLink.on) cell=$($cellLink.on) tblAddr=$addrTbl cellAddr=$addrCell"

$chartLink = Find-LinkOn 'chart'
$saLink = Find-LinkOn 'smartArt'
$ck4ok = ($cap -notmatch 'Repair') `
    -and ([string]$chartLink.on -eq 'chart') `
    -and ([string]$saLink.on -eq 'smartArt') `
    -and ($addrChart -match 'example.com/chart') `
    -and ($addrSa -match 'example.com/sa')
Add-Check 'pptx-Ck4' $ck4ok "chart=$($chartLink.on) sa=$($saLink.on) chartAddr=$addrChart saAddr=$addrSa"

$failTiming = Get-OutPath 'pptx-create-round5-fail-timing.pptx'
$failText = Get-OutPath 'pptx-create-round5-fail-text.pptx'
$strNotes = Get-OutPath 'pptx-create-round5-notes-str.pptx'
foreach ($p in @($failTiming, $failText, $strNotes)) {
    if (Test-Path $p) { Remove-Item -Force $p }
}

$rawTiming = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'create'
    path = $failTiming.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(@{ shapes = @(@{ text = 'a' }); timing = @() })
    }
} 'pptx-ck-fail-timing'
$rawText = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'create'
    path = $failText.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(
            @{
                shapes = @(@{ text = 'a' })
                pictures = @(
                    @{
                        file = $pngFile
                        hyperlink = @{ url = 'https://example.com/'; text = 'x' }
                    }
                )
            }
        )
    }
} 'pptx-ck-fail-text'

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $strNotes.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(@{ shapes = @(@{ text = 'a' }); notes = '只要文本' })
    }
} 'pptx-ck-notes-str' | Out-Null
$treeStr = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $strNotes.Replace('\', '/'); format = 'pptx'
    scope = @{ notes = $true; maths = $true; slide = 1 }
} 'pptx-ck-notes-str-inspect'
$strBody = @($treeStr.slides[0].notes.shapes | Where-Object { [string]$_.text -eq '只要文本' } | Select-Object -First 1)

$ck5ok = ($rawTiming.ExitCode -ne 0) `
    -and ((Get-PptErrorCode $rawTiming) -eq 'InvalidField') `
    -and ($rawText.ExitCode -ne 0) `
    -and ((Get-PptErrorCode $rawText) -eq 'InvalidField') `
    -and ([int]$treeStr.slides[0].notes.mathCount -eq 0) `
    -and ($null -ne $strBody)
Add-Check 'pptx-Ck5' $ck5ok "timing=$(Get-PptErrorCode $rawTiming) text=$(Get-PptErrorCode $rawText) strMath=$($treeStr.slides[0].notes.mathCount)"
