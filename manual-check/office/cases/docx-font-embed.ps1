# docx 字体嵌入：docs/checklist-docx.md 的 Fe1–Fe6。
# 用 ooxp 插入混淆后的 TrueType 字体，再用 Word 打开核对无修复，最后删光。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$fontFile = 'C:\Windows\Fonts\calibri.ttf'
if (-not (Test-Path $fontFile)) { throw "本机应有 $fontFile" }

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

function Get-ZipEntryBytes([string]$docx, [string]$pattern) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::OpenRead($docx)
    try {
        $entry = $z.Entries | Where-Object { $_.FullName -replace '\\','/' -like $pattern } | Select-Object -First 1
        if (-not $entry) { return $null }
        $ms = New-Object IO.MemoryStream
        $es = $entry.Open()
        try { $es.CopyTo($ms) } finally { $es.Dispose() }
        return $ms.ToArray()
    }
    finally { $z.Dispose() }
}

$src = Get-OutPath 'docx-font-embed-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = 'hello' }
        )
    }
} 'docx-font-embed-create' | Out-Null

$bare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-font-embed-inspect-bare'
$bareHasFonts = $null -ne $bare.embeddedFonts

$full = Get-OutPath 'docx-font-embed-insert.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $full.Replace('\', '/')
    ops = @(
        @{
            op = 'insertEmbeddedFont'; id = 'docx:document'
            name = 'Calibri'; file = $fontFile.Replace('\', '/'); style = 'regular'
        }
    )
} 'docx-font-embed-insert' | Out-Null

$treeCap = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $full.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; embeddedFonts = $true }
} 'docx-font-embed-inspect-insert'
$fonts = @($treeCap.embeddedFonts)
$fhit = @($fonts | Where-Object { [string]$_.id -eq 'font:1' }) | Select-Object -First 1
Add-Check 'docx-Fe1' (
    ([int]$bare.embeddedFontCount -eq 0) -and (-not $bareHasFonts) -and
    ($null -ne $bare.fontEmbed) -and ([bool]$bare.fontEmbed.trueType -eq $false) -and
    ([int]$treeCap.embeddedFontCount -eq 1) -and
    ($null -ne $fhit) -and ([string]$fhit.name -eq 'Calibri') -and
    ([string]$fhit.style -eq 'regular') -and ([bool]$fhit.subsetted -eq $false) -and
    ([bool]$treeCap.fontEmbed.trueType -eq $true)
) "bare=$($bare.embeddedFontCount) count=$($treeCap.embeddedFontCount) name=$($fhit.name) style=$($fhit.style) tt=$($treeCap.fontEmbed.trueType)"

$word = Ensure-MsWord $src
$docI = Open-WordDoc $word $full
$capI = [string]$docI.ActiveWindow.Caption
Add-Check 'docx-Fe2' ($capI -notmatch 'Repair') $capI
Close-WordDoc $docI

$expOut = Get-OutPath 'docx-font-embed-export.odttf'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $full.Replace('\', '/')
    id = 'font:1'
    output = $expOut.Replace('\', '/')
} 'docx-font-embed-export' | Out-Null
$zipBytes = Get-ZipEntryBytes $full 'word/fonts/font1.odttf'
$exported = if (Test-Path $expOut) { [IO.File]::ReadAllBytes($expOut) } else { @() }
$same = ($null -ne $zipBytes) -and ($exported.Length -gt 0) -and ($zipBytes.Length -eq $exported.Length) -and
    [System.Linq.Enumerable]::SequenceEqual([byte[]]$zipBytes, [byte[]]$exported)
Add-Check 'docx-Fe3' $same "zip=$($zipBytes.Length) export=$($exported.Length)"

$flagged = Get-OutPath 'docx-font-embed-flag.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $full.Replace('\', '/')
    output = $flagged.Replace('\', '/')
    ops = @(@{ op = 'setFontEmbed'; id = 'docx:document'; subset = $true })
} 'docx-font-embed-flag' | Out-Null
$treeF = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $flagged.Replace('\', '/'); format = 'docx'
} 'docx-font-embed-inspect-flag'
$docF = Open-WordDoc $word $flagged
$capF = [string]$docF.ActiveWindow.Caption
Close-WordDoc $docF
Add-Check 'docx-Fe5' (
    ([bool]$treeF.fontEmbed.subset -eq $true) -and ($capF -notmatch 'Repair')
) "subset=$($treeF.fontEmbed.subset) caption=$capF"

$del = Get-OutPath 'docx-font-embed-delete.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $flagged.Replace('\', '/')
    output = $del.Replace('\', '/')
    ops = @(@{ op = 'deleteEmbeddedFont'; id = 'font:1' })
} 'docx-font-embed-delete' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $del.Replace('\', '/'); format = 'docx'
    scope = @{ embeddedFonts = $true }
} 'docx-font-embed-inspect-delete'
$docD = Open-WordDoc $word $del
$capD = [string]$docD.ActiveWindow.Caption
Close-WordDoc $docD
Add-Check 'docx-Fe4' (
    ([int]$treeD.embeddedFontCount -eq 0) -and
    (-not (Test-ZipHas $del 'word/fonts/font1.odttf')) -and
    ($capD -notmatch 'Repair')
) "count=$($treeD.embeddedFontCount) zipFont=$(Test-ZipHas $del 'word/fonts/font1.odttf') caption=$capD"

$xlsx = Get-OutPath 'docx-font-embed-xlsx.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-font-embed-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-font-embed-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{
        op = 'insertEmbeddedFont'; id = 'docx:document'
        name = 'Calibri'; file = $fontFile.Replace('\', '/')
    })
} 'docx-font-embed-fail-xlsx'
try { $codeX = ($rawX.Stderr | ConvertFrom-Json).error.code } catch { $codeX = $null }
Add-Check 'docx-Fe6' (
    ($rawX.ExitCode -ne 0) -and ($codeX -eq 'InvalidField') -and (-not (Test-Path $failX))
) "xlsx=$codeX wroteX=$([bool](Test-Path $failX))"
