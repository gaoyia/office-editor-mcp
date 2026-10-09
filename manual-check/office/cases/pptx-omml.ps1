# pptx 幻灯片公式框：docs/checklist-pptx.md 的 Pom1–Pom6。
# COM 多半是 Type=17（文本框）。探测脚本不进本文件。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }
Add-Type -AssemblyName System.IO.Compression.FileSystem

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

$src = Get-OutPath 'pptx-omml-src.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'pptx'
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-omml-create-src' | Out-Null

$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

$inserted = Get-OutPath 'pptx-omml-inserted.pptx'
Close-PptPresByPath $ppt $inserted
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertMath'; id = 'slide:1'; text = 'E=mc^2' })
} 'pptx-omml-insert' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'pptx'
    scope = @{ maths = $true; slide = 1 }
} 'pptx-omml-inspect-insert'
$item = @($tree1.slides[0].maths) | Select-Object -First 1
$mathInShapes = $false
foreach ($sh in @($tree1.slides[0].shapes)) {
    if ([string]$sh.id -like '*/math:*') { $mathInShapes = $true }
}
$xmlI = Get-ZipPartText $inserted 'ppt/slides/slide1.xml'
$hasA14 = ($xmlI -like '*a14:m*') -and ($xmlI -like '*Requires="a14"*')
$pres1 = Open-PptPres $ppt $inserted
$cap1 = Get-PptCaption $pres1
Add-Check 'pptx-Pom1' (
    ([string]$item.text -eq 'E=mc^2') -and (-not $mathInShapes) -and $hasA14 -and ($cap1 -notmatch 'Repair')
) "text=$($item.text) inShapes=$mathInShapes a14=$hasA14 caption=$cap1"
Close-PptPres $pres1

$replaced = Get-OutPath 'pptx-omml-replaced.pptx'
Close-PptPresByPath $ppt $replaced
if (Test-Path $replaced) { Remove-Item $replaced -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'setMath'; id = [string]$item.id; text = 'a^2+b^2=c^2' })
} 'pptx-omml-set' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'pptx'
    scope = @{ maths = $true }
} 'pptx-omml-inspect-set'
$itemR = @($treeR.slides[0].maths) | Select-Object -First 1
$presR = Open-PptPres $ppt $replaced
$capR = Get-PptCaption $presR
Add-Check 'pptx-Pom2' (
    ([string]$itemR.id -eq [string]$item.id) -and ([string]$itemR.text -eq 'a^2+b^2=c^2') -and ($capR -notmatch 'Repair')
) "id=$($itemR.id) text=$($itemR.text) caption=$capR"
Close-PptPres $presR

$created = Get-OutPath 'pptx-omml-created.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(@{
            shapes = @(@{ text = '标题' })
            maths = @(@{ text = 'E=mc^2' })
        })
    }
} 'pptx-omml-create' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/'); format = 'pptx'
    scope = @{ maths = $true; slide = 1 }
} 'pptx-omml-inspect-create'
$itemC = @($treeC.slides[0].maths) | Select-Object -First 1
$mathInShapesC = $false
foreach ($sh in @($treeC.slides[0].shapes)) {
    if ([string]$sh.id -like '*/math:*') { $mathInShapesC = $true }
}
Close-PptPresByPath $ppt $created
$presC = Open-PptPres $ppt $created
$capC = Get-PptCaption $presC
Add-Check 'pptx-Pom3' (
    ([string]$itemC.text -eq 'E=mc^2') -and (-not $mathInShapesC) -and ($capC -notmatch 'Repair')
) "text=$($itemC.text) inShapes=$mathInShapesC caption=$capC"
Close-PptPres $presC

$deleted = Get-OutPath 'pptx-omml-deleted.pptx'
Close-PptPresByPath $ppt $deleted
if (Test-Path $deleted) { Remove-Item $deleted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $replaced.Replace('\', '/')
    output = $deleted.Replace('\', '/')
    ops = @(@{ op = 'deleteMath'; id = [string]$item.id })
} 'pptx-omml-delete' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $deleted.Replace('\', '/'); format = 'pptx'
    scope = @{ maths = $true }
} 'pptx-omml-inspect-delete'
$presD = Open-PptPres $ppt $deleted
$capD = Get-PptCaption $presD
Add-Check 'pptx-Pom4' (
    ([int]$treeD.mathCount -eq 0) -and ($capD -notmatch 'Repair')
) "count=$($treeD.mathCount) caption=$capD"
Close-PptPres $presD

$notesOut = Get-OutPath 'pptx-omml-notes.pptx'
Close-PptPresByPath $ppt $notesOut
if (Test-Path $notesOut) { Remove-Item $notesOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $notesOut.Replace('\', '/')
    ops = @(@{ op = 'insertMath'; id = 'slide:1/notes'; text = 'E=mc^2' })
} 'pptx-omml-notes-insert' | Out-Null
$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $notesOut.Replace('\', '/'); format = 'pptx'
    scope = @{ maths = $true; notes = $true }
} 'pptx-omml-inspect-notes'
$nItem = @($treeN.slides[0].notes.maths) | Select-Object -First 1
$presN = Open-PptPres $ppt $notesOut
$capN = Get-PptCaption $presN
Add-Check 'pptx-Pom5' (
    ([string]$nItem.text -eq 'E=mc^2') -and ([int]$treeN.mathCount -eq 0) -and ($capN -notmatch 'Repair')
) "text=$($nItem.text) root=$($treeN.mathCount) caption=$capN"
Close-PptPres $presN

$failOut = Get-OutPath 'pptx-omml-should-not.pptx'
if (Test-Path $failOut) { Remove-Item $failOut -Force }
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertMath'
        id = 'slide:1'
        text = 'x'
        kind = 'inline'
    })
} 'pptx-omml-reject'
$failCode = $null
try { $failCode = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$empty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'insertMath'; id = 'slide:1'; text = '' })
} 'pptx-omml-empty'
$emptyCode = $null
try { $emptyCode = ($empty.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'pptx-Pom6' (
    ($fail.ExitCode -ne 0) -and ($failCode -eq 'InvalidField') -and (-not (Test-Path $failOut)) -and ($empty.ExitCode -ne 0) -and ($emptyCode -eq 'InvalidField')
) "kind=$failCode empty=$emptyCode exit=$($fail.ExitCode)/$($empty.ExitCode)"
