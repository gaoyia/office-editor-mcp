# pptx run 级超链接：docs/checklist-pptx.md 的 Phr1–Phr5。
# 不算放映引擎。不写 a:hlinkHover。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.IO.Compression.FileSystem

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

function Get-ZipEntryText([string]$zipPath, [string]$entryName) {
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $e = $zip.GetEntry($entryName.Replace('\', '/'))
        if ($null -eq $e) { return $null }
        $sr = New-Object IO.StreamReader($e.Open(), [Text.Encoding]::UTF8)
        try { return $sr.ReadToEnd() }
        finally { $sr.Dispose() }
    }
    finally { $zip.Dispose() }
}

function Get-PptTextRangeHyperlinkAddress($shape) {
    $tf = $null
    $tr = $null
    $settings = $null
    $item = $null
    $hl = $null
    try {
        $tf = $shape.TextFrame
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
    }
}

function Get-PptShapeText($shape) {
    $tf = $null
    $tr = $null
    try {
        $tf = $shape.TextFrame
        $tr = $tf.TextRange
        return [string]$tr.Text
    }
    catch {
        return ''
    }
    finally {
        Release-Com $tr
        Release-Com $tf
    }
}

$hostPath = Get-OutPath 'pptx-run-links-host.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(
            @{ shapes = @(@{ text = 'Phr链' }, @{ text = '第二框' }) }
            @{ shapes = @(@{ text = '第二页' }) }
        )
    }
} 'pptx-phr-create' | Out-Null

$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $hostPath

$treeSh = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-phr-shapes'
$shapeId = [string]$treeSh.slides[0].shapes[0].id

$insOut = Get-OutPath 'pptx-run-links-insert.pptx'
Close-PptPresByPath $ppt $insOut
if (Test-Path $insOut) { Remove-Item -Force $insOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = $shapeId
        on = 'run'
        url = 'https://example.com/phr'
    })
} 'pptx-phr-insert' | Out-Null

$treeIns = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'pptx'
    scope = @{ hyperlinks = $true }
} 'pptx-phr-inspect-inserted'
$link0 = @($treeIns.slides[0].hyperlinks | Where-Object { $_ }) | Select-Object -First 1
$phr1ok = ([int]$treeIns.hyperlinkCount -eq 1) `
    -and ([string]$link0.on -eq 'run') `
    -and ([string]$link0.host -eq $shapeId) `
    -and ([string]$link0.text -eq 'Phr链')
Add-Check 'pptx-Phr1' $phr1ok "count=$($treeIns.hyperlinkCount) on=$($link0.on) host=$($link0.host) text=$($link0.text)"

$presI = Open-PptPres $ppt $insOut
$capI = Get-PptCaption $presI
$slideI = Get-PptSlide $presI 1
$shapeI = Get-PptShape $slideI 1
$addrI = Get-PptTextRangeHyperlinkAddress $shapeI
Release-Com $shapeI
Release-Com $slideI
Close-PptPres $presI
$xmlI = Get-ZipEntryText $insOut 'ppt/slides/slide1.xml'
$cnv = ''
if ($xmlI -match '(?s)<p:cNvPr\b[^>]*>.*?</p:cNvPr>') { $cnv = $Matches[0] }
elseif ($xmlI -match '<p:cNvPr\b[^/]*/>') { $cnv = $Matches[0] }
$cnvHas = $cnv -match 'hlinkClick'
$txHas = $xmlI -match '<a:hlinkClick\b'
$phr2ok = ($capI -notmatch 'Repair') -and $txHas -and (-not $cnvHas)
Add-Check 'pptx-Phr2' $phr2ok "caption=$capI addr=$addrI cnvHas=$cnvHas txHas=$txHas"

$lid = [string]$link0.id
$setOut = Get-OutPath 'pptx-run-links-set.pptx'
Close-PptPresByPath $ppt $setOut
if (Test-Path $setOut) { Remove-Item -Force $setOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $setOut.Replace('\', '/')
    ops = @(@{
        op = 'setHyperlink'
        id = $lid
        url = 'https://example.org/phr2'
    })
} 'pptx-phr-set' | Out-Null

$presS = Open-PptPres $ppt $setOut
$capS = Get-PptCaption $presS
$slideS = Get-PptSlide $presS 1
$shapeS = Get-PptShape $slideS 1
$addrS = Get-PptTextRangeHyperlinkAddress $shapeS
Release-Com $shapeS
Release-Com $slideS
Close-PptPres $presS
$phr3ok = ($capS -notmatch 'Repair') -and ($addrS -match 'example.org/phr2')
Add-Check 'pptx-Phr3' $phr3ok "caption=$capS addr=$addrS"

$delOut = Get-OutPath 'pptx-run-links-delete.pptx'
Close-PptPresByPath $ppt $delOut
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteHyperlink'; id = $lid })
} 'pptx-phr-delete' | Out-Null

$presD = Open-PptPres $ppt $delOut
$capD = Get-PptCaption $presD
$slideD = Get-PptSlide $presD 1
$shapeD = Get-PptShape $slideD 1
$addrD = Get-PptTextRangeHyperlinkAddress $shapeD
$textD = Get-PptShapeText $shapeD
Release-Com $shapeD
Release-Com $slideD
Close-PptPres $presD
$xmlD = Get-ZipEntryText $delOut 'ppt/slides/slide1.xml'
$phr4ok = ($capD -notmatch 'Repair') -and ($xmlD -notmatch 'hlinkClick') -and ($textD -match 'Phr')
Add-Check 'pptx-Phr4' $phr4ok "caption=$capD addr=$addrD text=$textD hasClick=$($xmlD -match 'hlinkClick')"

$badOnOut = Get-OutPath 'pptx-run-links-bad-on.pptx'
if (Test-Path $badOnOut) { Remove-Item -Force $badOnOut }
$failOn = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badOnOut.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = $shapeId
        on = 'other'
        url = 'https://example.com/'
    })
} 'pptx-phr-bad-on'
$codeOn = Get-PptErrorCode $failOn

$xlsxSrc = Get-OutPath 'pptx-run-links-xlsx.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsxSrc.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'pptx-phr-xlsx-create' | Out-Null
$xlsxOut = Get-OutPath 'pptx-run-links-xlsx-out.xlsx'
if (Test-Path $xlsxOut) { Remove-Item -Force $xlsxOut }
$failXlsx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsxSrc.Replace('\', '/')
    output = $xlsxOut.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = 'sheet:Sheet1'
        on = 'run'
        ref = 'A1'
        url = 'https://example.com/'
    })
} 'pptx-phr-xlsx'
$codeXlsx = Get-PptErrorCode $failXlsx

$badIdOut = Get-OutPath 'pptx-run-links-bad-id.pptx'
if (Test-Path $badIdOut) { Remove-Item -Force $badIdOut }
$failId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badIdOut.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = 'slide:1'
        on = 'run'
        url = 'https://example.com/'
    })
} 'pptx-phr-bad-id'
$codeId = Get-PptErrorCode $failId

$phr5ok = ($codeOn -eq 'InvalidField') -and (-not (Test-Path $badOnOut)) `
    -and ($codeXlsx -eq 'InvalidField') -and (-not (Test-Path $xlsxOut)) `
    -and ($codeId -eq 'InvalidId') -and (-not (Test-Path $badIdOut))
Add-Check 'pptx-Phr5' $phr5ok "on=$codeOn xlsx=$codeXlsx id=$codeId"
