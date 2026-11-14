# 用手测已经写出的 docx / xlsx / pptx 对四种只读出图。
# 不进 run.ps1，也不改那些用例原来的打开检查和 XML 检查。
# 库返回 NotImplemented 记跳过。库写出 PNG 才找 Office 16.0 金样。
# 金样连续两次不是差 0 的，这轮不跟库比。
#
#   pwsh -NoProfile -File .\manual-check\office\image-corpus.ps1
#   pwsh -NoProfile -File .\manual-check\office\image-corpus.ps1 -Phase inventory

[CmdletBinding()]
param(
    [ValidateSet('all', 'inventory', 'compare')]
    [string]$Phase = 'all'
)

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$utf8 = New-Object System.Text.UTF8Encoding $false
$repo = $global:OoxpRepoRoot
$outDir = Join-Path $global:OoxpOutDir 'image-corpus'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$rowsPath = Join-Path $outDir 'rows.json'
$diffScript = Join-Path $repo 'manual-check\pixel-diff.ps1'
$pngScript = Join-Path $repo 'manual-check\pdf-to-png.ps1'
$exe = $global:OoxpExe
$script:reqSeq = 0

function Write-Utf8([string]$path, [string]$text) {
    [IO.File]::WriteAllText($path, $text, $utf8)
}

function ConvertTo-Short([string]$text) {
    if (-not $text) { return '' }
    $one = ($text -replace '\s+', ' ').Trim()
    $one = $one -replace '\|', '/'
    if ($one.Length -gt 80) { return $one.Substring(0, 80) }
    return $one
}

function Invoke-OoxpQuiet($obj) {
    $script:reqSeq++
    $path = Join-Path $outDir ("req-{0}.json" -f $script:reqSeq)
    Write-Utf8 $path (($obj | ConvertTo-Json -Compress -Depth 8))
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $exe
    $psi.Arguments = "--input `"$path`" --lang zh-CN"
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [Text.Encoding]::UTF8
    $proc = [Diagnostics.Process]::Start($psi)
    $stdout = $proc.StandardOutput.ReadToEnd()
    $stderr = $proc.StandardError.ReadToEnd()
    $proc.WaitForExit()
    $code = $proc.ExitCode
    $proc.Dispose()
    Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
    $payload = $null
    $raw = if ($code -eq 0) { $stdout } else { $stderr }
    if ($raw) {
        try { $payload = $raw | ConvertFrom-Json } catch { $payload = $null }
    }
    return @{ Code = $code; Payload = $payload; Raw = (ConvertTo-Short $raw) }
}

function New-Row($case, $file, $action, $id, $result, $detail) {
    [pscustomobject]@{
        Case   = $case
        File   = $file
        Action = $action
        Id     = $id
        Result = $result
        Detail = (ConvertTo-Short $detail)
        Sheet  = ''
        Name   = ''
        Ref    = ''
        Lib    = ''
        Kind   = ''
    }
}

function Get-CaseMap {
    $map = @{}
    $cases = Get-ChildItem -LiteralPath (Join-Path $PSScriptRoot 'cases') -Filter '*.ps1'
    foreach ($case in $cases) {
        $text = [IO.File]::ReadAllText($case.FullName)
        $hits = [regex]::Matches($text, "Get-OutPath\s+['`"]([^'`"]+\.(?:docx|xlsx|pptx))['`"]")
        foreach ($hit in $hits) {
            $name = $hit.Groups[1].Value
            $stem = [IO.Path]::GetFileNameWithoutExtension($case.Name)
            if ($map.ContainsKey($name)) {
                if ($map[$name] -notlike "*$stem*") { $map[$name] = $map[$name] + ',' + $stem }
            }
            else { $map[$name] = $stem }
        }
    }
    return $map
}

function Add-Pages($rows, $case, $file, $full) {
    $n = 1
    while ($n -le 50) {
        $lib = Join-Path $outDir ("{0}-page-{1}-lib.png" -f $file, $n)
        if (Test-Path -LiteralPath $lib) { Remove-Item -LiteralPath $lib -Force }
        $res = Invoke-OoxpQuiet @{
            schemaVersion = 1; action = 'exportPageImage'; format = 'docx'
            path = $full.Replace('\', '/'); id = "page:$n"
            output = $lib.Replace('\', '/')
        }
        if ($res.Code -eq 0) {
            $row = New-Row $case $file 'exportPageImage' "page:$n" '待对照' ''
            $row.Lib = $lib
            $rows.Add($row) | Out-Null
            $n++
            continue
        }
        $err = if ($res.Payload -and $res.Payload.error) { [string]$res.Payload.error.code } else { '' }
        if ($err -eq 'NotImplemented' -and $n -eq 1) {
            $rows.Add((New-Row $case $file 'exportPageImage' 'page:1' '跳过' 'NotImplemented')) | Out-Null
        }
        elseif ($err -eq 'IdNotFound') {
            if ($n -eq 1) { $rows.Add((New-Row $case $file 'exportPageImage' 'page:1' '打不开' 'IdNotFound')) | Out-Null }
        }
        else {
            $id = if ($n -eq 1) { 'page:1' } else { "page:$n" }
            $rows.Add((New-Row $case $file 'exportPageImage' $id '打不开' $(if ($err) { $err } else { $res.Raw }))) | Out-Null
        }
        break
    }
}

function Add-Slides($rows, $case, $file, $full) {
    $res = Invoke-OoxpQuiet @{
        schemaVersion = 1; action = 'inspect'; format = 'pptx'
        path = $full.Replace('\', '/')
    }
    if ($res.Code -ne 0) {
        $err = if ($res.Payload -and $res.Payload.error) { [string]$res.Payload.error.code } else { $res.Raw }
        $rows.Add((New-Row $case $file 'exportSlideImage' '-' '打不开' $err)) | Out-Null
        return
    }
    if (-not $res.Payload.slides) { return }
    $slides = @($res.Payload.slides)
    $i = 0
    foreach ($slide in $slides) {
        $i++
        $id = [string]$slide.id
        $lib = Join-Path $outDir ("{0}-slide-{1}-lib.png" -f $file, $i)
        if (Test-Path -LiteralPath $lib) { Remove-Item -LiteralPath $lib -Force }
        $exp = Invoke-OoxpQuiet @{
            schemaVersion = 1; action = 'exportSlideImage'; format = 'pptx'
            path = $full.Replace('\', '/'); id = $id
            output = $lib.Replace('\', '/')
        }
        if ($exp.Code -eq 0) {
            $row = New-Row $case $file 'exportSlideImage' $id '待对照' ''
            $row.Lib = $lib
            $rows.Add($row) | Out-Null
        }
        else {
            $err = if ($exp.Payload -and $exp.Payload.error) { [string]$exp.Payload.error.code } else { $exp.Raw }
            $result = if ($err -eq 'NotImplemented') { '跳过' } else { '打不开' }
            $rows.Add((New-Row $case $file 'exportSlideImage' $id $result $err)) | Out-Null
        }
    }
}

function Add-Workbook($rows, $case, $file, $full) {
    $res = Invoke-OoxpQuiet @{
        schemaVersion = 1; action = 'inspect'; format = 'xlsx'
        path = $full.Replace('\', '/')
        scope = @{ charts = $true; sparklines = $true }
    }
    if ($res.Code -ne 0) {
        $err = if ($res.Payload -and $res.Payload.error) { [string]$res.Payload.error.code } else { $res.Raw }
        $rows.Add((New-Row $case $file 'exportChartImage' '-' '打不开' $err)) | Out-Null
        return
    }
    if (-not $res.Payload.sheets) { return }
    $chartN = 0
    $sparkN = 0
    foreach ($sheet in @($res.Payload.sheets)) {
        if ($sheet.charts) { foreach ($chart in @($sheet.charts)) {
            $chartN++
            $id = [string]$chart.id
            $lib = Join-Path $outDir ("{0}-chart-{1}-lib.png" -f $file, $chartN)
            if (Test-Path -LiteralPath $lib) { Remove-Item -LiteralPath $lib -Force }
            $exp = Invoke-OoxpQuiet @{
                schemaVersion = 1; action = 'exportChartImage'; format = 'xlsx'
                path = $full.Replace('\', '/'); id = $id
                output = $lib.Replace('\', '/')
            }
            if ($exp.Code -eq 0) {
                $row = New-Row $case $file 'exportChartImage' $id '待对照' ''
                $row.Sheet = [string]$sheet.name
                $row.Name = [string]$chart.name
                $row.Kind = [string]$sheet.kind
                $row.Lib = $lib
                $rows.Add($row) | Out-Null
            }
            else {
                $err = if ($exp.Payload -and $exp.Payload.error) { [string]$exp.Payload.error.code } else { $exp.Raw }
                $result = if ($err -eq 'NotImplemented') { '跳过' } else { '打不开' }
                $rows.Add((New-Row $case $file 'exportChartImage' $id $result $err)) | Out-Null
            }
        } }
        if ($sheet.sparklines) { foreach ($group in @($sheet.sparklines)) {
            $sparkN++
            $id = [string]$group.id
            $lib = Join-Path $outDir ("{0}-spark-{1}-lib.png" -f $file, $sparkN)
            if (Test-Path -LiteralPath $lib) { Remove-Item -LiteralPath $lib -Force }
            $exp = Invoke-OoxpQuiet @{
                schemaVersion = 1; action = 'exportSparklineImage'; format = 'xlsx'
                path = $full.Replace('\', '/'); id = $id
                output = $lib.Replace('\', '/')
            }
            $ref = ''
            if ($group.sparklines) {
                $ones = @($group.sparklines)
                if ($ones.Count -eq 1 -and $ones[0].ref) { $ref = [string]$ones[0].ref }
            }
            if ($exp.Code -eq 0) {
                $row = New-Row $case $file 'exportSparklineImage' $id '待对照' ''
                $row.Sheet = [string]$sheet.name
                $row.Ref = $ref
                $row.Lib = $lib
                $rows.Add($row) | Out-Null
            }
            else {
                $err = if ($exp.Payload -and $exp.Payload.error) { [string]$exp.Payload.error.code } else { $exp.Raw }
                $result = if ($err -eq 'NotImplemented') { '跳过' } else { '打不开' }
                $rows.Add((New-Row $case $file 'exportSparklineImage' $id $result $err)) | Out-Null
            }
        } }
    }
}

function Write-CorpusMd($rows, [int]$missing) {
    $pass = @($rows | Where-Object Result -eq '通过').Count
    $skip = @($rows | Where-Object Result -eq '跳过').Count
    $fail = @($rows | Where-Object Result -eq '失败').Count
    $wait = @($rows | Where-Object Result -eq '待对照').Count
    $unstable = @($rows | Where-Object Result -eq '金样不稳').Count
    $goldFail = @($rows | Where-Object Result -eq '金样失败').Count
    $closed = @($rows | Where-Object Result -eq '打不开').Count
    $uncompared = @($rows | Where-Object Result -eq '未比').Count
    $sb = New-Object System.Text.StringBuilder
    [void]$sb.AppendLine('# 用例出图对照清单')
    [void]$sb.AppendLine('')
    [void]$sb.AppendLine('由 `manual-check/office/image-corpus.ps1` 生成。像素图留在 `manual-check/office/out/image-corpus/`，不进仓库。')
    [void]$sb.AppendLine('')
    [void]$sb.AppendLine("通过 $pass。跳过 $skip。失败 $fail。待对照 $wait。金样不稳 $unstable。金样失败 $goldFail。未比 $uncompared。打不开 $closed。")
    [void]$sb.AppendLine("脚本点名但 out 目录里没有的文件 $missing 份，没有对象行。")
    [void]$sb.AppendLine('库报 `NotImplemented` 的 docx 只列 `page:1`。这种文件的页数要等能排版才知道。')
    [void]$sb.AppendLine('')
    [void]$sb.AppendLine('| 用例 | 文件 | 动作 | id | 结果 | 说明 |')
    [void]$sb.AppendLine('| --- | --- | --- | --- | --- | --- |')
    foreach ($row in $rows) {
        [void]$sb.AppendLine("| $($row.Case) | $($row.File) | $($row.Action) | $($row.Id) | $($row.Result) | $($row.Detail) |")
    }
    $md = Join-Path $PSScriptRoot 'image-corpus.md'
    Write-Utf8 $md $sb.ToString()
    Write-Host "wrote $md"
    Write-Host "通过 $pass  跳过 $skip  失败 $fail  待对照 $wait  金样不稳 $unstable  金样失败 $goldFail  未比 $uncompared  打不开 $closed"
}

function Invoke-DiffText($left, $right, $out) {
    $text = & pwsh -NoProfile -File $diffScript -Left $left -Right $right -Out $out -MaxDelta 0 | Out-String
    return $text.Trim()
}

function Test-Stable($left, $right, $out) {
    $text = Invoke-DiffText $left $right $out
    $ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    return @{ Ok = $ok; Text = $text }
}

function Set-PixelResult($row, $gold, $tag) {
    $diff = Join-Path $outDir ("{0}-{1}-diff.png" -f $row.File, $tag)
    $text = Invoke-DiffText $gold $row.Lib $diff
    if (($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')) {
        $row.Result = '通过'
        $row.Detail = 'differ=0 maxDelta=0'
        return
    }
    $row.Result = '失败'
    if ($text -match 'size-differ') { $row.Detail = $text }
    elseif ($text -match 'differ=(\d+).*maxDelta=(\d+)') { $row.Detail = "differ=$($Matches[1]) maxDelta=$($Matches[2])" }
    else { $row.Detail = $(if ($text) { $text } else { '没有对照输出' }) }
}

function Save-CellBitmap($img, $path) {
    $w = $img.Width - 1
    $h = $img.Height - 1
    if ($w -lt 1 -or $h -lt 1) { throw '单元格图太小' }
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    try {
        $src = New-Object System.Drawing.Rectangle 0, 0, $w, $h
        $g.DrawImage($img, $src, $src, [System.Drawing.GraphicsUnit]::Pixel)
    }
    finally { $g.Dispose() }
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

function Copy-CellPicture($ws, $ref, $path) {
    Add-Type -AssemblyName System.Windows.Forms -ErrorAction SilentlyContinue
    Add-Type -AssemblyName System.Drawing -ErrorAction SilentlyContinue
    $ws.Range($ref).CopyPicture(1, 2)
    Start-Sleep -Milliseconds 200
    $img = [System.Windows.Forms.Clipboard]::GetImage()
    if ($null -eq $img) { throw '剪贴板没有单元格图' }
    try { Save-CellBitmap $img $path }
    finally { $img.Dispose() }
}

function Test-EmptyCell($cell) {
    if ([string]$cell.Text) { return $false }
    $pattern = 0
    try { $pattern = [int]$cell.Interior.Pattern } catch { return $false }
    if ($pattern -ne -4142) { return $false }
    foreach ($edge in @([int]$cell.Borders.Item(7).LineStyle, [int]$cell.Borders.Item(8).LineStyle, [int]$cell.Borders.Item(9).LineStyle, [int]$cell.Borders.Item(10).LineStyle)) {
        if ($edge -ne -4142) { return $false }
    }
    return $true
}

function Get-ChartOrdinal([string]$name) {
    if ($name -match '(\d+)\s*$') { return [int]$Matches[1] }
    return -1
}

function Export-OneChart($chart, [string]$path) {
    if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force }
    [void]$chart.Export($path, 'PNG')
    return (Test-Path -LiteralPath $path) -and ((Get-Item -LiteralPath $path).Length -gt 0)
}

# Excel 对象名常是 Chart 1，包里的 cNvPr 名是图表 1。先对全文，再对末尾数字。
function Export-NamedChart($sheet, [string]$chartName, [string]$kind, [string]$path) {
    if ($kind -eq 'chartsheet') {
        return (Export-OneChart $sheet $path)
    }
    $charts = $sheet.ChartObjects()
    $held = @()
    try {
        $n = [int]$charts.Count
        $exact = $null
        $byNumber = @()
        for ($i = 1; $i -le $n; $i++) {
            $co = $charts.Item($i)
            $held += $co
            $name = [string]$co.Name
            if ($name -eq $chartName) { $exact = $co }
            elseif ((Get-ChartOrdinal $name) -eq (Get-ChartOrdinal $chartName) -and (Get-ChartOrdinal $chartName) -ge 0) {
                $byNumber += $co
            }
        }
        $pick = $null
        if ($exact) { $pick = $exact }
        elseif ($byNumber.Count -eq 1) { $pick = $byNumber[0] }
        elseif ($n -eq 1) { $pick = $held[0] }
        if (-not $pick) { return $false }
        $chart = $pick.Chart
        try { return (Export-OneChart $chart $path) }
        finally { Release-Com $chart }
    }
    finally {
        foreach ($co in $held) { Release-Com $co }
        Release-Com $charts
    }
}

function Compare-Pages($word, $full, $pageRows) {
    $pdf1 = Join-Path $outDir 'gold-a.pdf'
    $pdf2 = Join-Path $outDir 'gold-b.pdf'
    $doc = Open-WordDoc $word $full
    try {
        if (Test-Path $pdf1) { Remove-Item $pdf1 -Force }
        if (Test-Path $pdf2) { Remove-Item $pdf2 -Force }
        $doc.ExportAsFixedFormat($pdf1, 17)
        $doc.ExportAsFixedFormat($pdf2, 17)
    }
    finally { Close-WordDoc $doc }
    foreach ($row in $pageRows) {
        $n = 1
        if ($row.Id -match 'page:(\d+)') { $n = [int]$Matches[1] }
        $g1 = Join-Path $outDir ("{0}-{1}-gold-1.png" -f $row.File, $row.Id.Replace(':', '-'))
        $g2 = Join-Path $outDir ("{0}-{1}-gold-2.png" -f $row.File, $row.Id.Replace(':', '-'))
        & pwsh -NoProfile -File $pngScript -Pdf $pdf1 -Out $g1 -Page $n -Dpi 96
        if (-not (Test-Path -LiteralPath $g1)) { $row.Result = '金样失败'; $row.Detail = 'PDF 没有这一页'; continue }
        & pwsh -NoProfile -File $pngScript -Pdf $pdf2 -Out $g2 -Page $n -Dpi 96
        $stable = Test-Stable $g1 $g2 (Join-Path $outDir 'stable-diff.png')
        if (-not $stable.Ok) { $row.Result = '金样不稳'; $row.Detail = $stable.Text; continue }
        Set-PixelResult $row $g1 $row.Id.Replace(':', '-')
    }
}

function Compare-Slides($ppt, $full, $slideRows) {
    $pres = Open-PptPres $ppt $full
    try {
        foreach ($row in $slideRows) {
            $n = 1
            if ($row.Id -match 'slide:(\d+)') { $n = [int]$Matches[1] }
            $slide = Get-PptSlide $pres $n
            try {
                $g1 = Join-Path $outDir ("{0}-{1}-gold-1.png" -f $row.File, $row.Id.Replace(':', '-'))
                $g2 = Join-Path $outDir ("{0}-{1}-gold-2.png" -f $row.File, $row.Id.Replace(':', '-'))
                if (Test-Path $g1) { Remove-Item $g1 -Force }
                if (Test-Path $g2) { Remove-Item $g2 -Force }
                $slide.Export($g1, 'PNG')
                $slide.Export($g2, 'PNG')
                $stable = Test-Stable $g1 $g2 (Join-Path $outDir 'stable-diff.png')
                if (-not $stable.Ok) { $row.Result = '金样不稳'; $row.Detail = $stable.Text; continue }
                Set-PixelResult $row $g1 $row.Id.Replace(':', '-')
            }
            finally { Release-Com $slide }
        }
    }
    finally { Close-PptPres $pres }
}

function Compare-Book($excel, $full, $chartRows, $sparkRows) {
    $wb = Open-ExcelBook $excel $full
    try {
        foreach ($row in @($chartRows)) {
            $sheet = $null
            try {
                $sheets = $wb.Sheets
                try { $sheet = $sheets.Item($row.Sheet) }
                finally { Release-Com $sheets }
                $g1 = Join-Path $outDir ("{0}-{1}-gold-1.png" -f $row.File, ($row.Id -replace '[:/]', '-'))
                $g2 = Join-Path $outDir ("{0}-{1}-gold-2.png" -f $row.File, ($row.Id -replace '[:/]', '-'))
                $ok1 = Export-NamedChart $sheet $row.Name $row.Kind $g1
                $ok2 = Export-NamedChart $sheet $row.Name $row.Kind $g2
                if (-not $ok1 -or -not $ok2) { $row.Result = '金样失败'; $row.Detail = 'Excel 没有导出 PNG'; continue }
                $stable = Test-Stable $g1 $g2 (Join-Path $outDir 'stable-diff.png')
                if (-not $stable.Ok) { $row.Result = '金样不稳'; $row.Detail = $stable.Text; continue }
                Set-PixelResult $row $g1 ($row.Id -replace '[:/]', '-')
            }
            finally { Release-Com $sheet }
        }
        foreach ($row in @($sparkRows)) {
            if (-not $row.Ref) { $row.Result = '未比'; $row.Detail = '没有单格落点'; continue }
            $sheet = $null
            $cell = $null
            try {
                $sheets = $wb.Sheets
                try { $sheet = $sheets.Item($row.Sheet) }
                finally { Release-Com $sheets }
                $sheet.Activate() | Out-Null
                $excel.ActiveWindow.DisplayGridlines = $false
                $excel.ActiveWindow.Zoom = 100
                $cell = $sheet.Range($row.Ref)
                if (-not (Test-EmptyCell $cell)) { $row.Result = '未比'; $row.Detail = '落点格有字、填充或边框'; continue }
                $tag = $row.Id -replace '[:/]', '-'
                $g1 = Join-Path $outDir ("{0}-{1}-gold-1.png" -f $row.File, $tag)
                $g2 = Join-Path $outDir ("{0}-{1}-gold-2.png" -f $row.File, $tag)
                Copy-CellPicture $sheet $row.Ref $g1
                Copy-CellPicture $sheet $row.Ref $g2
                $stable = Test-Stable $g1 $g2 (Join-Path $outDir 'stable-diff.png')
                if (-not $stable.Ok) { $row.Result = '金样不稳'; $row.Detail = $stable.Text; continue }
                Set-PixelResult $row $g1 $tag
            }
            finally {
                Release-Com $cell
                Release-Com $sheet
            }
        }
    }
    finally {
        $wb.Close($false)
        Release-Com $wb
    }
}

function Invoke-Compare($rows) {
    $pending = @($rows | Where-Object Result -eq '待对照')
    Write-Host ("待对照 {0}" -f $pending.Count)
    $byFile = $pending | Group-Object File
    $excel = $null
    $word = $null
    $ppt = $null
    try {
        foreach ($group in $byFile) {
            $full = Join-Path $global:OoxpOutDir $group.Name
            $pages = @($group.Group | Where-Object Action -eq 'exportPageImage')
            $slides = @($group.Group | Where-Object Action -eq 'exportSlideImage')
            $charts = @($group.Group | Where-Object Action -eq 'exportChartImage')
            $sparks = @($group.Group | Where-Object Action -eq 'exportSparklineImage')
            Write-Host ("对照 {0} 页 {1} 幻灯片 {2} 图表 {3} 迷你图 {4}" -f $group.Name, $pages.Count, $slides.Count, $charts.Count, $sparks.Count)
            try {
                if ($pages.Count) {
                    if (-not $word) { $word = Ensure-MsWord $full }
                    Compare-Pages $word $full $pages
                }
                if ($slides.Count) {
                    if (-not $ppt) { $ppt = Ensure-MsPpt $full }
                    Compare-Slides $ppt $full $slides
                }
                if ($charts.Count -or $sparks.Count) {
                    if (-not $excel) { $excel = Ensure-MsExcel $full }
                    Compare-Book $excel $full $charts $sparks
                }
            }
            catch {
                foreach ($row in @($group.Group)) {
                    if ($row.Result -eq '待对照') {
                        $row.Result = '金样失败'
                        $row.Detail = ConvertTo-Short $_.Exception.Message
                    }
                }
                Write-Host ("金样失败 {0} {1}" -f $group.Name, (ConvertTo-Short $_.Exception.Message))
            }
        }
    }
    finally {
        foreach ($pair in @(
                @{ App = $excel; Pid = $global:OoxpExcelPid; Name = 'EXCEL' },
                @{ App = $word; Pid = $global:OoxpWordPid; Name = 'WINWORD' },
                @{ App = $ppt; Pid = $global:OoxpPptPid; Name = 'POWERPNT' }
            )) {
            if ($pair.App) { try { $pair.App.Quit() } catch {} ; Release-Com $pair.App }
            if ($pair.Pid) {
                Stop-Process -Id $pair.Pid -Force -ErrorAction SilentlyContinue
                Write-Host ("stopped {0} {1}" -f $pair.Name, $pair.Pid)
            }
        }
        $global:OoxpExcel = $null
        $global:OoxpWord = $null
        $global:OoxpPpt = $null
    }
}

function Invoke-Inventory {
    $map = Get-CaseMap
    $rows = New-Object System.Collections.Generic.List[object]
    $missing = 0
    $seen = 0
    foreach ($name in ($map.Keys | Sort-Object)) {
        $full = Join-Path $global:OoxpOutDir $name
        if (-not (Test-Path -LiteralPath $full)) { $missing++; continue }
        $seen++
        if (($seen % 100) -eq 0) { Write-Host "已看 $seen $($rows.Count) 行" }
        $case = $map[$name]
        $ext = [IO.Path]::GetExtension($name).ToLowerInvariant()
        try {
            switch ($ext) {
                '.docx' { Add-Pages $rows $case $name $full }
                '.pptx' { Add-Slides $rows $case $name $full }
                '.xlsx' { Add-Workbook $rows $case $name $full }
            }
        }
        catch {
            $action = switch ($ext) { '.docx' { 'exportPageImage' } '.pptx' { 'exportSlideImage' } default { 'exportChartImage' } }
            $rows.Add((New-Row $case $name $action '-' '打不开' $_.Exception.Message)) | Out-Null
        }
    }
    $ordered = @($rows | Sort-Object Case, File, Action, Id)
    Write-Utf8 $rowsPath (($ordered | ConvertTo-Json -Depth 4))
    return @{ Rows = $ordered; Missing = $missing }
}

if ($Phase -eq 'compare') {
    $loaded = Get-Content -LiteralPath $rowsPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $bag = @{ Rows = @($loaded); Missing = 0 }
    if (Test-Path (Join-Path $outDir 'missing.txt')) {
        $bag.Missing = [int](Get-Content -LiteralPath (Join-Path $outDir 'missing.txt') -Raw)
    }
}
else {
    $bag = Invoke-Inventory
    Set-Content -LiteralPath (Join-Path $outDir 'missing.txt') -Value $bag.Missing -Encoding utf8
}

if ($Phase -ne 'inventory') { Invoke-Compare $bag.Rows }
Write-CorpusMd $bag.Rows $bag.Missing
