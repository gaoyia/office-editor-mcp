# pptx 超链接：docs/checklist-pptx.md 的 Phl1–Phl5。
# 用 create 产物再 apply，不另做 PowerPoint 另存样例。

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

function Get-PptShapeHyperlinkAddress($shape) {
    $settings = $null
    $item = $null
    $hl = $null
    try {
        $settings = $shape.ActionSettings
        $item = $settings.Item(1)
        $hl = $item.Hyperlink
        return [string]$hl.Address
    } catch {
        return ''
    } finally {
        Release-Com $hl
        Release-Com $item
        Release-Com $settings
    }
}

function Get-PptShapeText($shape) {
    $tf = $null
    $tr = $null
    try {
        $tf = $shape.TextFrame
        $tr = $tf.TextRange
        return [string]$tr.Text
    } catch {
        return ''
    } finally {
        Release-Com $tr
        Release-Com $tf
    }
}

$hostPath = Get-OutPath 'pptx-hyperlinks-host.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(
            @{ shapes = @(@{ text = 'Phl链' }, @{ text = '第二框' }) }
            @{ shapes = @(@{ text = '第二页' }) }
        )
    }
} 'pptx-phl-create' | Out-Null

$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $hostPath

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
} 'pptx-phl-inspect'
$noList = -not ($tree0.slides[0].PSObject.Properties.Name -contains 'hyperlinks')
$hasCount = ($tree0.PSObject.Properties.Name -contains 'hyperlinkCount') -and ([int]$tree0.hyperlinkCount -eq 0)
$slideCount = ($tree0.slides[0].PSObject.Properties.Name -contains 'hyperlinkCount') -and ([int]$tree0.slides[0].hyperlinkCount -eq 0)
$treeList = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ hyperlinks = $true }
} 'pptx-phl-inspect-list'
$hasList = $treeList.slides[0].PSObject.Properties.Name -contains 'hyperlinks'
$listed = @($treeList.slides[0].hyperlinks | Where-Object { $_ })
$listEmpty = $hasList -and ($listed.Count -eq 0)
$phl1ok = $hasCount -and $slideCount -and $noList -and $listEmpty
Add-Check 'pptx-Phl1' $phl1ok "root=$($tree0.hyperlinkCount) slide=$($tree0.slides[0].hyperlinkCount) noList=$noList empty=$listEmpty hasList=$hasList"

$treeSh = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-phl-shapes'
$shapeId = [string]$treeSh.slides[0].shapes[0].id

$insOut = Get-OutPath 'pptx-hyperlinks-insert.pptx'
Close-PptPresByPath $ppt $insOut
if (Test-Path $insOut) { Remove-Item -Force $insOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = $shapeId
        url = 'https://example.com/phl'
    })
} 'pptx-phl-insert' | Out-Null

$presI = Open-PptPres $ppt $insOut
$capI = Get-PptCaption $presI
$slideI = Get-PptSlide $presI 1
$shapeI = Get-PptShape $slideI 1
$addrI = Get-PptShapeHyperlinkAddress $shapeI
Release-Com $shapeI
Release-Com $slideI
Close-PptPres $presI
$treeIns = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'pptx'
    scope = @{ hyperlinks = $true }
} 'pptx-phl-inspect-inserted'
$insKind = [string]((@($treeIns.slides[0].hyperlinks | Where-Object { $_ }) | Select-Object -First 1).kind)
$insUrl = [string]((@($treeIns.slides[0].hyperlinks | Where-Object { $_ }) | Select-Object -First 1).url)
$phl2ok = ($capI -notmatch 'Repair') -and ($addrI -match 'example.com/phl') -and ($insKind -eq 'url')
Add-Check 'pptx-Phl2' $phl2ok "caption=$capI addr=$addrI kind=$insKind inspect=$insUrl"

$lid = [string]((@($treeIns.slides[0].hyperlinks | Where-Object { $_ }) | Select-Object -First 1).id)
$setOut = Get-OutPath 'pptx-hyperlinks-set.pptx'
Close-PptPresByPath $ppt $setOut
if (Test-Path $setOut) { Remove-Item -Force $setOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $setOut.Replace('\', '/')
    ops = @(@{
        op = 'setHyperlink'
        id = $lid
        url = 'https://example.org/phl2'
    })
} 'pptx-phl-set' | Out-Null

$presS = Open-PptPres $ppt $setOut
$capS = Get-PptCaption $presS
$slideS = Get-PptSlide $presS 1
$shapeS = Get-PptShape $slideS 1
$addrS = Get-PptShapeHyperlinkAddress $shapeS
Release-Com $shapeS
Release-Com $slideS
Close-PptPres $presS
$phl3ok = ($capS -notmatch 'Repair') -and ($addrS -match 'example.org/phl2')
Add-Check 'pptx-Phl3' $phl3ok "caption=$capS addr=$addrS"

$delOut = Get-OutPath 'pptx-hyperlinks-delete.pptx'
Close-PptPresByPath $ppt $delOut
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteHyperlink'; id = $lid })
} 'pptx-phl-delete' | Out-Null

$presD = Open-PptPres $ppt $delOut
$capD = Get-PptCaption $presD
$slideD = Get-PptSlide $presD 1
$shapeD = Get-PptShape $slideD 1
$addrD = Get-PptShapeHyperlinkAddress $shapeD
$textD = Get-PptShapeText $shapeD
Release-Com $shapeD
Release-Com $slideD
Close-PptPres $presD
$phl4ok = ($capD -notmatch 'Repair') -and ($addrD -notmatch 'example.org') -and ($textD -match 'Phl')
Add-Check 'pptx-Phl4' $phl4ok "caption=$capD addr=$addrD text=$textD"

$badIdOut = Get-OutPath 'pptx-hyperlinks-bad-id.pptx'
if (Test-Path $badIdOut) { Remove-Item -Force $badIdOut }
$failId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badIdOut.Replace('\', '/')
    ops = @(@{ op = 'insertHyperlink'; id = 'slide:1'; url = 'https://example.com/' })
} 'pptx-phl-bad-id'
$codeId = Get-PptErrorCode $failId

$missOut = Get-OutPath 'pptx-hyperlinks-miss.pptx'
if (Test-Path $missOut) { Remove-Item -Force $missOut }
$failMiss = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $missOut.Replace('\', '/')
    ops = @(@{ op = 'insertHyperlink'; id = $shapeId })
} 'pptx-phl-miss'
$codeMiss = Get-PptErrorCode $failMiss
$phl5ok = ($codeId -eq 'InvalidId') -and (-not (Test-Path $badIdOut)) `
    -and ($codeMiss -eq 'InvalidField') -and (-not (Test-Path $missOut))
Add-Check 'pptx-Phl5' $phl5ok "id=$codeId miss=$codeMiss wroteId=$(Test-Path $badIdOut) wroteMiss=$(Test-Path $missOut)"
