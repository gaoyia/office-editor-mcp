# docx 构建基块：docs/checklist-docx.md 的 Bb1–Bb6。
# 用 ooxp 插入 glossary 文档部件、复制进正文，再用 Word 打开核对无修复，最后删光。

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

function Test-ZipHas([string]$path, [string]$name) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::OpenRead($path)
    try {
        return [bool]($z.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $name })
    } finally { $z.Dispose() }
}

$src = Get-OutPath 'docx-glossary-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = 'hello' }
        )
        headers = @(@{ type = 'default'; paragraphs = @(@{ text = $S.headerSrc }) })
    }
} 'docx-glossary-create' | Out-Null

$bare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-glossary-inspect-bare'
$bareHasParts = $null -ne $bare.docParts

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; headerFooter = $true }
} 'docx-glossary-inspect-src'
$hfPara = [string]$srcTree.headers[0].paragraphs[0].id

$full = Get-OutPath 'docx-glossary-insert.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $full.Replace('\', '/')
    ops = @(
        @{
            op = 'insertDocPart'; id = 'docx:document'
            name = 'Clause1'; text = 'glossary clause'
        }
    )
} 'docx-glossary-insert' | Out-Null

$treeCap = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $full.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; docParts = $true }
} 'docx-glossary-inspect-insert'
$parts = @($treeCap.docParts)
$phit = @($parts | Where-Object { [string]$_.id -eq 'docPart:1' }) | Select-Object -First 1
Add-Check 'docx-Bb1' (
    ([int]$bare.docPartCount -eq 0) -and (-not $bareHasParts) -and
    ([int]$treeCap.docPartCount -eq 1) -and
    ($null -ne $phit) -and ([string]$phit.name -eq 'Clause1') -and
    ([string]$phit.gallery -eq 'custQuickParts')
) "bare=$($bare.docPartCount) count=$($treeCap.docPartCount) name=$($phit.name) gallery=$($phit.gallery)"

$word = Ensure-MsWord $src
$docI = Open-WordDoc $word $full
$capI = [string]$docI.ActiveWindow.Caption
Add-Check 'docx-Bb2' ($capI -notmatch 'Repair') $capI
Close-WordDoc $docI

$content = Get-OutPath 'docx-glossary-content.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $full.Replace('\', '/')
    output = $content.Replace('\', '/')
    ops = @(
        @{ op = 'insertDocPartContent'; id = 'docx:document'; docPart = 'docPart:1' }
    )
} 'docx-glossary-content' | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $content.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-glossary-inspect-content'
$texts = @($treeC.paragraphs | ForEach-Object { [string]$_.text })
$docC = Open-WordDoc $word $content
$capC = [string]$docC.ActiveWindow.Caption
$body = [string]$docC.Content.Text
Add-Check 'docx-Bb3' (
    ($texts -contains 'glossary clause') -and
    ($body -match 'glossary clause') -and
    ($capC -notmatch 'Repair')
) "texts=$($texts -join '|') bodyHas=$($body -match 'glossary clause') caption=$capC"
Close-WordDoc $docC

$gone = Get-OutPath 'docx-glossary-delete.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $content.Replace('\', '/')
    output = $gone.Replace('\', '/')
    ops = @(@{ op = 'deleteDocPart'; id = 'docPart:1' })
} 'docx-glossary-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $gone.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; docParts = $true }
} 'docx-glossary-inspect-delete'
$docD = Open-WordDoc $word $gone
$capD = [string]$docD.ActiveWindow.Caption
Add-Check 'docx-Bb4' (
    ([int]$treeD.docPartCount -eq 0) -and
    ($capD -notmatch 'Repair')
) "count=$($treeD.docPartCount) caption=$capD"
Close-WordDoc $docD

$hasGloss = Test-ZipHas $gone 'word/glossary/document.xml'
Add-Check 'docx-Bb5' (-not $hasGloss) "glossary=$hasGloss"

$failHf = Get-OutPath 'docx-glossary-fail-hf.docx'
$rawHf = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failHf.Replace('\', '/')
    ops = @(@{ op = 'insertDocPartContent'; id = $hfPara; docPart = 'docPart:1' })
} 'docx-glossary-fail-hf'
try { $codeHf = ($rawHf.Stderr | ConvertFrom-Json).error.code } catch { $codeHf = $null }

$xlsx = Get-OutPath 'docx-glossary-xlsx.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-glossary-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-glossary-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{
        op = 'insertDocPart'; id = 'docx:document'
        name = 'Clause1'; text = 'glossary clause'
    })
} 'docx-glossary-fail-xlsx'
try { $codeX = ($rawX.Stderr | ConvertFrom-Json).error.code } catch { $codeX = $null }

Add-Check 'docx-Bb6' (
    ($rawHf.ExitCode -ne 0) -and ($codeHf -eq 'InvalidId') -and (-not (Test-Path $failHf)) -and
    ($rawX.ExitCode -ne 0) -and ($codeX -eq 'InvalidField') -and (-not (Test-Path $failX))
) "hf=$codeHf wroteHf=$([bool](Test-Path $failHf)) xlsx=$codeX wroteX=$([bool](Test-Path $failX))"
