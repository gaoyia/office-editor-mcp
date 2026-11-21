param([Parameter(Mandatory=$true)][string]$EvidenceRoot,[int]$ExpectedDpi=106)
$ErrorActionPreference='Stop'
$RepoRoot=Split-Path $PSScriptRoot
$EvidenceRoot=[IO.Path]::GetFullPath($EvidenceRoot)
$tempRoot=[IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')+'\'
if(-not ($EvidenceRoot.TrimEnd('\')+'\').StartsWith($tempRoot,[StringComparison]::OrdinalIgnoreCase)){throw 'EvidenceRoot must be an owned Temp directory, before launch'}
$Log=Join-Path $EvidenceRoot 'logs\real-lifecycle.log'
function Write-Log($s){$s="$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss zzz') $s";Add-Content -LiteralPath $Log -Value $s -Encoding UTF8;Write-Host $s}
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $RepoRoot 'manual-check\lib\office-com.ps1'),[Text.Encoding]::UTF8)))
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $RepoRoot 'manual-check\lib\owned-excel-books.ps1'),[Text.Encoding]::UTF8)))
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
[void][Reflection.Assembly]::LoadWithPartialName('Microsoft.Office.Interop.Excel')
if(-not ('LegendLifecycleNative' -as [type])){Add-Type 'using System;using System.Runtime.InteropServices;public static class LegendLifecycleNative{[DllImport("user32.dll")]public static extern uint GetDpiForWindow(IntPtr h);[DllImport("user32.dll")]public static extern uint GetClipboardSequenceNumber();}'}
function Save-Json($path,$value){[IO.File]::WriteAllText($path,($value|ConvertTo-Json -Depth 12),[Text.UTF8Encoding]::new($false))}
function Sha($path){(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash}
function Exclusive($path){$h=$null;try{$h=[IO.File]::Open($path,'Open','Read','None');return $true}catch{return $false}finally{if($h){$h.Dispose()}}}
$source=Join-Path $EvidenceRoot 'source\controlled-legend-typography.xlsx'
$sourceSha=Sha $source
$names=@('colCtrl','colLegTxPr','colEntry0Color','colEntry0Size','colEntry0Bold','colEntryMix','colEntry1Color')
$paths=@{};$before=@{}
foreach($phase in @('success','exception')){
 $seed=Join-Path $EvidenceRoot "source\$phase-seed.xlsx";$copy=Join-Path $EvidenceRoot "office-export\$phase-readonly.xlsx"
 if(Test-Path -LiteralPath $copy){throw "Refuse existing export copy $copy"}
 Copy-Item -LiteralPath $source -Destination $copy
 foreach($p in @($seed,$copy)){(Get-Item -LiteralPath $p).IsReadOnly=$true;if(-not (Exclusive $p)){throw "Not exclusive before launch: $p"};$before[$p]=Sha $p}
 $paths[$phase]=@($seed,$copy)
}
$initial=@(Get-Process EXCEL -ErrorAction SilentlyContinue)
if($initial.Count -or $global:OoxpExcel){throw 'Require clean observed Excel state; never reuse stale refs or launch over an existing session'}
$repairBefore=@{};Get-ChildItem $env:TEMP -Filter 'error*.xml' -File -ErrorAction SilentlyContinue|ForEach-Object{$repairBefore[$_.FullName]=$_.LastWriteTimeUtc.Ticks}
$app=$null;$scope=$null;$book=$null;$seedBook=$null;$cleanup=$null;$certs=[Collections.Generic.List[object]]::new();$reports=[Collections.Generic.List[object]]::new();$fatal=$null
try{
 # One sanctioned launch only. Both phases below use this same explicitly held application.
 $app=Ensure-MsExcel $paths.success[0]
 $scope=New-OwnedExcelBooks $app $EvidenceRoot $paths.success
 $seedBook=Open-ExcelBook $app $paths.success[0];Add-OwnedExcelBook $scope $seedBook;$seedBook=$null
 $dpi=[int][LegendLifecycleNative]::GetDpiForWindow([IntPtr]$app.Hwnd)
 if([string]$app.Version -ne '16.0' -or $dpi -ne $ExpectedDpi){throw 'Require Excel16 and expected window DPI; never change DPI'}
 foreach($phase in @('success','exception')){
  if($phase -eq 'exception'){
   # Previous scope has verified zero books; open new owned seed/copy, NOT another /x process.
   $scope=New-OwnedExcelBooks $app $EvidenceRoot $paths[$phase]
   $seedBook=Open-ExcelBook $app $paths[$phase][0];Add-OwnedExcelBook $scope $seedBook;$seedBook=$null
  }
  $sheet=$null;$sheets=$null;$window=$null;$objects=$null;$expectedException=$false;$phaseError=$null
  try{
   $book=Open-ExcelBook $app $paths[$phase][1];Add-OwnedExcelBook $scope $book
   if(-not $book.ReadOnly -or [string]$book.FullName -ine $paths[$phase][1]){throw 'Wrong or writable export workbook'}
   $sheets=$book.Worksheets;$sheet=$sheets.Item('Sheet1');$sheet.Activate();$window=$app.ActiveWindow;$window.Zoom=100;$window.DisplayGridlines=$false
   $objects=$sheet.ChartObjects();if($objects.Count -ne 7){throw 'Expected seven controlled charts'}
   $end=if($phase -eq 'success'){7}else{1}
   for($i=1;$i -le $end;$i++){
    $obj=$null;$chart=$null;$tl=$null;$title=$null;$img=$null
    $key=$names[$i-1];$prefix=if($phase -eq 'success'){$key}else{'exception-proof'}
    $out=Join-Path $EvidenceRoot "office-raw\$prefix-copypicture.png";$export=Join-Path $EvidenceRoot "office-raw\$prefix-chartexport.png"
    $c=[ordered]@{when=(Get-Date).ToString('yyyy-MM-dd HH:mm:ss zzz');zone='Asia/Shanghai';phase=$phase;sourceKind='derived-controlled-same-title-anchor-data-from-public-api-source';source=$paths[$phase][1];sourceMaster=$source;sourceShaBefore=$sourceSha;exclusiveReadBefore=$true;output=$out;id="sheet:Sheet1/chart:$($i+1)";excelVersion=[string]$app.Version;excelWindowDpi=$dpi;excelPid=$global:OoxpExcelPid;readOnly=[bool]$book.ReadOnly;saveCalled=$false;quitCalled=$false;status='NOT_EXPORTED'}
    try{
     $obj=$objects.Item($i);$chart=$obj.Chart;$title=$chart.ChartTitle;$c.title=[string]$title.Text
     $c.chartWidthPt=[double]$obj.Width;$c.chartHeightPt=[double]$obj.Height;$c.chartLeftPt=[double]$obj.Left;$c.chartTopPt=[double]$obj.Top
     $tl=$obj.TopLeftCell;$window.ScrollRow=[int]$tl.Row;$window.ScrollColumn=[int]$tl.Column;$null=$obj.Activate()
     Start-Sleep -Milliseconds 300
     $seq=[LegendLifecycleNative]::GetClipboardSequenceNumber()
     $null=$obj.CopyPicture([Microsoft.Office.Interop.Excel.XlPictureAppearance]::xlScreen,[Microsoft.Office.Interop.Excel.XlCopyPictureFormat]::xlBitmap)
     Start-Sleep -Milliseconds 200
     $img=[Windows.Forms.Clipboard]::GetImage();$after=[LegendLifecycleNative]::GetClipboardSequenceNumber();$c.clipboardSequenceBefore=$seq;$c.clipboardSequenceAfter=$after;$c.clipboardFormats=@([Windows.Forms.Clipboard]::GetDataObject().GetFormats())
     if(-not $img -or $seq -eq $after){throw 'Reject missing/stale Bitmap clipboard'}
     $c.copyApi='ChartObject.CopyPicture(xlScreen,xlBitmap)';$c.width=$img.Width;$c.height=$img.Height;$img.Save($out,[Drawing.Imaging.ImageFormat]::Png);$img.Dispose();$img=$null
     $c.outputSha256=Sha $out;$c.chartExportReturned=[bool]$chart.Export($export,'PNG',$false);if(-not $c.chartExportReturned){throw 'Chart.Export returned false'}
     $img=[Drawing.Image]::FromFile($export);$c.exportWidth=$img.Width;$c.exportHeight=$img.Height;$img.Dispose();$img=$null;$c.exportSha256=Sha $export
     $c.status='EXPORTED_PENDING_CLOSE';$certs.Add([pscustomobject]@{Path=(Join-Path $EvidenceRoot "certs\$prefix.json");Data=$c})
     Write-Log "$phase $key CopyPicture=$($c.width)x$($c.height) Export=$($c.exportWidth)x$($c.exportHeight)"
    }finally{if($img){$img.Dispose()};foreach($o in @($tl,$title,$chart,$obj)){Release-Com $o};$tl=$null;$title=$null;$chart=$null;$obj=$null}
   }
   if($phase -eq 'exception'){throw 'INTENTIONAL_OWNED_EXPORT_EXCEPTION'}
  }catch{
   $phaseError=$_.Exception.Message
   if($phase -eq 'exception' -and $phaseError -eq 'INTENTIONAL_OWNED_EXPORT_EXCEPTION'){$expectedException=$true;Write-Log 'Caught intentional exception after real export'}else{throw}
  }finally{
   foreach($o in @($objects,$window,$sheet,$sheets)){Release-Com $o};$objects=$null;$window=$null;$sheet=$null;$sheets=$null;$book=$null
   $cleanup=Close-OwnedExcelBooks $scope
   $reports.Add([pscustomobject]@{Phase=$phase;IntentionalExceptionCaught=$expectedException;Error=$phaseError;ProcessId=$scope.ProcessId;StartTime=$scope.StartTime;Cleanup=$cleanup})
   Save-Json (Join-Path $EvidenceRoot "logs\$phase-owned-cleanup.json") $reports[$reports.Count-1]
   $scope.App=$null;$scope=$null
   if($cleanup.VerificationError -or $cleanup.RemainingWorkbookCount -ne 0 -or @($cleanup.Books|Where-Object{-not $_.Closed}).Count){throw 'Owned cleanup did not verify both books closed; stop without new instance'}
   Write-Log "$phase verified owned seed+copy Close(false), remainingWorkbooks=0; windowPresent=$($cleanup.WindowStillPresent)"
  }
 }
}catch{$fatal=$_.Exception.Message;Write-Log "FATAL $fatal"}finally{
 # Covers failures between binding/registering and entering a phase as well.
 if($scope){try{$cleanup=Close-OwnedExcelBooks $scope;Save-Json (Join-Path $EvidenceRoot 'logs\fatal-owned-cleanup.json') $cleanup}catch{Write-Log "Cleanup error $($_.Exception.Message)"};$scope.App=$null}
 $book=$null;$seedBook=$null;$global:OoxpExcel=$null;Release-Com $app;$app=$null
 [GC]::Collect();[GC]::WaitForPendingFinalizers();[GC]::Collect()
 $repairs=@(Get-ChildItem $env:TEMP -Filter 'error*.xml' -File -ErrorAction SilentlyContinue|Where-Object{(-not $repairBefore.ContainsKey($_.FullName)) -or $repairBefore[$_.FullName] -ne $_.LastWriteTimeUtc.Ticks}|Where-Object{$_.Length -lt 1048576 -and ([IO.File]::ReadAllText($_.FullName)).Contains($EvidenceRoot)}|ForEach-Object{$_.FullName})
 $files=@();foreach($p in $before.Keys){$lock=Join-Path (Split-Path $p) ('~$'+[IO.Path]::GetFileName($p));$files += [pscustomobject]@{Path=$p;ShaBefore=$before[$p];ShaAfter=(Sha $p);ExclusiveRead=(Exclusive $p);OwnerLockPresent=(Test-Path -LiteralPath $lock)}}
 $cleanFiles=(@($files|Where-Object{$_.ShaBefore -ne $_.ShaAfter -or -not $_.ExclusiveRead -or $_.OwnerLockPresent}).Count -eq 0 -and (Sha $source) -eq $sourceSha)
 foreach($entry in $certs){$c=$entry.Data;$c.sourceShaAfter=Sha $c.source;$c.masterShaAfter=Sha $source;$c.sourceUnchanged=($c.sourceShaAfter -eq $sourceSha -and $c.masterShaAfter -eq $sourceSha);$c.exclusiveRead=(Exclusive $c.source);$c.ownerLockPresent=(Test-Path (Join-Path (Split-Path $c.source) ('~$'+[IO.Path]::GetFileName($c.source))));$c.repairLogs=$repairs;$c.closeError=$fatal
  $c.status=if($c.phase -eq 'success' -and -not $fatal -and $cleanFiles -and -not $repairs.Count){'CLEAN_PUBLIC_OFFICE_EXPORT'}elseif($c.phase -eq 'exception' -and -not $fatal -and $cleanFiles -and -not $repairs.Count){'INTENTIONAL_EXCEPTION_LIFECYCLE_PROOF_NOT_GOLD'}else{'EXPORT_OR_CLEANUP_INCOMPLETE_NOT_GOLD'};Save-Json $entry.Path $c
 }
 $processes=@(Get-Process EXCEL -ErrorAction SilentlyContinue|ForEach-Object{[pscustomobject]@{Pid=$_.Id;StartTime=$_.StartTime;Title=$_.MainWindowTitle;Hwnd=$_.MainWindowHandle.ToInt64()}})
 Save-Json (Join-Path $EvidenceRoot 'logs\lifecycle-summary.json') ([ordered]@{ObservedAt=(Get-Date).ToString('o');Zone='Asia/Shanghai';Fatal=$fatal;FilesClean=$cleanFiles;Files=$files;Repairs=$repairs;Reports=@($reports.ToArray());RemainingExcelProcesses=$processes;SaveCalled=$false;QuitCalled=$false;KillCalled=$false;NewInstancesLaunched=1})
 Write-Log "END filesClean=$cleanFiles ExcelProcesses=$($processes.Count); no Quit/kill; no additional instance allowed while own blank window remains"
}
if($fatal){throw $fatal}
