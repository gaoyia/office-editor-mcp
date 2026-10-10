# xlsx create 第五轮：docs/checklist-xlsx.md 的 C30–C32。
# 与 docs/examples/xlsx-create-round5.json 同构。不要和 C25–C29、Xns 撞。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

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

$path = Get-OutPath 'xlsx-create-round5.xlsx'
if (Test-Path $path) { Remove-Item -Force $path }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $path.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        cellStyles = @(@{
            name = 'OoxpNamed'
            font = @{ bold = $true }
            fill = @{ pattern = 'solid'; fg = @{ rgb = 'FFFFFF00' } }
        })
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 1.2; namedStyle = 'OoxpNamed' }
                @{ ref = 'B1'; value = 3 }
            )
            comments = @(@{
                cell = 'A1'
                text = '顶层批注'
                replies = @(@{ text = '一层回复'; author = 'Bob' })
            })
        })
    }
} 'xlsx-create-round5' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'xlsx'
    scope = @{
        sheet = 'Sheet1'
        range = 'A1:B1'
        cellStyles = $true
        comments = $true
    }
} 'xlsx-create-round5-inspect'

$named = @($tree.cellStyles | Where-Object { $_.name -eq 'OoxpNamed' })[0]
$a1 = @($tree.sheets[0].cells | Where-Object { $_.id -eq 'cell:Sheet1!A1' })[0]
$c30ok = ([int]$tree.cellStyleCount -eq 2) `
    -and ($null -ne $named) `
    -and ([string]$a1.fill.pattern -eq 'solid') `
    -and ([bool]$a1.font.bold)
Add-Check 'xlsx-C30' $c30ok "count=$($tree.cellStyleCount) name=$($named.name) pattern=$($a1.fill.pattern) bold=$($a1.font.bold)"

$cm = @($tree.sheets[0].comments)[0]
$reply = $null
if ($cm.replies) { $reply = @($cm.replies)[0] }
$c31ok = ([string]$cm.text -eq '顶层批注') `
    -and ($null -ne $reply) `
    -and ([string]$reply.text -eq '一层回复')
Add-Check 'xlsx-C31' $c31ok "text=$($cm.text) reply=$($reply.text)"

$excel = Ensure-MsExcel $path
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $path
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$rng = $s1.Range('A1')
$st = $rng.Style
$styleName = [string]$st.Name
Release-Com $st
$fill = [double]$rng.Interior.Color
$th = Read-Threaded $s1
$c32ok = ($cap -notmatch 'Repair') `
    -and ($styleName -eq 'OoxpNamed') `
    -and ([math]::Abs($fill - 65535) -lt 1) `
    -and ([int]$th.Count -ge 1) `
    -and ([int]$th.ReplyCount -ge 1) `
    -and ($th.Reply -like '*一层回复*')
Add-Check 'xlsx-C32' $c32ok "caption=$cap style=$styleName fill=$fill replies=$($th.ReplyCount) reply=$($th.Reply)"
Release-Com $rng
Release-Com $s1
Close-AllWorkbooks $excel
