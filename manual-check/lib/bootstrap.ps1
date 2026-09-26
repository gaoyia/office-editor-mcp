# 用例只需 dot-source 本文件，就能拿到公共库的全部函数。

. (Join-Path $PSScriptRoot 'common.ps1')
. (Join-Path $PSScriptRoot 'office-com.ps1')
. (Join-Path $PSScriptRoot 'fixtures.ps1')
