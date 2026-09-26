# 把 Excel 亲手另存的 apply 源文件拷进 crates/ooxp/tests/fixtures/。
$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', 'lib', 'bootstrap.ps1'))
Initialize-Suite -Name 'office' -Fresh
$src = Ensure-XlsxExcelSource -Force
$destDir = Join-Path $OoxpRepoRoot 'crates/ooxp/tests/fixtures'
New-Item -ItemType Directory -Force -Path $destDir | Out-Null
$dest = Join-Path $destDir 'excel-source.xlsx'
Copy-Item -Force $src $dest
Write-Host "COPIED $src -> $dest"
Write-Host ("SIZE " + (Get-Item $dest).Length)
Close-OfficeApps -Quit
