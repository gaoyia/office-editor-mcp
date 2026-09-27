# docx 编号列表：docs/checklist-docx.md 的 Nl1–Nl6。
# 用 ooxp 写入 numbering.xml 并绑段落，再用 Word 打开核对无修复。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'docx-numbering-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '第一项' }
            @{ text = '第二项' }
            @{ text = '第三项' }
            @{ text = '不绑' }
        )
    }
} 'docx-numbering-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-numbering-inspect'
$nl1ok = ($null -ne $tree0.numberingCount) -and ([int]$tree0.numberingCount -eq 0) -and
    ($null -eq $tree0.numberings)
Add-Check 'docx-Nl1' $nl1ok "count=$($tree0.numberingCount) list=$($null -ne $tree0.numberings)"

$ins = Get-OutPath 'docx-numbering-insert.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $ins.Replace('\', '/')
    ops = @(@{ op = 'insertNumbering'; id = 'docx:document'; kind = 'decimal' })
} 'docx-numbering-insert' | Out-Null

$word = Ensure-MsWord $ins
$docI = Open-WordDoc $word $ins
$capI = [string]$docI.ActiveWindow.Caption
Close-WordDoc $docI
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ins.Replace('\', '/'); format = 'docx'
    scope = @{ numbering = $true; from = 1; maxParas = 20 }
} 'docx-numbering-insert-inspect'
$nums = @($treeI.numberings)
$num = $nums | Select-Object -First 1
$nl3ok = ($capI -notmatch 'Repair') -and ([int]$treeI.numberingCount -ge 1) -and
    ($null -ne $num) -and ($null -ne $num.levels) -and (@($num.levels).Count -ge 1)
Add-Check 'docx-Nl3' $nl3ok "caption=$capI count=$($treeI.numberingCount) id=$($num.id)"

$body = @($treeI.paragraphs)
$p1 = @($body | Where-Object { [string]$_.text -eq '第一项' }) | Select-Object -First 1
$p2 = @($body | Where-Object { [string]$_.text -eq '第二项' }) | Select-Object -First 1
$p3 = @($body | Where-Object { [string]$_.text -eq '第三项' }) | Select-Object -First 1

$bound = Get-OutPath 'docx-numbering-bind.docx'
Close-WordDocByPath $word $bound
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $ins.Replace('\', '/')
    output = $bound.Replace('\', '/')
    ops = @(
        @{ op = 'setParaNumbering'; id = [string]$p1.id; num = [string]$num.id; ilvl = 0 }
        @{ op = 'setParaNumbering'; id = [string]$p2.id; num = [string]$num.id; ilvl = 0 }
    )
} 'docx-numbering-bind' | Out-Null

$treeB = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $bound.Replace('\', '/'); format = 'docx'
    scope = @{ numbering = $true; from = 1; maxParas = 20 }
} 'docx-numbering-bind-inspect'
$b1 = @($treeB.paragraphs) | Where-Object { $_.id -eq $p1.id } | Select-Object -First 1
$b2 = @($treeB.paragraphs) | Where-Object { $_.id -eq $p2.id } | Select-Object -First 1
$nl2ok = ([string]$b1.num -eq [string]$num.id) -and ($null -ne $b1.ilvl) -and
    ([string]$b2.num -eq [string]$num.id)
Add-Check 'docx-Nl2' $nl2ok "p1=$($b1.num)/$($b1.ilvl) p2=$($b2.num)"

$docB = Open-WordDoc $word $bound
$capB = [string]$docB.ActiveWindow.Caption
$r1 = $docB.Paragraphs.Item(1).Range
$lf1 = $r1.ListFormat
$listType = [int]$lf1.ListType
$wdListNoNumbering = 0
Release-Com $lf1
Release-Com $r1
$nl4ok = ($capB -notmatch 'Repair') -and ($listType -ne $wdListNoNumbering)
Add-Check 'docx-Nl4' $nl4ok "caption=$capB listType=$listType"
Close-WordDoc $docB

$absId = [string]$num.abstractId
$set = Get-OutPath 'docx-numbering-set.docx'
Close-WordDocByPath $word $set
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $bound.Replace('\', '/')
    output = $set.Replace('\', '/')
    ops = @(@{
        op = 'setNumbering'
        id = $absId
        ilvl = 0
        numFmt = 'bullet'
        lvlText = [string]([char]0xF0B7)
    })
} 'docx-numbering-set' | Out-Null
$docS = Open-WordDoc $word $set
$capS = [string]$docS.ActiveWindow.Caption
Close-WordDoc $docS
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $set.Replace('\', '/'); format = 'docx'
    scope = @{ numbering = $true }
} 'docx-numbering-set-inspect'
$hit = @($treeS.numberings) | Where-Object { $_.id -eq $num.id } | Select-Object -First 1
$lvl0 = @($hit.levels) | Where-Object { [int]$_.ilvl -eq 0 } | Select-Object -First 1
$nl5ok = ($capS -notmatch 'Repair') -and ([string]$lvl0.numFmt -eq 'bullet')
Add-Check 'docx-Nl5' $nl5ok "caption=$capS numFmt=$($lvl0.numFmt)"

$unb = Get-OutPath 'docx-numbering-unbind.docx'
Close-WordDocByPath $word $unb
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $bound.Replace('\', '/')
    output = $unb.Replace('\', '/')
    ops = @(@{ op = 'setParaNumbering'; id = [string]$p1.id; num = $null })
} 'docx-numbering-unbind' | Out-Null
$docU = Open-WordDoc $word $unb
$capU = [string]$docU.ActiveWindow.Caption
$rU = $docU.Paragraphs.Item(1).Range
$lfU = $rU.ListFormat
$listTypeU = [int]$lfU.ListType
Release-Com $lfU
Release-Com $rU
Close-WordDoc $docU
$treeU = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $unb.Replace('\', '/'); format = 'docx'
    scope = @{ numbering = $true; from = 1; maxParas = 20 }
} 'docx-numbering-unbind-inspect'
$u1 = @($treeU.paragraphs) | Where-Object { $_.id -eq $p1.id } | Select-Object -First 1
$nl6ok = ($capU -notmatch 'Repair') -and ($null -eq $u1.num) -and ($listTypeU -eq $wdListNoNumbering)
Add-Check 'docx-Nl6' $nl6ok "caption=$capU num=$($u1.num) listType=$listTypeU"
