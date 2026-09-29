# pptx 字段 a:fld：docs/checklist-pptx.md 的 Pf1–Pf5。
# 不算放映引擎。不刷新缓存。不要 COM 打开带 modifyVerifier 的文件。

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

$hostPath = Get-OutPath 'pptx-fields-host.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(
            @{ shapes = @(@{ text = '页1' }, @{ text = '第二框' }) }
            @{ shapes = @(@{ text = '第二页' }) }
        )
    }
} 'pptx-pf-create' | Out-Null

$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $hostPath

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
} 'pptx-pf-inspect'
$noList = -not ($tree0.slides[0].PSObject.Properties.Name -contains 'fields')
$hasCount = ($tree0.PSObject.Properties.Name -contains 'fieldCount') -and ([int]$tree0.fieldCount -eq 0)
$slideCount = ($tree0.slides[0].PSObject.Properties.Name -contains 'fieldCount') -and ([int]$tree0.slides[0].fieldCount -eq 0)

$treeSh = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-pf-shapes'
$shapeId = [string]$treeSh.slides[0].shapes[0].id

$insOut = Get-OutPath 'pptx-fields-slidenum.pptx'
Close-PptPresByPath $ppt $insOut
if (Test-Path $insOut) { Remove-Item -Force $insOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{
        op = 'insertField'
        id = $shapeId
        type = 'slidenum'
        text = '1'
    })
} 'pptx-pf-insert-slidenum' | Out-Null

$treeIns = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'pptx'
    scope = @{ fields = $true }
} 'pptx-pf-inspect-inserted'
$listed = @($treeIns.slides[0].fields | Where-Object { $_ })
$insType = if ($listed.Count -gt 0) { [string]$listed[0].type } else { '' }
$insHost = if ($listed.Count -gt 0) { [string]$listed[0].host } else { '' }
$fid = if ($listed.Count -gt 0) { [string]$listed[0].id } else { '' }
$pf1ok = $hasCount -and $slideCount -and $noList -and ([int]$treeIns.fieldCount -eq 1) `
    -and ($listed.Count -eq 1) -and ($insType -eq 'slidenum') -and ($insHost -eq $shapeId)
Add-Check 'pptx-Pf1' $pf1ok "bare=$($tree0.fieldCount) noList=$noList count=$($treeIns.fieldCount) type=$insType host=$insHost"

$presI = Open-PptPres $ppt $insOut
$capI = Get-PptCaption $presI
Close-PptPres $presI
$xmlI = Get-ZipPartText $insOut 'ppt/slides/slide1.xml'
$pf2ok = ($capI -notmatch 'Repair') -and ($xmlI -match 'type="slidenum"') -and ($xmlI -match '<a:fld')
Add-Check 'pptx-Pf2' $pf2ok "caption=$capI slidenum=$($xmlI -match 'type=""slidenum""')"

$dtOut = Get-OutPath 'pptx-fields-datetime.pptx'
Close-PptPresByPath $ppt $dtOut
if (Test-Path $dtOut) { Remove-Item -Force $dtOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $dtOut.Replace('\', '/')
    ops = @(@{
        op = 'insertField'
        id = $shapeId
        type = 'datetime1'
        text = '2026/9/3'
    })
} 'pptx-pf-insert-datetime' | Out-Null

$presD = Open-PptPres $ppt $dtOut
$capDt = Get-PptCaption $presD
Close-PptPres $presD
$xmlDt = Get-ZipPartText $dtOut 'ppt/slides/slide1.xml'
$pf3ok = ($capDt -notmatch 'Repair') -and ($xmlDt -match 'type="datetime1"') -and ($xmlDt -notmatch '<a:pPr')
Add-Check 'pptx-Pf3' $pf3ok "caption=$capDt datetime1=$($xmlDt -match 'type=""datetime1""')"

$delOut = Get-OutPath 'pptx-fields-delete.pptx'
Close-PptPresByPath $ppt $delOut
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteField'; id = $fid; keep = 'result' })
} 'pptx-pf-delete' | Out-Null

$presDel = Open-PptPres $ppt $delOut
$capDel = Get-PptCaption $presDel
Close-PptPres $presDel
$xmlDel = Get-ZipPartText $delOut 'ppt/slides/slide1.xml'
$pf4ok = ($capDel -notmatch 'Repair') -and ($xmlDel -notmatch '<a:fld') -and ($xmlDel -match '<a:t>1</a:t>')
Add-Check 'pptx-Pf4' $pf4ok "caption=$capDel hasFld=$($xmlDel -match '<a:fld') cache=$($xmlDel -match '<a:t>1</a:t>')"

$badIdOut = Get-OutPath 'pptx-fields-bad-id.pptx'
if (Test-Path $badIdOut) { Remove-Item -Force $badIdOut }
$failId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badIdOut.Replace('\', '/')
    ops = @(@{ op = 'insertField'; id = 'slide:1'; type = 'slidenum' })
} 'pptx-pf-bad-id'
$codeId = Get-PptErrorCode $failId

$notesOut = Get-OutPath 'pptx-fields-notes.pptx'
if (Test-Path $notesOut) { Remove-Item -Force $notesOut }
$failNotes = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $notesOut.Replace('\', '/')
    ops = @(@{ op = 'insertField'; id = 'slide:1/notes/shape:2'; type = 'slidenum' })
} 'pptx-pf-notes'
$codeNotes = Get-PptErrorCode $failNotes

$badTypeOut = Get-OutPath 'pptx-fields-bad-type.pptx'
if (Test-Path $badTypeOut) { Remove-Item -Force $badTypeOut }
$failType = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badTypeOut.Replace('\', '/')
    ops = @(@{ op = 'insertField'; id = $shapeId; type = 'other' })
} 'pptx-pf-bad-type'
$codeType = Get-PptErrorCode $failType

$instrOut = Get-OutPath 'pptx-fields-instr.pptx'
if (Test-Path $instrOut) { Remove-Item -Force $instrOut }
$failInstr = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $instrOut.Replace('\', '/')
    ops = @(@{ op = 'insertField'; id = $shapeId; instr = ' PAGE ' })
} 'pptx-pf-instr'
$codeInstr = Get-PptErrorCode $failInstr

$xlsxSrc = Get-OutPath 'pptx-fields-xlsx.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsxSrc.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'pptx-pf-xlsx-create' | Out-Null
$xlsxOut = Get-OutPath 'pptx-fields-xlsx-out.xlsx'
if (Test-Path $xlsxOut) { Remove-Item -Force $xlsxOut }
$failXlsx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsxSrc.Replace('\', '/')
    output = $xlsxOut.Replace('\', '/')
    ops = @(@{ op = 'insertField'; id = $shapeId; type = 'slidenum' })
} 'pptx-pf-xlsx'
$codeXlsx = Get-PptErrorCode $failXlsx

$pf5ok = ($codeId -eq 'InvalidId') -and (-not (Test-Path $badIdOut)) `
    -and ($codeNotes -eq 'InvalidId') -and (-not (Test-Path $notesOut)) `
    -and ($codeType -eq 'InvalidField') -and (-not (Test-Path $badTypeOut)) `
    -and ($codeInstr -eq 'InvalidField') -and (-not (Test-Path $instrOut)) `
    -and ($codeXlsx -eq 'InvalidField') -and (-not (Test-Path $xlsxOut))
Add-Check 'pptx-Pf5' $pf5ok "id=$codeId notes=$codeNotes type=$codeType instr=$codeInstr xlsx=$codeXlsx"
