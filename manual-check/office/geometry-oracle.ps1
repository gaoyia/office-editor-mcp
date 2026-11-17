param([Parameter(Mandatory=$true)][string]$EvidenceDir,[int]$ExistingExcelProcessId=0)
$ErrorActionPreference='Stop'
$root=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $root 'manual-check/lib/office-com.ps1'))))
function Write-Log($s) { Write-Host $s }
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$cases=Import-Csv (Join-Path $EvidenceDir 'before.csv') | Where-Object action -EQ 'sheet'
$seed=Join-Path $EvidenceDir ($cases[0].case+'-sheet.xlsx')
if($ExistingExcelProcessId -gt 0){$excel=Bind-OfficeApp (Get-Process -Id $ExistingExcelProcessId) 'XLMAIN' 'EXCEL7'}else{$excel=Ensure-MsExcel $seed}
# Ensure-MsExcel already opens this own seed. Open-ExcelBook reuses that held
# workbook on the first iteration: never close/reopen the bootstrap input.
$records=@()
$semantic=@()
$validated=[IO.File]::ReadAllText((Join-Path $EvidenceDir 'before-fixture-validation.json'))|ConvertFrom-Json
foreach($case in $cases) {
 $name=$case.case
 foreach($kind in @('sheet','chart')) {
  $path=Join-Path $EvidenceDir ($name+'-'+$kind+'.xlsx')
  $before=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
  $wb=Open-ExcelBook $excel $path;$ws=$wb.Worksheets.Item(1)
  try {
   $normal=$null;$normalFont=$null;$sample=$null;$cellFont=$null
   try {
    $styleName=$validated.PSObject.Properties[($name+'-'+$kind+'.xlsx')].Value.normalName
    try{$normal=$wb.Styles.Item($styleName)}catch{$normal=$wb.Styles.Item('Normal')}
    $normalFont=$normal.Font
    $sample=$ws.Range(($case.range -split ':')[0]+'');$cellFont=$sample.Font
    $semantic+=[pscustomobject]@{case=$name;kind=$kind;fullName=[string]$wb.FullName;readOnly=[bool]$wb.ReadOnly;normalName=[string]$normal.Name;normalFont=[string]$normalFont.Name;normalPt=[double]$normalFont.Size;cellFont=[string]$cellFont.Name;cellPt=[double]$cellFont.Size;seriesValues='';categories='';seriesFormula=''}
   } finally {foreach($ownRef in @($cellFont,$sample,$normalFont,$normal)){if($null -ne $ownRef){Release-Com $ownRef}}}
   $wb.Activate();$ws.Activate();$win=$excel.ActiveWindow;$win.Zoom=100;$win.DisplayGridlines=$false
   if($kind -eq 'sheet') {
    $range=$ws.Range($case.range);$cols=@();foreach($c in $range.Columns){$cols+=[pscustomobject]@{pt=[double]$c.Width;columnWidth=[double]$c.ColumnWidth;hidden=[bool]$c.Hidden};Release-Com $c}
    foreach($type in @('sheet','spark')) {
     if($type -eq 'sheet'){$r=$range}else{$r=$ws.Range(($case.range -split ':')[0])}
     for($attempt=0;$attempt -lt 3;$attempt++) {
 try{[void]$r.CopyPicture(1,2);break}catch{if($attempt -eq 2){throw};Start-Sleep -Milliseconds 200}
}
Start-Sleep -Milliseconds 200
     $img=[Windows.Forms.Clipboard]::GetImage();if($null -eq $img){throw 'Clipboard has no image'}
     try {
      $raw=Join-Path $EvidenceDir ($name+'-'+$type+'-gold-raw.png');$img.Save($raw,[Drawing.Imaging.ImageFormat]::Png)
      $w=$img.Width;$h=$img.Height
      if($type -eq 'spark') {
       $w=$w-1;$h=$h-1;$rect=[Drawing.Rectangle]::new(0,0,$w,$h);$bmp=([Drawing.Bitmap]$img).Clone($rect,$img.PixelFormat)
       try{$bmp.Save((Join-Path $EvidenceDir ($name+'-'+$type+'-gold.png')),[Drawing.Imaging.ImageFormat]::Png)}finally{$bmp.Dispose()}
      }
      $records+=[pscustomobject]@{case=$name;action=$type;range=if($type -eq 'sheet'){$case.range}else{($case.range -split ':')[0]};goldWidth=$w;goldHeight=$h;rawWidth=$img.Width;rawHeight=$img.Height;rangeWidthPt=[double]$r.Width;rangeHeightPt=[double]$r.Height;columns=($cols|ConvertTo-Json -Compress);oracle='Excel16 CopyPicture xlScreen xlBitmap; spark uses existing outer-1 crop, raw retained';error=''}
     }finally{$img.Dispose();if($type -eq 'spark'){Release-Com $r}}
    }
    Release-Com $range
   }else {
    $charts=$ws.ChartObjects();$obj=$charts.Item(1);$chart=$obj.Chart
    $seriesList=$chart.SeriesCollection();$series=$seriesList.Item(1)
    try {
     $values=@($series.Values);$categories=@($series.XValues)
     if($values.Count -ne 3 -or [double]$values[0] -ne 1 -or [double]$values[1] -ne 4 -or [double]$values[2] -ne 2){throw 'Oracle actual chart values differ from validated 1/4/2'}
     $semantic[-1].seriesValues=ConvertTo-Json -InputObject $values -Compress
     $semantic[-1].categories=ConvertTo-Json -InputObject $categories -Compress
     $semantic[-1].seriesFormula=[string]$series.Formula
    }finally{Release-Com $series;Release-Com $seriesList}
    $out=Join-Path $EvidenceDir ($name+'-chart-gold.png');[void]$chart.Export($out,'PNG');$im=[Drawing.Image]::FromFile($out)
    try{$records+=[pscustomobject]@{case=$name;action='chart';range='twoCell';goldWidth=$im.Width;goldHeight=$im.Height;rawWidth=$im.Width;rawHeight=$im.Height;rangeWidthPt=[double]$obj.Width;rangeHeightPt=[double]$obj.Height;columns='';oracle='Excel16 Chart.Export PNG';error=''}}finally{$im.Dispose();Release-Com $chart;Release-Com $obj;Release-Com $charts}
   }
   Release-Com $win
  }catch{$records+=[pscustomobject]@{case=$name;action=if($kind -eq 'sheet'){$type}else{'chart'};range=$case.range;goldWidth=0;goldHeight=0;rawWidth=0;rawHeight=0;rangeWidthPt=0;rangeHeightPt=0;columns='';oracle='failed';error=$_.Exception.Message};Write-Host "ORACLE ERROR $name $kind $_";throw}
  finally{$wb.Close($false);Release-Com $ws;Release-Com $wb}
  if((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $before){throw "Oracle modified input workbook: $path"}
 }
}
if(@($records|Where-Object error -NE '').Count -gt 0){$records|Export-Csv (Join-Path $EvidenceDir 'oracle-errors.csv') -NoTypeInformation -Encoding UTF8;throw 'Oracle export error: stop rather than reopen/retry workbooks'}
$semantic|Export-Csv (Join-Path $EvidenceDir 'oracle-semantics.csv') -NoTypeInformation -Encoding UTF8
$ownRemaining=@();foreach($held in @($excel.Workbooks)){try{if(([string]$held.FullName).StartsWith([IO.Path]::GetFullPath($EvidenceDir),[StringComparison]::OrdinalIgnoreCase)){$ownRemaining+=[string]$held.FullName}}finally{Release-Com $held}}
@{remainingOwnWorkbooks=$ownRemaining;count=$ownRemaining.Count;officeQuitCalled=$false}|ConvertTo-Json|Set-Content (Join-Path $EvidenceDir 'oracle-final-state.json') -Encoding UTF8
if($ownRemaining.Count -ne 0){throw 'Own test workbook remains open'}
$records|Export-Csv (Join-Path $EvidenceDir 'oracle.csv') -NoTypeInformation -Encoding UTF8
Write-Host ('ORACLE records='+$records.Count+' errors='+@($records|Where-Object error -NE '').Count)
# No Quit: never close any unrelated Office process or document.