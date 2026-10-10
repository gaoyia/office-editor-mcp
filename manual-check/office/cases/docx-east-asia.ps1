# docx 东亚排版：docs/checklist-docx.md 的 Ea1–Ea8。
# 用 ooxp 写段禁则、强调点、两行合一、拼音与整篇压缩，再用 Word 打开核对无修复。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings

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

$src = Get-OutPath 'docx-east-asia-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = $S.eaPara }
        )
    }
} 'docx-east-asia-create' | Out-Null

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true }
} 'docx-east-asia-inspect-src'
$paraId = [string]$srcTree.paragraphs[0].id
$runId = [string]$srcTree.paragraphs[0].runs[0].id

$paraOut = Get-OutPath 'docx-east-asia-para.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $paraOut.Replace('\', '/')
    ops = @(@{
        op = 'setParaFormat'; id = $paraId
        kinsoku = $true; overflowPunct = $true; autoSpaceDE = $false
    })
} 'docx-east-asia-para' | Out-Null

$treeP = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $paraOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-east-asia-inspect-para'
$ea = $treeP.paragraphs[0].eastAsia
Add-Check 'docx-Ea1' (
    ($ea.kinsoku -eq $true) -and
    ($ea.overflowPunct -eq $true) -and
    ($null -eq $ea.topLinePunct) -and
    ($ea.autoSpaceDE -eq $false) -and
    ($null -eq $ea.autoSpaceDN)
) "kinsoku=$($ea.kinsoku) overflow=$($ea.overflowPunct) top=$($ea.topLinePunct) de=$($ea.autoSpaceDE) dn=$($ea.autoSpaceDN)"

$word = Ensure-MsWord $src
$docP = Open-WordDoc $word $paraOut
$capP = [string]$docP.ActiveWindow.Caption
$fe = $null
$hp = $null
$de = $null
try {
    $fmt = $docP.Paragraphs.Item(1).Range.ParagraphFormat
    $fe = [int]$fmt.FarEastLineBreakControl
    $hp = [int]$fmt.HangingPunctuation
    $de = [int]$fmt.AddSpaceBetweenFarEastAndAlpha
} catch {
    $fe = 'err'
    $hp = $_.Exception.Message
}
Close-WordDoc $docP
Add-Check 'docx-Ea2' (
    ($capP -notmatch 'Repair') -and
    ($fe -ne 'err') -and ($fe -ne 0) -and ($hp -ne 0) -and ($de -eq 0)
) "caption=$capP FarEast=$fe Hang=$hp AutoSpaceDE=$de"

$runOut = Get-OutPath 'docx-east-asia-run.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $runOut.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $runId
        em = 'dot'
        eastAsianLayout = @{ combine = $true; combineBrackets = 'curly' }
    })
} 'docx-east-asia-run' | Out-Null

$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $runOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true }
} 'docx-east-asia-inspect-run'
$r0 = $treeR.paragraphs[0].runs[0]
$docR = Open-WordDoc $word $runOut
$capR = [string]$docR.ActiveWindow.Caption
$emMark = $null
try {
    $emMark = [int]$docR.Paragraphs.Item(1).Range.Characters.Item(1).Font.EmphasisMark
} catch {
    $emMark = 'err'
}
Close-WordDoc $docR
Add-Check 'docx-Ea3' (
    ([string]$r0.em -eq 'dot') -and
    ($capR -notmatch 'Repair') -and
    ($emMark -ne 'err') -and ($emMark -ne 0)
) "em=$($r0.em) caption=$capR EmphasisMark=$emMark"
Add-Check 'docx-Ea4' (
    ($r0.eastAsianLayout.combine -eq $true) -and
    ([string]$r0.eastAsianLayout.combineBrackets -eq 'curly') -and
    ($capR -notmatch 'Repair')
) "combine=$($r0.eastAsianLayout.combine) brackets=$($r0.eastAsianLayout.combineBrackets) caption=$capR"

$rubyOut = Get-OutPath 'docx-east-asia-ruby.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $rubyOut.Replace('\', '/')
    ops = @(@{
        op = 'insertRuby'; id = $paraId
        base = [string]$S.eaRubyBase
        rt = [string]$S.eaRubyRt
    })
} 'docx-east-asia-ruby' | Out-Null

$treeRuby = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $rubyOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; rubies = $true }
} 'docx-east-asia-inspect-ruby'
$rb = $treeRuby.rubies[0]
$docRuby = Open-WordDoc $word $rubyOut
$capRuby = [string]$docRuby.ActiveWindow.Caption
Close-WordDoc $docRuby
Add-Check 'docx-Ea5' (
    ([int]$treeRuby.rubyCount -eq 1) -and
    ([string]$rb.id -eq 'ruby:1') -and
    ([string]$rb.base -eq [string]$S.eaRubyBase) -and
    ([string]$rb.rt -eq [string]$S.eaRubyRt) -and
    ($capRuby -notmatch 'Repair')
) "count=$($treeRuby.rubyCount) id=$($rb.id) base=$($rb.base) rt=$($rb.rt) caption=$capRuby"

$rubySet = Get-OutPath 'docx-east-asia-ruby-set.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $rubyOut.Replace('\', '/')
    output = $rubySet.Replace('\', '/')
    ops = @(@{ op = 'setRuby'; id = 'ruby:1'; rt = [string]$S.eaRubyRtNew })
} 'docx-east-asia-ruby-set' | Out-Null

$treeSet = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $rubySet.Replace('\', '/'); format = 'docx'
    scope = @{ rubies = $true }
} 'docx-east-asia-inspect-ruby-set'
$docSet = Open-WordDoc $word $rubySet
$capSet = [string]$docSet.ActiveWindow.Caption
Close-WordDoc $docSet
Add-Check 'docx-Ea6' (
    ([string]$treeSet.rubies[0].rt -eq [string]$S.eaRubyRtNew) -and
    ([string]$treeSet.rubies[0].base -eq [string]$S.eaRubyBase) -and
    ($capSet -notmatch 'Repair')
) "rt=$($treeSet.rubies[0].rt) base=$($treeSet.rubies[0].base) caption=$capSet"

$setOut = Get-OutPath 'docx-east-asia-settings.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $setOut.Replace('\', '/')
    ops = @(@{
        op = 'setEastAsianSettings'; id = 'docx:document'
        characterSpacingControl = 'compressPunctuation'
        strictFirstAndLastChars = $false
    })
} 'docx-east-asia-settings' | Out-Null

$treeSetg = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $setOut.Replace('\', '/'); format = 'docx'
} 'docx-east-asia-inspect-settings'
$docSetg = Open-WordDoc $word $setOut
$capSetg = [string]$docSetg.ActiveWindow.Caption
Close-WordDoc $docSetg
Add-Check 'docx-Ea7' (
    ([string]$treeSetg.eastAsianSettings.characterSpacingControl -eq 'compressPunctuation') -and
    ($treeSetg.eastAsianSettings.strictFirstAndLastChars -eq $false) -and
    ($capSetg -notmatch 'Repair')
) "csc=$($treeSetg.eastAsianSettings.characterSpacingControl) strict=$($treeSetg.eastAsianSettings.strictFirstAndLastChars) caption=$capSetg"

$xlsx = Get-OutPath 'docx-east-asia.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-east-asia-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-east-asia-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{
        op = 'insertRuby'; id = $paraId
        base = [string]$S.eaRubyBase; rt = [string]$S.eaRubyRt
    })
} 'docx-east-asia-fail-xlsx'
try { $codeX = ($rawX.Stderr | ConvertFrom-Json).error.code } catch { $codeX = $null }

$failEmpty = Get-OutPath 'docx-east-asia-fail-empty.docx'
$rawE = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failEmpty.Replace('\', '/')
    ops = @(@{
        op = 'insertRuby'; id = $paraId
        base = ''; rt = [string]$S.eaRubyRt
    })
} 'docx-east-asia-fail-empty'
try { $codeE = ($rawE.Stderr | ConvertFrom-Json).error.code } catch { $codeE = $null }

Add-Check 'docx-Ea8' (
    ($rawX.ExitCode -ne 0) -and ($codeX -eq 'InvalidField') -and (-not (Test-Path $failX)) -and
    ($rawE.ExitCode -ne 0) -and ($codeE -eq 'InvalidField') -and (-not (Test-Path $failEmpty))
) "xlsx=$codeX wroteX=$([bool](Test-Path $failX)) empty=$codeE wroteE=$([bool](Test-Path $failEmpty))"
