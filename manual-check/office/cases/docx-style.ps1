# docx 样式：docs/checklist-docx.md 的 St1–St6。
# 用 ooxp 换段落样式、改 Normal / Heading1，再用 Word 打开核对无修复。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }
$S = $global:OoxpStrings

$src = Get-OutPath 'docx-style-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '正文段' }
            @{ style = 'Heading1'; text = '标题一段' }
            @{ text = '另一正文' }
        )
    }
} 'docx-style-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-style-inspect'
$treeList = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ styles = $true; from = 1; maxParas = 20 }
} 'docx-style-list'
$listed = @($treeList.styles)
$hasNormal = @($listed | Where-Object { [string]$_.id -eq 'style:Normal' }).Count -ge 1
$hasH2 = @($listed | Where-Object { [string]$_.id -eq 'style:Heading2' }).Count -ge 1
$st1ok = ($null -ne $tree0.styleCount) -and ([int]$tree0.styleCount -ge 4) -and
    ($null -eq $tree0.styles) -and $hasNormal -and $hasH2
Add-Check 'docx-St1' $st1ok "count=$($tree0.styleCount) list=$($listed.Count) normal=$hasNormal heading2=$hasH2"

$body = @($treeList.paragraphs)
$pBody = @($body | Where-Object { [string]$_.text -eq '正文段' }) | Select-Object -First 1
$pH1 = @($body | Where-Object { [string]$_.text -eq '标题一段' }) | Select-Object -First 1
$pOther = @($body | Where-Object { [string]$_.text -eq '另一正文' }) | Select-Object -First 1

$h2 = Get-OutPath 'docx-style-heading2.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $h2.Replace('\', '/')
    ops = @(@{ op = 'setParaStyle'; id = [string]$pBody.id; style = 'style:Heading2' })
} 'docx-style-heading2' | Out-Null

$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $h2.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-style-heading2-inspect'
$ph = @($treeH.paragraphs) | Where-Object { $_.id -eq $pBody.id } | Select-Object -First 1
$ol = @($treeH.outline) | Where-Object { $_.id -eq $pBody.id } | Select-Object -First 1
$st2ok = ([string]$ph.style -eq 'Heading2') -and ($null -ne $ol) -and ([int]$ol.level -eq 2)
Add-Check 'docx-St2' $st2ok "style=$($ph.style) outlineLevel=$($ol.level)"

$word = Ensure-MsWord $h2
$docH = Open-WordDoc $word $h2
$capH = [string]$docH.ActiveWindow.Caption
$styleName = [string]$docH.Paragraphs.Item(1).Style.NameLocal
Close-WordDoc $docH
$st3ok = ($capH -notmatch 'Repair') -and (
    ($styleName -eq 'Heading 2') -or ($styleName -eq $S.heading2) -or ($styleName -match '标题\s*2')
)
Add-Check 'docx-St3' $st3ok "caption=$capH style=$styleName"

$szOut = Get-OutPath 'docx-style-normal-sz.docx'
Close-WordDocByPath $word $szOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $szOut.Replace('\', '/')
    ops = @(@{ op = 'setStyle'; id = 'style:Normal'; sz = 48 })
} 'docx-style-normal-sz' | Out-Null
$docS = Open-WordDoc $word $szOut
$capS = [string]$docS.ActiveWindow.Caption
$sizeOther = [double]$docS.Paragraphs.Item(3).Range.Font.Size
Close-WordDoc $docS
$st4ok = ($capS -notmatch 'Repair') -and ([math]::Abs($sizeOther - 24) -lt 0.6)
Add-Check 'docx-St4' $st4ok "caption=$capS size=$sizeOther"

$boldOut = Get-OutPath 'docx-style-h1-bold.docx'
Close-WordDocByPath $word $boldOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $boldOut.Replace('\', '/')
    ops = @(@{ op = 'setStyle'; id = 'style:Heading1'; bold = $false })
} 'docx-style-h1-bold' | Out-Null
$docB = Open-WordDoc $word $boldOut
$capB = [string]$docB.ActiveWindow.Caption
$boldVal = [int]$docB.Paragraphs.Item(2).Range.Font.Bold
Close-WordDoc $docB
$st5ok = ($capB -notmatch 'Repair') -and ($boldVal -eq 0)
Add-Check 'docx-St5' $st5ok "caption=$capB bold=$boldVal"

$unb = Get-OutPath 'docx-style-unbind.docx'
Close-WordDocByPath $word $unb
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $h2.Replace('\', '/')
    output = $unb.Replace('\', '/')
    ops = @(@{ op = 'setParaStyle'; id = [string]$pBody.id; style = $null })
} 'docx-style-unbind' | Out-Null
$treeU = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $unb.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-style-unbind-inspect'
$pu = @($treeU.paragraphs) | Where-Object { $_.id -eq $pBody.id } | Select-Object -First 1
$docU = Open-WordDoc $word $unb
$capU = [string]$docU.ActiveWindow.Caption
Close-WordDoc $docU
$st6ok = ($capU -notmatch 'Repair') -and ($null -eq $pu.style)
Add-Check 'docx-St6' $st6ok "caption=$capU style=$($pu.style)"

# 避免未用变量告警
[void]$pH1
[void]$pOther
