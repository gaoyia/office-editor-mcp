param([Parameter(Mandatory=$true)][string]$EvidenceRoot,[Parameter(Mandatory=$true)][string]$ExpectedSourceSha,[int]$ExpectedDpi=106)
$ErrorActionPreference='Stop'
$RepoRoot=Split-Path $PSScriptRoot
$EvidenceRoot=[IO.Path]::GetFullPath($EvidenceRoot)
$temp=[IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')+'\'
if(-not ($EvidenceRoot.TrimEnd('\')+'\').StartsWith($temp,[StringComparison]::OrdinalIgnoreCase)){throw 'Require owned Temp evidence root'}
$Log=Join-Path $EvidenceRoot 'logs/native-export.log'
function Write-Log($Message){$line=[DateTimeOffset]::Now.ToString('o')+' '+$Message;Add-Content -LiteralPath $Log -Value $line -Encoding UTF8;Write-Host $line}
foreach($lib in @('office-com.ps1','owned-excel-books.ps1','clipboard-export-diagnostics.ps1','native-chart-export.ps1')){. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $RepoRoot ('manual-check/lib/'+$lib)),[Text.Encoding]::UTF8)))}
Add-Type -AssemblyName System.Drawing
if(-not ('NativeChartExportDpi' -as [type])){Add-Type 'using System;using System.Runtime.InteropServices;public static class NativeChartExportDpi{[DllImport("user32.dll")]public static extern uint GetDpiForWindow(IntPtr h);}'}
function Json($path,$value){Assert-NewNativeChartOutput $path;[IO.File]::WriteAllText($path,($value|ConvertTo-Json -Depth 18),[Text.UTF8Encoding]::new($false))}
function Sha($path){(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash}
function Exclusive($path){$f=$null;try{$f=[IO.File]::Open($path,'Open','Read','None');return $true}catch{return $false}finally{if($f){$f.Dispose()}}}
$source=Join-Path $EvidenceRoot 'source/f44-chartsheet-paper-source.xlsx';$seed=Join-Path $EvidenceRoot 'source/success-seed.xlsx';$copy=Join-Path $EvidenceRoot 'office-export/native-readonly.xlsx'
if((Sha $source) -ine $ExpectedSourceSha){throw 'Source SHA mismatch before launch'}
Assert-NewNativeChartOutput $copy;Copy-Item -LiteralPath $source -Destination $copy
$before=@{};foreach($p in @($seed,$copy)){(Get-Item -LiteralPath $p).IsReadOnly=$true;if(-not (Exclusive $p)){throw 'Owned file not exclusive before launch'};$before[$p]=Sha $p}
$names=@('A4p36','B5p36','B5p54','B5p72','B5l36','B5l54','B5l72')
foreach($name in $names){foreach($pass in 1,2){Assert-NewNativeChartOutput (Join-Path $EvidenceRoot "office-raw/$name-native-$pass.png")};Assert-NewNativeChartOutput (Join-Path $EvidenceRoot "certs/$name-native.json")}
if(@(Get-Process EXCEL -ErrorAction SilentlyContinue).Count -or $global:OoxpExcel){throw 'Existing Excel session; refuse launch/reuse'}
$repairsBefore=@{};Get-ChildItem $env:TEMP -Filter 'error*.xml' -File -ErrorAction SilentlyContinue|ForEach-Object{$repairsBefore[$_.FullName]=$_.LastWriteTimeUtc.Ticks}
$app=$null;$scope=$null;$books=$null;$book=$null;$window=$null;$cleanup=$null;$fatal=$null;$records=[Collections.Generic.List[object]]::new()
try{
 $app=Ensure-MsExcel $seed
 $scope=New-OwnedExcelBooks $app $EvidenceRoot @($seed,$copy)
 $b=Open-ExcelBook $app $seed;Add-OwnedExcelBook $scope $b;$b=$null
 $dpi=[int][NativeChartExportDpi]::GetDpiForWindow([IntPtr]$app.Hwnd)
 if([string]$app.Version -ne '16.0' -or $dpi -ne $ExpectedDpi){throw 'Require Excel16 and expected DPI; do not change system DPI'}
 $book=Open-ExcelBook $app $copy;Add-OwnedExcelBook $scope $book
 if(-not $book.ReadOnly -or [string]$book.FullName -ine $copy){throw 'Wrong or writable owned workbook'}
 $books=$book.Charts;$window=$app.ActiveWindow
 if($books.Count -ne 7){throw 'Expected seven explicitly named source chartsheets'}
 foreach($name in $names){
  $c=[ordered]@{When=[DateTimeOffset]::Now.ToString('o');Zone='Asia/Shanghai';Action='Chart.Export';ExportKind='EXCEL_CHARTSHEET_CHART_EXPORT_NATIVE';Status='NOT_EXPORTED';Source=$copy;SourceMaster=$source;SourceShaBefore=$ExpectedSourceSha;ReadOnly=[bool]$book.ReadOnly;RequestedName=$name;ExcelVersion=[string]$app.Version;ExcelWindowDpi=$dpi;ExcelPid=$scope.ProcessId;ClipboardPathUsed=$false;ClipboardBitmapVerified=$false;SaveCalled=$false;QuitCalled=$false;Exports=@()};$records.Add($c)
  $chart=$null;$setup=$null;$area=$null;$active=$null
  try{
   $chart=Get-NamedCopyPictureChart $books $name
   $null=$chart.Activate();$window.Zoom=100;Start-Sleep -Milliseconds 200
   $active=$app.ActiveSheet;$setup=$chart.PageSetup;$area=$chart.ChartArea
   $c.chartSheetName=[string]$chart.Name;$c.activeSheetName=[string]$active.Name;$c.ChartSheetIndex=[int]$chart.Index;$c.paperSize=[int]$setup.PaperSize;$c.orientation=[int]$setup.Orientation
   $c.marginsPt=[ordered]@{Left=[double]$setup.LeftMargin;Right=[double]$setup.RightMargin;Top=[double]$setup.TopMargin;Bottom=[double]$setup.BottomMargin};$c.ChartSize=[int]$setup.ChartSize;$c.Zoom=[int]$window.Zoom
   $expected=[ordered]@{Name=$name;PaperSize=13;Orientation=1;MarginPt=[int]$name.Substring($name.Length-2)}
   if($name -eq 'A4p36'){$expected.PaperSize=9};if($name.StartsWith('B5l')){$expected.Orientation=2}
   $c.ExpectedContext=$expected;Assert-CopyPictureChartContext $c $expected
   $c.ChartAreaAfterActivation=[ordered]@{WidthPt=[double]$area.Width;HeightPt=[double]$area.Height}
   foreach($pass in 1,2){
    $out=Join-Path $EvidenceRoot "office-raw/$name-native-$pass.png";Assert-NewNativeChartOutput $out
    $started=[DateTimeOffset]::Now.ToString('o');$returned=[bool]$chart.Export($out,'PNG',$false)
    $image=Read-NativeChartPng $out $returned
    $c.Exports+= [pscustomobject]@{Pass=$pass;StartedAt=$started;FinishedAt=[DateTimeOffset]::Now.ToString('o');Returned=$returned;Png=$image;ChartAreaWidthPt=[double]$area.Width;ChartAreaHeightPt=[double]$area.Height}
   }
   $c.RepeatBytesEqual=($c.Exports[0].Png.Sha256 -eq $c.Exports[1].Png.Sha256);$c.Status='NATIVE_EXPORTED_PENDING_CLOSE'
   Write-Output "$name native=$($c.Exports[0].Png.Width)x$($c.Exports[0].Png.Height) repeatEqual=$($c.RepeatBytesEqual)"
  }catch{$c.Error=$_.Exception.Message;$c.ErrorAt=[DateTimeOffset]::Now.ToString('o');throw}
  finally{foreach($o in @($active,$area,$setup,$chart)){Release-Com $o};$active=$null;$area=$null;$setup=$null;$chart=$null}
 }
}catch{$fatal=$_.Exception.Message;Write-Output "FATAL $fatal"}
finally{
 foreach($o in @($books,$window)){Release-Com $o};$books=$null;$window=$null;$book=$null
 if($scope){try{$cleanup=Close-OwnedExcelBooks $scope}catch{if(-not $fatal){$fatal=$_.Exception.Message}};$scope.App=$null}
 $global:OoxpExcel=$null;Release-Com $app;$app=$null;[GC]::Collect();[GC]::WaitForPendingFinalizers();[GC]::Collect()
 $files=@();foreach($p in $before.Keys){$files+=[pscustomobject]@{Path=$p;ShaBefore=$before[$p];ShaAfter=(Sha $p);ExclusiveRead=(Exclusive $p);OwnerLockPresent=(Test-Path -LiteralPath (Join-Path (Split-Path $p) ('~$'+[IO.Path]::GetFileName($p))))}}
 $repairs=@(Get-ChildItem $env:TEMP -Filter 'error*.xml' -File -ErrorAction SilentlyContinue|Where-Object{(-not $repairsBefore.ContainsKey($_.FullName)) -or $repairsBefore[$_.FullName] -ne $_.LastWriteTimeUtc.Ticks}|Where-Object{$_.Length -lt 1048576 -and ([IO.File]::ReadAllText($_.FullName)).Contains($EvidenceRoot)}|ForEach-Object{$_.FullName})
 $cleanFiles=(@($files|Where-Object{$_.ShaBefore -ne $_.ShaAfter -or -not $_.ExclusiveRead -or $_.OwnerLockPresent}).Count -eq 0 -and (Sha $source) -ieq $ExpectedSourceSha)
 $closed=($null -ne $cleanup -and -not $cleanup.VerificationError -and $cleanup.RemainingWorkbookCount -eq 0 -and $cleanup.Books.Count -eq 2 -and @($cleanup.Books|Where-Object{-not $_.Closed -or $_.Error}).Count -eq 0)
 foreach($c in $records){$c.SourceShaAfter=Sha $source;$c.FilesClean=$cleanFiles;$c.OwnedCloseVerified=$closed;$c.RepairLogs=$repairs;$c.GlobalError=$fatal;$c.Status=if(-not $fatal -and $cleanFiles -and $closed -and -not $repairs.Count -and $c.Exports.Count -eq 2){'CLEAN_PUBLIC_OFFICE_EXPORT'}else{'NATIVE_EXPORT_INCOMPLETE_NOT_GOLD'};Json (Join-Path $EvidenceRoot "certs/$($c.RequestedName)-native.json") $c}
 Json (Join-Path $EvidenceRoot 'logs/native-lifecycle-summary.json') ([ordered]@{ObservedAt=[DateTimeOffset]::Now.ToString('o');Zone='Asia/Shanghai';ExportKind='EXCEL_CHARTSHEET_CHART_EXPORT_NATIVE';Fatal=$fatal;FilesClean=$cleanFiles;Files=$files;Cleanup=$cleanup;Closed=$closed;Repairs=$repairs;CaseCount=$records.Count;NewInstancesLaunched=1;SaveCalled=$false;QuitCalled=$false;KillCalled=$false;ExcelProcessCount=@(Get-Process EXCEL -ErrorAction SilentlyContinue).Count})
}
if($fatal){throw $fatal};if(-not $closed -or -not $cleanFiles -or $repairs.Count){throw 'Native export cleanup/source verification failed'}
