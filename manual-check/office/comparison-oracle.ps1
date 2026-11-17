# Test oracle only; never Save, Quit, kill or request file repair.
param([Parameter(Mandatory=$true)][string]$Source,[Parameter(Mandatory=$true)][string]$Destination,[Parameter(Mandatory=$true)][string]$Range,[switch]$KeepApplication)
$ErrorActionPreference='Stop'
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $PSScriptRoot '..\lib\office-com.ps1'),[Text.Encoding]::UTF8)))
function Write-Log($s){Write-Host $s}
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
if(-not ('Pr10Clipboard' -as [type])){Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public static class Pr10Clipboard { [DllImport("user32.dll")] public static extern uint GetClipboardSequenceNumber(); [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h); }'}
$Source=[IO.Path]::GetFullPath($Source)
New-Item -ItemType Directory -Path $Destination -Force|Out-Null
$before=(Get-FileHash -LiteralPath $Source -Algorithm SHA256).Hash
$oldLogs=@{};Get-ChildItem $env:TEMP -Filter 'error*.xml' -File -ErrorAction SilentlyContinue|ForEach-Object{$oldLogs[$_.FullName]=$_.LastWriteTimeUtc.Ticks}
$app=$null;$wb=$null;$sheet=$null;$window=$null;$rangeObject=$null;$used=$null;$img=$null;$workbooks=$null
$record=[ordered]@{source=$Source;sourceShaBefore=$before;range=$Range;saveCalled=$false;quitCalled=$false;repairLogs=@();status='NOT_EXPORTED'}
try{
 $app=Ensure-MsExcel $Source;$wb=Open-ExcelBook $app $Source
 if([string]$wb.FullName -ne $Source){throw 'Wrong own workbook reference'}
 $wb.Activate();$sheet=$wb.Worksheets.Item(1);$sheet.Activate();$window=$app.ActiveWindow;$window.Zoom=100;$window.DisplayGridlines=$false
 $used=$sheet.UsedRange;$rangeObject=$sheet.Range($Range)
 $record.usedRange=[string]$used.Address();$record.widthPt=[double]$rangeObject.Width;$record.heightPt=[double]$rangeObject.Height
 $record.leftPt=[double]$rangeObject.Left;$record.topPt=[double]$rangeObject.Top;$record.excelWindowDpi=[Pr10Clipboard]::GetDpiForWindow([IntPtr]$app.Hwnd)
 $record.readOnly=[bool]$wb.ReadOnly;$record.zoom=[int]$window.Zoom
 $seqBefore=[Pr10Clipboard]::GetClipboardSequenceNumber();$rangeObject.CopyPicture(1,2);Start-Sleep -Milliseconds 250
 $img=[Windows.Forms.Clipboard]::GetImage();$seqAfter=[Pr10Clipboard]::GetClipboardSequenceNumber()
 $record.clipboardSequenceBefore=$seqBefore;$record.clipboardSequenceAfter=$seqAfter
 if($null -eq $img -or $seqAfter -eq $seqBefore){throw 'Clipboard did not receive a fresh image; refuse stale gold'}
 $record.width=$img.Width;$record.height=$img.Height;$record.output=Join-Path $Destination 'office-screen-raw.png'
 $img.Save($record.output,[Drawing.Imaging.ImageFormat]::Png);$record.outputSha256=(Get-FileHash $record.output -Algorithm SHA256).Hash
 $record.status='EXPORTED_PENDING_RECOVERY_CHECK'
}finally{
 if($img){$img.Dispose()}
 $closeFailed=$false
 if($wb){try{$wb.Close($false)}catch{$closeFailed=$true;$record.closeError=$_.Exception.Message;$record.closeHResult=$_.Exception.HResult}}
 foreach($x in @($used,$rangeObject,$window,$sheet,$wb)){if($null -ne $x){Release-Com $x}}
 $remaining=$null
 if($app -and -not $closeFailed){$remaining=0;$workbooks=$app.Workbooks;foreach($owned in @($workbooks)){try{if([string]$owned.FullName -eq $Source){$remaining++}}finally{Release-Com $owned}};Release-Com $workbooks;if(-not $KeepApplication){Release-Com $app;$global:OoxpExcel=$null}}
 $record.ownWorkbookCount=$remaining;$record.sourceShaAfter=(Get-FileHash -LiteralPath $Source -Algorithm SHA256).Hash;$record.sourceUnchanged=$record.sourceShaBefore -eq $record.sourceShaAfter
 $lock=Join-Path ([IO.Path]::GetDirectoryName($Source)) ('~$'+[IO.Path]::GetFileName($Source));$record.ownerLockPresent=Test-Path -LiteralPath $lock
 $record.exclusiveRead=$false
 try{$handle=[IO.File]::Open($Source,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::None);$handle.Dispose();$record.exclusiveRead=$true}catch{$record.exclusiveReadError=$_.Exception.Message}
 Get-ChildItem $env:TEMP -Filter 'error*.xml' -File -ErrorAction SilentlyContinue|Where-Object{(-not $oldLogs.ContainsKey($_.FullName)) -or $oldLogs[$_.FullName] -ne $_.LastWriteTimeUtc.Ticks}|ForEach-Object{if($_.Length -lt 1048576){$text=[IO.File]::ReadAllText($_.FullName);if($text.Contains($Source)){$record.repairLogs+=@($_.FullName);Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $Destination $_.Name)}}}
 if($record.repairLogs.Count -gt 0){$record.status='FAIL_OFFICE_REPAIR_IMAGE_NOT_GOLD'}
 elseif($record.status -eq 'EXPORTED_PENDING_RECOVERY_CHECK' -and $remaining -eq 0 -and $record.sourceUnchanged -and -not $record.ownerLockPresent -and $record.exclusiveRead){$record.status='RAW_EXPORT_CLEANUP_VERIFIED_NO_MATCHING_RECOVERY_LOG'}
 $record|ConvertTo-Json -Depth 6|Set-Content (Join-Path $Destination 'office-export.json') -Encoding UTF8
}
if($closeFailed){throw 'Own workbook Close failed; stop Office interaction and preserve close error'}
if($record.repairLogs.Count -gt 0){throw 'Office repaired own source; stop and preserve log; image is not gold'}
