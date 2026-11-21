$ErrorActionPreference='Stop'
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $PSScriptRoot 'office-com.ps1'),[Text.Encoding]::UTF8)))
. (Join-Path $PSScriptRoot 'owned-excel-books.ps1')
function Assert-Rejected([scriptblock]$Action, [string]$Expected) {
    $caught=$null
    try { & $Action } catch { $caught=$_.Exception.Message }
    if (-not $caught -or $caught -notlike ('*'+$Expected+'*')) { throw "Expected rejection: $Expected; got $caught" }
}
$fakeApp=[pscustomobject]@{Hwnd=[IntPtr]::Zero}
Assert-Rejected { New-OwnedExcelBooks $fakeApp $env:USERPROFILE @() } 'Temp directory'
$root=Join-Path $env:TEMP 'owned-excel-test'
Assert-Rejected { New-OwnedExcelBooks $fakeApp $root @((Join-Path $env:USERPROFILE 'user.xlsx')) } 'explicitly named'
$allowed=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$scope=[pscustomobject]@{Allowed=$allowed;Books=[Collections.Generic.List[object]]::new()}
Assert-Rejected { Add-OwnedExcelBook $scope ([pscustomobject]@{FullName=(Join-Path $env:USERPROFILE 'user.xlsx')}) } 'explicitly owned'
$scope=[pscustomobject]@{App=$fakeApp;ProcessId=123456789;StartTime=[datetime]::MinValue}
Assert-Rejected { Close-OwnedExcelBooks $scope } 'identity changed'
Write-Output 'PASS: path whitelist, ownership and stale-process rejection; no Excel launched/closed'