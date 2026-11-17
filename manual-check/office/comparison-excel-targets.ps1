# Exactly the existing two charts and three win/loss hosts; no new chart configuration.
param([Parameter(Mandatory=$true)][string]$Plan)
$ErrorActionPreference='Stop'
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $PSScriptRoot '..\lib\office-com.ps1'),[Text.Encoding]::UTF8)))
function Write-Log($s){Write-Host $s}
function Drop($o){if($null -ne $o -and [Runtime.InteropServices.Marshal]::IsComObject($o)){[void][Runtime.InteropServices.Marshal]::ReleaseComObject($o)}}
Add-Type -AssemblyName System.Drawing;Add-Type -AssemblyName System.Windows.Forms
if(-not ('Pr10ExcelCapture' -as [type])){Add-Type -TypeDefinition 'using System;using System.Runtime.InteropServices;public static class Pr10ExcelCapture{[DllImport("user32.dll")] public static extern uint GetClipboardSequenceNumber();[DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);}'}
$items=Get-Content -LiteralPath $Plan -Encoding UTF8 -Raw|ConvertFrom-Json
$groups=@($items|Group-Object Source|Where-Object{@($_.Group|Where-Object{-not (Test-Path -LiteralPath $_.Certificate)}).Count -gt 0});if($groups.Count -eq 0){Write-Host 'All targets already have terminal certificates; not reopening Office';return};$pre=@{};$logs=@{}
foreach($g in $groups){$h=[IO.File]::Open($g.Name,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::None);$h.Dispose();$pre[$g.Name]=(Get-FileHash -LiteralPath $g.Name -Algorithm SHA256).Hash}
Get-ChildItem $env:TEMP -Filter 'error*.xml' -File -ErrorAction SilentlyContinue|ForEach-Object{$logs[$_.FullName]=$_.LastWriteTimeUtc.Ticks}
$app=$null;$book=$null;$sheet=$null;$window=$null;$charts=$null;$done=@()
try{
 $app=Ensure-MsExcel $groups[0].Name
 foreach($group in $groups){
  $source=$group.Name;$certs=@();$book=$null;$sheet=$null;$window=$null;$charts=$null
  Write-Host ('OPEN OWN EXCEL '+$source)
  try{
   $book=Open-ExcelBook $app $source;$book.Activate();$sheet=$book.Worksheets.Item(1);$sheet.Activate();$window=$app.ActiveWindow;$window.Zoom=100;$window.DisplayGridlines=$false
   foreach($item in $group.Group){
    if(Test-Path -LiteralPath $item.Certificate){continue}
    $c=[ordered]@{platform='excel';source=$source;sourceShaBefore=$pre[$source];id=$item.Id;action=$item.Action;output=$item.Output;status='NOT_EXPORTED';saveCalled=$false;quitCalled=$false;excelWindowDpi=[Pr10ExcelCapture]::GetDpiForWindow([IntPtr]$app.Hwnd)}
    $range=$null;$img=$null;$chartObject=$null;$chart=$null;$seriesCollection=$null;$sparkGroups=$null;$spark=$null
    try{
     if($pre[$source].ToLowerInvariant() -ne $item.SourceSha){throw 'Source SHA mismatch before export'}
     if($item.Action -eq 'exportSparklineImage'){
      $range=$sheet.Range($item.Range);$c.range=$item.Range;$c.rangeLeftPt=[double]$range.Left;$c.rangeTopPt=[double]$range.Top;$c.rangeWidthPt=[double]$range.Width;$c.rangeHeightPt=[double]$range.Height
      $sparkGroups=$range.SparklineGroups;if($sparkGroups.Count -ne 1){throw 'Expected one actual spark group at explicit XML host'};$spark=$sparkGroups.Item(1);$c.sparkType=[int]$spark.Type;$c.sparkSource=[string]$spark.SourceData;$c.sparkHostFromXml=$item.Range;$c.xmlGroupOrdinal=$item.Index
      $seq=[Pr10ExcelCapture]::GetClipboardSequenceNumber();$range.CopyPicture(1,2);Start-Sleep -Milliseconds 200;$img=[Windows.Forms.Clipboard]::GetImage();$afterSeq=[Pr10ExcelCapture]::GetClipboardSequenceNumber();$c.clipboardSequenceBefore=$seq;$c.clipboardSequenceAfter=$afterSeq
      if($null -eq $img -or $seq -eq $afterSeq){throw 'Reject stale clipboard'};$img.Save($item.Output,[Drawing.Imaging.ImageFormat]::Png);$c.exportKind='EXCEL_SAME_SPARK_HOST_COPYPICTURE_SCREEN';$c.apiCanvasNote='Range CopyPicture includes native outer seam; standalone library spark uses drawable host rectangle. Raw is not cropped.'
     }else{
      if(-not $charts){$charts=$sheet.ChartObjects()};$chartObject=$charts.Item([int]$item.Index);$chart=$chartObject.Chart
      $c.chartObjectName=[string]$chartObject.Name;$c.chartLeftPt=[double]$chartObject.Left;$c.chartTopPt=[double]$chartObject.Top;$c.chartWidthPt=[double]$chartObject.Width;$c.chartHeightPt=[double]$chartObject.Height;$c.cacheFormulaActualCells=$item.CacheFormulaCells
      $seriesCollection=$chart.SeriesCollection();$seriesRows=@()
      for($si=1;$si -le $seriesCollection.Count;$si++){$s=$seriesCollection.Item($si);try{$seriesRows+=@([pscustomobject]@{name=[string]$s.Name;values=@($s.Values);categories=@($s.XValues)})}finally{Drop $s}}
      $c.series=$seriesRows;$ok=$chart.Export($item.Output,'PNG',$false);if(-not $ok){throw 'Chart.Export returned false'};$c.exportKind='EXCEL_CHART_EXPORT_NATIVE_SCREEN'
     }
     $picture=[Drawing.Image]::FromFile($item.Output);try{$c.width=$picture.Width;$c.height=$picture.Height}finally{$picture.Dispose()}
     $c.outputSha256=(Get-FileHash -LiteralPath $item.Output -Algorithm SHA256).Hash;$c.status='EXPORTED_PENDING_CLOSE'
    }catch{$c.status='FAILED_PUBLIC_EXPORT';$c.error=$_.Exception.Message;$c.hresult=$_.Exception.HResult}
    finally{if($img){$img.Dispose()};foreach($o in @($spark,$sparkGroups,$range,$seriesCollection,$chart,$chartObject)){Drop $o}}
    $certs+=@([pscustomobject]@{Record=$c;Path=$item.Certificate})
    if($c.status -ne 'EXPORTED_PENDING_CLOSE'){break}
   }
  }finally{
   $closeError=$null;if($book){try{$book.Close($false)}catch{$closeError=$_.Exception.Message}}
   foreach($o in @($charts,$window,$sheet,$book)){Drop $o};$book=$null
   $exclusive=$false;try{$h=[IO.File]::Open($source,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::None);$h.Dispose();$exclusive=$true}catch{}
   $shaAfter=(Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash;$recovery=@()
   Get-ChildItem $env:TEMP -Filter 'error*.xml' -File -ErrorAction SilentlyContinue|Where-Object{(-not $logs.ContainsKey($_.FullName)) -or $logs[$_.FullName] -ne $_.LastWriteTimeUtc.Ticks}|ForEach-Object{if($_.Length -lt 1048576){$txt=[IO.File]::ReadAllText($_.FullName);if($txt.Contains($source)){$recovery+=@($_.FullName);Copy-Item -LiteralPath $_.FullName -Destination (Join-Path (Split-Path -Parent $Plan) ('recovery-'+$_.Name))}}}
   foreach($entry in $certs){$c=$entry.Record;$c.sourceShaAfter=$shaAfter;$c.sourceUnchanged=$shaAfter -eq $pre[$source];$c.exclusiveRead=$exclusive;$c.ownWorkbookClosed=$exclusive -and -not $closeError;$c.repairLogs=$recovery
    if($closeError){$c.status='FAILED_OWN_WORKBOOK_CLOSE';$c.closeError=$closeError}
    elseif($recovery.Count){$c.status='FAILED_OFFICE_REPAIR_NOT_GOLD'}
    elseif($c.status -eq 'EXPORTED_PENDING_CLOSE' -and $c.sourceUnchanged -and $exclusive){$c.status='CLEAN_PUBLIC_OFFICE_EXPORT'}
    $c|ConvertTo-Json -Depth 9|Set-Content -LiteralPath $entry.Path -Encoding UTF8;$done+=$c}
  }
  if(@($certs|Where-Object{$_.Record.status -ne 'CLEAN_PUBLIC_OFFICE_EXPORT'}).Count){Write-Host 'STOP Excel platform after first failure';break}
 }
}finally{Drop $app;$global:OoxpExcel=$null}
$done|ConvertTo-Json -Depth 9|Set-Content (Join-Path (Split-Path -Parent $Plan) 'excel-remaining-progress.json') -Encoding UTF8
