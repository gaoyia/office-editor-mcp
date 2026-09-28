# pptx 自定义放映：docs/checklist-pptx.md 的 Cs1–Cs5。
# 不算放映引擎。COM 名单在 SlideShowSettings.NamedSlideShows。

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

function Get-ShowSnap($pres) {
    $sss = $pres.SlideShowSettings
    $nss = $sss.NamedSlideShows
    try {
        $n = [int]$nss.Count
        $rows = @()
        for ($i = 1; $i -le $n; $i++) {
            $item = $nss.Item($i)
            try {
                $rows += [pscustomobject]@{
                    Name  = [string]$item.Name
                    Count = [int]$item.Count
                }
            }
            finally { Release-Com $item }
        }
        return @{ Count = $n; Rows = $rows }
    }
    finally {
        Release-Com $nss
        Release-Com $sss
    }
}

function Test-SlideShowName($pres, [string]$name) {
    $sss = $pres.SlideShowSettings
    try {
        $sss.SlideShowName = $name
        return ([string]$sss.SlideShowName -eq $name)
    }
    catch {
        Write-Log "SlideShowName=$name 失败：$($_.Exception.Message)"
        return $false
    }
    finally { Release-Com $sss }
}

$tmpDir = Join-Path $env:TEMP 'ooxp-office'
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
$src3 = Join-Path $tmpDir 'pptx-custom-show-src.pptx'
$req = @{
    schemaVersion = 1; action = 'create'
    path = $src3.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(
            @{ shapes = @(@{ text = '页1' }) }
            @{ shapes = @(@{ text = '页2' }) }
            @{ shapes = @(@{ text = '页3' }) }
        )
    }
}
Invoke-OoxpRequest $req 'pptx-cs-create' | Out-Null
$ppt = Ensure-MsPpt (Ensure-PptxCreate)
Close-PptPresByPath $ppt $src3

$treeBare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src3.Replace('\', '/'); format = 'pptx'
} 'pptx-cs-inspect-bare'
$hasList0 = $null -ne $treeBare.PSObject.Properties['customShows']
$tourOut = Get-OutPath 'pptx-custom-show-tour.pptx'
Close-PptPresByPath $ppt $tourOut
if (Test-Path $tourOut) { Remove-Item -Force $tourOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src3.Replace('\', '/')
    output = $tourOut.Replace('\', '/')
    ops = @(@{
        op = 'insertCustomShow'; id = 'pptx:presentation'; name = 'Tour'
        slides = @('slide:1', 'slide:3')
    })
} 'pptx-cs-insert' | Out-Null
$treeTour = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tourOut.Replace('\', '/'); format = 'pptx'
    scope = @{ customShows = $true }
} 'pptx-cs-inspect-tour'
$s0 = $treeTour.customShows[0]
$cs1ok = ([int]$treeBare.customShowCount -eq 0) `
    -and (-not $hasList0) `
    -and ([int]$treeTour.customShowCount -eq 1) `
    -and ([string]$s0.id -eq 'show:1') `
    -and ([string]$s0.name -eq 'Tour') `
    -and ((@($s0.slides) -join ',') -eq 'slide:1,slide:3')
Add-Check 'pptx-Cs1' $cs1ok "bare=$($treeBare.customShowCount) list0=$hasList0 id=$($s0.id) name=$($s0.name) slides=$((@($s0.slides) -join ','))"

$presT = Open-PptPres $ppt $tourOut
$capT = Get-PptCaption $presT
$snapT = Get-ShowSnap $presT
$nameOk = Test-SlideShowName $presT 'Tour'
Close-PptPres $presT
$cs2ok = ($capT -notmatch 'Repair') `
    -and ([int]$snapT.Count -eq 1) `
    -and ($snapT.Rows[0].Name -eq 'Tour') `
    -and ([int]$snapT.Rows[0].Count -eq 2) `
    -and $nameOk
Add-Check 'pptx-Cs2' $cs2ok "cap=$capT count=$($snapT.Count) name=$($snapT.Rows[0].Name) slides=$($snapT.Rows[0].Count) set=$nameOk"

$renOut = Get-OutPath 'pptx-custom-show-guide.pptx'
Close-PptPresByPath $ppt $renOut
if (Test-Path $renOut) { Remove-Item -Force $renOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tourOut.Replace('\', '/')
    output = $renOut.Replace('\', '/')
    ops = @(@{
        op = 'setCustomShow'; id = 'show:1'; name = 'Guide'
        slides = @('slide:3', 'slide:1')
    })
} 'pptx-cs-set' | Out-Null
$treeG = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $renOut.Replace('\', '/'); format = 'pptx'
    scope = @{ customShows = $true }
} 'pptx-cs-inspect-guide'
$presG = Open-PptPres $ppt $renOut
$capG = Get-PptCaption $presG
$snapG = Get-ShowSnap $presG
Close-PptPres $presG
$g0 = $treeG.customShows[0]
$cs3ok = ($capG -notmatch 'Repair') `
    -and ([string]$g0.name -eq 'Guide') `
    -and ((@($g0.slides) -join ',') -eq 'slide:3,slide:1') `
    -and ($snapG.Rows[0].Name -eq 'Guide')
Add-Check 'pptx-Cs3' $cs3ok "cap=$capG name=$($g0.name) slides=$((@($g0.slides) -join ',')) com=$($snapG.Rows[0].Name)"

$delOut = Get-OutPath 'pptx-custom-show-delete.pptx'
Close-PptPresByPath $ppt $delOut
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $renOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteCustomShow'; id = 'show:1' })
} 'pptx-cs-delete' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'pptx'
    scope = @{ customShows = $true }
} 'pptx-cs-inspect-delete'
$presD = Open-PptPres $ppt $delOut
$capDel = Get-PptCaption $presD
$snapD = Get-ShowSnap $presD
Close-PptPres $presD

$oneOut = Get-OutPath 'pptx-custom-show-one.pptx'
Close-PptPresByPath $ppt $oneOut
if (Test-Path $oneOut) { Remove-Item -Force $oneOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src3.Replace('\', '/')
    output = $oneOut.Replace('\', '/')
    ops = @(@{
        op = 'insertCustomShow'; id = 'pptx:presentation'; name = 'Tour'
        slides = @('slide:1')
    })
} 'pptx-cs-one' | Out-Null
$sldOut = Get-OutPath 'pptx-custom-show-delslide.pptx'
Close-PptPresByPath $ppt $sldOut
if (Test-Path $sldOut) { Remove-Item -Force $sldOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $oneOut.Replace('\', '/')
    output = $sldOut.Replace('\', '/')
    ops = @(@{ op = 'deleteSlide'; id = 'slide:1' })
} 'pptx-cs-delslide' | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sldOut.Replace('\', '/'); format = 'pptx'
    scope = @{ customShows = $true }
} 'pptx-cs-inspect-delslide'
$presS = Open-PptPres $ppt $sldOut
$capSld = Get-PptCaption $presS
Close-PptPres $presS
$cs4ok = ($capDel -notmatch 'Repair') `
    -and ([int]$treeD.customShowCount -eq 0) `
    -and ([int]$snapD.Count -eq 0) `
    -and ($capSld -notmatch 'Repair') `
    -and ([int]$treeS.customShowCount -eq 1) `
    -and ((@($treeS.customShows[0].slides) -join ',') -eq '')
Add-Check 'pptx-Cs4' $cs4ok "delCap=$capDel count=$($treeD.customShowCount) sldCap=$capSld left=$((@($treeS.customShows[0].slides) -join ','))"

$badIdOut = Get-OutPath 'pptx-custom-show-bad-id.pptx'
$emptyOut = Get-OutPath 'pptx-custom-show-empty.pptx'
$takenOut = Get-OutPath 'pptx-custom-show-taken.pptx'
if (Test-Path $badIdOut) { Remove-Item -Force $badIdOut }
if (Test-Path $emptyOut) { Remove-Item -Force $emptyOut }
if (Test-Path $takenOut) { Remove-Item -Force $takenOut }
$failId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src3.Replace('\', '/')
    output = $badIdOut.Replace('\', '/')
    ops = @(@{ op = 'insertCustomShow'; id = 'slide:1'; name = 'Tour' })
} 'pptx-cs-bad-id'
$failEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src3.Replace('\', '/')
    output = $emptyOut.Replace('\', '/')
    ops = @(@{ op = 'insertCustomShow'; id = 'pptx:presentation'; name = '' })
} 'pptx-cs-empty'
$failTaken = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $tourOut.Replace('\', '/')
    output = $takenOut.Replace('\', '/')
    ops = @(@{ op = 'insertCustomShow'; id = 'pptx:presentation'; name = 'Tour' })
} 'pptx-cs-taken'
$xlsx = Ensure-XlsxCreate
$docx = Ensure-DocxCreate
$badX = Get-OutPath 'pptx-custom-show-fail-xlsx.xlsx'
$badD = Get-OutPath 'pptx-custom-show-fail-docx.docx'
if (Test-Path $badX) { Remove-Item -Force $badX }
if (Test-Path $badD) { Remove-Item -Force $badD }
$failX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $badX.Replace('\', '/')
    ops = @(@{ op = 'insertCustomShow'; id = 'pptx:presentation'; name = 'Tour' })
} 'pptx-cs-fail-xlsx'
$failD = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $docx.Replace('\', '/')
    output = $badD.Replace('\', '/')
    ops = @(@{ op = 'insertCustomShow'; id = 'pptx:presentation'; name = 'Tour' })
} 'pptx-cs-fail-docx'
$cs5ok = ((Get-PptErrorCode $failId) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $failEmpty) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failTaken) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failX) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failD) -eq 'InvalidField') `
    -and (-not (Test-Path $badIdOut)) `
    -and (-not (Test-Path $emptyOut)) `
    -and (-not (Test-Path $takenOut)) `
    -and (-not (Test-Path $badX)) `
    -and (-not (Test-Path $badD))
Add-Check 'pptx-Cs5' $cs5ok "id=$(Get-PptErrorCode $failId) empty=$(Get-PptErrorCode $failEmpty) taken=$(Get-PptErrorCode $failTaken) x=$(Get-PptErrorCode $failX) d=$(Get-PptErrorCode $failD)"
