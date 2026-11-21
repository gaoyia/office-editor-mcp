# Native PNG validation only. No clipboard access, rasterization or output replacement.
function Assert-NewNativeChartOutput([string]$Path) {
 if(Test-Path -LiteralPath $Path){throw 'Refuse existing native output'}
}
function Read-NativeChartPng([string]$Path,[bool]$ExportReturned) {
 if(-not $ExportReturned){throw 'Chart.Export returned false'}
 $bytes=[IO.File]::ReadAllBytes($Path)
 if($bytes.Length -lt 33 -or [BitConverter]::ToString($bytes,0,8) -ne '89-50-4E-47-0D-0A-1A-0A' -or [Text.Encoding]::ASCII.GetString($bytes,12,4) -ne 'IHDR'){throw 'Invalid native PNG signature/header'}
 $w=([uint32]$bytes[16]*16777216)+([uint32]$bytes[17]*65536)+([uint32]$bytes[18]*256)+$bytes[19]
 $h=([uint32]$bytes[20]*16777216)+([uint32]$bytes[21]*65536)+([uint32]$bytes[22]*256)+$bytes[23]
 if($w -lt 1 -or $h -lt 1 -or $w -gt 4096 -or $h -gt 4096){throw 'Native PNG dimensions outside resource bound'}
 $image=$null;try{$image=[Drawing.Image]::FromFile($Path);if($image.Width -ne $w -or $image.Height -ne $h){throw 'PNG header/decode dimension mismatch'}}finally{if($image){$image.Dispose()}}
 [pscustomobject]@{Path=$Path;Width=$w;Height=$h;Bytes=$bytes.Length;Sha256=(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash}
}
