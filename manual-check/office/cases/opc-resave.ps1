# Office 另存 create 产物：打开冒烟，不顶替已放弃的清单 O1。
# 零脏写回在 OPC 库测试里验。O1 记 SKIP，与三份清单的 🚫 一致。
#
# Word 对本目录做 SaveAs2 会挂住，所以放在最后一个用例，并套 60 秒超时。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Skip 'xlsx-O1' 'ooxp 没有「不改任何东西地保存」这个动作，CRC 不变这条验不了'
Add-Skip 'docx-O1' 'ooxp 没有「不改任何东西地保存」这个动作，CRC 不变这条验不了'
Add-Skip 'pptx-O1' 'ooxp 没有「不改任何东西地保存」这个动作，CRC 不变这条验不了'

# Excel 另存：读得动就说明 create 产物结构没问题。
$book = Ensure-XlsxCreate
$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$resaved = Get-OutPath 'xlsx-create-excel-resaved.xlsx'
if (Test-Path $resaved) { Remove-Item $resaved -Force }
$wb.SaveCopyAs($resaved)
$wb.Close($false)
Release-Com $wb
Add-Check 'xlsx-O1-excel-resave' (Test-Path $resaved) $resaved

$deck = Ensure-PptxCreate
$ppt = Ensure-MsPpt $deck
$pres = Open-PptPres $ppt $deck
$pptResaved = Get-OutPath 'pptx-create-ppt-resaved.pptx'
if (Test-Path $pptResaved) { Remove-Item $pptResaved -Force }
$pres.SaveCopyAs($pptResaved)
Close-PptPres $pres
Add-Check 'pptx-O1-ppt-resave' (Test-Path $pptResaved) $pptResaved

# Word 另存：已知会挂，用后台任务隔离，超时就记 SKIP，别拖死整轮。
$docxCreate = Ensure-DocxCreate
$docxResave = Get-OutPath 'docx-create-word-resaved.docx'
if (Test-Path $docxResave) { Remove-Item $docxResave -Force }
$libDir = [IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib')

Write-Log "Word SaveAs2 起后台任务，最多等 60 秒"
$job = Start-Job -ScriptBlock {
    param($libDir, $docPath, $outPath)
    $ErrorActionPreference = 'Stop'
    . (Join-Path $libDir 'bootstrap.ps1')
    Initialize-Suite -Name 'office'
    $word = Ensure-MsWord $docPath
    $doc = Open-WordDoc $word $docPath
    $doc.SaveAs2($outPath, 12)
    $doc.Close($false)
    return 'saved'
} -ArgumentList $libDir, $docxCreate, $docxResave

if (Wait-Job $job -Timeout 60) {
    $out = Receive-Job $job -ErrorAction SilentlyContinue
    $err = $job.ChildJobs[0].Error
    if ($err.Count) {
        Add-Check 'docx-O1-word-resave' $false ([string]$err[0])
    }
    else {
        Add-Check 'docx-O1-word-resave' (Test-Path $docxResave) "$out $docxResave"
    }
}
else {
    Stop-Job $job
    Add-Skip 'docx-O1-word-resave' 'Word SaveAs2 超过 60 秒没回，与 checklist-docx.md 记的挂起一致'
}
Remove-Job $job -Force
