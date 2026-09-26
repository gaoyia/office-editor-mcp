# 手测用例的公共部分：路径、日志、结果表、调 ooxp。
# 路径全部从本文件位置推出来，不写死 C:\Users\...，换机器不用改脚本。

$global:OoxpRepoRoot = (Resolve-Path ([IO.Path]::Combine($PSScriptRoot, '..', '..'))).Path
$global:OoxpExe = [IO.Path]::Combine($OoxpRepoRoot, 'target', 'debug', 'ooxp.exe')
$global:OoxpUtf8 = New-Object System.Text.UTF8Encoding $false

# 一轮验收（例如 office、wps）初始化一次。用例单跑时也会调，所以要幂等。
function Initialize-Suite {
    param(
        [Parameter(Mandatory)][string]$Name,
        [switch]$Fresh
    )
    $global:OoxpSuiteName = $Name
    $global:OoxpSuiteDir = [IO.Path]::Combine($OoxpRepoRoot, 'manual-check', $Name)
    $global:OoxpOutDir = Join-Path $OoxpSuiteDir 'out'
    $global:OoxpReqDir = Join-Path $OoxpOutDir 'requests'
    $global:OoxpLogPath = Join-Path $OoxpOutDir 'com-log.txt'
    New-Item -ItemType Directory -Force -Path $OoxpReqDir | Out-Null

    if (-not (Test-Path $OoxpExe)) {
        throw "找不到 $OoxpExe，先在仓库根目录跑 cargo build"
    }
    if ($Fresh) {
        if (Test-Path $OoxpLogPath) { Remove-Item $OoxpLogPath -Force }
        $global:OoxpResults = New-Object System.Collections.ArrayList
    }
    elseif (-not $global:OoxpResults) {
        $global:OoxpResults = New-Object System.Collections.ArrayList
    }

    # 中文字面量放 JSON，免得脚本文件的编码影响断言。
    $stringsPath = Join-Path $OoxpSuiteDir 'requests\_strings.json'
    $global:OoxpStrings = Get-Content -LiteralPath $stringsPath -Encoding UTF8 -Raw | ConvertFrom-Json
}

function Get-OutPath([string]$name) {
    Join-Path $global:OoxpOutDir $name
}

function Write-Log([string]$msg) {
    $line = "$(Get-Date -Format 'HH:mm:ss') $msg"
    Add-Content -Path $global:OoxpLogPath -Value $line -Encoding UTF8
    Write-Host $line
}

function Add-Result([string]$id, [string]$ok, [string]$detail) {
    [void]$global:OoxpResults.Add([pscustomobject]@{ id = $id; ok = $ok; detail = $detail })
    Write-Log ("RESULT $id $ok $detail")
}

# 用例里只写这一句：条件为真记 PASS，为假记 FAIL，detail 一律留下实测值。
function Add-Check([string]$id, [bool]$ok, [string]$detail) {
    Add-Result $id ($(if ($ok) { 'PASS' } else { 'FAIL' })) $detail
}

# 明知跑不了的（例如已知会挂起的那一步），记 SKIP 并写清原因，不要冒充 PASS。
function Add-Skip([string]$id, [string]$why) {
    Add-Result $id 'SKIP' $why
}

function Write-JsonFile([string]$path, $obj) {
    [System.IO.File]::WriteAllText($path, ($obj | ConvertTo-Json -Depth 12 -Compress), $global:OoxpUtf8)
}

# 不要用 cmd /c 抓输出：ooxp 吐 UTF-8，cmd 按控制台代码页（本机是 936）解，
# 中文会变成乱码，偶数长度的悄悄判错、奇数长度的直接把 JSON 撕坏。
# 这里显式指定 UTF-8 读两个流，并检查退出码。
function Invoke-Ooxp([string]$jsonPath) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $global:OoxpExe
    $psi.Arguments = "--input `"$jsonPath`""
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [System.Text.Encoding]::UTF8

    $proc = [System.Diagnostics.Process]::Start($psi)
    $out = $proc.StandardOutput.ReadToEnd().Trim()
    $err = $proc.StandardError.ReadToEnd().Trim()
    $proc.WaitForExit()
    $code = $proc.ExitCode
    $proc.Dispose()

    $name = [IO.Path]::GetFileName($jsonPath)
    if ($code -ne 0) {
        Write-Log "ooxp $name 退出码 $code stdout=$out stderr=$err"
        throw "ooxp $name 失败，退出码 $code：$(if ($err) { $err } else { $out })"
    }
    Write-Log "ooxp $name $out"
    return $out
}

# 请求对象落到 out/requests/ 再调用，产物和手写的请求分开放。
function Invoke-OoxpRequest($obj, [string]$name) {
    $path = Join-Path $global:OoxpReqDir "$name.json"
    Write-JsonFile $path $obj
    return Invoke-Ooxp $path
}

function Invoke-OoxpTree($obj, [string]$name) {
    (Invoke-OoxpRequest $obj $name) | ConvertFrom-Json
}

# 读手写的请求模板，把 path / output 改写到本机的 out 目录。
function Get-RequestTemplate([string]$name, [string]$outFile) {
    $path = Join-Path $global:OoxpSuiteDir "requests\$name.json"
    $obj = Get-Content -LiteralPath $path -Encoding UTF8 -Raw | ConvertFrom-Json
    $obj.path = (Get-OutPath $outFile).Replace('\', '/')
    return $obj
}

function Save-Results([string]$csvName) {
    $csv = Get-OutPath $csvName
    $global:OoxpResults | Export-Csv -Path $csv -NoTypeInformation -Encoding UTF8
    Write-Log "wrote $csvName"
}

function Show-Results {
    $global:OoxpResults | Format-Table -AutoSize | Out-String | Write-Host
    $fail = @($global:OoxpResults | Where-Object { $_.ok -eq 'FAIL' })
    $skip = @($global:OoxpResults | Where-Object { $_.ok -eq 'SKIP' })
    $pass = @($global:OoxpResults | Where-Object { $_.ok -eq 'PASS' })
    Write-Host ("PASS {0}  FAIL {1}  SKIP {2}" -f $pass.Count, $fail.Count, $skip.Count)
    if ($fail.Count) { $fail | ForEach-Object { Write-Host ("  FAIL " + $_.id + " " + $_.detail) } }
    return $fail.Count
}
