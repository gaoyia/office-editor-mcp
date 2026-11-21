$ErrorActionPreference='Stop'
$repo=Split-Path (Split-Path $PSScriptRoot)
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $PSScriptRoot 'clipboard-export-diagnostics.ps1'),[Text.Encoding]::UTF8)))
$count=0
function Assert-Equal($Actual,$Expected,[string]$Name) {
 if($Actual -ne $Expected){throw "FAIL $Name expected=$Expected actual=$Actual"}
 $script:count++
}
foreach($case in @(
 @($false,10,10,'missing_bitmap_and_stale_sequence'),
 @($false,10,11,'missing_bitmap_fresh_sequence'),
 @($true,10,10,'bitmap_present_stale_sequence'),
 @($true,10,11,'bitmap_present_fresh_sequence'),
 @($true,[uint32]::MaxValue,0,'bitmap_present_fresh_sequence')
)){
 $record=[ordered]@{guardEvaluated=$true;bitmapImageReturned=$case[0];clipboardSequenceBefore=$case[1];clipboardSequenceAfter=$case[2]}
 Assert-Equal (Get-CopyPictureGuardClassification $record) $case[3] 'classification'
}
Assert-Equal (Get-CopyPictureGuardClassification ([ordered]@{attemptError='CopyPicture COM failure'})) 'not_evaluated' 'API failure is not an invented Bitmap observation'
$root=Join-Path $env:TEMP ('clipboard-diagnostic-test-'+[guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($root)
$record=[ordered]@{copyApi='Chart.CopyPicture';copyInputs=[ordered]@{Appearance=1;Format=2;Size=1};chartSheetName='A4p36';title='same title';guardEvaluated=$true;bitmapImageReturned=$false;clipboardSequenceBefore=10;clipboardSequenceAfter=11}
$original=$null
try{throw 'Reject missing/stale Bitmap clipboard'}catch{$original=$_.Exception.Message;$record.attemptError=$original}finally{
 Save-CopyPictureAttemptDiagnostic $record (Join-Path $root 'failed-guard.json') -ReadFormats { [pscustomobject]@{DataObjectPresent=$true;NativeFormats=@('EnhancedMetafile');AutoFormats=@('EnhancedMetafile')} } -ReadSequence { 11 }
}
$d=Get-Content -Raw -LiteralPath (Join-Path $root 'failed-guard.json') | ConvertFrom-Json
Assert-Equal $d.attemptError 'Reject missing/stale Bitmap clipboard' 'original exception persisted'
Assert-Equal $d.guardClassification 'missing_bitmap_fresh_sequence' 'failed guard persisted'
Assert-Equal $d.copyInputs.Size 1 'precise chart-sheet Size input persisted'
Assert-Equal $d.postGuardDiagnostic.NativeFormats[0] 'EnhancedMetafile' 'native formats persisted'
Assert-Equal ([bool]$d.postGuardDiagnostic.StartedAt) $true 'timestamp persisted'
$r=[ordered]@{attemptError='primary'}
Save-CopyPictureAttemptDiagnostic $r (Join-Path $root 'format-reader-error.json') -ReadFormats {throw 'clipboard busy'} -ReadSequence { 22 }
$d=Get-Content -Raw -LiteralPath (Join-Path $root 'format-reader-error.json') | ConvertFrom-Json
Assert-Equal $d.attemptError 'primary' 'diagnostic read did not overwrite primary failure'
Assert-Equal $d.postGuardDiagnostic.Error 'clipboard busy' 'format enumeration exception persisted'
$r=[ordered]@{}
Save-CopyPictureAttemptDiagnostic $r (Join-Path $root 'sequence-reader-error.json') -ReadFormats {throw 'should not run'} -ReadSequence {throw 'sequence unavailable'}
$d=Get-Content -Raw -LiteralPath (Join-Path $root 'sequence-reader-error.json') | ConvertFrom-Json
Assert-Equal $d.postGuardDiagnostic.Error 'sequence unavailable' 'initial sequence error persisted'
Assert-Equal $d.postGuardDiagnostic.SequenceReadError 'sequence unavailable' 'final sequence error persisted'
$collision=$null;try{Save-CopyPictureAttemptDiagnostic $record (Join-Path $root 'failed-guard.json') -ReadFormats {throw 'must not run'} -ReadSequence {throw 'must not run'}}catch{$collision=$_.Exception.Message}
Assert-Equal $collision 'Refuse existing clipboard attempt diagnostic' 'no evidence overwrite'
$guard="if(-not `$img -or `$seq -eq `$after){throw 'Reject missing/stale Bitmap clipboard'}"
foreach($name in @('verify-legend-foundation-lifecycle.ps1','verify-chartsheet-clipboard-diagnostic.ps1')){
 $path=Join-Path $repo ('manual-check/'+$name);$text=[IO.File]::ReadAllText($path,[Text.Encoding]::UTF8);$tokens=$null;$errors=$null
 $null=[Management.Automation.Language.Parser]::ParseFile($path,[ref]$tokens,[ref]$errors)
 Assert-Equal $errors.Count 0 'runner parses'
 Assert-Equal ([regex]::Matches($text,[regex]::Escape($guard)).Count) 1 'original Bitmap/freshness guard unchanged'
 Assert-Equal ($text.IndexOf('$certs.Add(') -lt $text.IndexOf('$null=$obj.CopyPicture(') -or $text.IndexOf('$null=$chart.CopyPicture(') -gt $text.IndexOf('$certs.Add(')) $true 'failure record queued before COM export'
 Assert-Equal ($text.IndexOf('Save-CopyPictureAttemptDiagnostic') -gt $text.IndexOf($guard)) $true 'additional format reads only after guard decision'
 Assert-Equal ($text -match 'Close-OwnedExcelBooks \$scope') $true 'owned cleanup remains present'
}
$text=[IO.File]::ReadAllText((Join-Path $repo 'manual-check/verify-chartsheet-clipboard-diagnostic.ps1'))
Assert-Equal ($text -match '\$end=1') $true 'one control only'
Assert-Equal ($text -match '\$chart.CopyPicture\(.+::xlScreen,.+::xlBitmap,.+::xlScreen\)') $true 'same documented three inputs'
# Reversed collection regression: index 1 is B5l72, requested A4 is last.
$script:released=0
function Release-Com($Value){$script:released++}
$charts=[pscustomobject]@{Requested=$null;WrongName=$false}
$charts | Add-Member -MemberType ScriptMethod -Name Item -Value {
 param($Name);$this.Requested=$Name
 if($Name -is [int] -or $this.WrongName){return [pscustomobject]@{Name='B5l72'}}
 return [pscustomobject]@{Name=$Name}
}
$chart=Get-NamedCopyPictureChart $charts 'A4p36'
Assert-Equal $charts.Requested 'A4p36' 'lookup receives explicit name, not ordinal'
Assert-Equal $chart.Name 'A4p36' 'A4 control selected despite reverse source order'
Assert-Equal $script:released 0 'successful chart reference transferred to caller'
$charts.WrongName=$true;$bad=$null
try{Get-NamedCopyPictureChart $charts 'A4p36'}catch{$bad=$_.Exception.Message}
Assert-Equal ($bad -like 'Expected named chart A4p36 but received B5l72') $true 'wrong identity rejected before CopyPicture'
Assert-Equal $script:released 1 'rejected chart reference released'
Assert-Equal ($text -match 'Get-NamedCopyPictureChart \$objects \$key') $true 'bounded runner uses name-verified lookup'
$expected=[ordered]@{Name='A4p36';PaperSize=9;Orientation=1;MarginPt=36}
$good=[ordered]@{chartSheetName='A4p36';activeSheetName='A4p36';paperSize=9;orientation=1;marginsPt=[ordered]@{Left=36;Right=36;Top=36;Bottom=36}}
Assert-CopyPictureChartContext $good $expected
Assert-Equal $good.contextVerified $true 'correct named A4 PageSetup verified'
foreach($field in @('chartSheetName','activeSheetName','paperSize','orientation','Left','Right','Top','Bottom')){
 $copy=[ordered]@{chartSheetName='A4p36';activeSheetName='A4p36';paperSize=9;orientation=1;marginsPt=[ordered]@{Left=36;Right=36;Top=36;Bottom=36}}
 if($field -in @('Left','Right','Top','Bottom')){$copy.marginsPt[$field]=72}elseif($field -in @('paperSize','orientation')){$copy[$field]=13}else{$copy[$field]='B5l72'}
 $errorMessage=$null;try{Assert-CopyPictureChartContext $copy $expected}catch{$errorMessage=$_.Exception.Message}
 Assert-Equal ([bool]$errorMessage -and -not $copy.contextVerified) $true ('mismatch rejected: '+$field)
}
Assert-Equal ($text.IndexOf('Assert-CopyPictureChartContext $c') -lt $text.IndexOf('$null=$chart.CopyPicture(')) $true 'identity/PageSetup checked before export'
Write-Output "PASS $count checks: injected metadata readers only, guard/persistence/parser checks; no Office or real clipboard accessed. Unit logs: $root"
