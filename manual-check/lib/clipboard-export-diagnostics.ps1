# Metadata only. Never clears, sets, converts or obtains clipboard payloads.
# Diagnostic reads run AFTER the original Bitmap/freshness guard has decided.
function Get-CopyPictureGuardClassification([Collections.IDictionary]$Record) {
    if (-not $Record.Contains('guardEvaluated') -or -not $Record.guardEvaluated) { return 'not_evaluated' }
    $missing = -not [bool]$Record.bitmapImageReturned
    $stale = [uint32]$Record.clipboardSequenceBefore -eq [uint32]$Record.clipboardSequenceAfter
    if ($missing -and $stale) { return 'missing_bitmap_and_stale_sequence' }
    if ($missing) { return 'missing_bitmap_fresh_sequence' }
    if ($stale) { return 'bitmap_present_stale_sequence' }
    return 'bitmap_present_fresh_sequence'
}
function Get-ClipboardExportFormatMetadata {
    $data = $null
    try {
        $data = [Windows.Forms.Clipboard]::GetDataObject()
        if ($null -eq $data) { return [pscustomobject]@{ DataObjectPresent=$false; NativeFormats=@(); AutoFormats=@() } }
        [pscustomobject]@{ DataObjectPresent=$true; NativeFormats=@($data.GetFormats($false)); AutoFormats=@($data.GetFormats($true)) }
    } finally { $data = $null }
}
function Save-CopyPictureAttemptDiagnostic {
    param([Parameter(Mandatory=$true)][Collections.IDictionary]$Record,
          [Parameter(Mandatory=$true)][string]$Path,
          [scriptblock]$ReadFormats = { Get-ClipboardExportFormatMetadata },
          [scriptblock]$ReadSequence = { [LegendLifecycleNative]::GetClipboardSequenceNumber() })
    # Refuse collision rather than overwrite a previous attempt/evidence record.
    if (Test-Path -LiteralPath $Path) { throw 'Refuse existing clipboard attempt diagnostic' }
    $Record.guardClassification = Get-CopyPictureGuardClassification $Record
    $d = [ordered]@{ Status='ATTEMPT_METADATA_ONLY_NOT_GOLD'; StartedAt=[DateTimeOffset]::Now.ToString('o');
        Zone='Asia/Shanghai'; ApartmentState=[Threading.Thread]::CurrentThread.GetApartmentState().ToString();
        SequenceBeforeFormats=$null; SequenceAfterFormats=$null; DataObjectPresent=$null;
        NativeFormats=@(); AutoFormats=@(); Error=$null; FinishedAt=$null }
    try {
        $d.SequenceBeforeFormats = & $ReadSequence
        $formats = & $ReadFormats
        $d.DataObjectPresent = $formats.DataObjectPresent
        $d.NativeFormats = @($formats.NativeFormats)
        $d.AutoFormats = @($formats.AutoFormats)
    } catch { $d.Error = $_.Exception.Message }
    finally {
        try { $d.SequenceAfterFormats = & $ReadSequence } catch { $d.SequenceReadError = $_.Exception.Message }
        $d.FinishedAt = [DateTimeOffset]::Now.ToString('o')
        $Record.postGuardDiagnostic = $d
        [IO.File]::WriteAllText($Path, ($Record | ConvertTo-Json -Depth 16), [Text.UTF8Encoding]::new($false))
    }
}

# Chart sheet collections may be reverse creation order. Resolve explicit name,
# never an index whose label is guessed separately. Caller owns returned ref.
function Get-NamedCopyPictureChart($Charts, [string]$ExpectedName) {
    $chart=$null
    try {
        $chart=$Charts.Item($ExpectedName)
        if ([string]$chart.Name -cne $ExpectedName) { throw "Expected named chart $ExpectedName but received $($chart.Name)" }
        return $chart
    } catch { if($chart){Release-Com $chart}; throw }
}

# Fixture precondition only; no geometry adjustment, export or Office mutation.
function Assert-CopyPictureChartContext([Collections.IDictionary]$Record, [Collections.IDictionary]$Expected) {
    $Record.contextVerified=$false
    if($Record.chartSheetName -cne $Expected.Name -or $Record.activeSheetName -cne $Expected.Name){throw 'Requested/active chart identity mismatch before CopyPicture'}
    if($Record.paperSize -ne $Expected.PaperSize -or $Record.orientation -ne $Expected.Orientation){throw 'Chart PageSetup mismatch before CopyPicture'}
    foreach($edge in @('Left','Right','Top','Bottom')){
        if(-not $Record.marginsPt.Contains($edge) -or [double]$Record.marginsPt[$edge] -ne [double]$Expected.MarginPt){throw "Chart $edge margin mismatch before CopyPicture"}
    }
    $Record.contextVerified=$true
}
