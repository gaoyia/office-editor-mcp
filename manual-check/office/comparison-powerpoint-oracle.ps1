# Native PowerPoint test export at source-defined 96-DPI extents. No desktop defaults.
param([Parameter(Mandatory=$true)][string]$Plan)
$ErrorActionPreference='Stop';Add-Type -AssemblyName System.Drawing
function Drop($o){if($null -ne $o -and [Runtime.InteropServices.Marshal]::IsComObject($o)){[void][Runtime.InteropServices.Marshal]::ReleaseComObject($o)}}
$items=Get-Content -LiteralPath $Plan -Encoding UTF8 -Raw|ConvertFrom-Json
$groups=@($items|Group-Object Source|Where-Object{@($_.Group|Where-Object{-not (Test-Path -LiteralPath $_.Certificate)}).Count -gt 0});if($groups.Count -eq 0){Write-Host 'All targets already have terminal certificates; not reopening Office';return};$pre=@{}
foreach($g in $groups){$h=[IO.File]::Open($g.Name,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::None);$h.Dispose();$pre[$g.Name]=(Get-FileHash -LiteralPath $g.Name -Algorithm SHA256).Hash}
$app=$null;$doc=$null;$done=@()
try{
 $app=[Activator]::CreateInstance([Type]::GetTypeFromCLSID([Guid]'91493441-5A91-11CF-8700-00AA0060263B'))
 if([string]$app.Path -notlike '*Microsoft Office*'){throw 'Native Microsoft PowerPoint required'}
 foreach($group in $groups){
  $source=$group.Name;$certs=@();$pres=$null;$doc=$null;$slides=$null;$setup=$null
  try{
   $pres=$app.Presentations;$doc=$pres.Open($source,$true,$false,$false);$slides=$doc.Slides;$setup=$doc.PageSetup
   foreach($item in $group.Group){
    if(Test-Path -LiteralPath $item.Certificate){continue}
    New-Item -ItemType Directory -Path (Split-Path -Parent $item.Certificate) -Force|Out-Null
    $c=[ordered]@{platform='powerpoint';source=$source;sourceShaBefore=$pre[$source];id=$item.Id;output=$item.Output;status='NOT_EXPORTED';targetDpi=96;saveCalled=$false;quitCalled=$false;openReadOnly=$true}
    $slide=$null
    try{
     if($pre[$source].ToLowerInvariant() -ne $item.SourceSha){throw 'Source SHA mismatch'}
     $c.apiSlideWidthPt=[double]$setup.SlideWidth;$c.apiSlideHeightPt=[double]$setup.SlideHeight
     if([Math]::Abs($c.apiSlideWidthPt*12700-[double]$item.CanvasEmu[0]) -gt 2 -or [Math]::Abs($c.apiSlideHeightPt*12700-[double]$item.CanvasEmu[1]) -gt 2){throw 'Source and COM slide extents disagree'}
     $w=[int][Math]::Round([double]$item.CanvasEmu[0]/9525);$h=[int][Math]::Round([double]$item.CanvasEmu[1]/9525)
     $slide=$slides.Item([int]$item.Index);$slide.Export($item.Output,'PNG',$w,$h);$c.width=$w;$c.height=$h;$c.exportKind='POWERPOINT_SLIDE_EXPORT_NATIVE_EXPLICIT_96_EXTENTS';$c.outputSha256=(Get-FileHash $item.Output -Algorithm SHA256).Hash;$c.status='EXPORTED_PENDING_CLOSE'
    }catch{$c.status='FAILED_PUBLIC_EXPORT';$c.error=$_.Exception.Message;$c.hresult=$_.Exception.HResult}finally{Drop $slide}
    $certs+=@([pscustomobject]@{Record=$c;Path=$item.Certificate});if($c.status -ne 'EXPORTED_PENDING_CLOSE'){break}
   }
  }finally{
   $closeError=$null;if($doc){try{$doc.Saved=$true;$doc.Close()}catch{$closeError=$_.Exception.Message}}
   foreach($o in @($slides,$setup,$doc,$pres)){Drop $o};$doc=$null
   $after=(Get-FileHash $source -Algorithm SHA256).Hash;$exclusive=$false;try{$h=[IO.File]::Open($source,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::None);$h.Dispose();$exclusive=$true}catch{}
   foreach($e in $certs){$c=$e.Record;$c.sourceShaAfter=$after;$c.sourceUnchanged=$after -eq $pre[$source];$c.exclusiveRead=$exclusive;$c.ownDocumentClosed=$exclusive -and -not $closeError
    if($closeError){$c.closeError=$closeError;$c.status='FAILED_OWN_DOCUMENT_CLOSE'}elseif($c.status -eq 'EXPORTED_PENDING_CLOSE' -and $c.sourceUnchanged -and $exclusive){$c.status='CLEAN_PUBLIC_OFFICE_EXPORT'}
    $c|ConvertTo-Json -Depth 7|Set-Content -LiteralPath $e.Path -Encoding UTF8;$done+=$c}
  }
  if(@($certs|Where-Object{$_.Record.status -ne 'CLEAN_PUBLIC_OFFICE_EXPORT'}).Count){break}
 }
}finally{Drop $app}
$done|ConvertTo-Json -Depth 7|Set-Content (Join-Path (Split-Path -Parent $Plan) 'powerpoint-progress.json') -Encoding UTF8
