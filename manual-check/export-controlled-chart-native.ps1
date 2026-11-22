param([Parameter(Mandatory=$true)][string]$EvidenceRoot,[Parameter(Mandatory=$true)][string]$ExpectedSourceSha,[int]$ExpectedDpi=106)
$ErrorActionPreference='Stop'
$RepoRoot=Split-Path $PSScriptRoot
$EvidenceRoot=[IO.Path]::GetFullPath($EvidenceRoot)
$temp=[IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')+'\'
if(-not ($EvidenceRoot.TrimEnd('\')+'\').StartsWith($temp,[StringComparison]::OrdinalIgnoreCase)){throw 'Require owned Temp root'}
$Log=Join-Path $EvidenceRoot 'logs/native-export.log'
function Write-Log($Message){$line=[DateTimeOffset]::Now.ToString('o')+' '+$Message;Add-Content -LiteralPath $Log -Value $line -Encoding UTF8;Write-Host $line}
foreach($lib in @('office-com.ps1','owned-excel-books.ps1','native-chart-export.ps1')){. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $RepoRoot ('manual-check/lib/'+$lib)),[Text.Encoding]::UTF8)))}
Add-Type -AssemblyName System.Drawing
if(-not ('ControlledNativeDpi' -as [type])){Add-Type 'using System;using System.Runtime.InteropServices;public static class ControlledNativeDpi{[DllImport("user32.dll")]public static extern uint GetDpiForWindow(IntPtr h);}' }
function Json($path,$value){Assert-NewNativeChartOutput $path;[IO.File]::WriteAllText($path,($value|ConvertTo-Json -Depth 18),[Text.UTF8Encoding]::new($false))}
function Sha($p){(Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash}
function Exclusive($p){$f=$null;try{$f=[IO.File]::Open($p,'Open','Read','None');return $true}catch{return $false}finally{if($f){$f.Dispose()}}}
$cases=([IO.File]::ReadAllText((Join-Path $EvidenceRoot 'source/cases.json'),[Text.Encoding]::UTF8))|ConvertFrom-Json
Assert-ControlledNativeCases $cases
$source=Join-Path $EvidenceRoot 'source/controlled-source.xlsx';$seed=Join-Path $EvidenceRoot 'source/success-seed.xlsx';$copy=Join-Path $EvidenceRoot 'office-export/controlled-readonly.xlsx'
if((Sha $source) -ine $ExpectedSourceSha){throw 'Source SHA mismatch'}
Assert-NewNativeChartOutput $copy
foreach($case in $cases){Assert-NewNativeChartOutput (Join-Path $EvidenceRoot ('office-raw/'+$case.Id+'-native.png'));Assert-NewNativeChartOutput (Join-Path $EvidenceRoot ('certs/'+$case.Id+'.json'))}
if(@(Get-Process EXCEL -ErrorAction SilentlyContinue).Count -or $global:OoxpExcel){throw 'Existing Excel; refuse launch or reuse'}
Copy-Item -LiteralPath $source -Destination $copy
$before=@{};foreach($p in @($seed,$copy)){(Get-Item -LiteralPath $p).IsReadOnly=$true;if(-not (Exclusive $p)){throw 'Owned file not exclusive'};$before[$p]=Sha $p}
$repairsBefore=@{};Get-ChildItem $env:TEMP -Filter 'error*.xml' -File -ErrorAction SilentlyContinue|ForEach-Object{$repairsBefore[$_.FullName]=$_.LastWriteTimeUtc.Ticks}
$app=$null;$scope=$null;$book=$null;$window=$null;$cleanup=$null;$fatal=$null;$records=[Collections.Generic.List[object]]::new()
try{
 $app=Ensure-MsExcel $seed;$scope=New-OwnedExcelBooks $app $EvidenceRoot @($seed,$copy)
 $b=Open-ExcelBook $app $seed;Add-OwnedExcelBook $scope $b;$b=$null
 $dpi=[int][ControlledNativeDpi]::GetDpiForWindow([IntPtr]$app.Hwnd)
 if([string]$app.Version -ne '16.0' -or $dpi -ne $ExpectedDpi){throw 'Require Excel16 and expected DPI; do not alter DPI'}
 $book=Open-ExcelBook $app $copy;Add-OwnedExcelBook $scope $book
 if(-not $book.ReadOnly -or [string]$book.FullName -ine $copy){throw 'Wrong or writable workbook'}
 $window=$app.ActiveWindow;$window.Zoom=100
 foreach($case in $cases){
  $c=[ordered]@{When=[DateTimeOffset]::Now.ToString('o');Zone='Asia/Shanghai';SourceKind='OOXML_DERIVED_REEXPORTED_BY_EXCEL';ExportKind='EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE';Action='Chart.Export';Status='NOT_EXPORTED';SourceMaster=$source;SourceShaBefore=$ExpectedSourceSha;ReadOnly=[bool]$book.ReadOnly;Expected=$case;ExcelVersion=[string]$app.Version;ExcelWindowDpi=$dpi;ExcelPid=$scope.ProcessId;ProcessStartTime=$scope.StartTime.ToString('o');ClipboardPathUsed=$false;SaveCalled=$false;QuitCalled=$false};$records.Add($c)
  $sheet=$null;$objects=$null;$object=$null;$chart=$null;$title=$null;$active=$null
  try{
   $sheet=$book.Worksheets.Item([string]$case.Sheet);$null=$sheet.Activate();$objects=$sheet.ChartObjects();$object=$objects.Item([string]$case.Name);$null=$object.Activate();$chart=$object.Chart;$title=$chart.ChartTitle;$active=$app.ActiveSheet
   $c.SheetName=[string]$sheet.Name;$c.ActiveSheetName=[string]$active.Name;$c.ObjectName=[string]$object.Name;$c.Title=[string]$title.Text;$c.WidthPt=[double]$object.Width;$c.HeightPt=[double]$object.Height
   if($c.SheetName -cne $case.Sheet -or $c.ActiveSheetName -cne $case.Sheet -or $c.ObjectName -cne $case.Name -or $c.Title -cne $case.Title -or [Math]::Abs($c.WidthPt-$case.WidthPt) -gt 0.001 -or [Math]::Abs($c.HeightPt-$case.HeightPt) -gt 0.001){throw 'Explicit name/title/geometry contract mismatch'}
   $out=Join-Path $EvidenceRoot ('office-raw/'+$case.Id+'-native.png');Assert-NewNativeChartOutput $out;$c.ExportStartedAt=[DateTimeOffset]::Now.ToString('o');$returned=[bool]$chart.Export($out,'PNG',$false);$c.ExportFinishedAt=[DateTimeOffset]::Now.ToString('o');$c.Returned=$returned;$c.Png=Read-NativeChartPng $out $returned;$c.Status='NATIVE_EXPORTED_PENDING_CLOSE';Write-Output ($case.Id+' native='+$c.Png.Width+'x'+$c.Png.Height)
  }catch{$c.Error=$_.Exception.Message;throw}
  finally{foreach($o in @($active,$title,$chart,$object,$objects,$sheet)){Release-Com $o}}
 }
}catch{$fatal=$_.Exception.Message;Write-Output ('FATAL '+$fatal)}
finally{
 Release-Com $window;$window=$null;$book=$null
 if($scope){try{$cleanup=Close-OwnedExcelBooks $scope}catch{if(-not $fatal){$fatal=$_.Exception.Message}};$scope.App=$null}
 $global:OoxpExcel=$null;Release-Com $app;$app=$null;[GC]::Collect();[GC]::WaitForPendingFinalizers();[GC]::Collect()
 $files=@();foreach($p in $before.Keys){$files+=[pscustomobject]@{Path=$p;ShaBefore=$before[$p];ShaAfter=(Sha $p);ExclusiveRead=(Exclusive $p);OwnerLockPresent=(Test-Path -LiteralPath (Join-Path (Split-Path $p) ('~$'+[IO.Path]::GetFileName($p))))}}
 $repairs=@(Get-ChildItem $env:TEMP -Filter 'error*.xml' -File -ErrorAction SilentlyContinue|Where-Object{(-not $repairsBefore.ContainsKey($_.FullName)) -or $repairsBefore[$_.FullName] -ne $_.LastWriteTimeUtc.Ticks}|Where-Object{$_.Length -lt 1048576 -and ([IO.File]::ReadAllText($_.FullName)).Contains($EvidenceRoot)}|ForEach-Object{$_.FullName})
 $filesClean=(@($files|Where-Object{$_.ShaBefore -ne $_.ShaAfter -or -not $_.ExclusiveRead -or $_.OwnerLockPresent}).Count -eq 0 -and (Sha $source) -ieq $ExpectedSourceSha)
 $closed=($null -ne $cleanup -and -not $cleanup.VerificationError -and $cleanup.RemainingWorkbookCount -eq 0 -and $cleanup.Books.Count -eq 2 -and @($cleanup.Books|Where-Object{-not $_.Closed -or $_.Error}).Count -eq 0)
 foreach($c in $records){$c.SourceShaAfter=Sha $source;$c.FilesClean=$filesClean;$c.OwnedCloseVerified=$closed;$c.RepairLogs=$repairs;$c.GlobalError=$fatal;$c.Status=if(-not $fatal -and $closed -and $filesClean -and -not $repairs.Count -and $c.Returned){'CLEAN_PUBLIC_OFFICE_EXPORT'}else{'NATIVE_EXPORT_INCOMPLETE_NOT_GOLD'};Json (Join-Path $EvidenceRoot ('certs/'+$c.Expected.Id+'.json')) $c}
 Json (Join-Path $EvidenceRoot 'logs/native-lifecycle-summary.json') ([ordered]@{When=[DateTimeOffset]::Now.ToString('o');Fatal=$fatal;Closed=$closed;FilesClean=$filesClean;Files=$files;Cleanup=$cleanup;Repairs=$repairs;CaseCount=$records.Count;SaveCalled=$false;QuitCalled=$false;KillCalled=$false;ObservedExcelProcessCount=@(Get-Process EXCEL -ErrorAction SilentlyContinue).Count})
}
if($fatal){throw $fatal};if(-not $closed -or -not $filesClean -or $repairs.Count){throw 'Native lifecycle/source verification failed'}
