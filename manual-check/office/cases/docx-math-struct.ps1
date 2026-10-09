# docx 公式结构：docs/checklist-docx.md 的 Ms1–Ms8。
# 用 ooxp 按 expr 写 OMML，再用 Word 核对无修复、仍是公式区。不排版、不解析 TeX。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$naryChr = [string]([char]0x2211)

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

function Get-OMathInfo($doc) {
    $omaths = $null
    try {
        $omaths = $doc.OMaths
        $n = [int]$omaths.Count
        if ($n -lt 1) {
            return @{ Count = 0; Type = -1; Text = '' }
        }
        $om = $null
        try {
            $om = $omaths.Item(1)
            $typ = [int]$om.Type
            $rg = $null
            try {
                $rg = $om.Range
                $txt = [string]$rg.Text
            } finally { Release-Com $rg }
            return @{ Count = $n; Type = $typ; Text = $txt }
        } finally { Release-Com $om }
    } finally { Release-Com $omaths }
}

function Get-Expr0($tree) {
    @($tree.maths[0].expr)[0]
}

$fracExpr = @(
    @{
        type = 'frac'
        num = @(@{ type = 'text'; text = 'a' })
        den = @(@{ type = 'text'; text = 'b' })
    }
)

$src = Get-OutPath 'docx-math-struct-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = $S.hello }
        )
    }
} 'docx-math-struct-create' | Out-Null

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-math-struct-inspect-src'
$p1 = [string]$srcTree.paragraphs[0].id

$frac = Get-OutPath 'docx-math-struct-frac.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $frac.Replace('\', '/')
    ops = @(@{ op = 'insertMath'; id = $p1; kind = 'display'; expr = $fracExpr })
} 'docx-math-struct-insert-frac' | Out-Null

$treeFrac = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $frac.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; maths = $true }
} 'docx-math-struct-inspect-frac'
$eFrac = Get-Expr0 $treeFrac
$word = Ensure-MsWord $src
$docFrac = Open-WordDoc $word $frac
$capFrac = [string]$docFrac.ActiveWindow.Caption
$omFrac = Get-OMathInfo $docFrac
Add-Check 'docx-Ms1' (
    ([string]$treeFrac.maths[0].kind -eq 'display') -and
    ([string]$treeFrac.maths[0].text -eq 'ab') -and
    ([string]$eFrac.type -eq 'frac') -and
    ($capFrac -notmatch 'Repair') -and
    ([int]$omFrac.Count -ge 1)
) "kind=$($treeFrac.maths[0].kind) type=$($eFrac.type) text=$($treeFrac.maths[0].text) caption=$capFrac omaths=$($omFrac.Count)"
Close-WordDoc $docFrac

$rad = Get-OutPath 'docx-math-struct-rad.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $rad.Replace('\', '/')
    ops = @(@{
        op = 'insertMath'; id = $p1
        expr = @(@{
            type = 'rad'
            degHide = $true
            e = @(@{ type = 'text'; text = 'x' })
        })
    })
} 'docx-math-struct-insert-rad' | Out-Null

$treeRad = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $rad.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; maths = $true }
} 'docx-math-struct-inspect-rad'
$eRad = Get-Expr0 $treeRad
$docRad = Open-WordDoc $word $rad
$capRad = [string]$docRad.ActiveWindow.Caption
Add-Check 'docx-Ms2' (
    ([string]$eRad.type -eq 'rad') -and
    ([bool]$eRad.degHide -eq $true) -and
    ($capRad -notmatch 'Repair')
) "type=$($eRad.type) degHide=$($eRad.degHide) caption=$capRad"
Close-WordDoc $docRad

$nary = Get-OutPath 'docx-math-struct-nary.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $nary.Replace('\', '/')
    ops = @(@{
        op = 'insertMath'; id = $p1
        expr = @(@{
            type = 'nary'
            chr = $naryChr
            sub = @(@{ type = 'text'; text = 'i=1' })
            sup = @(@{ type = 'text'; text = 'n' })
            e = @(@{ type = 'text'; text = 'i' })
        })
    })
} 'docx-math-struct-insert-nary' | Out-Null

$treeNary = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nary.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; maths = $true }
} 'docx-math-struct-inspect-nary'
$eNary = Get-Expr0 $treeNary
$docNary = Open-WordDoc $word $nary
$capNary = [string]$docNary.ActiveWindow.Caption
Add-Check 'docx-Ms3' (
    ([string]$eNary.type -eq 'nary') -and
    ([string]$eNary.chr -eq $naryChr) -and
    ($capNary -notmatch 'Repair')
) "type=$($eNary.type) chr=$($eNary.chr) caption=$capNary"
Close-WordDoc $docNary

$nested = Get-OutPath 'docx-math-struct-nested.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $nested.Replace('\', '/')
    ops = @(@{
        op = 'insertMath'; id = $p1
        expr = @(@{
            type = 'sSup'
            e = @(@{
                type = 'delim'
                args = @(
                    , @(@{
                        type = 'frac'
                        num = @(@{ type = 'text'; text = 'a' })
                        den = @(@{ type = 'text'; text = 'b' })
                    })
                )
            })
            sup = @(@{ type = 'text'; text = '2' })
        })
    })
} 'docx-math-struct-insert-nested' | Out-Null

$treeNested = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nested.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; maths = $true }
} 'docx-math-struct-inspect-nested'
$eNested = Get-Expr0 $treeNested
$docNested = Open-WordDoc $word $nested
$capNested = [string]$docNested.ActiveWindow.Caption
Add-Check 'docx-Ms4' (
    ([string]$eNested.type -eq 'sSup') -and
    ([string]$treeNested.maths[0].text -eq 'ab2') -and
    ($capNested -notmatch 'Repair')
) "type=$($eNested.type) text=$($treeNested.maths[0].text) caption=$capNested"
Close-WordDoc $docNested

$linear = Get-OutPath 'docx-math-struct-linear.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $linear.Replace('\', '/')
    ops = @(@{ op = 'insertMath'; id = $p1; text = 'x' })
} 'docx-math-struct-insert-linear' | Out-Null

$setExpr = Get-OutPath 'docx-math-struct-set-expr.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $linear.Replace('\', '/')
    output = $setExpr.Replace('\', '/')
    ops = @(@{ op = 'setMath'; id = 'math:1'; expr = $fracExpr })
} 'docx-math-struct-set-expr' | Out-Null

$treeSet = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $setExpr.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; maths = $true }
} 'docx-math-struct-inspect-set-expr'
$eSet = Get-Expr0 $treeSet
$docSet = Open-WordDoc $word $setExpr
$capSet = [string]$docSet.ActiveWindow.Caption
Add-Check 'docx-Ms5' (
    ([string]$eSet.type -eq 'frac') -and
    ([string]$treeSet.maths[0].text -eq 'ab') -and
    ($capSet -notmatch 'Repair')
) "type=$($eSet.type) text=$($treeSet.maths[0].text) caption=$capSet"
Close-WordDoc $docSet

$flat = Get-OutPath 'docx-math-struct-flat.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $setExpr.Replace('\', '/')
    output = $flat.Replace('\', '/')
    ops = @(@{ op = 'setMath'; id = 'math:1'; text = 'z' })
} 'docx-math-struct-set-text' | Out-Null

$treeFlat = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $flat.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; maths = $true }
} 'docx-math-struct-inspect-flat'
$eFlat = Get-Expr0 $treeFlat
$docFlat = Open-WordDoc $word $flat
$capFlat = [string]$docFlat.ActiveWindow.Caption
$omFlat = Get-OMathInfo $docFlat
Add-Check 'docx-Ms6' (
    ([string]$treeFlat.maths[0].text -eq 'z') -and
    ([string]$eFlat.type -eq 'text') -and
    ([string]$eFlat.text -eq 'z') -and
    ($capFlat -notmatch 'Repair') -and
    ([int]$omFlat.Count -ge 1)
) "type=$($eFlat.type) text=$($treeFlat.maths[0].text) caption=$capFlat omaths=$($omFlat.Count)"
Close-WordDoc $docFlat

$failBoth = Get-OutPath 'docx-math-struct-both.docx'
if (Test-Path $failBoth) { Remove-Item -Force $failBoth }
$badBoth = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failBoth.Replace('\', '/')
    ops = @(@{ op = 'insertMath'; id = $p1; text = 'x'; expr = $fracExpr })
} 'docx-math-struct-both'
$codeBoth = $null
try { $codeBoth = ($badBoth.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'docx-Ms7' (
    ($badBoth.ExitCode -ne 0) -and
    ($codeBoth -eq 'InvalidField') -and
    (-not (Test-Path $failBoth))
) "code=$codeBoth wrote=$([bool](Test-Path $failBoth))"

$created = Get-OutPath 'docx-math-struct-create-expr.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = $S.hello; maths = @(@{ kind = 'inline'; expr = $fracExpr }) }
        )
    }
} 'docx-math-struct-create-expr' | Out-Null

$treeCreate = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; maths = $true }
} 'docx-math-struct-inspect-create'
$eCreate = Get-Expr0 $treeCreate
$docCreate = Open-WordDoc $word $created
$capCreate = [string]$docCreate.ActiveWindow.Caption
Add-Check 'docx-Ms8' (
    ([string]$eCreate.type -eq 'frac') -and
    ([string]$treeCreate.maths[0].text -eq 'ab') -and
    ($capCreate -notmatch 'Repair')
) "type=$($eCreate.type) text=$($treeCreate.maths[0].text) caption=$capCreate"
Close-WordDoc $docCreate
