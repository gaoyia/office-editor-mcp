# xlsx 批注：docs/checklist-xlsx.md 的 Xcm1–Xcm5。
# 用 create 产物再 apply，不另做 Excel 另存样例。

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

# 读工作表线程化批注。COM 包装对象用完即 Release-Com。
function Read-Threaded($sheet) {
    $text = ''
    $n = 0
    $reply = ''
    $replyN = 0
    $col = $null
    try { $col = $sheet.CommentsThreaded } catch { }
    if ($null -eq $col) {
        return @{ Count = 0; Text = ''; Reply = ''; ReplyCount = 0 }
    }
    try { $n = [int]$col.Count } catch { }
    if ($n -ge 1) {
        $c = $null
        try { $c = $col.Item(1) } catch { }
        if ($null -ne $c) {
            try { $text = [string]$c.Text() } catch { }
            $rs = $null
            try { $rs = $c.Replies } catch { }
            if ($null -ne $rs) {
                try { $replyN = [int]$rs.Count } catch { }
                if ($replyN -ge 1) {
                    $r = $null
                    try { $r = $rs.Item(1) } catch { }
                    if ($null -ne $r) {
                        try { $reply = [string]$r.Text() } catch { }
                        Release-Com $r
                    }
                }
                Release-Com $rs
            }
            Release-Com $c
        }
    }
    Release-Com $col
    return @{ Count = $n; Text = $text; Reply = $reply; ReplyCount = $replyN }
}

$hostPath = Get-OutPath 'xlsx-comments-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xcm-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xcm-inspect'
$noList = -not ($tree0.sheets[0].PSObject.Properties.Name -contains 'comments')
$hasCount = ($tree0.PSObject.Properties.Name -contains 'commentCount') -and ([int]$tree0.commentCount -eq 0)
$sheetCount = ($tree0.sheets[0].PSObject.Properties.Name -contains 'commentCount') -and ([int]$tree0.sheets[0].commentCount -eq 0)
$treeList = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ comments = $true }
} 'xlsx-xcm-inspect-list'
$hasList = $treeList.sheets[0].PSObject.Properties.Name -contains 'comments'
$listed = @($treeList.sheets[0].comments | Where-Object { $_ })
$listEmpty = $hasList -and ($listed.Count -eq 0)
$xcm1ok = $hasCount -and $sheetCount -and $noList -and $listEmpty
Add-Check 'xlsx-Xcm1' $xcm1ok "root=$($tree0.commentCount) sheet=$($tree0.sheets[0].commentCount) noList=$noList empty=$listEmpty hasList=$hasList"

$insOut = Get-OutPath 'xlsx-comments-insert.xlsx'
if (Test-Path $insOut) { Remove-Item -Force $insOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{
        op = 'insertComment'
        id = 'cell:Sheet1!A1'
        text = 'Xcm批注'
        author = 'ooxp'
        date = '2026-09-01T00:00:00.000'
    })
} 'xlsx-xcm-insert' | Out-Null

$excel = Ensure-MsExcel $insOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $insOut
$cap = [string]$wb.Windows.Item(1).Caption
$s = $wb.Worksheets.Item(1)
$th = Read-Threaded $s
$xcm2ok = ($cap -notmatch 'Repair') -and ($th.Text -match 'Xcm批注')
Add-Check 'xlsx-Xcm2' $xcm2ok "caption=$cap text=$($th.Text) n=$($th.Count)"
Close-AllWorkbooks $excel

$treeIns = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ comments = $true }
} 'xlsx-xcm-inspect-inserted'
$cid = [string]((@($treeIns.sheets[0].comments | Where-Object { $_ }) | Select-Object -First 1).id)

$replyOut = Get-OutPath 'xlsx-comments-reply.xlsx'
if (Test-Path $replyOut) { Remove-Item -Force $replyOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $replyOut.Replace('\', '/')
    ops = @(@{
        op = 'replyComment'
        id = $cid
        text = 'Xcm回复'
        author = 'ooxp'
        date = '2026-09-01T01:00:00.000'
    })
} 'xlsx-xcm-reply' | Out-Null

Close-AllWorkbooks $excel
$wbR = Open-ExcelBook $excel $replyOut
$capR = [string]$wbR.Windows.Item(1).Caption
$sR = $wbR.Worksheets.Item(1)
$thR = Read-Threaded $sR
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replyOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ comments = $true }
} 'xlsx-xcm-inspect-reply'
$inspectReply = $false
$rootC = @($treeR.sheets[0].comments | Where-Object { $_ }) | Select-Object -First 1
foreach ($r in @($rootC.replies | Where-Object { $_ })) {
    if (([string]$r.text) -eq 'Xcm回复') { $inspectReply = $true }
}
$xcm3ok = ($capR -notmatch 'Repair') -and $inspectReply -and ($thR.Reply -match 'Xcm回复')
Add-Check 'xlsx-Xcm3' $xcm3ok "caption=$capR com=$($thR.Reply) n=$($thR.ReplyCount) inspect=$inspectReply"
Close-AllWorkbooks $excel

$delOut = Get-OutPath 'xlsx-comments-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $replyOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteComment'; id = $cid })
} 'xlsx-xcm-delete' | Out-Null

Close-AllWorkbooks $excel
$wbD = Open-ExcelBook $excel $delOut
$capD = [string]$wbD.Windows.Item(1).Caption
$sD = $wbD.Worksheets.Item(1)
$thD = Read-Threaded $sD
$gone = ([int]$thD.Count -eq 0) -or [string]::IsNullOrWhiteSpace([string]$thD.Text)
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xcm-inspect-deleted'
$xcm4ok = ($capD -notmatch 'Repair') -and $gone -and ([int]$treeD.commentCount -eq 0)
Add-Check 'xlsx-Xcm4' $xcm4ok "caption=$capD gone=$gone n=$($thD.Count) count=$($treeD.commentCount)"
Close-AllWorkbooks $excel

$failOut = Get-OutPath 'xlsx-comments-bad.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'setCommentMeta'
        id = 'cell:Sheet1!A1/comment:{00000000-0000-4000-8000-000000000001}'
    })
} 'xlsx-xcm-missing'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xcm5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xcm5' $xcm5ok "exit=$($fail.ExitCode) code=$codeEmpty wrote=$(Test-Path $failOut)"
