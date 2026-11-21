$ErrorActionPreference='Stop'
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $PSScriptRoot 'native-chart-export.ps1'),[Text.Encoding]::UTF8)))
$path=Join-Path (Split-Path $PSScriptRoot) 'export-controlled-chart-native.ps1'
$tokens=$null;$errors=$null;$null=[Management.Automation.Language.Parser]::ParseFile($path,[ref]$tokens,[ref]$errors);if($errors.Count){throw ($errors|Out-String)}
$s=[IO.File]::ReadAllText($path);$count=1
$case=[pscustomobject]@{Id='valid';Sheet='Sheet1';Name='Controlled';Title='Same';WidthPt=288;HeightPt=180}
Assert-ControlledNativeCases @($case);$count++
$other=[pscustomobject]@{Id='second';Sheet='Sheet1';Name='Other';Title='Same';WidthPt=288;HeightPt=180};$encoded=@($case,$other)|ConvertTo-Json;$decoded=$encoded|ConvertFrom-Json;Assert-ControlledNativeCases $decoded;if(@($decoded).Count -ne 2){throw 'JSON case array not flattened correctly'};$count++
foreach($bad in @(@(),@([pscustomobject]@{Id='../escape'}),@($case,$case),@([pscustomobject]@{Id='x';Sheet='Sheet1';Name='X';Title='X';WidthPt=0;HeightPt=180}),@([pscustomobject]@{Id='x';Sheet='Sheet1';Name='X';Title='X';WidthPt=[double]::NaN;HeightPt=180}))){$rejected=$false;try{Assert-ControlledNativeCases $bad}catch{$rejected=$true};if(-not $rejected){throw 'Bad manifest accepted'};$count++}
if($s.IndexOf('function Write-Log') -ge $s.IndexOf('$app=Ensure-MsExcel')){throw 'Logger startup dependency missing'};$count++
if($s -notmatch 'finally\{' -or $s -notmatch 'Close-OwnedExcelBooks' -or $s -notmatch 'Explicit name/title/geometry contract mismatch'){throw 'Ownership/context safeguards missing'};$count++
if($s -match '\.CopyPicture\(|\.Save\(|\.Quit\(|Stop-Process|Remove-Item|-Force|SetProcessDpi'){throw 'Forbidden capability used'};$count++
if($s -notmatch 'OOXML_DERIVED_REEXPORTED_BY_EXCEL' -or $s -notmatch 'Assert-NewNativeChartOutput' -or $s -notmatch 'SourceShaAfter'){throw 'Provenance or collision guard missing'};$count++
Write-Output "PASS $count controlled-native checks; no Office used"
