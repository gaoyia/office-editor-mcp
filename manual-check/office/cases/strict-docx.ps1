# Word 16.0 另存 Strict Open XML。inspect 可读，apply 拒绝且不改文件。

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

function Read-ZipText([string]$zipPath, [string]$entryName) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $entry = $zip.GetEntry($entryName)
        if (-not $entry) { return '' }
        $reader = New-Object IO.StreamReader($entry.Open())
        try { return $reader.ReadToEnd() }
        finally { $reader.Dispose() }
    }
    finally { $zip.Dispose() }
}

$word = Ensure-MsWord (Ensure-DocxCreate)
$src = Get-OutPath 'strict-docx.docx'
$tmpDir = Join-Path $env:TEMP 'ooxp-office'
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
$tmp = Join-Path $tmpDir 'strict-docx.docx'
if (Test-Path $tmp) { Remove-Item -Force $tmp }
if (Test-Path $src) { Remove-Item -Force $src }

$doc = $word.Documents.Add()
$paragraph = $doc.Paragraphs.Item(1)
$range = $paragraph.Range
$range.Text = 'StrictHost'
Release-Com $range
Release-Com $paragraph
# 24 = wdFormatStrictOpenXMLDocument
$doc.SaveAs($tmp, 24)
Close-WordDoc $doc
Copy-Item -Force $tmp $src

$rels = Read-ZipText $src '_rels/.rels'
$isStrict = $rels.Contains('http://purl.oclc.org/ooxml/')
$tree = Invoke-OoxpTree @{
    schemaVersion = 1
    action = 'inspect'
    path = $src.Replace('\', '/')
} 'strict-docx-inspect'
$word = Ensure-MsWord (Ensure-DocxCreate)
Close-WordDocByPath $word $src
$doc = Open-WordDoc $word $src
$caption = [string]$doc.ActiveWindow.Caption
Close-WordDoc $doc
Add-Check 'strict-docx-inspect' ($isStrict -and ($null -ne $tree) -and ($caption -notmatch 'Repair')) "strict=$isStrict caption=$caption"

$before = (Get-FileHash -Algorithm SHA256 $src).Hash
$out = Get-OutPath 'strict-docx-out.docx'
if (Test-Path $out) { Remove-Item -Force $out }
$failed = Invoke-OoxpExpectFail @{
    schemaVersion = 1
    action = 'apply'
    path = $src.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(@{ op = 'setParaText'; id = 'para:index:1'; text = 'Z' })
} 'strict-docx-apply'
$after = (Get-FileHash -Algorithm SHA256 $src).Hash
$rejected = ($failed.ExitCode -eq 2) -and ($failed.Stderr -match 'NotImplemented') -and -not (Test-Path $out) -and ($before -eq $after)
Add-Check 'strict-docx-apply' $rejected "exit=$($failed.ExitCode) stderr=$($failed.Stderr)"
