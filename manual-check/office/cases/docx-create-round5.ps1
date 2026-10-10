# docx create 第五轮：docs/checklist-docx.md 的 Cz1–Cz7。
# 框内表与链、段边框底纹、runs、组合、writePassword。不算排版引擎。
# 不要和 Cy、Cs、Cf、C、Cd、Bx、Bf、Bd、Rh 撞。

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

function Get-OoxpErrorCode($raw) {
    try { return ($raw.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

function Open-WordDocWritePassword($word, [string]$path, [string]$writePassword) {
    foreach ($d in @($word.Documents)) {
        if ($d.FullName -and ($d.FullName -ieq $path)) { return $d }
    }
    $missing = [Type]::Missing
    return $word.Documents.Open($path, $false, $false, $false, $missing, $missing, $false, $writePassword)
}

function Count-ShapeTables($sh) {
    $ty = 0
    try { $ty = [int]$sh.Type } catch { return 0 }
    if ($ty -eq 6) {
        $n = 0
        $items = $null
        try {
            $items = $sh.GroupItems
            $c = [int]$items.Count
            for ($j = 1; $j -le $c; $j++) {
                $it = $null
                try {
                    $it = $items.Item($j)
                    $n += Count-ShapeTables $it
                } catch { }
                finally { Release-Com $it }
            }
        } catch { }
        finally { Release-Com $items }
        return $n
    }
    $tf = $null
    $tr = $null
    $tables = $null
    $n = 0
    try {
        $tf = $sh.TextFrame
        $tr = $tf.TextRange
        $tables = $tr.Tables
        $n = [int]$tables.Count
    } catch { $n = 0 }
    finally {
        Release-Com $tables
        Release-Com $tr
        Release-Com $tf
    }
    return $n
}

function Get-BoxTableCount($doc) {
    $n = 0
    $shapes = $null
    try {
        $shapes = $doc.Shapes
        $sc = [int]$shapes.Count
        for ($i = 1; $i -le $sc; $i++) {
            $sh = $null
            try {
                $sh = $shapes.Item($i)
                $n += Count-ShapeTables $sh
            } catch { }
            finally { Release-Com $sh }
        }
    } catch { }
    finally { Release-Com $shapes }
    return $n
}

function Get-WordShapeStats($doc) {
    $shapes = $null
    $groupN = 0
    $n = 0
    try {
        $shapes = $doc.Shapes
        try { $n = [int]$shapes.Count } catch { $n = 0 }
        for ($i = 1; $i -le $n; $i++) {
            $sh = $null
            try {
                $sh = $shapes.Item($i)
                $ty = 0
                try { $ty = [int]$sh.Type } catch { $ty = 0 }
                if ($ty -eq 6) { $groupN++ }
            } catch { }
            finally { Release-Com $sh }
        }
    }
    finally { Release-Com $shapes }
    return @{ Count = $n; Group = $groupN }
}

$path = Ensure-DocxCreateRound5 -Force
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'docx'
    scope = @{
        from = 1; maxParas = 20
        textBoxes = $true; tables = $true; hyperlinks = $true
        shapes = $true; runs = $true
    }
} 'docx-create-round5-inspect'

$bx = $null
try { $bx = $tree.paragraphs[1].textBoxes[0] } catch {}
Add-Check 'docx-Cz1' (
    ([int]$tree.tableCount -eq 0) -and
    ([int]$bx.tableCount -eq 1)
) "rootTables=$($tree.tableCount) boxTables=$($bx.tableCount)"

Add-Check 'docx-Cz2' (
    ([int]$tree.hyperlinkCount -eq 0) -and
    ([int]$bx.hyperlinkCount -eq 1)
) "rootHl=$($tree.hyperlinkCount) boxHl=$($bx.hyperlinkCount)"

$p0 = $tree.paragraphs[0]
$r0 = $null
try { $r0 = $p0.runs[0] } catch {}
Add-Check 'docx-Cz3' (
    ([string]$p0.borders.top.val -eq 'single') -and
    ([string]$p0.shd.fill -eq 'FFFF00') -and
    ([int]$p0.tabs[0].pos -eq 720) -and
    ([string]$r0.highlight -eq 'yellow') -and
    ($r0.strike -eq $true)
) "val=$($p0.borders.top.val) fill=$($p0.shd.fill) hl=$($r0.highlight) strike=$($r0.strike)"

$wpPath = Get-OutPath 'docx-create-round5-wp.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $wpPath.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(@{ text = 'a' })
        protection = @{ writePassword = 'secret' }
    }
} 'docx-create-round5-wp' | Out-Null
$wpTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $wpPath.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 5 }
} 'docx-create-round5-wp-inspect'
$wpJson = $wpTree | ConvertTo-Json -Depth 8 -Compress
$wpXml = Get-ZipPartText $wpPath 'word/settings.xml'
$docXml = Get-ZipPartText $path 'word/document.xml'

$failRuns = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'create'
    path = (Join-Path $global:OoxpOutDir 'docx-create-round5-empty-runs.docx').Replace('\', '/')
    format = 'docx'
    document = @{ paragraphs = @(@{ runs = @() }) }
} 'docx-create-round5-empty-runs'
$xlsxPath = (Join-Path $global:OoxpOutDir 'docx-create-round5-xlsx.xlsx').Replace('\', '/')
$failXlsx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'create'
    path = $xlsxPath
    format = 'xlsx'
    workbook = @{
        sheets = @(@{ name = 'Sheet1' })
        groups = @(@{ ids = @('a', 'b') })
    }
} 'docx-create-round5-xlsx-groups'
Add-Check 'docx-Cz7' (
    ($failRuns.ExitCode -ne 0) -and
    ((Get-OoxpErrorCode $failRuns) -eq 'InvalidField') -and
    ($failXlsx.ExitCode -ne 0) -and
    ((Get-OoxpErrorCode $failXlsx) -eq 'InvalidField')
) "runs=$(Get-OoxpErrorCode $failRuns) xlsx=$(Get-OoxpErrorCode $failXlsx)"

$seed = Ensure-DocxCreate
$word = Ensure-MsWord $seed
try { $word.AskToUpdateLinks = $false } catch {}
try { $word.Options.UpdateLinksAtOpen = $false } catch {}
Close-WordDocByPath $word $path
$doc = Open-WordDoc $word $path
$caption = [string]$doc.ActiveWindow.Caption
$st = Get-WordShapeStats $doc
$boxTbl = Get-BoxTableCount $doc
$hl = $null
$shd = $null
$rng = $null
$para = $null
$font = $null
$strike = $null
try {
    $para = $doc.Paragraphs.Item(1)
    $rng = $para.Range
    $hl = [int]$rng.HighlightColorIndex
    $font = $rng.Font
    $strike = [int]$font.StrikeThrough
    try { $shd = [int]$para.Shading.BackgroundPatternColor } catch {}
} catch {}
finally {
    Release-Com $font
    Release-Com $rng
    Release-Com $para
}
Add-Check 'docx-Cz4' (
    ($caption -notmatch 'Repair') -and
    ($docXml -match 'wpg:wgp') -and
    (($st.Group -ge 1) -or ($st.Count -ge 2))
) "caption=$caption groups=$($st.Group) shapes=$($st.Count)"
Add-Check 'docx-Cz6' (
    ($caption -notmatch 'Repair') -and
    ($caption -match 'docx-create-round5') -and
    (($boxTbl -ge 1) -or ($hl -eq 7) -or ($shd -eq 65535))
) "caption=$caption boxTbl=$boxTbl hl=$hl shd=$shd strike=$strike"
Close-WordDoc $doc

Close-WordDocByPath $word $wpPath
$docWp = Open-WordDocWritePassword $word $wpPath 'secret'
$capWp = [string]$docWp.ActiveWindow.Caption
$reserved = $null
$ro = $null
try { $reserved = [bool]$docWp.WriteReserved } catch {}
try { $ro = [bool]$docWp.ReadOnly } catch {}
Close-WordDoc $docWp
Add-Check 'docx-Cz5' (
    ($wpTree.protection.hasPassword -eq $true) -and
    ($wpJson -notmatch 'secret') -and
    ($wpXml -match 'writeProtection') -and
    ($capWp -notmatch 'Repair') -and
    ($capWp -match 'docx-create-round5-wp')
) "hasPassword=$($wpTree.protection.hasPassword) reserved=$reserved ro=$ro cap=$capWp"
