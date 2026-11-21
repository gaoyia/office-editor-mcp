$ErrorActionPreference='Stop'
$repo=Split-Path (Split-Path $PSScriptRoot)
. (Join-Path $PSScriptRoot 'native-chart-export.ps1')
Add-Type -AssemblyName System.Drawing
$count=0
function Check($Value,$Name){if(-not $Value){throw "FAIL $Name"};$script:count++}
function Reject([scriptblock]$Action,$Expected){$e=$null;try{& $Action}catch{$e=$_.Exception.Message};Check ($e -like ('*'+$Expected+'*')) $Expected}
$fixture=Join-Path $repo 'crates/ooxp/tests/fixtures/opaque-64x32-red.png'
$x=Read-NativeChartPng $fixture $true;Check ($x.Width -eq 64 -and $x.Height -eq 32) 'native PNG decoded';Check ($x.Sha256 -eq '71309897DD3D569D5E26A91D126922283A471232E035333B9150D3DE3380C06D') 'protected PNG unchanged'
Reject {Read-NativeChartPng $fixture $false} 'returned false'
Reject {Assert-NewNativeChartOutput $fixture} 'Refuse existing'
$root=Join-Path $env:TEMP ('native-chart-tests-'+[guid]::NewGuid().ToString('N'));[void][IO.Directory]::CreateDirectory($root)
$p=Join-Path $root 'bad.png';[IO.File]::WriteAllBytes($p,[byte[]](0..40));Reject {Read-NativeChartPng $p $true} 'Invalid native PNG'
$b=[IO.File]::ReadAllBytes($fixture);$b[16]=0;$b[17]=0;$b[18]=16;$b[19]=1;$p=Join-Path $root 'oversize.png';[IO.File]::WriteAllBytes($p,$b);Reject {Read-NativeChartPng $p $true} 'resource bound'
$path=Join-Path $repo 'manual-check/export-chartsheet-native-matrix.ps1';$s=[IO.File]::ReadAllText($path);$tokens=$null;$errors=$null;$null=[Management.Automation.Language.Parser]::ParseFile($path,[ref]$tokens,[ref]$errors);Check ($errors.Count -eq 0) 'collector parses'
Check ($s -notmatch '\.CopyPicture\s*\(|Clipboard\]::|Remove-Item|\.Quit\(|\.Save\(') 'no clipboard / replacement / Save / Quit path'
Check ($s -match 'Close-OwnedExcelBooks \$scope' -and $s -match 'finally\{') 'owned cleanup in finally'
Check ($s -match 'Assert-CopyPictureChartContext \$c \$expected') 'explicit name/PageSetup contract'
Check ($s -match 'EXCEL_CHARTSHEET_CHART_EXPORT_NATIVE' -and $s -match 'ClipboardBitmapVerified=\$false') 'distinct native provenance'
Check ($s -match 'Assert-NewNativeChartOutput \$out' -and $s -match '\$chart.Export\(\$out,') 'fresh unique native output'
$ast=[Management.Automation.Language.Parser]::ParseFile($path,[ref]$tokens,[ref]$errors)
$logger=$ast.Find({param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Write-Log'},$true)
Check ($null -ne $logger -and $logger.Extent.StartOffset -lt $s.IndexOf('$app=Ensure-MsExcel')) 'shared binding logging dependency defined before launch'
. ([scriptblock]::Create($logger.Extent.Text));$Log=Join-Path $root 'logger.txt';$pipeline=@(Write-Log 'unit-no-launch')
Check ($pipeline.Count -eq 0 -and (Test-Path -LiteralPath $Log)) 'logger emits no pipeline objects into bound application'
Write-Output "PASS $count native-export checks; no Office/clipboard used; temp=$root"
