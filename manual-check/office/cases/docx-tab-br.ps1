# 段落文本里的制表和换行写成 w:tab / w:br。Word 里仍能看见。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$path = Get-OutPath 'docx-tab-br.docx'
$text = "a" + [char]9 + "b" + "`n" + "c"
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $path.Replace('\', '/')
    document = @{ paragraphs = @(@{ text = $text }) }
} 'docx-tab-br-create' | Out-Null

$word = Ensure-MsWord $path
$doc = Open-WordDoc $word $path
$paras = $doc.Paragraphs
$p = $paras.Item(1)
$got = [string]$p.Range.Text
Release-Com $p
Release-Com $paras
$hasTab = $got.Contains([char]9)
$hasBreak = $got.Contains([char]11) -or $got.Contains("`n")
$ok = $hasTab -and $hasBreak -and $got.StartsWith('a') -and $got.Contains('b') -and $got.Contains('c')
$shown = $got.Replace([string][char]9, '<tab>').Replace([string][char]11, '<br>').Replace("`r", '<p>').Replace("`n", '<lf>')
Add-Check 'docx-tab-br' $ok "text=$shown"
$doc.Close($false)
Release-Com $doc
