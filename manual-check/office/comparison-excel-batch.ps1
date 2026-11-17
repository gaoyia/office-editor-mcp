param([Parameter(Mandatory=$true)][string]$Plan)
$ErrorActionPreference='Stop'
$items=Get-Content -LiteralPath $Plan -Encoding UTF8 -Raw|ConvertFrom-Json
$completed=@()
try{
 foreach($item in $items){
  Write-Host ('OWN RANGE '+$item.Source+' '+$item.Range)
  & (Join-Path $PSScriptRoot 'comparison-oracle.ps1') -Source $item.Source -Destination $item.Destination -Range $item.Range -KeepApplication
  $record=Get-Content (Join-Path $item.Destination 'office-export.json') -Encoding UTF8 -Raw|ConvertFrom-Json
  $completed+=$record
  $completed|ConvertTo-Json -Depth 7|Set-Content (Join-Path (Split-Path -Parent $Plan) 'excel-batch-progress.json') -Encoding UTF8
  if($record.status -ne 'RAW_EXPORT_CLEANUP_VERIFIED_NO_MATCHING_RECOVERY_LOG'){throw 'Stop on own workbook/export/repair verification failure'}
 }
}finally{
 if($global:OoxpExcel){[void][Runtime.InteropServices.Marshal]::ReleaseComObject($global:OoxpExcel);$global:OoxpExcel=$null}
}
